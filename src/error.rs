#[derive(Debug, thiserror::Error)]
pub enum HemliError {
    #[error("secret '{secret}' not found in namespace '{namespace}'")]
    NotFound { namespace: String, secret: String },

    #[error("no source command provided and secret is not cached")]
    NoSource,

    #[error(
        "no modifications specified; provide at least one of --ttl, --clear-ttl, --source-sh, or --source-cmd"
    )]
    NoModifications,

    #[error("source command failed: {0}")]
    SourceFailed(String),

    #[error(transparent)]
    Keyring(#[from] keyring::Error),

    #[error(transparent)]
    Serialization(#[from] serde_json::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}
