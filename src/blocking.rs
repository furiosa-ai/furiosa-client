pub use crate::dss::{CalibrateRequest, QuantizeRequest};
pub use crate::{ClientError, CompileRequest};

pub struct FuriosaClient {
    _runtime: tokio::runtime::Runtime,
    handle: tokio::runtime::Handle,
    inner: super::FuriosaClient,
}

impl FuriosaClient {
    pub fn new<S: AsRef<str>>(runtime_version: S) -> Result<FuriosaClient, ClientError> {
        let _runtime = tokio::runtime::Runtime::new().expect("fail to create tokio runtime");
        let handle = _runtime.handle().clone();

        Ok(FuriosaClient { inner: super::FuriosaClient::new(runtime_version)?, _runtime, handle })
    }

    pub fn compile(&self, request: CompileRequest) -> Result<Box<[u8]>, ClientError> {
        self.handle.block_on(self.inner.compile(request))
    }

    pub fn build_calibration_model(
        &self,
        request: CalibrateRequest,
    ) -> Result<Box<[u8]>, ClientError> {
        self.handle.block_on(self.inner.build_calibration_model(request))
    }

    pub fn quantize(&self, request: QuantizeRequest) -> Result<Box<[u8]>, ClientError> {
        self.handle.block_on(self.inner.quantize(request))
    }
}
