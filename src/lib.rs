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

use std::io;
use std::path::PathBuf;

use log::{info, warn};
use reqwest::multipart::{Form, Part};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::ClientError::ApiError;
use serde_json::Value;

pub static FURIOSA_API_ENDPOINT_ENV: &str = "FURIOSA_API_ENDPOINT";
static ACCESS_KEY_ID_ENV: &str = "FURIOSA_ACCESS_KEY_ID";
static SECRET_ACCESS_KEY_ENV: &str = "FURIOSA_SECRET_ACCESS_KEY";
static DEFAULT_FURIOSA_API_ENDPOINT: &str = "https://api.furiosa.ai/api/v1";

static APPLICATION_OCTET_STREAM_MIME: &str = "application/octet-stream";

static USER_AGENT: &str = "FuriosaAI Rust Client (version: 0.1.1)";
static ACCESS_KEY_ID_HTTP_HEADER: &str = "X-FuriosaAI-Access-Key-ID";
static SECRET_ACCESS_KEY_HTTP_HEADER: &str = "X-FuriosaAI-Secret-Access-KEY";
static REQUEST_ID_HTTP_HEADER: &str = "X-Request-Id";

static TARGET_NPU_SPEC_PART_NAME: &str = "target_npu_spec";
static COMPILER_CONFIG_PART_NAME: &str = "compiler_config";
static TARGET_IR_PART_NAME: &str = "target_ir";
static SOURCE_PART_NAME: &str = "source";

#[derive(Error, Debug)]
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

#[derive(Copy, Clone)]
pub enum TargetIr {
    Dfg,
    Ldfg,
    Cdfg,
    Gir,
    Lir,
    Enf,
}

impl TargetIr {
    fn as_str(&self) -> &str {
        use TargetIr::*;
        match self {
            Dfg => "dfg",
            Ldfg => "ldfg",
            Cdfg => "cdfg",
            Gir => "gir",
            Lir => "lir",
            Enf => "enf",
        }
    }
}

pub struct CompileRequest {
    target_npu_spec: Value,
    compiler_config: Option<Value>,
    target_ir: TargetIr,
    filename: String,
    source: Vec<u8>,
}

impl CompileRequest {
    pub fn new<S: AsRef<[u8]>>(target_npu_spec: Value, source: S) -> CompileRequest {
        CompileRequest {
            target_npu_spec,
            compiler_config: None,
            target_ir: TargetIr::Enf,
            filename: String::from("noname"),
            source: source.as_ref().to_vec(),
        }
    }

    pub fn target_ir(mut self, target_format: TargetIr) -> CompileRequest {
        self.target_ir = target_format;
        self
    }

    pub fn compile_config(mut self, compile_config: Value) -> CompileRequest {
        self.compiler_config = Some(compile_config);
        self
    }

    pub fn filename(mut self, filename: &str) -> CompileRequest {
        self.filename = String::from(filename);
        self
    }
}

pub struct FuriosaClient {
    client: reqwest::Client,
    endpoint: String,
    access_key_id: String,
    secret_access_key: String,
}

fn credential_file_path() -> Option<PathBuf> {
    dirs::home_dir()
        .map(|mut home| {
            home.push(".furiosa/credential");
            home
        })
        .filter(|p| p.exists())
}

fn get_credential_from_file() -> Result<(), ClientError> {
    if let Some(path) = credential_file_path() {
        dotenv::from_path(path)?;
        Ok(())
    } else {
        Err(ClientError::io_error(io::ErrorKind::NotFound, "credential file not found"))
    }
}

pub fn get_endpoint_from_env() -> String {
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
            val
        }
        Err(e) => {
            warn!(
                "the environment variable '{}' is invalid, \
                and the default endpoint will be used",
                e
            );
            String::from(DEFAULT_FURIOSA_API_ENDPOINT)
        }
    }
}

impl FuriosaClient {
    pub fn new() -> Result<FuriosaClient, ClientError> {
        // Try to read $HOME/.furiosa/credential and set credentials to environment variables
        match get_credential_from_file() {
            Ok(_) => {}
            Err(ClientError::Io(_)) => {
                // ignore the file not found error because it's optional
            }
            Err(e) => return Err(e),
        };

        // Try to get both API KEYs and exist if KEYs are not set
        let access_key_id = std::env::var(ACCESS_KEY_ID_ENV).map_err(|_| ClientError::NoApiKey)?;
        let secret_access_key =
            std::env::var(SECRET_ACCESS_KEY_ENV).map_err(|_| ClientError::NoApiKey)?;

        let endpoint = get_endpoint_from_env();
        let client = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .build()
            .expect("fail to create HTTP Client");

        info!("Connecting API Endpoint: {}", &endpoint);
        Ok(FuriosaClient { client, endpoint, access_key_id, secret_access_key })
    }

    #[inline]
    fn api_v1_path(&self, path: &str) -> String {
        format!("{}/{}", &self.endpoint, path)
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    pub async fn compile(&self, request: CompileRequest) -> Result<Box<[u8]>, ClientError> {
        let mut model_image = Part::bytes(request.source.clone());
        model_image = model_image.file_name(request.filename.clone());

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

        let response = self
            .client
            .post(&self.api_v1_path("compiler"))
            .header(REQUEST_ID_HTTP_HEADER, Uuid::new_v4().to_hyphenated().to_string())
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
                            Err(ApiError(format!("fail to fetch the compiled binary: {}", e)))
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
}
