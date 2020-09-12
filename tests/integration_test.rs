use furiosa_client::{CompileRequest, FuriosaClient, TargetIr};
use serde_json::Value;

#[test]
#[ignore]
fn test_compile_with_default() {
    env_logger::init();

    let target_npu_spec: Value =
        serde_yaml::from_str(include_str!("../configs/64dpes.yml")).unwrap();
    let compiler_config: Value = serde_json::from_str("{}").unwrap();

    let client = FuriosaClient::new().unwrap();
    let binary = std::fs::read("models/tflite/MNISTnet_uint8_quant.tflite").expect("fail to read");
    let request = CompileRequest::new(target_npu_spec, binary).compile_config(compiler_config);

    let result = client.compile(request);
    assert!(result.is_ok(), format!("{:?}", result.err()));
}

#[test]
#[ignore]
fn test_compile_with_target_ir() {
    env_logger::init();

    let target_npu_spec: Value =
        serde_yaml::from_str(include_str!("../configs/64dpes.yml")).unwrap();
    let compiler_config: Value = serde_json::from_str("{}").unwrap();

    let client = FuriosaClient::new().unwrap();
    let binary = std::fs::read("models/tflite/MNISTnet_uint8_quant.tflite").expect("fail to read");
    let request = CompileRequest::new(target_npu_spec, binary)
        .compile_config(compiler_config)
        .target_ir(TargetIr::Lir);

    let result = client.compile(request);
    assert!(result.is_ok(), format!("{:?}", result.err()));
}