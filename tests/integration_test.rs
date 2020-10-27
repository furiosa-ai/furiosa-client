use furiosa_client::{get_endpoint_from_env, ClientError, CompileRequest, FuriosaClient, TargetIr, FURIOSA_API_ENDPOINT_ENV, CalibrateRequest, QuantizeRequest};
use serde_json::Value;
use std::io;

#[test]
fn test_get_endpoint_from_env() -> Result<(), ClientError> {
    let origin_endpoint = get_endpoint_from_env()?;

    std::env::set_var(FURIOSA_API_ENDPOINT_ENV, "https://test.api/api/v1/////");
    assert_eq!("https://test.api/api/v1", &get_endpoint_from_env()?);

    std::env::set_var(FURIOSA_API_ENDPOINT_ENV, "https://test.api/api/v1");
    assert_eq!("https://test.api/api/v1", &get_endpoint_from_env()?);

    std::env::set_var(FURIOSA_API_ENDPOINT_ENV, origin_endpoint);
    Ok(())
}

#[cfg(feature = "blocking")]
#[test]
#[ignore]
fn test_blocking_compile_with_default() {
    use furiosa_client::blocking;

    env_logger::init();

    let target_npu_spec: Value =
        serde_yaml::from_str(include_str!("../configs/64dpes.yml")).unwrap();
    let compiler_config: Value = serde_json::from_str("{}").unwrap();

    let client = blocking::FuriosaClient::new().unwrap();
    let binary = std::fs::read("models/tflite/MNISTnet_uint8_quant.tflite").expect("fail to read");
    let request = CompileRequest::new(target_npu_spec, binary).compile_config(compiler_config);

    let result = client.compile(request);
    assert!(result.is_ok(), format!("{:?}", result.err()));
}

#[tokio::test]
#[ignore]
async fn test_compile_with_default() {
    env_logger::init();

    let target_npu_spec: Value =
        serde_yaml::from_str(include_str!("../configs/64dpes.yml")).unwrap();
    let compiler_config: Value = serde_json::from_str("{}").unwrap();

    let client = FuriosaClient::new().unwrap();
    let binary =
        tokio::fs::read("models/tflite/MNISTnet_uint8_quant.tflite").await.expect("fail to read");
    let request = CompileRequest::new(target_npu_spec, &binary).compile_config(compiler_config);

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
    let binary =
        tokio::fs::read("models/tflite/MNISTnet_uint8_quant.tflite").await.expect("fail to read");
    let request = CompileRequest::new(target_npu_spec, binary)
        .compile_config(compiler_config)
        .target_ir(TargetIr::Lir);

    let result = client.compile(request).await;
    assert!(result.is_ok(), format!("{:?}", result.err()));
}

#[tokio::test]
#[ignore]
async fn test_build_calibration_model() -> io::Result<()> {
    env_logger::init();

    let orig_model = tokio::fs::read("models/quantization/test.onnx").await?;

    let calibration_req = CalibrateRequest {
        source: orig_model,
        filename: "test.onnx".to_string(),
        input_tensors: vec!["input".to_string()]
    };

    let client = FuriosaClient::new().unwrap();
    let result = client.calibrate(calibration_req).await;
    assert!(result.is_ok(), format!("{:?}", result.err()));

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_quantize() -> io::Result<()> {
    env_logger::init();

    let orig_model = tokio::fs::read("models/quantization/test.onnx").await?;
    let dynamic_ranges = serde_json::from_str(
r#"{
  "input": [
    4.337553946243133e-06,
    0.9999983906745911
  ],
  "5": [
    -0.6236848831176758,
    1.7029087543487549
  ],
  "6": [
    0.0,
    1.7029087543487549
  ],
  "7": [
    0.0,
    1.7029087543487549
  ],
  "8": [
    -1.2079784870147705,
    1.0805176496505737
  ],
  "9": [
    0.0,
    1.0805176496505737
  ],
  "output": [
    0.0,
    1.0805176496505737
  ]
}
"#).expect("fail to parse JSON");

    let quantize_req = QuantizeRequest {
        source: orig_model,
        filename: "test.onnx".to_string(),
        input_tensors: vec!["input".to_string()],
        dynamic_ranges,
    };

    let client = FuriosaClient::new().unwrap();
    let result = client.quantize(quantize_req).await;
    assert!(result.is_ok(), format!("{:?}", result.err()));

    Ok(())
}
