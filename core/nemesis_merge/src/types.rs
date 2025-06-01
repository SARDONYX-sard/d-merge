use dashmap::DashMap;
use indexmap::IndexMap;
use json_patch::{JsonPath, Patch, ValueWithPriority};
use rayon::prelude::*;
use simd_json::BorrowedValue;
use std::{collections::HashMap, path::PathBuf};

/// - key: template file stem(e.g. `0_master`)
/// - value: output_path(hkx file path), borrowed json (from template xml)
pub type BorrowedTemplateMap<'a> = DashMap<&'a str, (PathBuf, BorrowedValue<'a>)>;

/// - key: full path
/// - value: nemesis xml
pub type OwnedPatchMap = IndexMap<PathBuf, (String, usize)>;

/// - key: full path(For adsf)
/// - value: adsf patch
#[derive(Debug, Default)]
pub struct OwnedAdsfPatchMap(pub IndexMap<PathBuf, (String, usize)>);
impl OwnedAdsfPatchMap {
    #[inline]
    pub fn new() -> Self {
        Self(IndexMap::new())
    }

    #[inline]
    pub fn insert(&mut self, path: PathBuf, patch: (String, usize)) -> Option<(String, usize)> {
        self.0.insert(path, patch)
    }
}

/// - key: template name (e.g., `"0_master"`, `"defaultmale"`)
/// - value: Map<jsonPath, (patch, priority)>
///
/// # Intended
/// ```txt
/// - "0_master": {
///      ["#0001", "hkbProjectData", "variable"]: OneFIeld { patch, priority },
///      ["#0001", "hkbProjectData", "variableNames", "[0:10]"]: Seq {
///              [(patch, priority), (patch, priority)]
///          }
///   }
/// - "_1stperson/0_master": {
///      ["#0001", "hkbProjectData", "variable"]: { patch, priority }
///   }
/// ```
pub type PatchMapForEachTemplate<'a> = DashMap<&'a str, PatchMap<'a>>;

/// - key: json path. e.g. `["#0001", "hkbProjectData", "[0:10]"]`
/// - value: `Map<jsonPath, { patch, priority }>`
#[derive(Debug, Clone)]
pub struct PatchMap<'a>(pub DashMap<JsonPath<'a>, Patch<'a>>);

impl<'a> PatchMap<'a> {
    #[inline]
    pub fn new() -> Self {
        Self(DashMap::new())
    }

    pub fn insert(
        &self,
        key: JsonPath<'a>,
        new_value: ValueWithPriority<'a>,
        kind: PatchKind,
    ) -> Result<(), TypeError> {
        #[cfg(feature = "tracing")]
        let cloned_key = key.clone();

        match self.0.entry(key) {
            dashmap::Entry::Occupied(mut existing) => {
                let old_patch = existing.get_mut();
                match old_patch {
                    Patch::One(old_value) => match kind {
                        PatchKind::OneField => {
                            if new_value.priority > old_value.priority {
                                tracing::info!(
                                "One Patch conflict at {cloned_key:?}: (priority {}) is overwritten by (priority {})",
                                old_value.priority,
                                new_value.priority
                                );
                                *old_value = new_value;
                            }
                        }
                        PatchKind::Seq => {
                            return Err(TypeError {
                                actual: PatchKind::OneField,
                                expected: PatchKind::Seq,
                            })
                        }
                    },
                    Patch::Seq(old_items) => {
                        match kind {
                            PatchKind::Seq => old_items.push(new_value),
                            PatchKind::OneField => {
                                return Err(TypeError {
                                    actual: PatchKind::Seq,
                                    expected: PatchKind::OneField,
                                })
                            }
                        };
                    }
                }
            }
            dashmap::Entry::Vacant(ve) => match kind {
                PatchKind::OneField => {
                    ve.insert(Patch::One(new_value));
                }
                PatchKind::Seq => {
                    ve.insert(Patch::Seq(vec![new_value]));
                }
            },
        }

        Ok(())
    }

    pub fn extend<I>(&self, key: JsonPath<'a>, new_values: I) -> Result<(), TypeError>
    where
        I: IntoParallelIterator<Item = ValueWithPriority<'a>>,
    {
        match self.0.entry(key) {
            dashmap::Entry::Occupied(mut existing) => {
                let old_patch = existing.get_mut();
                match old_patch {
                    Patch::Seq(old_items) => {
                        old_items.par_extend(new_values);
                        Ok(())
                    }
                    Patch::One(_) => Err(TypeError {
                        actual: PatchKind::OneField,
                        expected: PatchKind::Seq,
                    }),
                }
            }
            dashmap::Entry::Vacant(ve) => {
                ve.insert(Patch::Seq(new_values.into_par_iter().collect()));
                Ok(())
            }
        }
    }
}

// NOTE: Why need this? -> json fails to serde because Map keys cannot be an array. To prevent this, convert only the keys as String.
#[cfg(feature = "serde")]
impl<'a> serde::Serialize for PatchMap<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap as _;

        let map = serializer.serialize_map(Some(self.0.len()))?;
        let mut map = map;

        for item in self.0.iter() {
            // JsonPath -> "/a/b/c"
            let joined = item.key().join("/");
            map.serialize_entry(&joined, item.value())?;
        }

        map.end()
    }
}
#[cfg(feature = "serde")]
impl<'de: 'a, 'a> serde::Deserialize<'de> for PatchMap<'a> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct PatchMapVisitor<'a> {
            marker: std::marker::PhantomData<&'a ()>,
        }

        impl<'de: 'a, 'a> serde::de::Visitor<'de> for PatchMapVisitor<'a> {
            type Value = PatchMap<'a>;

            fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
                formatter.write_str("a map with slash-separated json paths as keys")
            }

            fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
            where
                M: serde::de::MapAccess<'de>,
            {
                let map = DashMap::new();

                while let Some((key, value)) = access.next_entry::<String, Patch<'de>>()? {
                    let json_path: Vec<std::borrow::Cow<'a, str>> = key
                        .split('/')
                        .map(|s| std::borrow::Cow::Owned(s.to_string()))
                        .collect();
                    map.insert(json_path, value);
                }

                Ok(PatchMap(map))
            }
        }

        deserializer.deserialize_map(PatchMapVisitor {
            marker: std::marker::PhantomData,
        })
    }
}

#[derive(Debug)]
pub enum PatchKind {
    OneField,
    Seq,
}

#[derive(Debug)]
pub struct TypeError {
    actual: PatchKind,
    expected: PatchKind,
}

impl core::fmt::Display for TypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Expected PatchMap Insert type: {:?}. But got {:?}",
            self.expected, self.actual
        )
    }
}

impl std::error::Error for TypeError {}

/// HashMap showing which index (e.g. `#0000`) of each template (e.g. `0_master.xml`)
/// contains `hkbBehaviorGraphStringData
///
/// This information exists because it is needed to replace variables such as the Nemesis variable `$variableID[]$`.
#[derive(Debug, Default, Clone)]
pub struct VariableClassMap<'a>(pub DashMap<&'a str, &'a str>);
impl VariableClassMap<'_> {
    pub fn new() -> Self {
        Self(DashMap::new())
    }
}

/// - key: template_name(e.g. `0_master`, `_1stperson/0_master`)
/// - value: priority
pub type PriorityMap<'a> = HashMap<&'a str, usize>;
