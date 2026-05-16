use avutil::{AvError, AvErrorKind, AvResult, Packet, Rational, SideData};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawVideoPixelFormat {
    Gray8,
    Rgb24,
    Rgba,
    Yuv420p,
}

impl RawVideoPixelFormat {
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawVideoInfo {
    width: usize,
    height: usize,
    pixel_format: RawVideoPixelFormat,
    frame_rate: Rational,
    frame_size: usize,
    frame_count: usize,
}

impl RawVideoInfo {
    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn pixel_format(&self) -> RawVideoPixelFormat {
        self.pixel_format
    }

    pub fn frame_rate(&self) -> Rational {
        self.frame_rate
    }

    pub fn frame_size(&self) -> usize {
        self.frame_size
    }

    pub fn frame_count(&self) -> usize {
        self.frame_count
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawVideoDemuxer<'a> {
    info: RawVideoInfo,
    input: &'a [u8],
    next_frame: usize,
}

impl<'a> RawVideoDemuxer<'a> {
    pub fn open(
        input: &'a [u8],
        width: usize,
        height: usize,
        pixel_format: RawVideoPixelFormat,
        frame_rate: Rational,
    ) -> AvResult<Self> {
        validate_dimensions(width, height)?;
        validate_frame_rate(frame_rate)?;
        let frame_size = pixel_format.frame_size(width, height)?;
        if frame_size == 0 {
            return Err(AvError::invalid_argument(
                "rawvideo frame size must be non-zero",
            ));
        }
        if input.len() % frame_size != 0 {
            return Err(AvError::new(
                AvErrorKind::EndOfFile,
                "rawvideo input ends with a partial frame",
            ));
        }

        Ok(Self {
            info: RawVideoInfo {
                width,
                height,
                pixel_format,
                frame_rate,
                frame_size,
                frame_count: input.len() / frame_size,
            },
            input,
            next_frame: 0,
        })
    }

    pub fn info(&self) -> &RawVideoInfo {
        &self.info
    }

    pub fn read_packet(&mut self) -> AvResult<Option<Packet>> {
        if self.next_frame == self.info.frame_count {
            return Ok(None);
        }

        let start = self
            .next_frame
            .checked_mul(self.info.frame_size)
            .ok_or_else(|| AvError::invalid_data("rawvideo packet offset overflow"))?;
        let end = start
            .checked_add(self.info.frame_size)
            .ok_or_else(|| AvError::invalid_data("rawvideo packet end overflow"))?;
        let pts = i64::try_from(self.next_frame)
            .map_err(|_| AvError::invalid_data("rawvideo packet PTS does not fit i64"))?;

        let mut packet = Packet::new(self.input[start..end].to_vec(), 0);
        packet.set_pts(Some(pts));
        packet.set_dts(Some(pts));
        packet.set_duration(1)?;
        packet.push_side_data(SideData::new(
            "rawvideo_pix_fmt",
            self.info.pixel_format.name().as_bytes().to_vec(),
        )?);
        self.next_frame += 1;
        Ok(Some(packet))
    }
}

fn validate_dimensions(width: usize, height: usize) -> AvResult<()> {
    if width == 0 || height == 0 {
        return Err(AvError::invalid_argument(
            "rawvideo dimensions must be non-zero",
        ));
    }
    Ok(())
}

fn validate_frame_rate(frame_rate: Rational) -> AvResult<()> {
    if frame_rate.num() <= 0 || frame_rate.den() <= 0 {
        return Err(AvError::invalid_argument(
            "rawvideo frame rate must be positive",
        ));
    }
    Ok(())
}

fn checked_area(width: usize, height: usize) -> AvResult<usize> {
    width
        .checked_mul(height)
        .ok_or_else(|| AvError::invalid_argument("rawvideo frame area overflow"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slices_fixed_size_rgb_frames_with_timing() {
        let input = vec![
            0, 1, 2, 3, 4, 5, //
            6, 7, 8, 9, 10, 11,
        ];
        let mut demuxer = RawVideoDemuxer::open(
            &input,
            1,
            2,
            RawVideoPixelFormat::Rgb24,
            Rational::new(30, 1).unwrap(),
        )
        .unwrap();

        assert_eq!(demuxer.info().width(), 1);
        assert_eq!(demuxer.info().height(), 2);
        assert_eq!(demuxer.info().pixel_format(), RawVideoPixelFormat::Rgb24);
        assert_eq!(demuxer.info().frame_rate(), Rational::new(30, 1).unwrap());
        assert_eq!(demuxer.info().frame_size(), 6);
        assert_eq!(demuxer.info().frame_count(), 2);

        let first = demuxer.read_packet().unwrap().unwrap();
        assert_eq!(first.data(), &[0, 1, 2, 3, 4, 5]);
        assert_eq!(first.pts(), Some(0));
        assert_eq!(first.dts(), Some(0));
        assert_eq!(first.duration(), 1);
        assert_eq!(first.side_data()[0].kind(), "rawvideo_pix_fmt");
        assert_eq!(first.side_data()[0].data(), b"rgb24");

        let second = demuxer.read_packet().unwrap().unwrap();
        assert_eq!(second.data(), &[6, 7, 8, 9, 10, 11]);
        assert_eq!(second.pts(), Some(1));
        assert_eq!(second.dts(), Some(1));
        assert!(demuxer.read_packet().unwrap().is_none());
    }

    #[test]
    fn computes_frame_sizes_for_supported_pixel_formats() {
        assert_eq!(
            RawVideoDemuxer::open(
                &[0; 4],
                2,
                2,
                RawVideoPixelFormat::Gray8,
                Rational::new(1, 1).unwrap(),
            )
            .unwrap()
            .info()
            .frame_size(),
            4
        );
        assert_eq!(
            RawVideoDemuxer::open(
                &[0; 4],
                1,
                1,
                RawVideoPixelFormat::Rgba,
                Rational::new(1, 1).unwrap(),
            )
            .unwrap()
            .info()
            .frame_size(),
            4
        );
        assert_eq!(
            RawVideoDemuxer::open(
                &[0; 12],
                4,
                2,
                RawVideoPixelFormat::Yuv420p,
                Rational::new(1, 1).unwrap(),
            )
            .unwrap()
            .info()
            .frame_size(),
            12
        );
    }

    #[test]
    fn accepts_empty_input_as_zero_frames_when_geometry_is_valid() {
        let mut demuxer = RawVideoDemuxer::open(
            &[],
            2,
            2,
            RawVideoPixelFormat::Gray8,
            Rational::new(24, 1).unwrap(),
        )
        .unwrap();

        assert_eq!(demuxer.info().frame_count(), 0);
        assert!(demuxer.read_packet().unwrap().is_none());
    }

    #[test]
    fn rejects_invalid_geometry_frame_rate_and_truncated_frames() {
        assert!(RawVideoDemuxer::open(
            &[0; 4],
            0,
            2,
            RawVideoPixelFormat::Gray8,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoDemuxer::open(
            &[0; 12],
            3,
            2,
            RawVideoPixelFormat::Yuv420p,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoDemuxer::open(
            &[0; 4],
            2,
            2,
            RawVideoPixelFormat::Gray8,
            Rational::new(0, 1).unwrap(),
        )
        .is_err());

        let err = RawVideoDemuxer::open(
            &[0; 5],
            1,
            2,
            RawVideoPixelFormat::Rgb24,
            Rational::new(30, 1).unwrap(),
        )
        .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::EndOfFile);
    }
}
