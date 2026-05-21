use core::fmt;
use std::io;

pub type AvResult<T> = Result<T, AvError>;
pub const AV_ERROR_MAX_STRING_SIZE: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AvErrorCode(i32);

impl AvErrorCode {
    pub const BSF_NOT_FOUND: Self = Self::fferrtag(0xf8, b'B', b'S', b'F');
    pub const BUG: Self = Self::fferrtag(b'B', b'U', b'G', b'!');
    pub const BUFFER_TOO_SMALL: Self = Self::fferrtag(b'B', b'U', b'F', b'S');
    pub const DECODER_NOT_FOUND: Self = Self::fferrtag(0xf8, b'D', b'E', b'C');
    pub const DEMUXER_NOT_FOUND: Self = Self::fferrtag(0xf8, b'D', b'E', b'M');
    pub const ENCODER_NOT_FOUND: Self = Self::fferrtag(0xf8, b'E', b'N', b'C');
    pub const EOF: Self = Self::fferrtag(b'E', b'O', b'F', b' ');
    pub const EXIT: Self = Self::fferrtag(b'E', b'X', b'I', b'T');
    pub const EXTERNAL: Self = Self::fferrtag(b'E', b'X', b'T', b' ');
    pub const FILTER_NOT_FOUND: Self = Self::fferrtag(0xf8, b'F', b'I', b'L');
    pub const INVALIDDATA: Self = Self::fferrtag(b'I', b'N', b'D', b'A');
    pub const MUXER_NOT_FOUND: Self = Self::fferrtag(0xf8, b'M', b'U', b'X');
    pub const OPTION_NOT_FOUND: Self = Self::fferrtag(0xf8, b'O', b'P', b'T');
    pub const PATCHWELCOME: Self = Self::fferrtag(b'P', b'A', b'W', b'E');
    pub const PROTOCOL_NOT_FOUND: Self = Self::fferrtag(0xf8, b'P', b'R', b'O');
    pub const STREAM_NOT_FOUND: Self = Self::fferrtag(0xf8, b'S', b'T', b'R');
    pub const BUG2: Self = Self::fferrtag(b'B', b'U', b'G', b' ');
    pub const UNKNOWN: Self = Self::fferrtag(b'U', b'N', b'K', b'N');
    pub const EXPERIMENTAL: Self = Self::from_raw(-0x2bb2_afa8);
    pub const INPUT_CHANGED: Self = Self::from_raw(-0x636e_6701);
    pub const OUTPUT_CHANGED: Self = Self::from_raw(-0x636e_6702);
    pub const INPUT_AND_OUTPUT_CHANGED: Self =
        Self::from_raw(Self::INPUT_CHANGED.raw() | Self::OUTPUT_CHANGED.raw());
    pub const HTTP_BAD_REQUEST: Self = Self::fferrtag(0xf8, b'4', b'0', b'0');
    pub const HTTP_UNAUTHORIZED: Self = Self::fferrtag(0xf8, b'4', b'0', b'1');
    pub const HTTP_FORBIDDEN: Self = Self::fferrtag(0xf8, b'4', b'0', b'3');
    pub const HTTP_NOT_FOUND: Self = Self::fferrtag(0xf8, b'4', b'0', b'4');
    pub const HTTP_TOO_MANY_REQUESTS: Self = Self::fferrtag(0xf8, b'4', b'2', b'9');
    pub const HTTP_OTHER_4XX: Self = Self::fferrtag(0xf8, b'4', b'X', b'X');
    pub const HTTP_SERVER_ERROR: Self = Self::fferrtag(0xf8, b'5', b'X', b'X');

    pub const EPERM: Self = Self::from_posix_errno(1);
    pub const ENOENT: Self = Self::from_posix_errno(2);
    pub const EINTR: Self = Self::from_posix_errno(4);
    pub const EIO: Self = Self::from_posix_errno(5);
    pub const EAGAIN: Self = Self::from_posix_errno(11);
    pub const ENOMEM: Self = Self::from_posix_errno(12);
    pub const EACCES: Self = Self::from_posix_errno(13);
    pub const EINVAL: Self = Self::from_posix_errno(22);
    pub const EPIPE: Self = Self::from_posix_errno(32);
    pub const ENOSYS: Self = Self::from_posix_errno(38);

    pub const fn from_raw(value: i32) -> Self {
        Self(value)
    }

    pub const fn from_posix_errno(errno: i32) -> Self {
        Self(-errno)
    }

    pub const fn fferrtag(a: u8, b: u8, c: u8, d: u8) -> Self {
        let tag = (a as i32) | ((b as i32) << 8) | ((c as i32) << 16) | ((d as i32) << 24);
        Self(-tag)
    }

    pub const fn raw(self) -> i32 {
        self.0
    }

    pub fn description(self) -> Option<&'static str> {
        av_error_description(self.raw())
    }

    pub fn make_error_string(self) -> String {
        av_make_error_string(self.raw())
    }
}

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
    code: Option<AvErrorCode>,
    io_kind: Option<io::ErrorKind>,
}

impl AvError {
    pub fn new(kind: AvErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            code: default_code_for_kind(kind),
            io_kind: None,
        }
    }

    pub fn with_code(kind: AvErrorKind, code: AvErrorCode, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            code: Some(code),
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
            code: code_from_io_error_kind(io_kind),
            io_kind: Some(io_kind),
        }
    }

    pub fn kind(&self) -> AvErrorKind {
        self.kind
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn code(&self) -> Option<AvErrorCode> {
        self.code
    }

    pub fn io_kind(&self) -> Option<io::ErrorKind> {
        self.io_kind
    }

    pub fn is_eof(&self) -> bool {
        self.kind == AvErrorKind::EndOfFile
    }

    pub fn ffmpeg_description(&self) -> Option<&'static str> {
        self.code.and_then(AvErrorCode::description)
    }

    pub fn ffmpeg_error_string(&self) -> Option<String> {
        self.code.map(AvErrorCode::make_error_string)
    }
}

pub fn av_error_description(errnum: i32) -> Option<&'static str> {
    let code = AvErrorCode::from_raw(errnum);
    Some(match code {
        AvErrorCode::BSF_NOT_FOUND => "Bitstream filter not found",
        AvErrorCode::BUG | AvErrorCode::BUG2 => "Internal bug, should not have happened",
        AvErrorCode::BUFFER_TOO_SMALL => "Buffer too small",
        AvErrorCode::DECODER_NOT_FOUND => "Decoder not found",
        AvErrorCode::DEMUXER_NOT_FOUND => "Demuxer not found",
        AvErrorCode::ENCODER_NOT_FOUND => "Encoder not found",
        AvErrorCode::EOF => "End of file",
        AvErrorCode::EXIT => "Immediate exit requested",
        AvErrorCode::EXTERNAL => "Generic error in an external library",
        AvErrorCode::FILTER_NOT_FOUND => "Filter not found",
        AvErrorCode::INPUT_CHANGED => "Input changed",
        AvErrorCode::INVALIDDATA => "Invalid data found when processing input",
        AvErrorCode::MUXER_NOT_FOUND => "Muxer not found",
        AvErrorCode::OPTION_NOT_FOUND => "Option not found",
        AvErrorCode::OUTPUT_CHANGED => "Output changed",
        AvErrorCode::PATCHWELCOME => "Not yet implemented in FFmpeg, patches welcome",
        AvErrorCode::PROTOCOL_NOT_FOUND => "Protocol not found",
        AvErrorCode::STREAM_NOT_FOUND => "Stream not found",
        AvErrorCode::UNKNOWN => "Unknown error occurred",
        AvErrorCode::EXPERIMENTAL => "Experimental feature",
        AvErrorCode::HTTP_BAD_REQUEST => "Server returned 400 Bad Request",
        AvErrorCode::HTTP_UNAUTHORIZED => "Server returned 401 Unauthorized (authorization failed)",
        AvErrorCode::HTTP_FORBIDDEN => "Server returned 403 Forbidden (access denied)",
        AvErrorCode::HTTP_NOT_FOUND => "Server returned 404 Not Found",
        AvErrorCode::HTTP_TOO_MANY_REQUESTS => "Server returned 429 Too Many Requests",
        AvErrorCode::HTTP_OTHER_4XX => {
            "Server returned 4XX Client Error, but not one of 40{0,1,3,4}"
        }
        AvErrorCode::HTTP_SERVER_ERROR => "Server returned 5XX Server Error reply",
        AvErrorCode::EPERM => "Operation not permitted",
        AvErrorCode::ENOENT => "No such file or directory",
        AvErrorCode::EINTR => "Interrupted system call",
        AvErrorCode::EIO => "Input/output error",
        AvErrorCode::EAGAIN => "Resource temporarily unavailable",
        AvErrorCode::ENOMEM => "Cannot allocate memory",
        AvErrorCode::EACCES => "Permission denied",
        AvErrorCode::EINVAL => "Invalid argument",
        AvErrorCode::EPIPE => "Broken pipe",
        AvErrorCode::ENOSYS => "Function not implemented",
        _ => return None,
    })
}

pub fn av_make_error_string(errnum: i32) -> String {
    match av_strerror(errnum) {
        Ok(description) => description.to_string(),
        Err(generic) => generic,
    }
}

pub fn av_strerror(errnum: i32) -> Result<&'static str, String> {
    av_error_description(errnum).ok_or_else(|| format!("Error number {errnum} occurred"))
}

fn default_code_for_kind(kind: AvErrorKind) -> Option<AvErrorCode> {
    match kind {
        AvErrorKind::InvalidData => Some(AvErrorCode::INVALIDDATA),
        AvErrorKind::InvalidArgument => Some(AvErrorCode::EINVAL),
        AvErrorKind::NotFound => Some(AvErrorCode::ENOENT),
        AvErrorKind::EndOfFile => Some(AvErrorCode::EOF),
        AvErrorKind::Unsupported => Some(AvErrorCode::ENOSYS),
        AvErrorKind::External => Some(AvErrorCode::EXTERNAL),
        AvErrorKind::Bug => Some(AvErrorCode::BUG),
    }
}

fn code_from_io_error_kind(kind: io::ErrorKind) -> Option<AvErrorCode> {
    match kind {
        io::ErrorKind::NotFound => Some(AvErrorCode::ENOENT),
        io::ErrorKind::PermissionDenied => Some(AvErrorCode::EACCES),
        io::ErrorKind::Interrupted => Some(AvErrorCode::EINTR),
        io::ErrorKind::WouldBlock => Some(AvErrorCode::EAGAIN),
        io::ErrorKind::BrokenPipe => Some(AvErrorCode::EPIPE),
        io::ErrorKind::InvalidInput => Some(AvErrorCode::EINVAL),
        io::ErrorKind::UnexpectedEof => Some(AvErrorCode::EOF),
        io::ErrorKind::InvalidData => Some(AvErrorCode::INVALIDDATA),
        io::ErrorKind::Unsupported => Some(AvErrorCode::ENOSYS),
        _ => None,
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
        assert_eq!(err.code(), Some(AvErrorCode::INVALIDDATA));
        assert_eq!(err.io_kind(), None);
        assert_eq!(err.to_string(), "InvalidData: bad packet");
    }

    #[test]
    fn error_codes_match_ffmpeg_tag_constants() {
        assert_eq!(AV_ERROR_MAX_STRING_SIZE, 64);
        assert_eq!(
            AvErrorCode::INVALIDDATA,
            AvErrorCode::fferrtag(b'I', b'N', b'D', b'A')
        );
        assert_eq!(
            AvErrorCode::EOF,
            AvErrorCode::fferrtag(b'E', b'O', b'F', b' ')
        );
        assert_eq!(
            AvErrorCode::EXTERNAL,
            AvErrorCode::fferrtag(b'E', b'X', b'T', b' ')
        );
        assert_eq!(
            AvErrorCode::BSF_NOT_FOUND,
            AvErrorCode::fferrtag(0xf8, b'B', b'S', b'F')
        );
        assert_eq!(AvErrorCode::EXPERIMENTAL.raw(), -0x2bb2_afa8);
        assert_eq!(AvErrorCode::INPUT_CHANGED.raw(), -0x636e_6701);
        assert_eq!(AvErrorCode::OUTPUT_CHANGED.raw(), -0x636e_6702);
        assert_eq!(AvErrorCode::EINVAL, AvErrorCode::from_posix_errno(22));
        assert_eq!(AvErrorCode::EINVAL.raw(), -22);
        assert_eq!(AvErrorCode::ENOENT.raw(), -2);
        assert_eq!(AvErrorCode::ENOSYS.raw(), -38);
        assert_eq!(
            AvErrorCode::INPUT_AND_OUTPUT_CHANGED.raw(),
            AvErrorCode::INPUT_CHANGED.raw() | AvErrorCode::OUTPUT_CHANGED.raw()
        );
        assert_eq!(
            AvErrorCode::from_raw(AvErrorCode::HTTP_TOO_MANY_REQUESTS.raw()),
            AvErrorCode::HTTP_TOO_MANY_REQUESTS
        );
        assert!(AvErrorCode::INVALIDDATA.raw() < 0);
    }

    #[test]
    fn custom_error_codes_are_preserved() {
        let err = AvError::with_code(
            AvErrorKind::NotFound,
            AvErrorCode::STREAM_NOT_FOUND,
            "missing stream",
        );

        assert_eq!(err.kind(), AvErrorKind::NotFound);
        assert_eq!(err.code(), Some(AvErrorCode::STREAM_NOT_FOUND));
        assert_eq!(err.message(), "missing stream");
        assert_eq!(err.io_kind(), None);
    }

    #[test]
    fn error_descriptions_match_ffmpeg_defined_table() {
        let cases = [
            (AvErrorCode::BSF_NOT_FOUND, "Bitstream filter not found"),
            (AvErrorCode::BUG, "Internal bug, should not have happened"),
            (AvErrorCode::BUFFER_TOO_SMALL, "Buffer too small"),
            (AvErrorCode::DECODER_NOT_FOUND, "Decoder not found"),
            (AvErrorCode::DEMUXER_NOT_FOUND, "Demuxer not found"),
            (AvErrorCode::ENCODER_NOT_FOUND, "Encoder not found"),
            (AvErrorCode::EOF, "End of file"),
            (AvErrorCode::EXIT, "Immediate exit requested"),
            (
                AvErrorCode::EXTERNAL,
                "Generic error in an external library",
            ),
            (AvErrorCode::FILTER_NOT_FOUND, "Filter not found"),
            (AvErrorCode::INPUT_CHANGED, "Input changed"),
            (
                AvErrorCode::INVALIDDATA,
                "Invalid data found when processing input",
            ),
            (AvErrorCode::MUXER_NOT_FOUND, "Muxer not found"),
            (AvErrorCode::OPTION_NOT_FOUND, "Option not found"),
            (AvErrorCode::OUTPUT_CHANGED, "Output changed"),
            (
                AvErrorCode::PATCHWELCOME,
                "Not yet implemented in FFmpeg, patches welcome",
            ),
            (AvErrorCode::PROTOCOL_NOT_FOUND, "Protocol not found"),
            (AvErrorCode::STREAM_NOT_FOUND, "Stream not found"),
            (AvErrorCode::UNKNOWN, "Unknown error occurred"),
            (AvErrorCode::EXPERIMENTAL, "Experimental feature"),
            (
                AvErrorCode::HTTP_BAD_REQUEST,
                "Server returned 400 Bad Request",
            ),
            (
                AvErrorCode::HTTP_UNAUTHORIZED,
                "Server returned 401 Unauthorized (authorization failed)",
            ),
            (
                AvErrorCode::HTTP_FORBIDDEN,
                "Server returned 403 Forbidden (access denied)",
            ),
            (AvErrorCode::HTTP_NOT_FOUND, "Server returned 404 Not Found"),
            (
                AvErrorCode::HTTP_TOO_MANY_REQUESTS,
                "Server returned 429 Too Many Requests",
            ),
            (
                AvErrorCode::HTTP_OTHER_4XX,
                "Server returned 4XX Client Error, but not one of 40{0,1,3,4}",
            ),
            (
                AvErrorCode::HTTP_SERVER_ERROR,
                "Server returned 5XX Server Error reply",
            ),
            (AvErrorCode::EPERM, "Operation not permitted"),
            (AvErrorCode::ENOENT, "No such file or directory"),
            (AvErrorCode::EINTR, "Interrupted system call"),
            (AvErrorCode::EIO, "Input/output error"),
            (AvErrorCode::EAGAIN, "Resource temporarily unavailable"),
            (AvErrorCode::ENOMEM, "Cannot allocate memory"),
            (AvErrorCode::EACCES, "Permission denied"),
            (AvErrorCode::EINVAL, "Invalid argument"),
            (AvErrorCode::EPIPE, "Broken pipe"),
            (AvErrorCode::ENOSYS, "Function not implemented"),
        ];

        for (code, description) in cases {
            assert_eq!(code.description(), Some(description));
        }
        assert_eq!(
            AvErrorCode::BUG2.description(),
            AvErrorCode::BUG.description()
        );
        assert_eq!(
            AvErrorCode::INPUT_AND_OUTPUT_CHANGED.description(),
            AvErrorCode::INPUT_CHANGED.description()
        );
        assert_eq!(AvErrorCode::from_raw(-123456).description(), None);
    }

    #[test]
    fn av_strerror_and_make_error_string_follow_ffmpeg_shape() {
        assert_eq!(
            av_strerror(AvErrorCode::OPTION_NOT_FOUND.raw()).unwrap(),
            "Option not found"
        );
        assert_eq!(
            AvErrorCode::EXTERNAL.make_error_string(),
            "Generic error in an external library"
        );
        assert_eq!(
            av_strerror(AvErrorCode::EINVAL.raw()).unwrap(),
            "Invalid argument"
        );
        assert_eq!(
            av_strerror(-123456).unwrap_err(),
            "Error number -123456 occurred"
        );
        assert_eq!(
            av_make_error_string(-123456),
            "Error number -123456 occurred"
        );
    }

    #[test]
    fn constructors_cover_common_error_kinds() {
        let errors = [
            (
                AvError::invalid_argument("bad option"),
                AvErrorKind::InvalidArgument,
                Some(AvErrorCode::EINVAL),
            ),
            (
                AvError::not_found("missing stream"),
                AvErrorKind::NotFound,
                Some(AvErrorCode::ENOENT),
            ),
            (
                AvError::end_of_file("truncated"),
                AvErrorKind::EndOfFile,
                Some(AvErrorCode::EOF),
            ),
            (
                AvError::unsupported("codec"),
                AvErrorKind::Unsupported,
                Some(AvErrorCode::ENOSYS),
            ),
            (
                AvError::external("system"),
                AvErrorKind::External,
                Some(AvErrorCode::EXTERNAL),
            ),
            (
                AvError::bug("invariant"),
                AvErrorKind::Bug,
                Some(AvErrorCode::BUG),
            ),
        ];

        for (error, kind, code) in errors {
            assert_eq!(error.kind(), kind);
            assert_eq!(error.code(), code);
            assert_eq!(error.io_kind(), None);
        }

        let invalid = AvError::invalid_data("bad packet");
        assert_eq!(
            invalid.ffmpeg_description(),
            Some("Invalid data found when processing input")
        );
        assert_eq!(
            invalid.ffmpeg_error_string(),
            Some("Invalid data found when processing input".to_string())
        );
        assert_eq!(
            AvError::not_found("missing").ffmpeg_description(),
            Some("No such file or directory")
        );
    }

    #[test]
    fn io_errors_map_to_stable_av_error_kinds() {
        let cases = [
            (
                io::ErrorKind::NotFound,
                AvErrorKind::NotFound,
                Some(AvErrorCode::ENOENT),
            ),
            (
                io::ErrorKind::UnexpectedEof,
                AvErrorKind::EndOfFile,
                Some(AvErrorCode::EOF),
            ),
            (
                io::ErrorKind::InvalidData,
                AvErrorKind::InvalidData,
                Some(AvErrorCode::INVALIDDATA),
            ),
            (
                io::ErrorKind::InvalidInput,
                AvErrorKind::InvalidArgument,
                Some(AvErrorCode::EINVAL),
            ),
            (
                io::ErrorKind::Interrupted,
                AvErrorKind::External,
                Some(AvErrorCode::EINTR),
            ),
            (
                io::ErrorKind::WouldBlock,
                AvErrorKind::External,
                Some(AvErrorCode::EAGAIN),
            ),
            (
                io::ErrorKind::BrokenPipe,
                AvErrorKind::External,
                Some(AvErrorCode::EPIPE),
            ),
            (
                io::ErrorKind::Unsupported,
                AvErrorKind::Unsupported,
                Some(AvErrorCode::ENOSYS),
            ),
            (
                io::ErrorKind::PermissionDenied,
                AvErrorKind::External,
                Some(AvErrorCode::EACCES),
            ),
        ];

        for (io_kind, av_kind, code) in cases {
            let err = AvError::from_io_error("AVIO", io::Error::new(io_kind, "source failure"));
            assert_eq!(err.kind(), av_kind);
            assert_eq!(err.code(), code);
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
