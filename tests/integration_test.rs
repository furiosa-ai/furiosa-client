use furiosa_client::{
    get_endpoint_from_env, CompileRequest, FuriosaClient, TargetIr, FURIOSA_API_ENDPOINT_ENV,
};
use serde_json::Value;

#[test]
fn test_get_endpoint_from_env() {
    let origin_endpoint = get_endpoint_from_env();

    std::env::set_var(FURIOSA_API_ENDPOINT_ENV, "https://test.api/api/v1/////");
    assert_eq!("https://test.api/api/v1", &get_endpoint_from_env());

    std::env::set_var(FURIOSA_API_ENDPOINT_ENV, "https://test.api/api/v1");
    assert_eq!("https://test.api/api/v1", &get_endpoint_from_env());

    std::env::set_var(FURIOSA_API_ENDPOINT_ENV, origin_endpoint);
}

#[tokio::test]
#[ignore]
async fn test_compile_with_default() {
    env_logger::init();

    let target_npu_spec: Value =
        serde_yaml::from_str(include_str!("../configs/64dpes.yml")).unwrap();
    let compiler_config: Value = serde_json::from_str("{}").unwrap();

    let client = FuriosaClient::new().unwrap();
    let binary = std::fs::read("models/tflite/MNISTnet_uint8_quant.tflite").expect("fail to read");
    let request = CompileRequest::new(target_npu_spec, binary).compile_config(compiler_config);

    let result = client.compile(request).await;
    assert!(result.is_ok(), format!("{:?}", result.err()));
}

#[tokio::test]
#[ignore]
async fn test_compile_with_target_ir() {
    env_logger::init();

    let target_npu_spec: Value =
        serde_yaml::from_str(include_str!("../configs/64dpes.yml")).unwrap();
    let compiler_config: Value = serde_json::from_str("{}").unwrap();

    let client = FuriosaClient::new().unwrap();
    let binary = std::fs::read("models/tflite/MNISTnet_uint8_quant.tflite").expect("fail to read");
    let request = CompileRequest::new(target_npu_spec, binary)
        .compile_config(compiler_config)
        .target_ir(TargetIr::Lir);

    let result = client.compile(request).await;
    assert!(result.is_ok(), format!("{:?}", result.err()));
}
