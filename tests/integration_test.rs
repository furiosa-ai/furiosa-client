use furiosa_client::{CompileRequest, FuriosaClient, TargetFormat};
use serde_json::Value;

#[test]
fn test_compile() {
    env_logger::init();

    let target_npu_spec: Value =
        serde_yaml::from_str(include_str!("../configs/64dpes.yml")).unwrap();
    let compiler_config: Value = serde_json::from_str("{}").unwrap();

    let client = FuriosaClient::new().unwrap();
    let binary = std::fs::read("models/tflite/MNISTnet_uint8_quant.tflite").expect("fail to read");
    let request = CompileRequest::new(target_npu_spec, TargetFormat::Enf, binary)
        .compile_config(compiler_config);

    let result = client.compile(request);
    assert!(result.is_ok(), format!("{:?}", result.err()));
}
