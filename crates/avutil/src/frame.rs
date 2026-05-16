use crate::{AvError, AvResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameData {
    Video(VideoFrame),
    Audio(AudioFrame),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pts: Option<i64>,
    data: FrameData,
}

impl Frame {
    pub fn video(frame: VideoFrame) -> Self {
        Self {
            pts: None,
            data: FrameData::Video(frame),
        }
    }

    pub fn audio(frame: AudioFrame) -> Self {
        Self {
            pts: None,
            data: FrameData::Audio(frame),
        }
    }

    pub fn pts(&self) -> Option<i64> {
        self.pts
    }

    pub fn set_pts(&mut self, pts: Option<i64>) {
        self.pts = pts;
    }

    pub fn data(&self) -> &FrameData {
        &self.data
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoFrame {
    width: usize,
    height: usize,
    pixel_format: String,
    planes: Vec<Vec<u8>>,
}

impl VideoFrame {
    pub fn new(
        width: usize,
        height: usize,
        pixel_format: impl Into<String>,
        planes: Vec<Vec<u8>>,
    ) -> AvResult<Self> {
        if width == 0 || height == 0 {
            return Err(AvError::invalid_argument(
                "video frame dimensions must be non-zero",
            ));
        }

        let pixel_format = pixel_format.into();
        if pixel_format.trim().is_empty() {
            return Err(AvError::invalid_argument(
                "video pixel format must not be empty",
            ));
        }

        if planes.is_empty() {
            return Err(AvError::invalid_argument(
                "video frame must contain at least one plane",
            ));
        }

        Ok(Self {
            width,
            height,
            pixel_format,
            planes,
        })
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn pixel_format(&self) -> &str {
        &self.pixel_format
    }

    pub fn planes(&self) -> &[Vec<u8>] {
        &self.planes
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioFrame {
    sample_rate: u32,
    channels: u16,
    sample_format: String,
    samples_per_channel: usize,
    planes: Vec<Vec<u8>>,
}

impl AudioFrame {
    pub fn new(
        sample_rate: u32,
        channels: u16,
        sample_format: impl Into<String>,
        samples_per_channel: usize,
        planes: Vec<Vec<u8>>,
    ) -> AvResult<Self> {
        if sample_rate == 0 {
            return Err(AvError::invalid_argument(
                "audio sample rate must be non-zero",
            ));
        }

        if channels == 0 {
            return Err(AvError::invalid_argument(
                "audio channel count must be non-zero",
            ));
        }

        let sample_format = sample_format.into();
        if sample_format.trim().is_empty() {
            return Err(AvError::invalid_argument(
                "audio sample format must not be empty",
            ));
        }

        if planes.is_empty() {
            return Err(AvError::invalid_argument(
                "audio frame must contain at least one plane",
            ));
        }

        Ok(Self {
            sample_rate,
            channels,
            sample_format,
            samples_per_channel,
            planes,
        })
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn channels(&self) -> u16 {
        self.channels
    }

    pub fn sample_format(&self) -> &str {
        &self.sample_format
    }

    pub fn samples_per_channel(&self) -> usize {
        self.samples_per_channel
    }

    pub fn planes(&self) -> &[Vec<u8>] {
        &self.planes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn video_frame_validates_required_shape() {
        assert!(VideoFrame::new(0, 1, "yuv420p", vec![vec![0]]).is_err());
        assert!(VideoFrame::new(1, 1, "", vec![vec![0]]).is_err());
        assert!(VideoFrame::new(1, 1, "gray", Vec::new()).is_err());

        let frame = VideoFrame::new(2, 2, "gray", vec![vec![0, 1, 2, 3]]).unwrap();
        assert_eq!(frame.width(), 2);
        assert_eq!(frame.height(), 2);
        assert_eq!(frame.pixel_format(), "gray");
    }

    #[test]
    fn audio_frame_validates_required_shape() {
        assert!(AudioFrame::new(0, 2, "s16", 1, vec![vec![0]]).is_err());
        assert!(AudioFrame::new(48_000, 0, "s16", 1, vec![vec![0]]).is_err());
        assert!(AudioFrame::new(48_000, 2, "", 1, vec![vec![0]]).is_err());

        let frame = AudioFrame::new(48_000, 2, "s16", 1024, vec![vec![0; 4096]]).unwrap();
        assert_eq!(frame.sample_rate(), 48_000);
        assert_eq!(frame.channels(), 2);
        assert_eq!(frame.samples_per_channel(), 1024);
    }

    #[test]
    fn frame_wraps_audio_or_video_with_optional_pts() {
        let video = VideoFrame::new(1, 1, "gray", vec![vec![0]]).unwrap();
        let mut frame = Frame::video(video);

        assert_eq!(frame.pts(), None);
        frame.set_pts(Some(42));
        assert_eq!(frame.pts(), Some(42));
        assert!(matches!(frame.data(), FrameData::Video(_)));
    }
}
