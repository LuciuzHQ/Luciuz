use thiserror::Error;

#[derive(Debug, Error)]
pub enum LuciuzError {
    #[error("configuration error: {0}")]
    Config(String),

    #[error("io error: {0}")]
    Io(String),

    #[error("runtime error: {0}")]
    Runtime(String),
}
