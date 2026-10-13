use rhai::{EvalAltResult, Position};

/// Everything that can go wrong while building or querying geometry.
///
/// Every fallible operation in this crate returns [`Result`]. When called from a Rhai script the
/// error is turned into a runtime error carrying [`Error`]'s `Display` text, which scripts can
/// intercept with `try { ... } catch (err) { ... }`.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum Error {
    /// OpenCASCADE threw an exception (`<exception type>: <message>`).
    #[error("OpenCASCADE error: {0}")]
    Occt(String),
    /// An OpenCASCADE algorithm finished without producing a result.
    #[error("{0} failed")]
    OperationFailed(&'static str),
    /// A shape of the wrong topological type was passed to an operation.
    #[error("expected {expected}, got {actual}")]
    WrongShapeType { expected: &'static str, actual: &'static str },
    /// An argument was out of range or otherwise unusable.
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
    /// A selection was empty where at least one shape was required.
    #[error("selection is empty: {0}")]
    EmptySelection(String),
    /// An index was out of range.
    #[error("index {index} out of range for selection of {len} shapes")]
    IndexOutOfRange { index: i64, len: usize },
    /// A selector string could not be parsed.
    #[error("invalid selector {selector:?}: {message}")]
    Selector { selector: String, message: String },
    /// Reading or writing a file failed.
    #[error("{0}")]
    Io(String),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

impl From<cxx::Exception> for Error {
    fn from(exception: cxx::Exception) -> Self {
        Error::Occt(exception.what().to_owned())
    }
}

impl From<Error> for Box<EvalAltResult> {
    fn from(error: Error) -> Self {
        EvalAltResult::ErrorRuntime(error.to_string().into(), Position::NONE).into()
    }
}

pub(crate) fn invalid(message: impl Into<String>) -> Error {
    Error::InvalidArgument(message.into())
}
