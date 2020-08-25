use furiosa_client::{FuriosaClient, SourceFormat, TargetFormat};

#[test]
fn test_compile() {
    env_logger::init();
    let client = FuriosaClient::new().unwrap();
    let result = client.compile_from_file(
        SourceFormat::Tflite,
        TargetFormat::Enf,
        "models/tflite/MNISTnet_uint8_quant.tflite",
    );
    assert!(result.is_ok(), "fail to compile");
}
