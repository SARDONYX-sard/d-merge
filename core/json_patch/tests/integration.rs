use json_patch::ptr_mut::PointerMut as _;
use simd_json::borrowed::Value;

#[cfg_attr(
    feature = "tracing",
    // quick_tracing::init(file = "../../dummy/test_ptr_mut.log", stdio = false)
    quick_tracing::init(file = "./test_ptr_mut.log", stdio = false)
)]
#[test]
fn test_ptr_mut() {
    let mut bytes = include_bytes!("./test_0067.json").to_vec();
    let mut json: Value = simd_json::from_slice(&mut bytes).unwrap();

    let json_path = json_patch::json_path![
        "#0067",
        "hkbStateMachineTransitionInfoArray",
        "transitions",
        "[1]",
        // "hkbStateMachineTransitionInfo",
        "eventId"
    ];
    dbg!(json.ptr_mut(json_path));
}
