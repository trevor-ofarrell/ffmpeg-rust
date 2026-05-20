pub use avutil::PixelFormat;
use avutil::{AvError, AvResult, Frame, Packet, VideoFrame};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawVideoDecoder {
    width: usize,
    height: usize,
    pixel_format: PixelFormat,
}

impl RawVideoDecoder {
    pub fn new(width: usize, height: usize, pixel_format: PixelFormat) -> AvResult<Self> {
        if width == 0 || height == 0 {
            return Err(AvError::invalid_argument(
                "rawvideo dimensions must be non-zero",
            ));
        }

        pixel_format.frame_size(width, height)?;
        Ok(Self {
            width,
            height,
            pixel_format,
        })
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn pixel_format(&self) -> PixelFormat {
        self.pixel_format
    }

    pub fn frame_size(&self) -> usize {
        self.pixel_format
            .frame_size(self.width, self.height)
            .expect("validated rawvideo frame geometry")
    }

    pub fn decode_packet(&self, packet: &Packet) -> AvResult<Frame> {
        let expected = self.frame_size();
        if packet.data().len() != expected {
            return Err(AvError::invalid_data(format!(
                "rawvideo packet has {} bytes, expected {expected}",
                packet.data().len()
            )));
        }

        let planes = self
            .pixel_format
            .split_planes(packet.data(), self.width, self.height)?;
        let video = VideoFrame::new(self.width, self.height, self.pixel_format, planes)?;
        let mut frame = Frame::video(video);
        frame.set_pts(packet.pts());
        Ok(frame)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use avutil::{AvErrorKind, FrameData};

    #[test]
    fn decodes_gray8_packet_to_single_plane_frame() {
        let decoder = RawVideoDecoder::new(2, 2, PixelFormat::Gray8).unwrap();
        let mut packet = Packet::new(vec![0, 1, 2, 3], 0);
        packet.set_pts(Some(42));

        let frame = decoder.decode_packet(&packet).unwrap();

        assert_eq!(frame.pts(), Some(42));
        match frame.data() {
            FrameData::Video(video) => {
                assert_eq!(video.width(), 2);
                assert_eq!(video.height(), 2);
                assert_eq!(video.pixel_format(), PixelFormat::Gray8);
                assert_eq!(video.pixel_format_name(), "gray");
                assert_eq!(video.planes(), &[vec![0, 1, 2, 3]]);
            }
            FrameData::Audio(_) => panic!("expected video frame"),
            FrameData::Empty => panic!("expected video frame"),
        }
    }

    #[test]
    fn decodes_yuv420p_packet_to_three_planes() {
        let decoder = RawVideoDecoder::new(4, 2, PixelFormat::Yuv420p).unwrap();
        let packet = Packet::new(vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11], 0);

        let frame = decoder.decode_packet(&packet).unwrap();

        match frame.data() {
            FrameData::Video(video) => {
                assert_eq!(video.pixel_format(), PixelFormat::Yuv420p);
                assert_eq!(video.pixel_format_name(), "yuv420p");
                assert_eq!(video.planes()[0], vec![0, 1, 2, 3, 4, 5, 6, 7]);
                assert_eq!(video.planes()[1], vec![8, 9]);
                assert_eq!(video.planes()[2], vec![10, 11]);
            }
            FrameData::Audio(_) => panic!("expected video frame"),
            FrameData::Empty => panic!("expected video frame"),
        }
    }

    #[test]
    fn validates_dimensions_and_packet_size() {
        assert!(RawVideoDecoder::new(0, 2, PixelFormat::Rgb24).is_err());
        assert!(RawVideoDecoder::new(3, 2, PixelFormat::Yuv420p).is_err());

        let decoder = RawVideoDecoder::new(2, 2, PixelFormat::Rgb24).unwrap();
        assert_eq!(decoder.frame_size(), 12);
        let short = Packet::new(vec![0; 11], 0);
        let long = Packet::new(vec![0; 13], 0);

        assert_eq!(
            decoder.decode_packet(&short).unwrap_err().kind(),
            AvErrorKind::InvalidData
        );
        assert_eq!(
            decoder.decode_packet(&long).unwrap_err().kind(),
            AvErrorKind::InvalidData
        );
    }

    #[test]
    fn packed_rgb_and_rgba_use_single_payload_plane() {
        let rgb = RawVideoDecoder::new(1, 2, PixelFormat::Rgb24).unwrap();
        let bgr = RawVideoDecoder::new(1, 2, PixelFormat::Bgr24).unwrap();
        let rgba = RawVideoDecoder::new(1, 1, PixelFormat::Rgba).unwrap();
        let bgra = RawVideoDecoder::new(1, 1, PixelFormat::Bgra).unwrap();
        let argb = RawVideoDecoder::new(1, 1, PixelFormat::Argb).unwrap();
        let abgr = RawVideoDecoder::new(1, 1, PixelFormat::Abgr).unwrap();

        assert_eq!(rgb.frame_size(), 6);
        assert_eq!(bgr.frame_size(), 6);
        assert_eq!(rgba.frame_size(), 4);
        assert_eq!(bgra.frame_size(), 4);
        assert_eq!(argb.frame_size(), 4);
        assert_eq!(abgr.frame_size(), 4);
        assert!(rgb
            .decode_packet(&Packet::new(vec![1, 2, 3, 4, 5, 6], 0))
            .is_ok());
        assert!(bgr
            .decode_packet(&Packet::new(vec![1, 2, 3, 4, 5, 6], 0))
            .is_ok());
        assert!(rgba
            .decode_packet(&Packet::new(vec![1, 2, 3, 4], 0))
            .is_ok());
        assert!(bgra
            .decode_packet(&Packet::new(vec![1, 2, 3, 4], 0))
            .is_ok());
        assert!(argb
            .decode_packet(&Packet::new(vec![1, 2, 3, 4], 0))
            .is_ok());
        assert!(abgr
            .decode_packet(&Packet::new(vec![1, 2, 3, 4], 0))
            .is_ok());
    }
}
