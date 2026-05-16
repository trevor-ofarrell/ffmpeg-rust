use core::fmt;

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
}

impl AvError {
    pub fn new(kind: AvErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
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

    pub fn kind(&self) -> AvErrorKind {
        self.kind
    }

    pub fn message(&self) -> &str {
        &self.message
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
        assert_eq!(err.to_string(), "InvalidData: bad packet");
    }
}
