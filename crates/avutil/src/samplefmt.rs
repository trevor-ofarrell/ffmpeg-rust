use crate::{AvError, AvResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SampleFormat {
    S16,
}

impl SampleFormat {
    pub fn name(self) -> &'static str {
        match self {
            Self::S16 => "s16",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "s16" => Some(Self::S16),
            _ => None,
        }
    }

    pub fn bytes_per_sample(self) -> usize {
        match self {
            Self::S16 => 2,
        }
    }

    pub fn is_planar(self) -> bool {
        false
    }

    pub fn plane_count(self, channels: u16) -> AvResult<usize> {
        validate_channels(channels)?;
        Ok(if self.is_planar() {
            usize::from(channels)
        } else {
            1
        })
    }

    pub fn bytes_per_sample_frame(self, channels: u16) -> AvResult<usize> {
        validate_channels(channels)?;
        self.bytes_per_sample()
            .checked_mul(usize::from(channels))
            .ok_or_else(|| AvError::invalid_argument("sample format frame size overflow"))
    }

    pub fn plane_sizes(self, samples_per_channel: usize, channels: u16) -> AvResult<Vec<usize>> {
        let plane_count = self.plane_count(channels)?;
        if self.is_planar() {
            let plane_size = samples_per_channel
                .checked_mul(self.bytes_per_sample())
                .ok_or_else(|| AvError::invalid_argument("sample format plane size overflow"))?;
            Ok(vec![plane_size; plane_count])
        } else {
            let frame_size = self.bytes_per_sample_frame(channels)?;
            let payload_size = samples_per_channel
                .checked_mul(frame_size)
                .ok_or_else(|| AvError::invalid_argument("sample format payload size overflow"))?;
            Ok(vec![payload_size])
        }
    }
}

fn validate_channels(channels: u16) -> AvResult<()> {
    if channels == 0 {
        return Err(AvError::invalid_argument(
            "sample format channel count must be non-zero",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AvErrorKind;

    #[test]
    fn sample_formats_report_ffmpeg_names_and_layout() {
        assert_eq!(SampleFormat::from_name("s16"), Some(SampleFormat::S16));
        assert_eq!(SampleFormat::from_name("s16p"), None);
        assert_eq!(SampleFormat::S16.name(), "s16");
        assert!(!SampleFormat::S16.is_planar());
        assert_eq!(SampleFormat::S16.plane_count(2).unwrap(), 1);
    }

    #[test]
    fn sample_formats_compute_packed_payload_sizes() {
        assert_eq!(SampleFormat::S16.bytes_per_sample(), 2);
        assert_eq!(SampleFormat::S16.bytes_per_sample_frame(1).unwrap(), 2);
        assert_eq!(SampleFormat::S16.bytes_per_sample_frame(6).unwrap(), 12);
        assert_eq!(SampleFormat::S16.plane_sizes(1024, 2).unwrap(), vec![4096]);
        assert_eq!(SampleFormat::S16.plane_sizes(0, 2).unwrap(), vec![0]);
    }

    #[test]
    fn sample_formats_reject_invalid_channel_counts() {
        assert_eq!(
            SampleFormat::S16
                .bytes_per_sample_frame(0)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidArgument
        );
    }
}
