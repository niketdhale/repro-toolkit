use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum ReproError {
    #[error("failed to open PDX archive at {path}: {source}")]
    OpenArchive {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to read PDX archive: {0}")]
    Archive(#[from] zip::result::ZipError),

    #[error("PDX archive contains no ODX diagnostic layer container (*.odx-d)")]
    NoDiagLayerContainer,

    #[error("failed to read entry '{name}' from archive: {source}")]
    ArchiveEntry {
        name: String,
        #[source]
        source: std::io::Error,
    },

    #[error("entry '{name}' is not valid UTF-8: {source}")]
    InvalidUtf8 {
        name: String,
        #[source]
        source: std::string::FromUtf8Error,
    },

    #[error("failed to parse XML in '{name}': {source}")]
    Xml {
        name: String,
        #[source]
        source: roxmltree::Error,
    },

    #[error("no diagnostic layer with services was found in the ODX data")]
    NoServicesFound,

    #[error("failed to serialize repro sequence to JSON: {0}")]
    Serialize(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, ReproError>;
