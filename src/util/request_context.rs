use std::cell::RefCell;
use std::future::Future;

use log4rs::encode::pattern::PatternEncoder;
use log4rs::encode::{Encode, Write as LogWrite};

tokio::task_local! {
    static X_REQUEST_ID: RefCell<Option<String>>;
}

thread_local! {
    static X_REQUEST_ID_TLS: RefCell<Option<String>> = const { RefCell::new(None) };
}

pub fn current_request_id() -> Option<String> {
    X_REQUEST_ID
        .try_with(|cell| cell.borrow().clone())
        .ok()
        .flatten()
        .or_else(|| X_REQUEST_ID_TLS.with(|cell| cell.borrow().clone()))
}

struct TlsRequestIdGuard {
    cleared: bool,
}

impl TlsRequestIdGuard {
    fn enter(id: String) -> Self {
        X_REQUEST_ID_TLS.with(|cell| *cell.borrow_mut() = Some(id));
        Self { cleared: false }
    }
}

impl Drop for TlsRequestIdGuard {
    fn drop(&mut self) {
        if !self.cleared {
            X_REQUEST_ID_TLS.with(|cell| *cell.borrow_mut() = None);
        }
    }
}

pub fn with_request_id<F, T>(id: impl Into<String>, f: F) -> T
where
    F: FnOnce() -> T,
{
    let _guard = TlsRequestIdGuard::enter(id.into());
    f()
}

pub async fn run_with_request_id<F, Fut, T>(id: impl Into<String>, f: F) -> T
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = T>,
{
    let id = id.into();
    X_REQUEST_ID
        .scope(RefCell::new(Some(id.clone())), async move {
            let _guard = TlsRequestIdGuard::enter(id);
            f().await
        })
        .await
}

#[derive(Debug)]
pub struct RequestIdEncoder {
    inner: PatternEncoder,
}

impl RequestIdEncoder {
    pub fn new(pattern: &str) -> Self {
        Self {
            inner: PatternEncoder::new(pattern),
        }
    }
}

impl Encode for RequestIdEncoder {
    fn encode(&self, w: &mut dyn LogWrite, record: &log::Record) -> anyhow::Result<()> {
        match current_request_id() {
            Some(id) => write!(w, "[x-request-id={}] ", id)?,
            None => write!(w, "[x-request-id=-] ")?,
        }
        self.inner.encode(w, record)
    }
}
