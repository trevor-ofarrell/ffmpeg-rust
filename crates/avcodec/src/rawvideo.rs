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
    fn decodes_high_bit_depth_gray_packet_to_single_plane_frame() {
        let decoder = RawVideoDecoder::new(2, 1, PixelFormat::Gray10Le).unwrap();
        let mut packet = Packet::new(vec![0x01, 0x02, 0x03, 0x04], 0);
        packet.set_pts(Some(8));

        let frame = decoder.decode_packet(&packet).unwrap();

        assert_eq!(decoder.frame_size(), 4);
        assert_eq!(frame.pts(), Some(8));
        match frame.data() {
            FrameData::Video(video) => {
                assert_eq!(video.width(), 2);
                assert_eq!(video.height(), 1);
                assert_eq!(video.pixel_format(), PixelFormat::Gray10Le);
                assert_eq!(video.pixel_format_name(), "gray10le");
                assert_eq!(video.planes(), &[vec![0x01, 0x02, 0x03, 0x04]]);
            }
            FrameData::Audio(_) | FrameData::Empty => panic!("expected video frame"),
        }
    }

    #[test]
    fn decodes_gray32_packet_to_single_plane_frame() {
        let decoder = RawVideoDecoder::new(2, 1, PixelFormat::Gray32Be).unwrap();
        let mut packet = Packet::new(vec![0, 1, 2, 3, 4, 5, 6, 7], 0);
        packet.set_pts(Some(9));

        let frame = decoder.decode_packet(&packet).unwrap();

        assert_eq!(decoder.frame_size(), 8);
        assert_eq!(frame.pts(), Some(9));
        match frame.data() {
            FrameData::Video(video) => {
                assert_eq!(video.width(), 2);
                assert_eq!(video.height(), 1);
                assert_eq!(video.pixel_format(), PixelFormat::Gray32Be);
                assert_eq!(video.pixel_format_name(), "gray32be");
                assert_eq!(video.planes(), &[vec![0, 1, 2, 3, 4, 5, 6, 7]]);
            }
            FrameData::Audio(_) | FrameData::Empty => panic!("expected video frame"),
        }
    }

    #[test]
    fn decodes_gray_float_packets_to_single_plane_frames() {
        let grayf16 = RawVideoDecoder::new(2, 1, PixelFormat::GrayF16Le).unwrap();
        let mut packet = Packet::new(vec![0, 1, 2, 3], 0);
        packet.set_pts(Some(10));

        let frame = grayf16.decode_packet(&packet).unwrap();

        assert_eq!(grayf16.frame_size(), 4);
        assert_eq!(frame.pts(), Some(10));
        match frame.data() {
            FrameData::Video(video) => {
                assert_eq!(video.width(), 2);
                assert_eq!(video.height(), 1);
                assert_eq!(video.pixel_format(), PixelFormat::GrayF16Le);
                assert_eq!(video.pixel_format_name(), "grayf16le");
                assert_eq!(video.planes(), &[vec![0, 1, 2, 3]]);
            }
            FrameData::Audio(_) | FrameData::Empty => panic!("expected video frame"),
        }

        let grayf32 = RawVideoDecoder::new(2, 1, PixelFormat::GrayF32Be).unwrap();
        let frame = grayf32
            .decode_packet(&Packet::new(vec![4, 5, 6, 7, 8, 9, 10, 11], 0))
            .unwrap();

        match frame.data() {
            FrameData::Video(video) => {
                assert_eq!(video.pixel_format(), PixelFormat::GrayF32Be);
                assert_eq!(video.pixel_format_name(), "grayf32be");
                assert_eq!(video.planes(), &[vec![4, 5, 6, 7, 8, 9, 10, 11]]);
            }
            FrameData::Audio(_) | FrameData::Empty => panic!("expected video frame"),
        }
    }

    #[test]
    fn decodes_gbrp_packet_to_three_planes() {
        let decoder = RawVideoDecoder::new(2, 1, PixelFormat::Gbrp).unwrap();
        let mut packet = Packet::new(vec![0, 1, 2, 3, 4, 5], 0);
        packet.set_pts(Some(12));

        let frame = decoder.decode_packet(&packet).unwrap();

        assert_eq!(decoder.frame_size(), 6);
        assert_eq!(frame.pts(), Some(12));
        match frame.data() {
            FrameData::Video(video) => {
                assert_eq!(video.width(), 2);
                assert_eq!(video.height(), 1);
                assert_eq!(video.pixel_format(), PixelFormat::Gbrp);
                assert_eq!(video.pixel_format_name(), "gbrp");
                assert_eq!(video.planes(), &[vec![0, 1], vec![2, 3], vec![4, 5]]);
            }
            FrameData::Audio(_) | FrameData::Empty => panic!("expected video frame"),
        }
    }

    #[test]
    fn decodes_gbrp9_packet_to_three_planes() {
        let decoder = RawVideoDecoder::new(2, 1, PixelFormat::Gbrp9Le).unwrap();
        let mut packet = Packet::new((0..12).collect(), 0);
        packet.set_pts(Some(13));

        let frame = decoder.decode_packet(&packet).unwrap();

        assert_eq!(decoder.frame_size(), 12);
        assert_eq!(frame.pts(), Some(13));
        match frame.data() {
            FrameData::Video(video) => {
                assert_eq!(video.width(), 2);
                assert_eq!(video.height(), 1);
                assert_eq!(video.pixel_format(), PixelFormat::Gbrp9Le);
                assert_eq!(video.pixel_format_name(), "gbrp9le");
                assert_eq!(
                    video.planes(),
                    &[vec![0, 1, 2, 3], vec![4, 5, 6, 7], vec![8, 9, 10, 11]]
                );
            }
            FrameData::Audio(_) | FrameData::Empty => panic!("expected video frame"),
        }
    }

    #[test]
    fn decodes_gbrp10_packet_to_three_planes() {
        let decoder = RawVideoDecoder::new(2, 1, PixelFormat::Gbrp10Le).unwrap();
        let mut packet = Packet::new((0..12).collect(), 0);
        packet.set_pts(Some(14));

        let frame = decoder.decode_packet(&packet).unwrap();

        assert_eq!(decoder.frame_size(), 12);
        assert_eq!(frame.pts(), Some(14));
        match frame.data() {
            FrameData::Video(video) => {
                assert_eq!(video.width(), 2);
                assert_eq!(video.height(), 1);
                assert_eq!(video.pixel_format(), PixelFormat::Gbrp10Le);
                assert_eq!(video.pixel_format_name(), "gbrp10le");
                assert_eq!(
                    video.planes(),
                    &[vec![0, 1, 2, 3], vec![4, 5, 6, 7], vec![8, 9, 10, 11]]
                );
            }
            FrameData::Audio(_) | FrameData::Empty => panic!("expected video frame"),
        }
    }

    #[test]
    fn decodes_gbrp12_packet_to_three_planes() {
        let decoder = RawVideoDecoder::new(2, 1, PixelFormat::Gbrp12Le).unwrap();
        let mut packet = Packet::new((0..12).collect(), 0);
        packet.set_pts(Some(15));

        let frame = decoder.decode_packet(&packet).unwrap();

        assert_eq!(decoder.frame_size(), 12);
        assert_eq!(frame.pts(), Some(15));
        match frame.data() {
            FrameData::Video(video) => {
                assert_eq!(video.width(), 2);
                assert_eq!(video.height(), 1);
                assert_eq!(video.pixel_format(), PixelFormat::Gbrp12Le);
                assert_eq!(video.pixel_format_name(), "gbrp12le");
                assert_eq!(
                    video.planes(),
                    &[vec![0, 1, 2, 3], vec![4, 5, 6, 7], vec![8, 9, 10, 11]]
                );
            }
            FrameData::Audio(_) | FrameData::Empty => panic!("expected video frame"),
        }
    }

    #[test]
    fn decodes_gbrp14_packet_to_three_planes() {
        let decoder = RawVideoDecoder::new(2, 1, PixelFormat::Gbrp14Le).unwrap();
        let mut packet = Packet::new((0..12).collect(), 0);
        packet.set_pts(Some(16));

        let frame = decoder.decode_packet(&packet).unwrap();

        assert_eq!(decoder.frame_size(), 12);
        assert_eq!(frame.pts(), Some(16));
        match frame.data() {
            FrameData::Video(video) => {
                assert_eq!(video.width(), 2);
                assert_eq!(video.height(), 1);
                assert_eq!(video.pixel_format(), PixelFormat::Gbrp14Le);
                assert_eq!(video.pixel_format_name(), "gbrp14le");
                assert_eq!(
                    video.planes(),
                    &[vec![0, 1, 2, 3], vec![4, 5, 6, 7], vec![8, 9, 10, 11]]
                );
            }
            FrameData::Audio(_) | FrameData::Empty => panic!("expected video frame"),
        }
    }

    #[test]
    fn decodes_gbrp16_packet_to_three_planes() {
        let decoder = RawVideoDecoder::new(2, 1, PixelFormat::Gbrp16Le).unwrap();
        let mut packet = Packet::new((0..12).collect(), 0);
        packet.set_pts(Some(17));

        let frame = decoder.decode_packet(&packet).unwrap();

        assert_eq!(decoder.frame_size(), 12);
        assert_eq!(frame.pts(), Some(17));
        match frame.data() {
            FrameData::Video(video) => {
                assert_eq!(video.width(), 2);
                assert_eq!(video.height(), 1);
                assert_eq!(video.pixel_format(), PixelFormat::Gbrp16Le);
                assert_eq!(video.pixel_format_name(), "gbrp16le");
                assert_eq!(
                    video.planes(),
                    &[vec![0, 1, 2, 3], vec![4, 5, 6, 7], vec![8, 9, 10, 11]]
                );
            }
            FrameData::Audio(_) | FrameData::Empty => panic!("expected video frame"),
        }
    }

    #[test]
    fn decodes_gbrpf_packet_to_three_planes() {
        let decoder = RawVideoDecoder::new(2, 1, PixelFormat::GbrpF16Le).unwrap();
        let mut packet = Packet::new((0..12).collect(), 0);
        packet.set_pts(Some(171));

        let frame = decoder.decode_packet(&packet).unwrap();

        assert_eq!(decoder.frame_size(), 12);
        assert_eq!(frame.pts(), Some(171));
        match frame.data() {
            FrameData::Video(video) => {
                assert_eq!(video.width(), 2);
                assert_eq!(video.height(), 1);
                assert_eq!(video.pixel_format(), PixelFormat::GbrpF16Le);
                assert_eq!(video.pixel_format_name(), "gbrpf16le");
                assert_eq!(
                    video.planes(),
                    &[vec![0, 1, 2, 3], vec![4, 5, 6, 7], vec![8, 9, 10, 11]]
                );
            }
            FrameData::Audio(_) | FrameData::Empty => panic!("expected video frame"),
        }

        let gbrpf32 = RawVideoDecoder::new(1, 1, PixelFormat::GbrpF32Be).unwrap();
        assert_eq!(gbrpf32.frame_size(), 12);
        assert!(gbrpf32
            .decode_packet(&Packet::new((0..12).collect(), 0))
            .is_ok());
        assert_eq!(
            gbrpf32
                .decode_packet(&Packet::new((0..11).collect(), 0))
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
    }

    #[test]
    fn decodes_gbrap_packet_to_four_planes() {
        let decoder = RawVideoDecoder::new(2, 1, PixelFormat::Gbrap).unwrap();
        let mut packet = Packet::new((0..8).collect(), 0);
        packet.set_pts(Some(18));

        let frame = decoder.decode_packet(&packet).unwrap();

        assert_eq!(decoder.frame_size(), 8);
        assert_eq!(frame.pts(), Some(18));
        match frame.data() {
            FrameData::Video(video) => {
                assert_eq!(video.width(), 2);
                assert_eq!(video.height(), 1);
                assert_eq!(video.pixel_format(), PixelFormat::Gbrap);
                assert_eq!(video.pixel_format_name(), "gbrap");
                assert_eq!(
                    video.planes(),
                    &[vec![0, 1], vec![2, 3], vec![4, 5], vec![6, 7]]
                );
            }
            FrameData::Audio(_) | FrameData::Empty => panic!("expected video frame"),
        }
    }

    #[test]
    fn decodes_gbrapf32_packet_to_four_planes() {
        let decoder = RawVideoDecoder::new(1, 1, PixelFormat::GbrapF32Le).unwrap();
        let mut packet = Packet::new((0..16).collect(), 0);
        packet.set_pts(Some(19));

        let frame = decoder.decode_packet(&packet).unwrap();

        assert_eq!(decoder.frame_size(), 16);
        assert_eq!(frame.pts(), Some(19));
        match frame.data() {
            FrameData::Video(video) => {
                assert_eq!(video.width(), 1);
                assert_eq!(video.height(), 1);
                assert_eq!(video.pixel_format(), PixelFormat::GbrapF32Le);
                assert_eq!(video.pixel_format_name(), "gbrapf32le");
                assert_eq!(
                    video.planes(),
                    &[
                        vec![0, 1, 2, 3],
                        vec![4, 5, 6, 7],
                        vec![8, 9, 10, 11],
                        vec![12, 13, 14, 15]
                    ]
                );
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
    fn decodes_yaf_packets_to_single_plane_frames() {
        let yaf16 = RawVideoDecoder::new(2, 1, PixelFormat::Yaf16Le).unwrap();
        let mut packet = Packet::new((0..8).collect(), 0);
        packet.set_pts(Some(25));

        let frame = yaf16.decode_packet(&packet).unwrap();

        assert_eq!(yaf16.frame_size(), 8);
        match frame.data() {
            FrameData::Video(video) => {
                assert_eq!(video.pixel_format(), PixelFormat::Yaf16Le);
                assert_eq!(video.line_sizes(), &[8]);
                assert_eq!(video.planes(), &[vec![0, 1, 2, 3, 4, 5, 6, 7]]);
            }
            FrameData::Audio(_) | FrameData::Empty => panic!("expected video frame"),
        }

        let yaf32 = RawVideoDecoder::new(1, 1, PixelFormat::Yaf32Be).unwrap();
        let frame = yaf32
            .decode_packet(&Packet::new((8..16).collect(), 0))
            .unwrap();

        assert_eq!(yaf32.frame_size(), 8);
        match frame.data() {
            FrameData::Video(video) => {
                assert_eq!(video.pixel_format(), PixelFormat::Yaf32Be);
                assert_eq!(video.line_sizes(), &[8]);
                assert_eq!(video.planes(), &[vec![8, 9, 10, 11, 12, 13, 14, 15]]);
            }
            FrameData::Audio(_) | FrameData::Empty => panic!("expected video frame"),
        }

        assert_eq!(
            yaf32
                .decode_packet(&Packet::new(vec![0; 7], 0))
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
    }

    #[test]
    fn decodes_xv_packed_yuv_packets_to_single_plane_frames() {
        for (pixel_format, expected_name, width, height, payload, expected_line_size) in [
            (
                PixelFormat::Xv30Le,
                "xv30le",
                2,
                1,
                (0..8).collect::<Vec<_>>(),
                8,
            ),
            (
                PixelFormat::Xv36Be,
                "xv36be",
                2,
                1,
                (8..20).collect::<Vec<_>>(),
                12,
            ),
            (
                PixelFormat::Xv48Le,
                "xv48le",
                1,
                2,
                (20..36).collect::<Vec<_>>(),
                8,
            ),
            (
                PixelFormat::V30xBe,
                "v30xbe",
                2,
                1,
                (36..44).collect::<Vec<_>>(),
                8,
            ),
        ] {
            let decoder = RawVideoDecoder::new(width, height, pixel_format).unwrap();
            let mut packet = Packet::new(payload.clone(), 0);
            packet.set_pts(Some(29));

            let frame = decoder.decode_packet(&packet).unwrap();

            assert_eq!(decoder.frame_size(), payload.len());
            assert_eq!(frame.pts(), Some(29));
            match frame.data() {
                FrameData::Video(video) => {
                    assert_eq!(video.width(), width);
                    assert_eq!(video.height(), height);
                    assert_eq!(video.pixel_format(), pixel_format);
                    assert_eq!(video.pixel_format_name(), expected_name);
                    assert_eq!(video.line_sizes(), &[expected_line_size]);
                    assert_eq!(video.planes(), &[payload]);
                }
                FrameData::Audio(_) | FrameData::Empty => panic!("expected video frame"),
            }
        }

        let decoder = RawVideoDecoder::new(1, 1, PixelFormat::Xv48Le).unwrap();
        assert_eq!(
            decoder
                .decode_packet(&Packet::new(vec![0; 7], 0))
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
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
    fn decodes_ayuv64_packets_to_single_plane_frames() {
        for (pixel_format, expected_name, payload) in [
            (
                PixelFormat::Ayuv64Le,
                "ayuv64le",
                (0..16).collect::<Vec<_>>(),
            ),
            (
                PixelFormat::Ayuv64Be,
                "ayuv64be",
                (16..32).collect::<Vec<_>>(),
            ),
        ] {
            let decoder = RawVideoDecoder::new(1, 2, pixel_format).unwrap();
            let mut packet = Packet::new(payload.clone(), 0);
            packet.set_pts(Some(17));

            let frame = decoder.decode_packet(&packet).unwrap();

            assert_eq!(decoder.frame_size(), 16);
            assert_eq!(frame.pts(), Some(17));
            match frame.data() {
                FrameData::Video(video) => {
                    assert_eq!(video.width(), 1);
                    assert_eq!(video.height(), 2);
                    assert_eq!(video.pixel_format(), pixel_format);
                    assert_eq!(video.pixel_format_name(), expected_name);
                    assert_eq!(video.line_sizes(), &[8]);
                    assert_eq!(video.planes(), &[payload]);
                }
                FrameData::Audio(_) | FrameData::Empty => panic!("expected video frame"),
            }
        }

        let decoder = RawVideoDecoder::new(1, 1, PixelFormat::Ayuv64Le).unwrap();
        assert_eq!(
            decoder
                .decode_packet(&Packet::new(vec![0; 7], 0))
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
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
    fn decodes_high_bit_depth_planar_yuv_packets_to_three_planes() {
        for (format, name, width, height, payload_len, expected_planes, expected_lines) in [
            (
                PixelFormat::Yuv420p9Le,
                "yuv420p9le",
                4,
                2,
                24,
                vec![
                    (0..16).collect::<Vec<_>>(),
                    (16..20).collect::<Vec<_>>(),
                    (20..24).collect::<Vec<_>>(),
                ],
                vec![8, 4, 4],
            ),
            (
                PixelFormat::Yuv422p9Be,
                "yuv422p9be",
                4,
                3,
                48,
                vec![
                    (0..24).collect::<Vec<_>>(),
                    (24..36).collect::<Vec<_>>(),
                    (36..48).collect::<Vec<_>>(),
                ],
                vec![8, 4, 4],
            ),
            (
                PixelFormat::Yuv444p9Be,
                "yuv444p9be",
                3,
                2,
                36,
                vec![
                    (0..12).collect::<Vec<_>>(),
                    (12..24).collect::<Vec<_>>(),
                    (24..36).collect::<Vec<_>>(),
                ],
                vec![6, 6, 6],
            ),
            (
                PixelFormat::Yuv420p10Le,
                "yuv420p10le",
                4,
                2,
                24,
                vec![
                    (0..16).collect::<Vec<_>>(),
                    (16..20).collect::<Vec<_>>(),
                    (20..24).collect::<Vec<_>>(),
                ],
                vec![8, 4, 4],
            ),
            (
                PixelFormat::Yuv440p10Le,
                "yuv440p10le",
                3,
                2,
                24,
                vec![
                    (0..12).collect::<Vec<_>>(),
                    (12..18).collect::<Vec<_>>(),
                    (18..24).collect::<Vec<_>>(),
                ],
                vec![6, 6, 6],
            ),
            (
                PixelFormat::Yuv422p12Be,
                "yuv422p12be",
                4,
                3,
                48,
                vec![
                    (0..24).collect::<Vec<_>>(),
                    (24..36).collect::<Vec<_>>(),
                    (36..48).collect::<Vec<_>>(),
                ],
                vec![8, 4, 4],
            ),
            (
                PixelFormat::Yuv444p10Be,
                "yuv444p10be",
                3,
                2,
                36,
                vec![
                    (0..12).collect::<Vec<_>>(),
                    (12..24).collect::<Vec<_>>(),
                    (24..36).collect::<Vec<_>>(),
                ],
                vec![6, 6, 6],
            ),
            (
                PixelFormat::Yuv420p14Le,
                "yuv420p14le",
                4,
                2,
                24,
                vec![
                    (0..16).collect::<Vec<_>>(),
                    (16..20).collect::<Vec<_>>(),
                    (20..24).collect::<Vec<_>>(),
                ],
                vec![8, 4, 4],
            ),
            (
                PixelFormat::Yuv422p16Be,
                "yuv422p16be",
                4,
                3,
                48,
                vec![
                    (0..24).collect::<Vec<_>>(),
                    (24..36).collect::<Vec<_>>(),
                    (36..48).collect::<Vec<_>>(),
                ],
                vec![8, 4, 4],
            ),
            (
                PixelFormat::Yuv444p16Be,
                "yuv444p16be",
                3,
                2,
                36,
                vec![
                    (0..12).collect::<Vec<_>>(),
                    (12..24).collect::<Vec<_>>(),
                    (24..36).collect::<Vec<_>>(),
                ],
                vec![6, 6, 6],
            ),
        ] {
            let decoder = RawVideoDecoder::new(width, height, format).unwrap();
            let packet = Packet::new((0_u8..payload_len).collect(), 0);
            let frame = decoder.decode_packet(&packet).unwrap();

            assert_eq!(decoder.frame_size(), usize::from(payload_len));
            match frame.data() {
                FrameData::Video(video) => {
                    assert_eq!(video.pixel_format(), format);
                    assert_eq!(video.pixel_format_name(), name);
                    assert_eq!(video.line_sizes(), expected_lines.as_slice());
                    assert_eq!(video.planes(), expected_planes.as_slice());
                }
                FrameData::Audio(_) | FrameData::Empty => panic!("expected video frame"),
            }
        }
    }

    #[test]
    fn decodes_packed_yuv422_packets_to_single_plane_frames() {
        for (pixel_format, expected_name, payload) in [
            (PixelFormat::Yuyv422, "yuyv422", vec![1, 2, 3, 4]),
            (PixelFormat::Uyvy422, "uyvy422", vec![5, 6, 7, 8]),
            (PixelFormat::Yvyu422, "yvyu422", vec![9, 10, 11, 12]),
        ] {
            let decoder = RawVideoDecoder::new(2, 1, pixel_format).unwrap();
            let mut packet = Packet::new(payload.clone(), 0);
            packet.set_pts(Some(21));

            let frame = decoder.decode_packet(&packet).unwrap();

            assert_eq!(decoder.frame_size(), 4);
            assert_eq!(frame.pts(), Some(21));
            match frame.data() {
                FrameData::Video(video) => {
                    assert_eq!(video.width(), 2);
                    assert_eq!(video.height(), 1);
                    assert_eq!(video.pixel_format(), pixel_format);
                    assert_eq!(video.pixel_format_name(), expected_name);
                    assert_eq!(video.line_sizes(), &[4]);
                    assert_eq!(video.planes(), &[payload]);
                }
                FrameData::Audio(_) | FrameData::Empty => panic!("expected video frame"),
            }
        }
    }

    #[test]
    fn decodes_uyyvyy411_packet_to_single_plane_frame() {
        let decoder = RawVideoDecoder::new(4, 2, PixelFormat::Uyyvyy411).unwrap();
        let payload = (0_u8..12).collect::<Vec<_>>();
        let mut packet = Packet::new(payload.clone(), 0);
        packet.set_pts(Some(22));

        let frame = decoder.decode_packet(&packet).unwrap();

        assert_eq!(decoder.frame_size(), 12);
        assert_eq!(frame.pts(), Some(22));
        match frame.data() {
            FrameData::Video(video) => {
                assert_eq!(video.width(), 4);
                assert_eq!(video.height(), 2);
                assert_eq!(video.pixel_format(), PixelFormat::Uyyvyy411);
                assert_eq!(video.pixel_format_name(), "uyyvyy411");
                assert_eq!(video.line_sizes(), &[6]);
                assert_eq!(video.planes(), &[payload]);
            }
            FrameData::Audio(_) | FrameData::Empty => panic!("expected video frame"),
        }

        assert!(RawVideoDecoder::new(6, 1, PixelFormat::Uyyvyy411).is_err());
        assert_eq!(
            decoder
                .decode_packet(&Packet::new(vec![0; 11], 0))
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
    }

    #[test]
    fn decodes_semiplanar_yuv_packets_to_two_plane_frames() {
        for (
            pixel_format,
            expected_name,
            width,
            height,
            payload,
            expected_lines,
            expected_planes,
        ) in [
            (
                PixelFormat::Nv12,
                "nv12",
                4,
                2,
                (0_u8..12).collect::<Vec<_>>(),
                vec![4, 4],
                vec![(0_u8..8).collect::<Vec<_>>(), (8_u8..12).collect()],
            ),
            (
                PixelFormat::Nv21,
                "nv21",
                4,
                2,
                (12_u8..24).collect::<Vec<_>>(),
                vec![4, 4],
                vec![(12_u8..20).collect::<Vec<_>>(), (20_u8..24).collect()],
            ),
            (
                PixelFormat::Nv16,
                "nv16",
                4,
                3,
                (0_u8..24).collect::<Vec<_>>(),
                vec![4, 4],
                vec![(0_u8..12).collect::<Vec<_>>(), (12_u8..24).collect()],
            ),
            (
                PixelFormat::Nv20Le,
                "nv20le",
                4,
                3,
                (0_u8..48).collect::<Vec<_>>(),
                vec![8, 8],
                vec![(0_u8..24).collect::<Vec<_>>(), (24_u8..48).collect()],
            ),
            (
                PixelFormat::Nv24,
                "nv24",
                3,
                2,
                (0_u8..18).collect::<Vec<_>>(),
                vec![3, 6],
                vec![(0_u8..6).collect::<Vec<_>>(), (6_u8..18).collect()],
            ),
            (
                PixelFormat::Nv42,
                "nv42",
                3,
                2,
                (18_u8..36).collect::<Vec<_>>(),
                vec![3, 6],
                vec![(18_u8..24).collect::<Vec<_>>(), (24_u8..36).collect()],
            ),
            (
                PixelFormat::P010Le,
                "p010le",
                4,
                2,
                (0_u8..24).collect::<Vec<_>>(),
                vec![8, 8],
                vec![(0_u8..16).collect::<Vec<_>>(), (16_u8..24).collect()],
            ),
            (
                PixelFormat::P012Be,
                "p012be",
                4,
                2,
                (24_u8..48).collect::<Vec<_>>(),
                vec![8, 8],
                vec![(24_u8..40).collect::<Vec<_>>(), (40_u8..48).collect()],
            ),
            (
                PixelFormat::P216Le,
                "p216le",
                4,
                3,
                (0_u8..48).collect::<Vec<_>>(),
                vec![8, 8],
                vec![(0_u8..24).collect::<Vec<_>>(), (24_u8..48).collect()],
            ),
            (
                PixelFormat::P412Be,
                "p412be",
                3,
                2,
                (0_u8..36).collect::<Vec<_>>(),
                vec![6, 12],
                vec![(0_u8..12).collect::<Vec<_>>(), (12_u8..36).collect()],
            ),
        ] {
            let decoder = RawVideoDecoder::new(width, height, pixel_format).unwrap();
            let mut packet = Packet::new(payload.clone(), 0);
            packet.set_pts(Some(22));

            let frame = decoder.decode_packet(&packet).unwrap();

            assert_eq!(decoder.frame_size(), payload.len());
            assert_eq!(frame.pts(), Some(22));
            match frame.data() {
                FrameData::Video(video) => {
                    assert_eq!(video.width(), width);
                    assert_eq!(video.height(), height);
                    assert_eq!(video.pixel_format(), pixel_format);
                    assert_eq!(video.pixel_format_name(), expected_name);
                    assert_eq!(video.line_sizes(), expected_lines.as_slice());
                    assert_eq!(video.planes(), expected_planes.as_slice());
                }
                FrameData::Audio(_) | FrameData::Empty => panic!("expected video frame"),
            }
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
    fn decodes_yuva_packets_to_four_planes() {
        for (format, name, width, height, payload_len, expected_planes, expected_lines) in [
            (
                PixelFormat::Yuva420p,
                "yuva420p",
                4,
                2,
                20,
                vec![
                    (0..8).collect::<Vec<_>>(),
                    (8..10).collect::<Vec<_>>(),
                    (10..12).collect::<Vec<_>>(),
                    (12..20).collect::<Vec<_>>(),
                ],
                vec![4, 2, 2, 4],
            ),
            (
                PixelFormat::Yuva422p,
                "yuva422p",
                4,
                3,
                36,
                vec![
                    (0..12).collect::<Vec<_>>(),
                    (12..18).collect::<Vec<_>>(),
                    (18..24).collect::<Vec<_>>(),
                    (24..36).collect::<Vec<_>>(),
                ],
                vec![4, 2, 2, 4],
            ),
            (
                PixelFormat::Yuva444p,
                "yuva444p",
                3,
                2,
                24,
                vec![
                    (0..6).collect::<Vec<_>>(),
                    (6..12).collect::<Vec<_>>(),
                    (12..18).collect::<Vec<_>>(),
                    (18..24).collect::<Vec<_>>(),
                ],
                vec![3, 3, 3, 3],
            ),
            (
                PixelFormat::Yuva420p9Le,
                "yuva420p9le",
                4,
                2,
                40,
                vec![
                    (0..16).collect::<Vec<_>>(),
                    (16..20).collect::<Vec<_>>(),
                    (20..24).collect::<Vec<_>>(),
                    (24..40).collect::<Vec<_>>(),
                ],
                vec![8, 4, 4, 8],
            ),
            (
                PixelFormat::Yuva422p10Be,
                "yuva422p10be",
                4,
                3,
                72,
                vec![
                    (0..24).collect::<Vec<_>>(),
                    (24..36).collect::<Vec<_>>(),
                    (36..48).collect::<Vec<_>>(),
                    (48..72).collect::<Vec<_>>(),
                ],
                vec![8, 4, 4, 8],
            ),
            (
                PixelFormat::Yuva422p12Le,
                "yuva422p12le",
                4,
                3,
                72,
                vec![
                    (0..24).collect::<Vec<_>>(),
                    (24..36).collect::<Vec<_>>(),
                    (36..48).collect::<Vec<_>>(),
                    (48..72).collect::<Vec<_>>(),
                ],
                vec![8, 4, 4, 8],
            ),
            (
                PixelFormat::Yuva444p16Be,
                "yuva444p16be",
                3,
                2,
                48,
                vec![
                    (0..12).collect::<Vec<_>>(),
                    (12..24).collect::<Vec<_>>(),
                    (24..36).collect::<Vec<_>>(),
                    (36..48).collect::<Vec<_>>(),
                ],
                vec![6, 6, 6, 6],
            ),
        ] {
            let decoder = RawVideoDecoder::new(width, height, format).unwrap();
            let packet = Packet::new((0_u8..payload_len).collect::<Vec<_>>(), 0);
            let frame = decoder.decode_packet(&packet).unwrap();

            assert_eq!(decoder.frame_size(), usize::from(payload_len));
            match frame.data() {
                FrameData::Video(video) => {
                    assert_eq!(video.pixel_format(), format);
                    assert_eq!(video.pixel_format_name(), name);
                    assert_eq!(video.line_sizes(), expected_lines.as_slice());
                    assert_eq!(video.planes(), expected_planes.as_slice());
                }
                FrameData::Audio(_) | FrameData::Empty => panic!("expected video frame"),
            }
        }
    }

    #[test]
    fn decodes_yuvj_packets_to_three_planes() {
        for (format, name, width, height, payload_len, expected_planes) in [
            (
                PixelFormat::YuvJ420p,
                "yuvj420p",
                4,
                2,
                12,
                vec![
                    (0..8).collect::<Vec<_>>(),
                    (8..10).collect::<Vec<_>>(),
                    (10..12).collect::<Vec<_>>(),
                ],
            ),
            (
                PixelFormat::YuvJ422p,
                "yuvj422p",
                4,
                3,
                24,
                vec![
                    (0..12).collect::<Vec<_>>(),
                    (12..18).collect::<Vec<_>>(),
                    (18..24).collect::<Vec<_>>(),
                ],
            ),
            (
                PixelFormat::YuvJ411p,
                "yuvj411p",
                4,
                3,
                18,
                vec![
                    (0..12).collect::<Vec<_>>(),
                    (12..15).collect::<Vec<_>>(),
                    (15..18).collect::<Vec<_>>(),
                ],
            ),
            (
                PixelFormat::YuvJ440p,
                "yuvj440p",
                3,
                2,
                12,
                vec![
                    (0..6).collect::<Vec<_>>(),
                    (6..9).collect::<Vec<_>>(),
                    (9..12).collect::<Vec<_>>(),
                ],
            ),
            (
                PixelFormat::YuvJ444p,
                "yuvj444p",
                3,
                2,
                18,
                vec![
                    (0..6).collect::<Vec<_>>(),
                    (6..12).collect::<Vec<_>>(),
                    (12..18).collect::<Vec<_>>(),
                ],
            ),
        ] {
            let decoder = RawVideoDecoder::new(width, height, format).unwrap();
            let packet = Packet::new((0_u8..payload_len).collect::<Vec<_>>(), 0);
            let frame = decoder.decode_packet(&packet).unwrap();

            match frame.data() {
                FrameData::Video(video) => {
                    assert_eq!(video.pixel_format(), format);
                    assert_eq!(video.pixel_format_name(), name);
                    assert_eq!(video.planes(), expected_planes.as_slice());
                }
                FrameData::Audio(_) | FrameData::Empty => panic!("expected video frame"),
            }
        }
    }

    #[test]
    fn validates_dimensions_and_packet_size() {
        assert!(RawVideoDecoder::new(0, 2, PixelFormat::Rgb24).is_err());
        assert!(RawVideoDecoder::new(3, 2, PixelFormat::Yuv420p).is_err());
        assert!(RawVideoDecoder::new(3, 2, PixelFormat::YuvJ420p).is_err());
        assert!(RawVideoDecoder::new(3, 2, PixelFormat::Yuv422p).is_err());
        assert!(RawVideoDecoder::new(3, 2, PixelFormat::YuvJ422p).is_err());
        assert!(RawVideoDecoder::new(3, 2, PixelFormat::Yuv420p9Le).is_err());
        assert!(RawVideoDecoder::new(4, 3, PixelFormat::Yuv420p9Be).is_err());
        assert!(RawVideoDecoder::new(3, 2, PixelFormat::Yuv422p9Le).is_err());
        assert!(RawVideoDecoder::new(3, 2, PixelFormat::Yuv444p9Be).is_ok());
        assert!(RawVideoDecoder::new(3, 2, PixelFormat::Yuv420p10Le).is_err());
        assert!(RawVideoDecoder::new(4, 3, PixelFormat::Yuv420p12Be).is_err());
        assert!(RawVideoDecoder::new(3, 2, PixelFormat::Yuv422p10Be).is_err());
        assert!(RawVideoDecoder::new(4, 3, PixelFormat::Yuv422p12Be).is_ok());
        assert!(RawVideoDecoder::new(3, 2, PixelFormat::Yuv444p10Le).is_ok());
        assert!(RawVideoDecoder::new(3, 2, PixelFormat::Yuv420p14Le).is_err());
        assert!(RawVideoDecoder::new(4, 3, PixelFormat::Yuv420p16Be).is_err());
        assert!(RawVideoDecoder::new(3, 2, PixelFormat::Yuv422p14Le).is_err());
        assert!(RawVideoDecoder::new(3, 2, PixelFormat::Yuv444p16Be).is_ok());
        assert!(RawVideoDecoder::new(3, 2, PixelFormat::Yuyv422).is_err());
        assert!(RawVideoDecoder::new(2, 3, PixelFormat::Yuyv422).is_ok());
        assert!(RawVideoDecoder::new(3, 2, PixelFormat::Y210Le).is_err());
        assert!(RawVideoDecoder::new(2, 2, PixelFormat::Y210Be).is_ok());
        assert!(RawVideoDecoder::new(3, 2, PixelFormat::Y212Le).is_err());
        assert!(RawVideoDecoder::new(2, 2, PixelFormat::Y212Be).is_ok());
        assert!(RawVideoDecoder::new(3, 2, PixelFormat::Y216Le).is_err());
        assert!(RawVideoDecoder::new(2, 2, PixelFormat::Y216Be).is_ok());
        assert!(RawVideoDecoder::new(3, 2, PixelFormat::Nv12).is_err());
        assert!(RawVideoDecoder::new(2, 3, PixelFormat::Nv12).is_err());
        assert!(RawVideoDecoder::new(2, 2, PixelFormat::Nv12).is_ok());
        assert!(RawVideoDecoder::new(3, 2, PixelFormat::Nv16).is_err());
        assert!(RawVideoDecoder::new(4, 3, PixelFormat::Nv16).is_ok());
        assert!(RawVideoDecoder::new(3, 2, PixelFormat::Nv20Be).is_err());
        assert!(RawVideoDecoder::new(4, 3, PixelFormat::Nv20Be).is_ok());
        assert!(RawVideoDecoder::new(3, 2, PixelFormat::Nv24).is_ok());
        assert!(RawVideoDecoder::new(3, 2, PixelFormat::P010Le).is_err());
        assert!(RawVideoDecoder::new(4, 3, PixelFormat::P010Be).is_err());
        assert!(RawVideoDecoder::new(4, 2, PixelFormat::P012Le).is_ok());
        assert!(RawVideoDecoder::new(4, 2, PixelFormat::P016Be).is_ok());
        assert!(RawVideoDecoder::new(3, 2, PixelFormat::P210Le).is_err());
        assert!(RawVideoDecoder::new(4, 3, PixelFormat::P212Be).is_ok());
        assert!(RawVideoDecoder::new(4, 3, PixelFormat::P216Le).is_ok());
        assert!(RawVideoDecoder::new(3, 2, PixelFormat::P410Be).is_ok());
        assert!(RawVideoDecoder::new(3, 2, PixelFormat::P416Le).is_ok());
        assert!(RawVideoDecoder::new(3, 2, PixelFormat::Yuv411p).is_err());
        assert!(RawVideoDecoder::new(3, 2, PixelFormat::YuvJ411p).is_err());
        assert!(RawVideoDecoder::new(4, 3, PixelFormat::Yuv411p).is_ok());
        assert!(RawVideoDecoder::new(4, 3, PixelFormat::Yuv410p).is_err());
        assert!(RawVideoDecoder::new(2, 4, PixelFormat::Yuv410p).is_err());
        assert!(RawVideoDecoder::new(4, 4, PixelFormat::Yuv410p).is_ok());
        assert!(RawVideoDecoder::new(3, 3, PixelFormat::Yuv440p).is_err());
        assert!(RawVideoDecoder::new(3, 3, PixelFormat::YuvJ440p).is_err());
        assert!(RawVideoDecoder::new(3, 2, PixelFormat::Yuv440p).is_ok());
        assert!(RawVideoDecoder::new(3, 3, PixelFormat::Yuv440p10Le).is_err());
        assert!(RawVideoDecoder::new(3, 2, PixelFormat::Yuv440p10Le).is_ok());
        assert!(RawVideoDecoder::new(3, 3, PixelFormat::Yuv440p12Be).is_err());
        assert!(RawVideoDecoder::new(3, 2, PixelFormat::Yuv440p12Be).is_ok());
        assert!(RawVideoDecoder::new(3, 2, PixelFormat::Yuv444p).is_ok());
        assert!(RawVideoDecoder::new(3, 2, PixelFormat::Yuva420p).is_err());
        assert!(RawVideoDecoder::new(4, 3, PixelFormat::Yuva420p).is_err());
        assert!(RawVideoDecoder::new(3, 2, PixelFormat::Yuva422p).is_err());
        assert!(RawVideoDecoder::new(4, 3, PixelFormat::Yuva422p).is_ok());
        assert!(RawVideoDecoder::new(3, 2, PixelFormat::Yuva444p).is_ok());
        assert!(RawVideoDecoder::new(3, 2, PixelFormat::Yuva420p9Le).is_err());
        assert!(RawVideoDecoder::new(4, 3, PixelFormat::Yuva420p10Be).is_err());
        assert!(RawVideoDecoder::new(3, 2, PixelFormat::Yuva422p16Le).is_err());
        assert!(RawVideoDecoder::new(3, 2, PixelFormat::Yuva444p12Be).is_ok());
        assert!(RawVideoDecoder::new(3, 2, PixelFormat::YuvJ444p).is_ok());

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
    fn decodes_low_bit_depth_rgb_byte_packets_to_single_plane_frames() {
        for (pixel_format, expected_name, payload) in [
            (PixelFormat::Rgb8, "rgb8", vec![1, 2, 3, 4]),
            (PixelFormat::Bgr8, "bgr8", vec![5, 6, 7, 8]),
            (PixelFormat::Rgb4Byte, "rgb4_byte", vec![9, 10, 11, 12]),
            (PixelFormat::Bgr4Byte, "bgr4_byte", vec![13, 14, 15, 16]),
        ] {
            let decoder = RawVideoDecoder::new(4, 1, pixel_format).unwrap();
            let mut packet = Packet::new(payload.clone(), 0);
            packet.set_pts(Some(17));

            let frame = decoder.decode_packet(&packet).unwrap();

            assert_eq!(decoder.frame_size(), 4);
            assert_eq!(frame.pts(), Some(17));
            match frame.data() {
                FrameData::Video(video) => {
                    assert_eq!(video.width(), 4);
                    assert_eq!(video.height(), 1);
                    assert_eq!(video.pixel_format(), pixel_format);
                    assert_eq!(video.pixel_format_name(), expected_name);
                    assert_eq!(video.line_sizes(), &[4]);
                    assert_eq!(video.planes(), &[payload]);
                }
                FrameData::Audio(_) | FrameData::Empty => panic!("expected video frame"),
            }
        }
    }

    #[test]
    fn decodes_pal8_packet_to_single_index_plane_frame() {
        let decoder = RawVideoDecoder::new(2, 2, PixelFormat::Pal8).unwrap();
        let mut packet = Packet::new(vec![0, 1, 2, 3], 0);
        packet.set_pts(Some(20));

        let frame = decoder.decode_packet(&packet).unwrap();

        assert_eq!(decoder.frame_size(), 4);
        assert_eq!(frame.pts(), Some(20));
        match frame.data() {
            FrameData::Video(video) => {
                assert_eq!(video.width(), 2);
                assert_eq!(video.height(), 2);
                assert_eq!(video.pixel_format(), PixelFormat::Pal8);
                assert_eq!(video.pixel_format_name(), "pal8");
                assert_eq!(video.line_sizes(), &[2]);
                assert_eq!(video.planes(), &[vec![0, 1, 2, 3]]);
            }
            FrameData::Audio(_) | FrameData::Empty => panic!("expected video frame"),
        }
    }

    #[test]
    fn decodes_bayer_packets_to_single_plane_frames() {
        for (pixel_format, expected_name, width, height, payload, expected_line_size) in [
            (
                PixelFormat::BayerBggr8,
                "bayer_bggr8",
                2,
                2,
                vec![0, 1, 2, 3],
                2,
            ),
            (
                PixelFormat::BayerRggb16Be,
                "bayer_rggb16be",
                2,
                1,
                vec![4, 5, 6, 7],
                4,
            ),
        ] {
            let decoder = RawVideoDecoder::new(width, height, pixel_format).unwrap();
            let mut packet = Packet::new(payload.clone(), 0);
            packet.set_pts(Some(21));

            let frame = decoder.decode_packet(&packet).unwrap();

            assert_eq!(decoder.frame_size(), payload.len());
            assert_eq!(frame.pts(), Some(21));
            match frame.data() {
                FrameData::Video(video) => {
                    assert_eq!(video.width(), width);
                    assert_eq!(video.height(), height);
                    assert_eq!(video.pixel_format(), pixel_format);
                    assert_eq!(video.pixel_format_name(), expected_name);
                    assert_eq!(video.line_sizes(), &[expected_line_size]);
                    assert_eq!(video.planes(), &[payload]);
                }
                FrameData::Audio(_) | FrameData::Empty => panic!("expected video frame"),
            }
        }

        let decoder = RawVideoDecoder::new(2, 1, PixelFormat::BayerRggb16Be).unwrap();
        assert_eq!(
            decoder
                .decode_packet(&Packet::new(vec![0; 3], 0))
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
    }

    #[test]
    fn decodes_packed_4bit_rgb_packets_to_single_plane_frames() {
        for (pixel_format, expected_name, payload) in [
            (PixelFormat::Rgb4, "rgb4", vec![1, 2, 3, 4]),
            (PixelFormat::Bgr4, "bgr4", vec![5, 6, 7, 8]),
        ] {
            let decoder = RawVideoDecoder::new(3, 2, pixel_format).unwrap();
            let mut packet = Packet::new(payload.clone(), 0);
            packet.set_pts(Some(18));

            let frame = decoder.decode_packet(&packet).unwrap();

            assert_eq!(decoder.frame_size(), 4);
            assert_eq!(frame.pts(), Some(18));
            match frame.data() {
                FrameData::Video(video) => {
                    assert_eq!(video.width(), 3);
                    assert_eq!(video.height(), 2);
                    assert_eq!(video.pixel_format(), pixel_format);
                    assert_eq!(video.pixel_format_name(), expected_name);
                    assert_eq!(video.line_sizes(), &[2]);
                    assert_eq!(video.planes(), &[payload]);
                }
                FrameData::Audio(_) | FrameData::Empty => panic!("expected video frame"),
            }
        }
    }

    #[test]
    fn decodes_mono_bitstream_packets_to_single_plane_frames() {
        for (pixel_format, expected_name, payload) in [
            (
                PixelFormat::MonoWhite,
                "monow",
                vec![0x80, 0x01, 0xff, 0x00],
            ),
            (PixelFormat::MonoBlack, "monob", vec![0xaa, 0x55]),
        ] {
            let (width, height, expected_size, expected_line_size) =
                if pixel_format == PixelFormat::MonoWhite {
                    (9, 2, 4, 2)
                } else {
                    (16, 1, 2, 2)
                };
            let decoder = RawVideoDecoder::new(width, height, pixel_format).unwrap();
            let mut packet = Packet::new(payload.clone(), 0);
            packet.set_pts(Some(19));

            let frame = decoder.decode_packet(&packet).unwrap();

            assert_eq!(decoder.frame_size(), expected_size);
            assert_eq!(frame.pts(), Some(19));
            match frame.data() {
                FrameData::Video(video) => {
                    assert_eq!(video.width(), width);
                    assert_eq!(video.height(), height);
                    assert_eq!(video.pixel_format(), pixel_format);
                    assert_eq!(video.pixel_format_name(), expected_name);
                    assert_eq!(video.line_sizes(), &[expected_line_size]);
                    assert_eq!(video.planes(), &[payload]);
                }
                FrameData::Audio(_) | FrameData::Empty => panic!("expected video frame"),
            }
        }
    }

    #[test]
    fn decodes_packed_16bit_rgb_packets_to_single_plane_frames() {
        for (pixel_format, expected_name, payload) in [
            (PixelFormat::Rgb565Le, "rgb565le", vec![1, 2, 3, 4]),
            (PixelFormat::Rgb555Be, "rgb555be", vec![5, 6, 7, 8]),
            (PixelFormat::Bgr565Le, "bgr565le", vec![9, 10, 11, 12]),
            (PixelFormat::Bgr555Be, "bgr555be", vec![13, 14, 15, 16]),
            (PixelFormat::Rgb444Le, "rgb444le", vec![17, 18, 19, 20]),
            (PixelFormat::Bgr444Be, "bgr444be", vec![21, 22, 23, 24]),
        ] {
            let decoder = RawVideoDecoder::new(2, 1, pixel_format).unwrap();
            let mut packet = Packet::new(payload.clone(), 0);
            packet.set_pts(Some(11));

            let frame = decoder.decode_packet(&packet).unwrap();

            assert_eq!(decoder.frame_size(), 4);
            assert_eq!(frame.pts(), Some(11));
            match frame.data() {
                FrameData::Video(video) => {
                    assert_eq!(video.width(), 2);
                    assert_eq!(video.height(), 1);
                    assert_eq!(video.pixel_format(), pixel_format);
                    assert_eq!(video.pixel_format_name(), expected_name);
                    assert_eq!(video.line_sizes(), &[4]);
                    assert_eq!(video.planes(), &[payload]);
                }
                FrameData::Audio(_) | FrameData::Empty => panic!("expected video frame"),
            }
        }
    }

    #[test]
    fn decodes_high_bit_packed_yuv_packets_to_single_plane_frames() {
        for (pixel_format, expected_name, payload) in [
            (PixelFormat::Y210Le, "y210le", (0..16).collect::<Vec<_>>()),
            (PixelFormat::Y210Be, "y210be", (16..32).collect::<Vec<_>>()),
            (PixelFormat::Y212Le, "y212le", (32..48).collect::<Vec<_>>()),
            (PixelFormat::Y212Be, "y212be", (48..64).collect::<Vec<_>>()),
            (PixelFormat::Y216Le, "y216le", (64..80).collect::<Vec<_>>()),
            (PixelFormat::Y216Be, "y216be", (80..96).collect::<Vec<_>>()),
        ] {
            let decoder = RawVideoDecoder::new(2, 2, pixel_format).unwrap();
            let mut packet = Packet::new(payload.clone(), 0);
            packet.set_pts(Some(21));

            let frame = decoder.decode_packet(&packet).unwrap();

            assert_eq!(decoder.frame_size(), 16);
            assert_eq!(frame.pts(), Some(21));
            match frame.data() {
                FrameData::Video(video) => {
                    assert_eq!(video.width(), 2);
                    assert_eq!(video.height(), 2);
                    assert_eq!(video.pixel_format(), pixel_format);
                    assert_eq!(video.pixel_format_name(), expected_name);
                    assert_eq!(video.line_sizes(), &[8]);
                    assert_eq!(video.planes(), &[payload]);
                }
                FrameData::Audio(_) | FrameData::Empty => panic!("expected video frame"),
            }
        }

        let decoder = RawVideoDecoder::new(2, 2, PixelFormat::Y210Le).unwrap();
        assert_eq!(
            decoder
                .decode_packet(&Packet::new(vec![0; 15], 0))
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
    }

    #[test]
    fn decodes_xyz12_packets_to_single_plane_frames() {
        for (pixel_format, expected_name, payload) in [
            (PixelFormat::Xyz12Le, "xyz12le", (0..12).collect::<Vec<_>>()),
            (
                PixelFormat::Xyz12Be,
                "xyz12be",
                (12..24).collect::<Vec<_>>(),
            ),
        ] {
            let decoder = RawVideoDecoder::new(1, 2, pixel_format).unwrap();
            let mut packet = Packet::new(payload.clone(), 0);
            packet.set_pts(Some(22));

            let frame = decoder.decode_packet(&packet).unwrap();

            assert_eq!(decoder.frame_size(), 12);
            assert_eq!(frame.pts(), Some(22));
            match frame.data() {
                FrameData::Video(video) => {
                    assert_eq!(video.width(), 1);
                    assert_eq!(video.height(), 2);
                    assert_eq!(video.pixel_format(), pixel_format);
                    assert_eq!(video.pixel_format_name(), expected_name);
                    assert_eq!(video.line_sizes(), &[6]);
                    assert_eq!(video.planes(), &[payload]);
                }
                FrameData::Audio(_) | FrameData::Empty => panic!("expected video frame"),
            }
        }

        let decoder = RawVideoDecoder::new(1, 1, PixelFormat::Xyz12Le).unwrap();
        assert_eq!(
            decoder
                .decode_packet(&Packet::new(vec![0; 5], 0))
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
    }

    #[test]
    fn decodes_x2rgb10_packets_to_single_plane_frames() {
        for (pixel_format, expected_name, payload) in [
            (
                PixelFormat::X2Rgb10Le,
                "x2rgb10le",
                (0..16).collect::<Vec<_>>(),
            ),
            (
                PixelFormat::X2Rgb10Be,
                "x2rgb10be",
                (16..32).collect::<Vec<_>>(),
            ),
            (
                PixelFormat::X2Bgr10Le,
                "x2bgr10le",
                (32..48).collect::<Vec<_>>(),
            ),
            (
                PixelFormat::X2Bgr10Be,
                "x2bgr10be",
                (48..64).collect::<Vec<_>>(),
            ),
        ] {
            let decoder = RawVideoDecoder::new(2, 2, pixel_format).unwrap();
            let mut packet = Packet::new(payload.clone(), 0);
            packet.set_pts(Some(23));

            let frame = decoder.decode_packet(&packet).unwrap();

            assert_eq!(decoder.frame_size(), 16);
            assert_eq!(frame.pts(), Some(23));
            match frame.data() {
                FrameData::Video(video) => {
                    assert_eq!(video.width(), 2);
                    assert_eq!(video.height(), 2);
                    assert_eq!(video.pixel_format(), pixel_format);
                    assert_eq!(video.pixel_format_name(), expected_name);
                    assert_eq!(video.line_sizes(), &[8]);
                    assert_eq!(video.planes(), &[payload]);
                }
                FrameData::Audio(_) | FrameData::Empty => panic!("expected video frame"),
            }
        }

        let decoder = RawVideoDecoder::new(1, 1, PixelFormat::X2Rgb10Le).unwrap();
        assert_eq!(
            decoder
                .decode_packet(&Packet::new(vec![0; 3], 0))
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
    }

    #[test]
    fn decodes_packed_8bit_yuv_packets_to_single_plane_frames() {
        for (pixel_format, expected_name, payload, expected_size, expected_line_size) in [
            (
                PixelFormat::Vuya,
                "vuya",
                (0..16).collect::<Vec<_>>(),
                16,
                8,
            ),
            (
                PixelFormat::Vuyx,
                "vuyx",
                (16..32).collect::<Vec<_>>(),
                16,
                8,
            ),
            (
                PixelFormat::Ayuv,
                "ayuv",
                (32..48).collect::<Vec<_>>(),
                16,
                8,
            ),
            (
                PixelFormat::Uyva,
                "uyva",
                (48..64).collect::<Vec<_>>(),
                16,
                8,
            ),
            (
                PixelFormat::Vyu444,
                "vyu444",
                (64..76).collect::<Vec<_>>(),
                12,
                6,
            ),
        ] {
            let decoder = RawVideoDecoder::new(2, 2, pixel_format).unwrap();
            let mut packet = Packet::new(payload.clone(), 0);
            packet.set_pts(Some(24));

            let frame = decoder.decode_packet(&packet).unwrap();

            assert_eq!(decoder.frame_size(), expected_size);
            assert_eq!(frame.pts(), Some(24));
            match frame.data() {
                FrameData::Video(video) => {
                    assert_eq!(video.width(), 2);
                    assert_eq!(video.height(), 2);
                    assert_eq!(video.pixel_format(), pixel_format);
                    assert_eq!(video.pixel_format_name(), expected_name);
                    assert_eq!(video.line_sizes(), &[expected_line_size]);
                    assert_eq!(video.planes(), &[payload]);
                }
                FrameData::Audio(_) | FrameData::Empty => panic!("expected video frame"),
            }
        }

        let decoder = RawVideoDecoder::new(1, 1, PixelFormat::Vuya).unwrap();
        assert_eq!(
            decoder
                .decode_packet(&Packet::new(vec![0; 3], 0))
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
        let decoder = RawVideoDecoder::new(1, 1, PixelFormat::Vyu444).unwrap();
        assert_eq!(
            decoder
                .decode_packet(&Packet::new(vec![0; 2], 0))
                .unwrap_err()
                .kind(),
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
