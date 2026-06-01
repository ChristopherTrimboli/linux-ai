use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("configuration error: {0}")]
    Config(String),

    #[error("no API key configured for provider '{0}'")]
    MissingApiKey(String),

    #[error("unknown provider '{0}'")]
    UnknownProvider(String),

    #[error("provider request failed ({status}): {body}")]
    Provider { status: u16, body: String },

    #[error("tool '{0}' not found")]
    ToolNotFound(String),

    #[error("invalid tool input: {0}")]
    ToolInput(String),

    #[error("access denied: {0}")]
    AccessDenied(String),

    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}

impl Error {
    pub fn other(msg: impl Into<String>) -> Self {
        Error::Other(msg.into())
    }
}
