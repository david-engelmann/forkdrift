//! The refusal family of the tool.
//!
//! Every refusal the reader or the comparison raises is a
//! ForkdriftError, and the command line answers it with one
//! error line and exit status 2. An artifact refusal and an
//! interface mismatch are the members the Python
//! implementation carried, where both derived from the one
//! base error, and the source name refusal of FD-33 stands
//! beside them in the same family.

/// The base refusal of the tool, carrying the refusal text.
#[derive(Debug, thiserror::Error)]
pub enum ForkdriftError {
    /// A refusal the artifact reader raises over a malformed
    /// artifact.
    #[error("{0}")]
    Artifact(#[from] ArtifactError),
    /// A refusal the comparison raises when the two artifacts
    /// name different interfaces.
    #[error("{0}")]
    InterfaceMismatch(#[from] InterfaceMismatchError),
    /// A refusal the comparison raises when a source name
    /// carries a line break (FD-33).
    #[error("{0}")]
    SourceName(#[from] SourceNameError),
}

/// A refusal over a malformed artifact, carrying its text.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct ArtifactError(pub String);

impl ArtifactError {
    pub(crate) fn new(text: String) -> Self {
        ArtifactError(text)
    }
}

/// A refusal over two artifacts that name different
/// interfaces, carrying its text.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct InterfaceMismatchError(pub String);

impl InterfaceMismatchError {
    pub(crate) fn new(text: String) -> Self {
        InterfaceMismatchError(text)
    }
}

/// A refusal over a source name that carries a line break,
/// carrying its text. The text names the side and the defect
/// and interpolates no part of the name, since quoting the
/// name is the defect the refusal stands against.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct SourceNameError(pub String);

impl SourceNameError {
    pub(crate) fn new(side: &str) -> Self {
        SourceNameError(format!("the {side} source name carries a line break"))
    }
}
