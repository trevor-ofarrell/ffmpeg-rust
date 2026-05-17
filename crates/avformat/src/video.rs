use avutil::{AvError, AvErrorKind, AvResult, PixelFormat};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VideoStreamParameters {
    width: usize,
    height: usize,
    pixel_format: PixelFormat,
    frame_size: usize,
}

impl VideoStreamParameters {
    pub fn new(width: usize, height: usize, pixel_format: PixelFormat) -> AvResult<Self> {
        Self::validate(
            width,
            height,
            pixel_format,
            AvErrorKind::InvalidArgument,
            "video stream",
        )
    }

    pub fn with_context(
        width: usize,
        height: usize,
        pixel_format: PixelFormat,
        context: &'static str,
    ) -> AvResult<Self> {
        Self::validate(
            width,
            height,
            pixel_format,
            AvErrorKind::InvalidArgument,
            context,
        )
    }

    pub fn from_container(
        width: usize,
        height: usize,
        pixel_format: PixelFormat,
        context: &'static str,
    ) -> AvResult<Self> {
        Self::validate(
            width,
            height,
            pixel_format,
            AvErrorKind::InvalidData,
            context,
        )
    }

    pub fn from_u32_with_context(
        width: u32,
        height: u32,
        pixel_format: PixelFormat,
        context: &'static str,
    ) -> AvResult<Self> {
        Self::validate(
            usize::try_from(width).map_err(|_| {
                video_error(
                    AvErrorKind::InvalidArgument,
                    format!("{context} width is out of range"),
                )
            })?,
            usize::try_from(height).map_err(|_| {
                video_error(
                    AvErrorKind::InvalidArgument,
                    format!("{context} height is out of range"),
                )
            })?,
            pixel_format,
            AvErrorKind::InvalidArgument,
            context,
        )
    }

    pub fn width(self) -> usize {
        self.width
    }

    pub fn height(self) -> usize {
        self.height
    }

    pub fn pixel_format(self) -> PixelFormat {
        self.pixel_format
    }

    pub fn frame_size(self) -> usize {
        self.frame_size
    }

    pub fn frame_count_in_bytes(
        self,
        byte_len: usize,
        error_kind: AvErrorKind,
        message: impl Into<String>,
    ) -> AvResult<usize> {
        if byte_len % self.frame_size != 0 {
            return Err(video_error(error_kind, message));
        }
        Ok(byte_len / self.frame_size)
    }

    pub fn validate_frame_payload_len(
        self,
        byte_len: usize,
        error_kind: AvErrorKind,
        context: &'static str,
    ) -> AvResult<()> {
        if byte_len != self.frame_size {
            return Err(video_error(
                error_kind,
                format!(
                    "{context} has {byte_len} bytes, expected {}",
                    self.frame_size
                ),
            ));
        }
        Ok(())
    }

    fn validate(
        width: usize,
        height: usize,
        pixel_format: PixelFormat,
        error_kind: AvErrorKind,
        context: &'static str,
    ) -> AvResult<Self> {
        if width == 0 || height == 0 {
            return Err(video_error(
                error_kind,
                format!("{context} dimensions must be non-zero"),
            ));
        }

        let frame_size = pixel_format
            .frame_size(width, height)
            .map_err(|err| video_error(error_kind, err.message().to_owned()))?;
        if frame_size == 0 {
            return Err(video_error(
                error_kind,
                format!("{context} frame size must be non-zero"),
            ));
        }

        Ok(Self {
            width,
            height,
            pixel_format,
            frame_size,
        })
    }
}

fn video_error(kind: AvErrorKind, message: impl Into<String>) -> AvError {
    AvError::new(kind, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_user_parameters_and_derives_frame_size() {
        let params = VideoStreamParameters::new(2, 2, PixelFormat::Rgb24).unwrap();

        assert_eq!(params.width(), 2);
        assert_eq!(params.height(), 2);
        assert_eq!(params.pixel_format(), PixelFormat::Rgb24);
        assert_eq!(params.frame_size(), 12);
    }

    #[test]
    fn container_parameters_return_invalid_data_for_untrusted_fields() {
        let err = VideoStreamParameters::from_container(0, 2, PixelFormat::Gray8, "rawvideo")
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);
        assert_eq!(err.message(), "rawvideo dimensions must be non-zero");

        let err = VideoStreamParameters::from_container(3, 2, PixelFormat::Yuv420p, "rawvideo")
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);
    }

    #[test]
    fn counts_complete_frames_and_validates_exact_payloads() {
        let params = VideoStreamParameters::new(1, 2, PixelFormat::Rgb24).unwrap();

        assert_eq!(
            params
                .frame_count_in_bytes(12, AvErrorKind::EndOfFile, "partial rawvideo frame")
                .unwrap(),
            2
        );
        assert_eq!(
            params
                .frame_count_in_bytes(11, AvErrorKind::EndOfFile, "partial rawvideo frame")
                .unwrap_err()
                .kind(),
            AvErrorKind::EndOfFile
        );
        assert!(params
            .validate_frame_payload_len(6, AvErrorKind::InvalidData, "rawvideo packet")
            .is_ok());
        let err = params
            .validate_frame_payload_len(5, AvErrorKind::InvalidData, "rawvideo packet")
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);
        assert_eq!(err.message(), "rawvideo packet has 5 bytes, expected 6");
    }
}
