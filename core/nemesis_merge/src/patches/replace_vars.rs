//! # Nemesis variables
//! - Nemesis abstracts the index of the eventName inserted in `eventNames`(or `variableNames`) of `hkbBehaviorGraphStringData` and makes it a variable since it may come here.
//! 1. in the creation stage of this patch, it is necessary to replace eventName with index, so it is reserved as a string here. 2.
//! 2. replace them in the merge stage of patch.

use json_patch::ptr_mut::PointerMut;
use simd_json::{borrowed::Value, derived::ValueTryAsArray, StaticNode};

#[allow(unused)]
#[allow(clippy::unwrap_used)]
fn replace_var(template: &mut Value<'_>) {
    let var_array = template
        .ptr_mut(&["106", "hkbBehaviorGraphStringData", "variableNames"])
        .unwrap();
    let var_array = var_array.try_as_array().unwrap();

    let (index, _) = var_array
        .iter()
        .enumerate()
        .find(|(_, item)| **item == Value::String("SpeedWalk".into()))
        .unwrap();

    let target = template
        .ptr_mut(&[
            "#sample$1",
            "hkbVariableBindingSet",
            "bindings",
            "[0]",
            "variableIndex",
        ])
        .unwrap();
    *target = Value::Static(StaticNode::U64(index as u64));
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
        replace_var(&mut template);

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
