use crate::{ClientError, CompileRequest};

pub struct FuriosaClient {
    _runtime: tokio::runtime::Runtime,
    handle: tokio::runtime::Handle,
    inner: super::FuriosaClient,
}

impl FuriosaClient {
    pub fn new() -> Result<FuriosaClient, ClientError> {
        let _runtime = tokio::runtime::Runtime::new().expect("fail to create tokio runtime");
        let handle = _runtime.handle().clone();

        Ok(FuriosaClient { inner: super::FuriosaClient::new()?, _runtime, handle })
    }

    pub fn compile(&self, request: CompileRequest) -> Result<Box<[u8]>, ClientError> {
        self.handle.block_on(async { self.inner.compile(request).await })
    }
}
