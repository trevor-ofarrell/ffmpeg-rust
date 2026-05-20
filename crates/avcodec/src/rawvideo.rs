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
    fn decodes_gray16_packet_to_single_plane_frame() {
        let decoder = RawVideoDecoder::new(2, 1, PixelFormat::Gray16Le).unwrap();
        let mut packet = Packet::new(vec![0, 1, 2, 3], 0);
        packet.set_pts(Some(7));

        let frame = decoder.decode_packet(&packet).unwrap();

        assert_eq!(decoder.frame_size(), 4);
        assert_eq!(frame.pts(), Some(7));
        match frame.data() {
            FrameData::Video(video) => {
                assert_eq!(video.width(), 2);
                assert_eq!(video.height(), 1);
                assert_eq!(video.pixel_format(), PixelFormat::Gray16Le);
                assert_eq!(video.pixel_format_name(), "gray16le");
                assert_eq!(video.planes(), &[vec![0, 1, 2, 3]]);
            }
            FrameData::Audio(_) | FrameData::Empty => panic!("expected video frame"),
        }
    }

    #[test]
    fn decodes_ya8_packet_to_single_plane_frame() {
        let decoder = RawVideoDecoder::new(2, 1, PixelFormat::Ya8).unwrap();
        let mut packet = Packet::new(vec![0x10, 0xff, 0x80, 0x40], 0);
        packet.set_pts(Some(9));

        let frame = decoder.decode_packet(&packet).unwrap();

        assert_eq!(decoder.frame_size(), 4);
        assert_eq!(frame.pts(), Some(9));
        match frame.data() {
            FrameData::Video(video) => {
                assert_eq!(video.width(), 2);
                assert_eq!(video.height(), 1);
                assert_eq!(video.pixel_format(), PixelFormat::Ya8);
                assert_eq!(video.pixel_format_name(), "ya8");
                assert_eq!(video.planes(), &[vec![0x10, 0xff, 0x80, 0x40]]);
            }
            FrameData::Audio(_) | FrameData::Empty => panic!("expected video frame"),
        }
    }

    #[test]
    fn decodes_ya16_packet_to_single_plane_frame() {
        let decoder = RawVideoDecoder::new(2, 1, PixelFormat::Ya16Be).unwrap();
        let mut packet = Packet::new(vec![0x00, 0x10, 0xff, 0xff, 0x80, 0x00, 0x40, 0x00], 0);
        packet.set_pts(Some(10));

        let frame = decoder.decode_packet(&packet).unwrap();

        assert_eq!(decoder.frame_size(), 8);
        assert_eq!(frame.pts(), Some(10));
        match frame.data() {
            FrameData::Video(video) => {
                assert_eq!(video.width(), 2);
                assert_eq!(video.height(), 1);
                assert_eq!(video.pixel_format(), PixelFormat::Ya16Be);
                assert_eq!(video.pixel_format_name(), "ya16be");
                assert_eq!(
                    video.planes(),
                    &[vec![0x00, 0x10, 0xff, 0xff, 0x80, 0x00, 0x40, 0x00]]
                );
            }
            FrameData::Audio(_) | FrameData::Empty => panic!("expected video frame"),
        }
    }

    #[test]
    fn decodes_rgba64_packet_to_single_plane_frame() {
        let decoder = RawVideoDecoder::new(1, 1, PixelFormat::Rgba64Le).unwrap();
        let mut packet = Packet::new(vec![0, 1, 2, 3, 4, 5, 6, 7], 0);
        packet.set_pts(Some(11));

        let frame = decoder.decode_packet(&packet).unwrap();

        assert_eq!(decoder.frame_size(), 8);
        assert_eq!(frame.pts(), Some(11));
        match frame.data() {
            FrameData::Video(video) => {
                assert_eq!(video.width(), 1);
                assert_eq!(video.height(), 1);
                assert_eq!(video.pixel_format(), PixelFormat::Rgba64Le);
                assert_eq!(video.pixel_format_name(), "rgba64le");
                assert_eq!(video.planes(), &[vec![0, 1, 2, 3, 4, 5, 6, 7]]);
            }
            FrameData::Audio(_) | FrameData::Empty => panic!("expected video frame"),
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
    fn decodes_yuv422p_packet_to_three_planes() {
        let decoder = RawVideoDecoder::new(4, 3, PixelFormat::Yuv422p).unwrap();
        let packet = Packet::new((0..24).collect(), 0);

        let frame = decoder.decode_packet(&packet).unwrap();

        match frame.data() {
            FrameData::Video(video) => {
                assert_eq!(video.pixel_format(), PixelFormat::Yuv422p);
                assert_eq!(video.pixel_format_name(), "yuv422p");
                assert_eq!(video.planes()[0], (0..12).collect::<Vec<_>>());
                assert_eq!(video.planes()[1], (12..18).collect::<Vec<_>>());
                assert_eq!(video.planes()[2], (18..24).collect::<Vec<_>>());
            }
            FrameData::Audio(_) | FrameData::Empty => panic!("expected video frame"),
        }
    }

    #[test]
    fn decodes_yuv411p_packet_to_three_planes() {
        let decoder = RawVideoDecoder::new(4, 3, PixelFormat::Yuv411p).unwrap();
        let packet = Packet::new((0..18).collect(), 0);

        let frame = decoder.decode_packet(&packet).unwrap();

        match frame.data() {
            FrameData::Video(video) => {
                assert_eq!(video.pixel_format(), PixelFormat::Yuv411p);
                assert_eq!(video.pixel_format_name(), "yuv411p");
                assert_eq!(video.planes()[0], (0..12).collect::<Vec<_>>());
                assert_eq!(video.planes()[1], (12..15).collect::<Vec<_>>());
                assert_eq!(video.planes()[2], (15..18).collect::<Vec<_>>());
            }
            FrameData::Audio(_) | FrameData::Empty => panic!("expected video frame"),
        }
    }

    #[test]
    fn decodes_yuv410p_packet_to_three_planes() {
        let decoder = RawVideoDecoder::new(4, 4, PixelFormat::Yuv410p).unwrap();
        let packet = Packet::new((0..18).collect(), 0);

        let frame = decoder.decode_packet(&packet).unwrap();

        match frame.data() {
            FrameData::Video(video) => {
                assert_eq!(video.pixel_format(), PixelFormat::Yuv410p);
                assert_eq!(video.pixel_format_name(), "yuv410p");
                assert_eq!(video.planes()[0], (0..16).collect::<Vec<_>>());
                assert_eq!(video.planes()[1], vec![16]);
                assert_eq!(video.planes()[2], vec![17]);
            }
            FrameData::Audio(_) | FrameData::Empty => panic!("expected video frame"),
        }
    }

    #[test]
    fn decodes_yuv440p_packet_to_three_planes() {
        let decoder = RawVideoDecoder::new(3, 2, PixelFormat::Yuv440p).unwrap();
        let packet = Packet::new((0..12).collect(), 0);

        let frame = decoder.decode_packet(&packet).unwrap();

        match frame.data() {
            FrameData::Video(video) => {
                assert_eq!(video.pixel_format(), PixelFormat::Yuv440p);
                assert_eq!(video.pixel_format_name(), "yuv440p");
                assert_eq!(video.planes()[0], (0..6).collect::<Vec<_>>());
                assert_eq!(video.planes()[1], (6..9).collect::<Vec<_>>());
                assert_eq!(video.planes()[2], (9..12).collect::<Vec<_>>());
            }
            FrameData::Audio(_) | FrameData::Empty => panic!("expected video frame"),
        }
    }

    #[test]
    fn decodes_yuv444p_packet_to_three_planes() {
        let decoder = RawVideoDecoder::new(3, 2, PixelFormat::Yuv444p).unwrap();
        let packet = Packet::new((0..18).collect(), 0);

        let frame = decoder.decode_packet(&packet).unwrap();

        match frame.data() {
            FrameData::Video(video) => {
                assert_eq!(video.pixel_format(), PixelFormat::Yuv444p);
                assert_eq!(video.pixel_format_name(), "yuv444p");
                assert_eq!(video.planes()[0], (0..6).collect::<Vec<_>>());
                assert_eq!(video.planes()[1], (6..12).collect::<Vec<_>>());
                assert_eq!(video.planes()[2], (12..18).collect::<Vec<_>>());
            }
            FrameData::Audio(_) | FrameData::Empty => panic!("expected video frame"),
        }
    }

    #[test]
    fn validates_dimensions_and_packet_size() {
        assert!(RawVideoDecoder::new(0, 2, PixelFormat::Rgb24).is_err());
        assert!(RawVideoDecoder::new(3, 2, PixelFormat::Yuv420p).is_err());
        assert!(RawVideoDecoder::new(3, 2, PixelFormat::Yuv422p).is_err());
        assert!(RawVideoDecoder::new(3, 2, PixelFormat::Yuv411p).is_err());
        assert!(RawVideoDecoder::new(4, 3, PixelFormat::Yuv411p).is_ok());
        assert!(RawVideoDecoder::new(4, 3, PixelFormat::Yuv410p).is_err());
        assert!(RawVideoDecoder::new(2, 4, PixelFormat::Yuv410p).is_err());
        assert!(RawVideoDecoder::new(4, 4, PixelFormat::Yuv410p).is_ok());
        assert!(RawVideoDecoder::new(3, 3, PixelFormat::Yuv440p).is_err());
        assert!(RawVideoDecoder::new(3, 2, PixelFormat::Yuv440p).is_ok());
        assert!(RawVideoDecoder::new(3, 2, PixelFormat::Yuv444p).is_ok());

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
        let rgb48le = RawVideoDecoder::new(1, 1, PixelFormat::Rgb48Le).unwrap();
        let rgb48be = RawVideoDecoder::new(1, 1, PixelFormat::Rgb48Be).unwrap();
        let bgr48le = RawVideoDecoder::new(1, 1, PixelFormat::Bgr48Le).unwrap();
        let bgr48be = RawVideoDecoder::new(1, 1, PixelFormat::Bgr48Be).unwrap();
        let rgba64le = RawVideoDecoder::new(1, 1, PixelFormat::Rgba64Le).unwrap();
        let rgba64be = RawVideoDecoder::new(1, 1, PixelFormat::Rgba64Be).unwrap();
        let bgra64le = RawVideoDecoder::new(1, 1, PixelFormat::Bgra64Le).unwrap();
        let bgra64be = RawVideoDecoder::new(1, 1, PixelFormat::Bgra64Be).unwrap();
        let rgba = RawVideoDecoder::new(1, 1, PixelFormat::Rgba).unwrap();
        let bgra = RawVideoDecoder::new(1, 1, PixelFormat::Bgra).unwrap();
        let argb = RawVideoDecoder::new(1, 1, PixelFormat::Argb).unwrap();
        let abgr = RawVideoDecoder::new(1, 1, PixelFormat::Abgr).unwrap();
        let zero_rgb = RawVideoDecoder::new(1, 1, PixelFormat::ZeroRgb).unwrap();
        let rgb0 = RawVideoDecoder::new(1, 1, PixelFormat::Rgb0).unwrap();
        let zero_bgr = RawVideoDecoder::new(1, 1, PixelFormat::ZeroBgr).unwrap();
        let bgr0 = RawVideoDecoder::new(1, 1, PixelFormat::Bgr0).unwrap();

        assert_eq!(rgb.frame_size(), 6);
        assert_eq!(bgr.frame_size(), 6);
        assert_eq!(rgb48le.frame_size(), 6);
        assert_eq!(rgb48be.frame_size(), 6);
        assert_eq!(bgr48le.frame_size(), 6);
        assert_eq!(bgr48be.frame_size(), 6);
        assert_eq!(rgba64le.frame_size(), 8);
        assert_eq!(rgba64be.frame_size(), 8);
        assert_eq!(bgra64le.frame_size(), 8);
        assert_eq!(bgra64be.frame_size(), 8);
        assert_eq!(rgba.frame_size(), 4);
        assert_eq!(bgra.frame_size(), 4);
        assert_eq!(argb.frame_size(), 4);
        assert_eq!(abgr.frame_size(), 4);
        assert_eq!(zero_rgb.frame_size(), 4);
        assert_eq!(rgb0.frame_size(), 4);
        assert_eq!(zero_bgr.frame_size(), 4);
        assert_eq!(bgr0.frame_size(), 4);
        assert!(rgb
            .decode_packet(&Packet::new(vec![1, 2, 3, 4, 5, 6], 0))
            .is_ok());
        assert!(bgr
            .decode_packet(&Packet::new(vec![1, 2, 3, 4, 5, 6], 0))
            .is_ok());
        assert!(rgb48le
            .decode_packet(&Packet::new(vec![1, 2, 3, 4, 5, 6], 0))
            .is_ok());
        assert!(rgb48be
            .decode_packet(&Packet::new(vec![1, 2, 3, 4, 5, 6], 0))
            .is_ok());
        assert!(bgr48le
            .decode_packet(&Packet::new(vec![1, 2, 3, 4, 5, 6], 0))
            .is_ok());
        assert!(bgr48be
            .decode_packet(&Packet::new(vec![1, 2, 3, 4, 5, 6], 0))
            .is_ok());
        assert!(rgba64le
            .decode_packet(&Packet::new(vec![1, 2, 3, 4, 5, 6, 7, 8], 0))
            .is_ok());
        assert!(rgba64be
            .decode_packet(&Packet::new(vec![1, 2, 3, 4, 5, 6, 7, 8], 0))
            .is_ok());
        assert!(bgra64le
            .decode_packet(&Packet::new(vec![1, 2, 3, 4, 5, 6, 7, 8], 0))
            .is_ok());
        assert!(bgra64be
            .decode_packet(&Packet::new(vec![1, 2, 3, 4, 5, 6, 7, 8], 0))
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
        assert!(zero_rgb
            .decode_packet(&Packet::new(vec![1, 2, 3, 4], 0))
            .is_ok());
        assert!(rgb0
            .decode_packet(&Packet::new(vec![1, 2, 3, 4], 0))
            .is_ok());
        assert!(zero_bgr
            .decode_packet(&Packet::new(vec![1, 2, 3, 4], 0))
            .is_ok());
        assert!(bgr0
            .decode_packet(&Packet::new(vec![1, 2, 3, 4], 0))
            .is_ok());
    }
}
