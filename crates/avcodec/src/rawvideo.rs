use avutil::{AvError, AvResult, Frame, Packet, VideoFrame};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    Gray8,
    Rgb24,
    Rgba,
    Yuv420p,
}

impl PixelFormat {
    pub fn name(self) -> &'static str {
        match self {
            Self::Gray8 => "gray",
            Self::Rgb24 => "rgb24",
            Self::Rgba => "rgba",
            Self::Yuv420p => "yuv420p",
        }
    }

    fn frame_size(self, width: usize, height: usize) -> AvResult<usize> {
        let pixels = checked_area(width, height)?;
        match self {
            Self::Gray8 => Ok(pixels),
            Self::Rgb24 => pixels
                .checked_mul(3)
                .ok_or_else(|| AvError::invalid_argument("rawvideo frame size overflow")),
            Self::Rgba => pixels
                .checked_mul(4)
                .ok_or_else(|| AvError::invalid_argument("rawvideo frame size overflow")),
            Self::Yuv420p => {
                if width % 2 != 0 || height % 2 != 0 {
                    return Err(AvError::invalid_argument(
                        "yuv420p rawvideo dimensions must be even",
                    ));
                }

                let chroma = checked_area(width / 2, height / 2)?;
                pixels
                    .checked_add(chroma)
                    .and_then(|size| size.checked_add(chroma))
                    .ok_or_else(|| AvError::invalid_argument("rawvideo frame size overflow"))
            }
        }
    }

    fn split_planes(self, data: &[u8], width: usize, height: usize) -> AvResult<Vec<Vec<u8>>> {
        match self {
            Self::Gray8 | Self::Rgb24 | Self::Rgba => Ok(vec![data.to_vec()]),
            Self::Yuv420p => {
                let y_size = checked_area(width, height)?;
                let uv_size = checked_area(width / 2, height / 2)?;
                let y_end = y_size;
                let u_end = y_end + uv_size;
                let v_end = u_end + uv_size;
                Ok(vec![
                    data[..y_end].to_vec(),
                    data[y_end..u_end].to_vec(),
                    data[u_end..v_end].to_vec(),
                ])
            }
        }
    }
}

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
        let video = VideoFrame::new(self.width, self.height, self.pixel_format.name(), planes)?;
        let mut frame = Frame::video(video);
        frame.set_pts(packet.pts());
        Ok(frame)
    }
}

fn checked_area(width: usize, height: usize) -> AvResult<usize> {
    width
        .checked_mul(height)
        .ok_or_else(|| AvError::invalid_argument("rawvideo frame area overflow"))
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
                assert_eq!(video.pixel_format(), "gray");
                assert_eq!(video.planes(), &[vec![0, 1, 2, 3]]);
            }
            FrameData::Audio(_) => panic!("expected video frame"),
        }
    }

    #[test]
    fn decodes_yuv420p_packet_to_three_planes() {
        let decoder = RawVideoDecoder::new(4, 2, PixelFormat::Yuv420p).unwrap();
        let packet = Packet::new(vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11], 0);

        let frame = decoder.decode_packet(&packet).unwrap();

        match frame.data() {
            FrameData::Video(video) => {
                assert_eq!(video.pixel_format(), "yuv420p");
                assert_eq!(video.planes()[0], vec![0, 1, 2, 3, 4, 5, 6, 7]);
                assert_eq!(video.planes()[1], vec![8, 9]);
                assert_eq!(video.planes()[2], vec![10, 11]);
            }
            FrameData::Audio(_) => panic!("expected video frame"),
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
        let rgba = RawVideoDecoder::new(1, 1, PixelFormat::Rgba).unwrap();

        assert_eq!(rgb.frame_size(), 6);
        assert_eq!(rgba.frame_size(), 4);
        assert!(rgb
            .decode_packet(&Packet::new(vec![1, 2, 3, 4, 5, 6], 0))
            .is_ok());
        assert!(rgba
            .decode_packet(&Packet::new(vec![1, 2, 3, 4], 0))
            .is_ok());
    }
}
