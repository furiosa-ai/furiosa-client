//! Furiosa API client
//!
//! # Authencation of API Key
//! Furiosa API client will try to read $HOME/.furiosa/credential or
//! FURIOSA_ACCESS_KEY_ID and FURIOSA_SECRET_ACCESS_KEY from environment variables.
//!
//! $HOME/.furiosa/credential file should be as follow:
//! ```sh
//! FURIOSA_ACCESS_KEY_ID=XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
//! FURIOSA_SECRET_ACCESS_KEY=YYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYY
//! ```

use std::env::VarError;
use std::io;
use std::path::PathBuf;
use std::time::Duration;

use bytes::Bytes;
use lazy_static::lazy_static;
use log::{error, info};
use reqwest::multipart::{Form, Part};
use reqwest::{RequestBuilder, Response};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub use crate::compile::{CompileRequest, TargetIr};
use crate::compile::{CompileTask, CompileTaskPhase};
pub use crate::dss::{CalibrateRequest, OptimizeRequest, QuantizeRequest};
use crate::ClientError::{ApiError, CompilationFailed};

#[cfg(feature = "blocking")]
pub mod blocking;
mod compile;
mod dss;

pub static FURIOSA_API_ENDPOINT_ENV: &str = "FURIOSA_API_ENDPOINT";
static ACCESS_KEY_ID_ENV: &str = "FURIOSA_ACCESS_KEY_ID";
static SECRET_ACCESS_KEY_ENV: &str = "FURIOSA_SECRET_ACCESS_KEY";
static DEFAULT_FURIOSA_API_ENDPOINT: &str = "https://api.furiosa.ai";

static APPLICATION_OCTET_STREAM_MIME: &str = "application/octet-stream";
static ACCESS_KEY_ID_HTTP_HEADER: &str = "X-FuriosaAI-Access-Key-ID";
static SECRET_ACCESS_KEY_HTTP_HEADER: &str = "X-FuriosaAI-Secret-Access-KEY";
static REQUEST_ID_HTTP_HEADER: &str = "X-Request-Id";
static FURIOSA_SDK_VERSION_HEADER: &str = "X-FuriosaAI-SDK-Version";
static FURIOSA_SDK_VERSION_VALUE: &str = "0.2.1";

lazy_static! {
    pub static ref FURIOSA_CLIENT_USER_AGENT: String = {
        let mut user_agent = String::from("FuriosaAI Rust Client (ver.");
        user_agent.push_str(env!("CARGO_PKG_VERSION"));
        user_agent.push(')');
        user_agent
    };
}

static TARGET_NPU_SPEC_PART_NAME: &str = "target_npu_spec";
static COMPILER_CONFIG_PART_NAME: &str = "compiler_config";
static TARGET_IR_PART_NAME: &str = "target_ir";
static SOURCE_PART_NAME: &str = "source";
static DSS_INPUT_TENSORS_PART_NAME: &str = "input_tensors";
static DSS_DYNAMIC_RANGES_PART_NAME: &str = "dynamic_ranges";

#[derive(thiserror::Error, Debug)]
pub enum ClientError {
    #[error("IO Error: {0}")]
    Io(io::Error),
    #[error("Error parsing line: '{0}', error at line index: {1}")]
    ConfigParse(String, usize),
    #[error("{0}")]
    ConfigEnvVar(std::env::VarError),
    #[error("FURIOSA_ACCESS_KEY_ID, FURIOSA_SECRET_ACCESS_KEY must be set")]
    NoApiKey,
    #[error("ApiError: {0}")]
    ApiError(String),
    #[error("Compilation failed:\n{0}")]
    CompilationFailed(String),
}

impl ClientError {
    pub fn io_error(kind: io::ErrorKind, msg: &str) -> ClientError {
        ClientError::Io(io::Error::new(kind, msg.to_string()))
    }
}

impl From<dotenv::Error> for ClientError {
    fn from(e: dotenv::Error) -> Self {
        match e {
            dotenv::Error::Io(e) => ClientError::Io(e),
            dotenv::Error::LineParse(line, error_idx) => ClientError::ConfigParse(line, error_idx),
            dotenv::Error::EnvVar(e) => ClientError::ConfigEnvVar(e),
            _ => unreachable!(),
        }
    }
}

impl From<io::Error> for ClientError {
    fn from(e: io::Error) -> Self {
        ClientError::Io(e)
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct ApiResponse {
    pub error_code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
}

pub struct FuriosaClient {
    client: reqwest::Client,
    endpoint: String,
    access_key_id: String,
    secret_access_key: String,
}

fn config_file_path(file: &str) -> Option<PathBuf> {
    dirs::home_dir()
        .map(|mut home| {
            home.push(format!(".furiosa/{}", file));
            home
        })
        .filter(|p| p.exists())
}

fn load_config_file_(file: &str) -> Result<(), ClientError> {
    if let Some(path) = config_file_path(file) {
        dotenv::from_path(path)?;
        Ok(())
    } else {
        Err(ClientError::io_error(io::ErrorKind::NotFound, &format!("{} file not found", file)))
    }
}

fn load_config_file(file: &str) -> Result<(), ClientError> {
    match load_config_file_(file) {
        Ok(_) => {}
        Err(ClientError::Io(_)) => {
            // ignore the file not found error because it's optional
        }
        Err(e) => return Err(e),
    };
    Ok(())
}

pub fn get_endpoint_from_env() -> Result<String, ClientError> {
    match std::env::var(FURIOSA_API_ENDPOINT_ENV) {
        Ok(mut val) => {
            // remove the trailing slash
            loop {
                if val.ends_with('/') {
                    val.remove(val.len() - 1);
                } else {
                    break;
                }
            }
            Ok(val)
        }
        Err(VarError::NotPresent) => Ok(String::from(DEFAULT_FURIOSA_API_ENDPOINT)),
        Err(e) => Err(ClientError::ConfigEnvVar(e)),
    }
}

#[derive(Deserialize)]
pub struct VersionInfo {
    pub version: String,
    pub revision: String,
    pub build_time: String,
}

impl FuriosaClient {
    pub fn new() -> Result<FuriosaClient, ClientError> {
        // Try to read $HOME/.furiosa/config including extra configurations
        load_config_file("config")?;
        // Try to read $HOME/.furiosa/credential and set credentials to environment variables
        load_config_file("credential")?;

        // Try to get both API KEYs and exist if KEYs are not set
        let access_key_id = std::env::var(ACCESS_KEY_ID_ENV).map_err(|_| ClientError::NoApiKey)?;
        let secret_access_key =
            std::env::var(SECRET_ACCESS_KEY_ENV).map_err(|_| ClientError::NoApiKey)?;

        let endpoint = get_endpoint_from_env()?;
        let client = reqwest::Client::builder()
            .user_agent(FURIOSA_CLIENT_USER_AGENT.as_str())
            .build()
            .expect("fail to create HTTP Client");

        info!("Connecting API Endpoint: {}", &endpoint);
        Ok(FuriosaClient { client, endpoint, access_key_id, secret_access_key })
    }

    fn set_default_headers(&self, b: RequestBuilder) -> RequestBuilder {
        b.header(ACCESS_KEY_ID_HTTP_HEADER, &self.access_key_id)
            .header(SECRET_ACCESS_KEY_HTTP_HEADER, &self.secret_access_key)
            .header(FURIOSA_SDK_VERSION_HEADER, FURIOSA_SDK_VERSION_VALUE)
    }

    #[inline]
    fn api_root_path(&self, path: &str) -> String {
        format!("{}/{}", &self.endpoint, path)
    }

    #[inline]
    fn api_v1_path(&self, path: &str) -> String {
        format!("{}/api/v1/{}", &self.endpoint, path)
    }

    #[inline]
    fn api_v1alpha_path(&self, api: &str, path: &str) -> String {
        format!("{}/api/{}/v1alpha1/{}", &self.endpoint, api, path)
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    pub async fn server_version(&self) -> Result<VersionInfo, ClientError> {
        let path = &self.api_root_path("version");
        let response = self.client.get(path).send().await;
        make_response(path, response, |bytes| Ok(serde_json::from_slice(&bytes).unwrap())).await
    }

    pub async fn compile(&self, request: CompileRequest) -> Result<Box<[u8]>, ClientError> {
        let mut model_image = Part::bytes(request.source);
        model_image = model_image.file_name(request.filename);

        model_image =
            model_image.mime_str(APPLICATION_OCTET_STREAM_MIME).expect("Invalid MIME type");

        let mut form: Form = Form::new()
            .text(TARGET_IR_PART_NAME, request.target_ir.as_str().to_string())
            .text(
                TARGET_NPU_SPEC_PART_NAME,
                serde_json::to_string(&request.target_npu_spec).unwrap(),
            )
            .part(SOURCE_PART_NAME, model_image);

        if let Some(compiler_config) = &request.compiler_config {
            form = form
                .text(COMPILER_CONFIG_PART_NAME, serde_json::to_string(compiler_config).unwrap());
        };

        let path = &self.api_v1alpha_path("compiler", "tasks");
        let req = self
            .client
            .post(path)
            .header(REQUEST_ID_HTTP_HEADER, Uuid::new_v4().to_hyphenated().to_string());
        let response = self.set_default_headers(req).multipart(form).send().await;

        let mut task: CompileTask =
            make_response(path, response, |bytes| Ok(serde_json::from_slice(&bytes).unwrap()))
                .await?;

        let task_id = task.task_id;

        loop {
            if task.phase.is_completed() {
                break;
            }

            tokio::time::delay_for(Duration::from_millis(500)).await;
            let path = self.api_v1alpha_path("compiler", &format!("tasks/{}", &task_id));
            let response = self.set_default_headers(self.client.get(&path)).send().await;
            task =
                make_response(&path, response, |bytes| Ok(serde_json::from_slice(&bytes).unwrap()))
                    .await?;
        }

        match &task.phase {
            CompileTaskPhase::Succeeded => {
                let path = self.api_v1alpha_path(
                    "compiler",
                    &format!("tasks/{}/artifacts/output.enf", &task_id),
                );
                let response = self.set_default_headers(self.client.get(&path)).send().await;
                return make_response(&path, response, |bytes| {
                    Ok(bytes.to_vec().into_boxed_slice())
                })
                .await;
            }
            CompileTaskPhase::Failed => {
                let path = self.api_v1alpha_path("compiler", &format!("tasks/{}/logs", &task_id));
                let response = self.set_default_headers(self.client.get(&path)).send().await;
                let log_message: String = make_response(&path, response, |bytes| {
                    Ok(String::from_utf8_lossy(&bytes).to_string())
                })
                .await?;
                Err(CompilationFailed(log_message))
            }
            _ => unreachable!("cannot reach non-terminal phase"),
        }
    }

    pub async fn optimize(&self, request: OptimizeRequest) -> Result<Box<[u8]>, ClientError> {
        let mut model_image = Part::bytes(request.source);
        model_image = model_image.file_name(request.filename);

        model_image =
            model_image.mime_str(APPLICATION_OCTET_STREAM_MIME).expect("Invalid MIME type");

        let form: Form = Form::new().part(SOURCE_PART_NAME, model_image);
        let request = self
            .client
            .post(&self.api_v1_path("dss/optimize"))
            .header(REQUEST_ID_HTTP_HEADER, Uuid::new_v4().to_hyphenated().to_string());
        let response = self.set_default_headers(request).multipart(form).send().await;

        match response {
            Ok(res) => {
                if res.status().is_success() {
                    match res.bytes().await {
                        Ok(bytes) => Ok(bytes.to_vec().into_boxed_slice()),
                        Err(e) => {
                            Err(ApiError(format!("fail to fetch the calibration onnx: {}", e)))
                        }
                    }
                } else {
                    let response: ApiResponse = match res.json().await {
                        Ok(api_response) => api_response,
                        Err(e) => return Err(ApiError(format!("fail to get API response: {}", e))),
                    };
                    Err(ApiError(format!("fail to compile: {}", &response.message)))
                }
            }
            Err(e) => Err(ApiError(format!("{}", e))),
        }
    }

    pub async fn build_calibration_model(
        &self,
        request: CalibrateRequest,
    ) -> Result<Box<[u8]>, ClientError> {
        let mut model_image = Part::bytes(request.source);
        model_image = model_image.file_name(request.filename);

        model_image =
            model_image.mime_str(APPLICATION_OCTET_STREAM_MIME).expect("Invalid MIME type");

        let input_tensors = serde_json::to_string(&request.input_tensors).map_err(|_| {
            ClientError::ApiError("Failed to serialize 'input_tenosrs'.".to_string())
        })?;
        let form: Form = Form::new()
            .text(DSS_INPUT_TENSORS_PART_NAME, input_tensors)
            .part(SOURCE_PART_NAME, model_image);
        let request = self
            .client
            .post(&self.api_v1_path("dss/build-calibration-model"))
            .header(REQUEST_ID_HTTP_HEADER, Uuid::new_v4().to_hyphenated().to_string());
        let response = self
            .set_default_headers(request)
            .header(ACCESS_KEY_ID_HTTP_HEADER, &self.access_key_id)
            .header(SECRET_ACCESS_KEY_HTTP_HEADER, &self.secret_access_key)
            .multipart(form)
            .send()
            .await;

        match response {
            Ok(res) => {
                if res.status().is_success() {
                    match res.bytes().await {
                        Ok(bytes) => Ok(bytes.to_vec().into_boxed_slice()),
                        Err(e) => {
                            Err(ApiError(format!("fail to fetch the calibration onnx: {}", e)))
                        }
                    }
                } else {
                    let response: ApiResponse = match res.json().await {
                        Ok(api_response) => api_response,
                        Err(e) => return Err(ApiError(format!("fail to get API response: {}", e))),
                    };
                    Err(ApiError(format!("fail to compile: {}", &response.message)))
                }
            }
            Err(e) => Err(ApiError(format!("{}", e))),
        }
    }

    pub async fn quantize(&self, request: QuantizeRequest) -> Result<Box<[u8]>, ClientError> {
        let mut model_image = Part::bytes(request.source);
        model_image = model_image.file_name(request.filename);

        model_image =
            model_image.mime_str(APPLICATION_OCTET_STREAM_MIME).expect("Invalid MIME type");

        let input_tensors = serde_json::to_string(&request.input_tensors).map_err(|_| {
            ClientError::ApiError("Failed to serialize 'input_tensors'.".to_string())
        })?;
        let dynamic_ranges = serde_json::to_string(&request.dynamic_ranges).map_err(|_| {
            ClientError::ApiError("Failed to serialize 'dynamic_ranges'.".to_string())
        })?;
        let form: Form = Form::new()
            .text(DSS_INPUT_TENSORS_PART_NAME, input_tensors)
            .text(DSS_DYNAMIC_RANGES_PART_NAME, dynamic_ranges)
            .part(SOURCE_PART_NAME, model_image);

        let request = self
            .client
            .post(&self.api_v1_path("dss/quantize"))
            .header(REQUEST_ID_HTTP_HEADER, Uuid::new_v4().to_hyphenated().to_string());
        let response = self.set_default_headers(request).multipart(form).send().await;

        match response {
            Ok(res) => {
                if res.status().is_success() {
                    match res.bytes().await {
                        Ok(bytes) => Ok(bytes.to_vec().into_boxed_slice()),
                        Err(e) => Err(ApiError(format!("fail to fetch the quantized onnx: {}", e))),
                    }
                } else {
                    let response: ApiResponse = match res.json().await {
                        Ok(api_response) => api_response,
                        Err(e) => return Err(ApiError(format!("fail to get API response: {}", e))),
                    };
                    Err(ApiError(format!("fail to compile: {}", &response.message)))
                }
            }
            Err(e) => Err(ApiError(format!("{}", e))),
        }
    }
}

async fn make_response<F, T>(
    path: &str,
    response: Result<Response, reqwest::Error>,
    f: F,
) -> Result<T, ClientError>
where
    F: Fn(Bytes) -> Result<T, ClientError>,
{
    match response {
        Ok(response) => {
            if response.status().is_success() {
                match response.bytes().await {
                    Ok(bytes) => f(bytes),
                    Err(e) => Err(ApiError(format!("fail to deserialize the bytes: {}", e))),
                }
            } else {
                let err_response: ApiResponse = match response.json().await {
                    Ok(api_response) => api_response,
                    Err(e) => {
                        let msg =
                            format!("fail to deserialize the error response from {}: {}", path, e);
                        return Err(ApiError(msg));
                    }
                };
                Err(ApiError(format!("fail to call API {}: {}", path, &err_response.message)))
            }
        }
        Err(e) => Err(ApiError(format!("{}", e))),
    }
}
