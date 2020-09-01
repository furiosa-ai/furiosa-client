# Furiosa Web Service API Client
This is a client of Furiosa Web service API.

# Example

To run the following example, you should get an API key and set the key according to [the instruction](https://github.com/furiosa-ai/furiosa-client#how-to-set-api-keys).

```rust
use furiosa_client::{FuriosaClient, SourceFormat, TargetFormat};

let target_npu_spec: serde:json::Value = serde_yaml::from_str(include_str!("../configs/64dpes.yml")).unwrap();
let compiler_config: Value = serde_json::from_str("{}").unwrap();

let client = FuriosaClient::new().unwrap();
let binary = std::fs::read("models/tflite/MNISTnet_uint8_quant.tflite").expect("fail to read");
let request = CompileRequest::new(target_npu_spec, binary).compile_config(compiler_config);
let result: Box<[u8]> = client.compile(request).unwrap();
```

Please see a full example at the [integration tests](https://github.com/furiosa-ai/furiosa-client/blob/master/tests/integration_test.rs).

# Building

The library embeds the API endpoint depending on a specified cargo feature. 
By default, `cargo build` will embed the sandbox API endpoint for testing and development.

If you build with the feature 'local_api', it will embed the `http://localhost:8080/api/v1` for the API endpoint.
It will be useful for debugging.

To build the library and executable files using local API endpoint:
```sh
cargo build --features "local_api"
```

To build the library and executable files using production API endpoint:
```sh
cargo build --release --features "production_api"
```

Both require API keys. 
Please watch [this video](https://drive.google.com/file/d/1DLj4i6SEvGeq5eDnemTK15Trajamc8LW/view?usp=sharing) 
in order to learn how to generate API keys.

# How to set API keys

## Shell environment variables
Please set the two environment variables as follow and then run your program:
```sh
export FURIOSA_ACCESS_KEY_ID=XXXXXXXXXXXXXXXXXXXXXXXXXXXXX
export FURIOSA_SECRET_ACCESS_KEY=YYYYYYYYYYYYYYYYYYYYYYYYYY
``` 

## Credential file
Please put your API keys at `$HOME/.furiosa/credential` as follow:
```sh
FURIOSA_ACCESS_KEY_ID=XXXXXXXXXXXXXXXXXXXXXXXXXXXXX
FURIOSA_SECRET_ACCESS_KEY=YYYYYYYYYYYYYYYYYYYYYYYYYY
```
