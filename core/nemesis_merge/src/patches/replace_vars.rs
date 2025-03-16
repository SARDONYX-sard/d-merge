//! # Nemesis variables
//! - Nemesis abstracts the index of the eventName inserted in `eventNames`(or `variableNames`) of `hkbBehaviorGraphStringData` and makes it a variable since it may come here.
//! 1. in the creation stage of this patch, it is necessary to replace eventName with index, so it is reserved as a string here. 2.
//! 2. replace them in the merge stage of patch.

use json_patch::{json_path, ptr_mut::PointerMut, JsonPath};
use simd_json::{base::ValueTryAsArrayMut, borrowed::Value, StaticNode};

enum IdKind {
    Event,
    Variable,
}

struct NemesisVar<'xml> {
    ///  hkbBehaviorGraphStringData template index (pre create)
    index: &'xml str,

    /// SpeedWalk
    id: &'xml str,
    /// e.g. "#sample$1", "hkbVariableBindingSet", "bindings", "[0]", "variableIndex",
    id_path: JsonPath<'xml>,
    /// - e.g. $eventID[sampleEvent]$ -> Event
    /// - e.g. $variableID[sampleName]$ -> Variable
    id_kind: IdKind,
}

// FIXME: Temp
impl Default for NemesisVar<'_> {
    fn default() -> Self {
        NemesisVar {
            index: "106",
            id: "SpeedWalk",
            id_path: json_path![
                "#sample$1",
                "hkbVariableBindingSet",
                "bindings",
                "[0]",
                "variableIndex",
            ],
            id_kind: IdKind::Event,
        }
    }
}

#[allow(clippy::unwrap_used)]
fn replace_var_common_process(
    template: &mut Value<'_>,
    event_path: &[&str],
    id_path: JsonPath<'_>,
    id: &str,
) {
    let search_target_array = template
        .ptr_mut(event_path)
        .unwrap()
        .try_as_array_mut()
        .unwrap();

    let (index, _) = search_target_array
        .iter()
        .enumerate()
        .find(|(_, item)| **item == Value::String(id.into()))
        .unwrap();

    let target = template.ptr_mut(id_path).unwrap();
    *target = Value::Static(StaticNode::U64(index as u64)); // assumed cast type as u64
}

pub fn replace_var(template: &mut Value<'_>, nemesis_vars: NemesisVar<'_>) {
    let NemesisVar {
        index,
        id,
        id_path,
        id_kind,
    } = nemesis_vars;

    match id_kind {
        IdKind::Variable => {
            let var_path = &[index, "hkbBehaviorGraphStringData", "variableNames"];
            replace_var_common_process(template, var_path, id_path, id);
        }
        IdKind::Event => {
            let event_path = &[index, "hkbBehaviorGraphStringData", "eventNames"];
            replace_var_common_process(template, event_path, id_path, id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_complex_xml_replacement() {
        // To generate template
        // use serde_hkx_features::ClassMap;
        // use simd_json::serde::to_borrowed_value;
        // let template_xml = include_str!(
        //     "../../../../resource/assets/templates/meshes/actors/character/behaviors/0_master.xml"
        // );
        // let ast: ClassMap = serde_hkx::from_str(template_xml).unwrap();
        // let value = to_borrowed_value(ast).unwrap();
        // std::fs::write("../../dummy/template_types.txt", format!("{value:#?}")).unwrap();

        let mut template = simd_json::json_typed!(borrowed, {
            "106": {
                "hkbBehaviorGraphStringData": {
                    "__ptr": "#0106",
                    "memSizeAndFlags": 0,
                    "referenceCount": 0,
                    "eventNames": [
                        "FootLeft",
                        "FootRight",
                    ],
                    "variableNames": [
                      "Speed",               // 0
                      "Direction",           // 1
                      "TurnDelta",           // 2
                      "iSyncIdleLocomotion", // 3
                      "SpeedWalk",           // 4
                    ]
                }
            },
            "#sample$1": {
                "hkbVariableBindingSet": {
                    "__ptr": "#sample$1",
                    "bindings": [
                        {
                            "variableIndex": "$variableID[SpeedWalk]$", // <- replace to 4
                            "bitIndex": -1,
                            "bindingType": "BINDING_TYPE_VARIABLE"
                        }
                    ]
                }
            }
        });

        // call replacer this
        replace_var(&mut template, NemesisVar::default());

        let expected = simd_json::json_typed!(borrowed, {
            "106": {
                "hkbBehaviorGraphStringData": {
                    "__ptr": "#0106",
                    "memSizeAndFlags": 0,
                    "referenceCount": 0,
                    "eventNames": [
                        "FootLeft",
                        "FootRight",
                    ],
                    "variableNames": [
                      "Speed",               // 0
                      "Direction",           // 1
                      "TurnDelta",           // 2
                      "iSyncIdleLocomotion", // 3
                      "SpeedWalk",           // 4
                    ]
                }
            },
            "#sample$1": {
                "hkbVariableBindingSet": {
                    "__ptr": "#sample$1",
                    "bindings": [
                        {
                            "variableIndex": 4, // <- replaced to 4
                            "bitIndex": -1,
                            "bindingType": "BINDING_TYPE_VARIABLE"
                        }
                    ]
                }
            }
        });
        assert_eq!(template, expected);
    }
}
