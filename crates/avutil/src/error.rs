use core::fmt;
use std::io;

pub type AvResult<T> = Result<T, AvError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AvErrorKind {
    InvalidData,
    InvalidArgument,
    NotFound,
    EndOfFile,
    Unsupported,
    External,
    Bug,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvError {
    kind: AvErrorKind,
    message: String,
    io_kind: Option<io::ErrorKind>,
}

impl AvError {
    pub fn new(kind: AvErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            io_kind: None,
        }
    }

    pub fn invalid_data(message: impl Into<String>) -> Self {
        Self::new(AvErrorKind::InvalidData, message)
    }

    pub fn invalid_argument(message: impl Into<String>) -> Self {
        Self::new(AvErrorKind::InvalidArgument, message)
    }

    pub fn unsupported(message: impl Into<String>) -> Self {
        Self::new(AvErrorKind::Unsupported, message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(AvErrorKind::NotFound, message)
    }

    pub fn end_of_file(message: impl Into<String>) -> Self {
        Self::new(AvErrorKind::EndOfFile, message)
    }

    pub fn external(message: impl Into<String>) -> Self {
        Self::new(AvErrorKind::External, message)
    }

    pub fn bug(message: impl Into<String>) -> Self {
        Self::new(AvErrorKind::Bug, message)
    }

    pub fn from_io_error(context: impl AsRef<str>, error: io::Error) -> Self {
        let io_kind = error.kind();
        let message = error_message_with_context(context.as_ref(), error);
        Self {
            kind: kind_from_io_error_kind(io_kind),
            message,
            io_kind: Some(io_kind),
        }
    }

    pub fn kind(&self) -> AvErrorKind {
        self.kind
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn io_kind(&self) -> Option<io::ErrorKind> {
        self.io_kind
    }

    pub fn is_eof(&self) -> bool {
        self.kind == AvErrorKind::EndOfFile
    }
}

fn kind_from_io_error_kind(kind: io::ErrorKind) -> AvErrorKind {
    match kind {
        io::ErrorKind::NotFound => AvErrorKind::NotFound,
        io::ErrorKind::UnexpectedEof => AvErrorKind::EndOfFile,
        io::ErrorKind::InvalidData => AvErrorKind::InvalidData,
        io::ErrorKind::InvalidInput => AvErrorKind::InvalidArgument,
        io::ErrorKind::Unsupported => AvErrorKind::Unsupported,
        _ => AvErrorKind::External,
    }
}

fn error_message_with_context(context: &str, error: io::Error) -> String {
    if context.is_empty() {
        error.to_string()
    } else {
        format!("{context}: {error}")
    }
}

impl fmt::Display for AvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)
    }
}

impl std::error::Error for AvError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_exposes_kind_and_message() {
        let err = AvError::invalid_data("bad packet");

        assert_eq!(err.kind(), AvErrorKind::InvalidData);
        assert_eq!(err.message(), "bad packet");
        assert_eq!(err.io_kind(), None);
        assert_eq!(err.to_string(), "InvalidData: bad packet");
    }

    #[test]
    fn constructors_cover_common_error_kinds() {
        let errors = [
            (
                AvError::invalid_argument("bad option"),
                AvErrorKind::InvalidArgument,
            ),
            (AvError::not_found("missing stream"), AvErrorKind::NotFound),
            (AvError::end_of_file("truncated"), AvErrorKind::EndOfFile),
            (AvError::unsupported("codec"), AvErrorKind::Unsupported),
            (AvError::external("system"), AvErrorKind::External),
            (AvError::bug("invariant"), AvErrorKind::Bug),
        ];

        for (error, kind) in errors {
            assert_eq!(error.kind(), kind);
            assert_eq!(error.io_kind(), None);
        }
    }

    #[test]
    fn io_errors_map_to_stable_av_error_kinds() {
        let cases = [
            (io::ErrorKind::NotFound, AvErrorKind::NotFound),
            (io::ErrorKind::UnexpectedEof, AvErrorKind::EndOfFile),
            (io::ErrorKind::InvalidData, AvErrorKind::InvalidData),
            (io::ErrorKind::InvalidInput, AvErrorKind::InvalidArgument),
            (io::ErrorKind::Unsupported, AvErrorKind::Unsupported),
            (io::ErrorKind::PermissionDenied, AvErrorKind::External),
        ];

        for (io_kind, av_kind) in cases {
            let err = AvError::from_io_error("AVIO", io::Error::new(io_kind, "source failure"));
            assert_eq!(err.kind(), av_kind);
            assert_eq!(err.io_kind(), Some(io_kind));
            assert!(err.message().contains("AVIO"));
            assert!(err.message().contains("source failure"));
        }
    }

    #[test]
    fn eof_predicate_tracks_error_kind() {
        assert!(AvError::end_of_file("done").is_eof());
        assert!(AvError::from_io_error(
            "read",
            io::Error::new(io::ErrorKind::UnexpectedEof, "short read")
        )
        .is_eof());
        assert!(!AvError::invalid_data("bad packet").is_eof());
    }
}
