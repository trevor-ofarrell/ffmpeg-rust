use avutil::{AvError, AvErrorKind, AvResult, ChannelLayout, ChannelLayoutSpec, SampleFormat};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioStreamParameters {
    sample_rate: u32,
    channels: u16,
    channel_layout: Option<ChannelLayoutSpec>,
    sample_format: SampleFormat,
    bytes_per_sample_frame: usize,
}

impl AudioStreamParameters {
    pub fn new(sample_rate: u32, channels: u16, sample_format: SampleFormat) -> AvResult<Self> {
        Self::validate(
            sample_rate,
            channels,
            sample_format,
            AvErrorKind::InvalidArgument,
            "audio stream",
        )
    }

    pub fn from_container(
        sample_rate: u32,
        channels: u16,
        sample_format: SampleFormat,
        context: &'static str,
    ) -> AvResult<Self> {
        Self::validate(
            sample_rate,
            channels,
            sample_format,
            AvErrorKind::InvalidData,
            context,
        )
    }

    pub fn with_context(
        sample_rate: u32,
        channels: u16,
        sample_format: SampleFormat,
        context: &'static str,
    ) -> AvResult<Self> {
        Self::validate(
            sample_rate,
            channels,
            sample_format,
            AvErrorKind::InvalidArgument,
            context,
        )
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn channels(&self) -> u16 {
        self.channels
    }

    pub fn channel_layout(&self) -> Option<ChannelLayout> {
        self.channel_layout
            .as_ref()
            .and_then(ChannelLayoutSpec::as_native)
    }

    pub fn channel_layout_spec(&self) -> Option<ChannelLayoutSpec> {
        self.channel_layout.clone()
    }

    pub fn sample_format(&self) -> SampleFormat {
        self.sample_format
    }

    pub fn bytes_per_sample_frame(&self) -> usize {
        self.bytes_per_sample_frame
    }

    pub fn bits_per_sample(&self) -> AvResult<u16> {
        let bits = self
            .sample_format
            .bytes_per_sample()
            .checked_mul(8)
            .ok_or_else(|| AvError::invalid_argument("audio stream bits per sample overflow"))?;
        u16::try_from(bits)
            .map_err(|_| AvError::invalid_argument("audio stream bits per sample overflow"))
    }

    pub fn sample_frames_in_bytes(
        &self,
        byte_len: usize,
        error_kind: AvErrorKind,
        message: impl Into<String>,
    ) -> AvResult<usize> {
        if byte_len % self.bytes_per_sample_frame != 0 {
            return Err(audio_error(error_kind, message));
        }
        Ok(byte_len / self.bytes_per_sample_frame)
    }

    fn validate(
        sample_rate: u32,
        channels: u16,
        sample_format: SampleFormat,
        error_kind: AvErrorKind,
        context: &'static str,
    ) -> AvResult<Self> {
        if sample_rate == 0 {
            return Err(audio_error(
                error_kind,
                format!("{context} sample rate must be non-zero"),
            ));
        }
        if channels == 0 {
            return Err(audio_error(
                error_kind,
                format!("{context} channel count must be non-zero"),
            ));
        }

        let bytes_per_sample_frame = sample_format
            .bytes_per_sample()
            .checked_mul(usize::from(channels))
            .ok_or_else(|| {
                audio_error(error_kind, format!("{context} sample frame size overflow"))
            })?;

        Ok(Self {
            sample_rate,
            channels,
            channel_layout: Some(ChannelLayoutSpec::default_for_count(channels)?),
            sample_format,
            bytes_per_sample_frame,
        })
    }
}

fn audio_error(kind: AvErrorKind, message: impl Into<String>) -> AvError {
    AvError::new(kind, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_user_parameters_and_derives_audio_metadata() {
        let params = AudioStreamParameters::new(48_000, 2, SampleFormat::S16).unwrap();

        assert_eq!(params.sample_rate(), 48_000);
        assert_eq!(params.channels(), 2);
        assert_eq!(params.channel_layout(), Some(ChannelLayout::stereo()));
        assert_eq!(
            params.channel_layout_spec(),
            Some(ChannelLayoutSpec::Native(ChannelLayout::stereo()))
        );
        assert_eq!(params.sample_format(), SampleFormat::S16);
        assert_eq!(params.bytes_per_sample_frame(), 4);
        assert_eq!(params.bits_per_sample().unwrap(), 16);

        let unspecified = AudioStreamParameters::new(48_000, 9, SampleFormat::S16).unwrap();
        assert_eq!(unspecified.channels(), 9);
        assert_eq!(unspecified.channel_layout(), None);
        assert_eq!(
            unspecified.channel_layout_spec(),
            Some(ChannelLayoutSpec::unspecified(9).unwrap())
        );
        assert_eq!(
            unspecified.channel_layout_spec().unwrap().describe(),
            "9 channels"
        );
        assert_eq!(unspecified.bytes_per_sample_frame(), 18);
    }

    #[test]
    fn container_parameters_return_invalid_data_for_untrusted_fields() {
        let err =
            AudioStreamParameters::from_container(0, 2, SampleFormat::S16, "WAV").unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);
        assert_eq!(err.message(), "WAV sample rate must be non-zero");

        let err =
            AudioStreamParameters::from_container(48_000, 0, SampleFormat::S16, "WAV").unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);
        assert_eq!(err.message(), "WAV channel count must be non-zero");
    }

    #[test]
    fn counts_whole_sample_frames_in_byte_payloads() {
        let params = AudioStreamParameters::new(44_100, 1, SampleFormat::S16).unwrap();

        assert_eq!(
            params
                .sample_frames_in_bytes(6, AvErrorKind::InvalidData, "partial audio frame")
                .unwrap(),
            3
        );
        let err = params
            .sample_frames_in_bytes(5, AvErrorKind::EndOfFile, "partial audio frame")
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::EndOfFile);
        assert_eq!(err.message(), "partial audio frame");
    }
}
