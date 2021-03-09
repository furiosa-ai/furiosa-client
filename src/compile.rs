use std::borrow::Cow;

use serde::Deserialize;
use serde_json::Value;

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
    pub fn as_str(&self) -> &str {
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
    pub target_npu_spec: Value,
    pub compiler_config: Option<Value>,
    pub target_ir: TargetIr,
    pub filename: String,
    pub source: Vec<u8>,
}

impl CompileRequest {
    pub fn new<'a, S: Into<Cow<'a, [u8]>>>(target_npu_spec: Value, source: S) -> CompileRequest {
        CompileRequest {
            target_npu_spec,
            compiler_config: None,
            target_ir: TargetIr::Enf,
            filename: String::from("noname"),
            source: match source.into() {
                Cow::Borrowed(value) => Vec::from(value),
                Cow::Owned(value) => value,
            },
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

#[derive(Deserialize)]
pub struct CompileTask {
    pub version: i32,
    pub task_id: String,
    pub phase: CompileTaskPhase,
    pub submit_time: i64,
    pub start_time: Option<i64>,
    pub finish_time: Option<i64>,
    pub progress: f32,
    pub error_message: Option<String>,
}

#[derive(Deserialize, Eq, PartialEq)]
pub enum CompileTaskPhase {
    Pending,
    Running,
    Succeeded,
    Failed,
}

impl CompileTaskPhase {
    pub fn is_completed(&self) -> bool {
        self == &CompileTaskPhase::Succeeded || self == &CompileTaskPhase::Failed
    }
}
