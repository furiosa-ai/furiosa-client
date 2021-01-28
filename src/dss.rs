use std::collections::HashMap;

pub struct OptimizeRequest {
    pub filename: String,
    pub source: Vec<u8>,
}

pub struct CalibrateRequest {
    pub filename: String,
    pub source: Vec<u8>,
    pub input_tensors: Vec<String>,
}

pub struct QuantizeRequest {
    pub filename: String,
    pub source: Vec<u8>,
    pub input_tensors: Vec<String>,
    pub dynamic_ranges: HashMap<String, (f32, f32)>,
}
