use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LuascanError {
    #[error("failed to create log dir")]
    LogDirIo(#[from] std::io::Error),
    #[error("failed to read config file {path}: {source}")]
    ConfigIo {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse config file {path}: {source}")]
    ConfigParse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
    #[error("failed to get current dir path: {source}")]
    CurrentDir {
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse lua script: {source}")]
    ParseError {
        #[source]
        source: full_moon::Error,
    },
    #[error("failed to start tokio runtime: {source}")]
    Runtime {
        #[source]
        source: std::io::Error,
    },
}
