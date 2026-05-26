use crate::paths::AppPaths;
use std::sync::{Arc, Mutex};

#[derive(Debug, Default, Clone)]
pub struct LastErrorState {
    inner: Arc<Mutex<Option<String>>>,
}

impl LastErrorState {
    pub fn set(&self, message: impl Into<String>) {
        *self.inner.lock().expect("last error lock poisoned") = Some(message.into());
    }

    pub fn get(&self) -> Option<String> {
        self.inner.lock().expect("last error lock poisoned").clone()
    }
}

pub fn init(paths: &AppPaths, level: &str) -> anyhow::Result<()> {
    std::fs::create_dir_all(paths.logs_dir())?;
    let filter = match level {
        "Debug" => "debug",
        "Warning" => "warn",
        "Error" => "error",
        _ => "info",
    };
    let file_appender = tracing_appender::rolling::daily(paths.logs_dir(), "voiceinsert.log");
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(file_appender)
        .try_init();

    Ok(())
}
