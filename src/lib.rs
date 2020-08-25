use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use log::info;
use reqwest;
use reqwest::blocking::multipart::{Form, Part};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::ClientError::ApiError;

static ACCESS_KEY_ID_ENV: &str = "FURIOSA_ACCESS_KEY_ID";
static SECRET_ACCESS_KEY_ENV: &str = "FURIOSA_SECRET_ACCESS_KEY";

static APPLICATION_OCTET_STREAM_MIME: &str = "application/octet-stream";

static USER_AGENT: &str = "FuriosaAI Rust Client";
static ACCESS_KEY_ID_HTTP_HEADER: &str = "X-FuriosaAI-Access-Key-ID";
static SECRET_ACCESS_KEY_HTTP_HEADER: &str = "X-FuriosaAI-Secret-Access-KEY";
static REQUEST_ID_HTTP_HEADER: &str = "X-Request-Id";

static SOURCE_FORMAT_PART_NAME: &str = "source_format";
static TARGET_FORMAT_PART_NAME: &str = "target_format";
static TARGET_NPU_SPEC_PART_NAME: &str = "target_npu_spec";
static SOURCE_PART_NAME: &str = "source";

fn api_v1_path(path: &str) -> String {
    if cfg!(feature = "local_api") {
        format!("http://localhost:8080/api/v1/{}", path)
    } else {
        format!("http://internal-furiosa-api-backend-dev-887583302.ap-northeast-2.elb.amazonaws.com:8080/api/v1/{}", path)
    }
}

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
    ApiError(String)
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
            _ => unreachable!()
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
pub enum SourceFormat {
    Tflite,
    Onnx,
    Dfg,
    Ldfg,
    Cdfg,
    Gir,
    Lir
}

impl SourceFormat {
    fn as_str(&self) -> &str {
        use SourceFormat::*;
        match self {
            Tflite => "tflite",
            Onnx => "onnx",
            Dfg => "dfg",
            Ldfg => "ldfg",
            Cdfg => "cdfg",
            Gir => "gir",
            Lir => "lir",
        }
    }
}

#[derive(Copy, Clone)]
pub enum TargetFormat {
    Dfg,
    Ldfg,
    Cdfg,
    Gir,
    Lir,
    Enf
}

impl TargetFormat {
    fn as_str(&self) -> &str {
        use TargetFormat::*;
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

pub struct FuriosaClient {
    client: reqwest::blocking::Client,
    access_key_id: String,
    secret_access_key: String,
}

fn credential_file_path() -> Option<PathBuf> {
    dirs::home_dir().map(|home| {
        let mut path = PathBuf::from(home);
        path.push(".furiosa/credential");
        path
    }).filter(|p| p.exists())
}

fn get_credential_from_file() -> Result<(), ClientError> {
    if let Some(path) = credential_file_path() {
        dotenv::from_path(path)?;
        Ok(())
    } else {
        Err(ClientError::io_error(io::ErrorKind::NotFound, "credential file not found"))
    }
}

impl FuriosaClient {
    pub fn new() -> Result<FuriosaClient, ClientError> {
        info!("Connecting API Endpoint: {}", api_v1_path(""));

        // Try to read $HOME/.furiosa/credential and set credentials to environment variables
        match get_credential_from_file() {
            Ok(_) => {},
            Err(ClientError::Io(_)) => {
                // ignore the file not found error because it's optional
            },
            Err(e) => return Err(e)
        };

        // Try to get both KEYs and exist if KEYs are not set
        let access_key_id = std::env::var(ACCESS_KEY_ID_ENV)
            .map_err(|_| ClientError::NoApiKey)?;
        let secret_access_key = std::env::var(SECRET_ACCESS_KEY_ENV)
            .map_err(|_| ClientError::NoApiKey)?;

        let client = reqwest::blocking::Client::builder()
            .user_agent(USER_AGENT)
            .build()
            .expect("fail to create HTTP Client");

        Ok(FuriosaClient {
            client,
            access_key_id,
            secret_access_key
        })
    }

    pub fn compile_from_file<P: AsRef<Path>>(&self, src_format: SourceFormat, target_format: TargetFormat,
                             path: P) -> Result<Box<[u8]>, ClientError> {
        let path = path.as_ref();
        let filename = path.file_name().map(|f| f.to_str().expect("invalid filename"));
        let buf = std::fs::read(path)?;
        Ok(self.compile(src_format, target_format, buf, filename)?)
    }

    pub fn compile<F: AsRef<str>>(&self,
                                  src_format: SourceFormat,
                                  target_format: TargetFormat,
                                  binary: Vec<u8>,
                                  filename: Option<F>) -> Result<Box<[u8]>, ClientError> {

        let mut model_image = Part::bytes(binary);
        if let Some(filename) = filename {
            model_image = model_image.file_name(filename.as_ref().to_string());
        }
        model_image = model_image.mime_str(APPLICATION_OCTET_STREAM_MIME)
            .expect("Invalid MIME type");

        let target_spec: BTreeMap<String, i64> =
            serde_yaml::from_str(include_str!("../configs/64dpes.yml")).unwrap();

        let form: Form = Form::new()
            .text(SOURCE_FORMAT_PART_NAME, src_format.as_str().to_string())
            .text(TARGET_FORMAT_PART_NAME, target_format.as_str().to_string())
            .text(TARGET_NPU_SPEC_PART_NAME, serde_json::to_string(&target_spec).unwrap())
            .part(SOURCE_PART_NAME, model_image);

        let response = self.client.post(&api_v1_path("compiler"))
            .header(REQUEST_ID_HTTP_HEADER, Uuid::new_v4().to_hyphenated().to_string())
            .header(ACCESS_KEY_ID_HTTP_HEADER, &self.access_key_id)
            .header(SECRET_ACCESS_KEY_HTTP_HEADER, &self.secret_access_key)
            .multipart(form)
            .send();

        match response {
            Ok(res) => {
                if res.status().is_success() {
                    match res.bytes() {
                        Ok(bytes) => Ok(bytes.to_vec().into_boxed_slice()),
                        Err(e) => Err(ApiError(format!("fail to fetch the compiled binary: {}", e)))
                    }
                } else {
                    let response: ApiResponse = res.json().unwrap();
                    eprintln!("{:?}", &response);
                    return Err(ApiError(format!("fail to compile: {}", &response.message)));
                }
            },
            Err(e) => Err(ApiError(format!("{}", e)))
        }
    }
}