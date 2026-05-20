use crate::VideoStreamParameters;
use avutil::{AvError, AvErrorKind, AvResult, Packet, PixelFormat, Rational, SideData};

pub type RawVideoPixelFormat = PixelFormat;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawVideoInfo {
    video: VideoStreamParameters,
    frame_rate: Rational,
    frame_count: usize,
}

impl RawVideoInfo {
    pub fn width(&self) -> usize {
        self.video.width()
    }

    pub fn height(&self) -> usize {
        self.video.height()
    }

    pub fn pixel_format(&self) -> RawVideoPixelFormat {
        self.video.pixel_format()
    }

    pub fn frame_rate(&self) -> Rational {
        self.frame_rate
    }

    pub fn frame_size(&self) -> usize {
        self.video.frame_size()
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
        let video = VideoStreamParameters::with_context(width, height, pixel_format, "rawvideo")?;
        validate_frame_rate(frame_rate)?;
        let frame_count = video.frame_count_in_bytes(
            input.len(),
            AvErrorKind::EndOfFile,
            "rawvideo input ends with a partial frame",
        )?;

        Ok(Self {
            info: RawVideoInfo {
                video,
                frame_rate,
                frame_count,
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
            .checked_mul(self.info.frame_size())
            .ok_or_else(|| AvError::invalid_data("rawvideo packet offset overflow"))?;
        let end = start
            .checked_add(self.info.frame_size())
            .ok_or_else(|| AvError::invalid_data("rawvideo packet end overflow"))?;
        let pts = i64::try_from(self.next_frame)
            .map_err(|_| AvError::invalid_data("rawvideo packet PTS does not fit i64"))?;

        let mut packet = Packet::new(self.input[start..end].to_vec(), 0);
        packet.set_pts(Some(pts));
        packet.set_dts(Some(pts));
        packet.set_duration(1)?;
        packet.push_side_data(SideData::new(
            "rawvideo_pix_fmt",
            self.info.pixel_format().name().as_bytes().to_vec(),
        )?);
        self.next_frame += 1;
        Ok(Some(packet))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawVideoMuxer {
    info: RawVideoInfo,
    data: Vec<u8>,
    finished: bool,
}

impl RawVideoMuxer {
    pub fn new(
        width: usize,
        height: usize,
        pixel_format: RawVideoPixelFormat,
        frame_rate: Rational,
    ) -> AvResult<Self> {
        let video = VideoStreamParameters::with_context(width, height, pixel_format, "rawvideo")?;
        validate_frame_rate(frame_rate)?;

        Ok(Self {
            info: RawVideoInfo {
                video,
                frame_rate,
                frame_count: 0,
            },
            data: Vec::new(),
            finished: false,
        })
    }

    pub fn info(&self) -> &RawVideoInfo {
        &self.info
    }

    pub fn data_len(&self) -> usize {
        self.data.len()
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }

    pub fn write_packet(&mut self, packet: &Packet) -> AvResult<()> {
        if self.finished {
            return Err(AvError::invalid_argument(
                "cannot write packet after rawvideo muxer is finished",
            ));
        }
        if packet.stream_index() != 0 {
            return Err(AvError::invalid_argument(format!(
                "rawvideo muxer only accepts stream 0, got stream {}",
                packet.stream_index()
            )));
        }
        self.info.video.validate_frame_payload_len(
            packet.data().len(),
            AvErrorKind::InvalidData,
            "rawvideo packet",
        )?;

        let new_len = self
            .data
            .len()
            .checked_add(packet.data().len())
            .ok_or_else(|| AvError::invalid_argument("rawvideo data size overflow"))?;
        let new_frame_count = self
            .info
            .frame_count
            .checked_add(1)
            .ok_or_else(|| AvError::invalid_argument("rawvideo frame count overflow"))?;

        self.data.reserve(new_len - self.data.len());
        self.data.extend_from_slice(packet.data());
        self.info.frame_count = new_frame_count;
        Ok(())
    }

    pub fn render(&self) -> Vec<u8> {
        self.data.clone()
    }

    pub fn finish(&mut self) -> Vec<u8> {
        self.finished = true;
        self.render()
    }
}

fn validate_frame_rate(frame_rate: Rational) -> AvResult<()> {
    if frame_rate.num() <= 0 || frame_rate.den() <= 0 {
        return Err(AvError::invalid_argument(
            "rawvideo frame rate must be positive",
        ));
    }
    Ok(())
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
                &[0; 6],
                1,
                2,
                RawVideoPixelFormat::Bgr24,
                Rational::new(1, 1).unwrap(),
            )
            .unwrap()
            .info()
            .frame_size(),
            6
        );
        for format in [RawVideoPixelFormat::Gray16Le, RawVideoPixelFormat::Gray16Be] {
            assert_eq!(
                RawVideoDemuxer::open(&[0; 4], 2, 1, format, Rational::new(1, 1).unwrap(),)
                    .unwrap()
                    .info()
                    .frame_size(),
                4
            );
        }
        for format in [RawVideoPixelFormat::Gray32Le, RawVideoPixelFormat::Gray32Be] {
            assert_eq!(
                RawVideoDemuxer::open(&[0; 8], 2, 1, format, Rational::new(1, 1).unwrap(),)
                    .unwrap()
                    .info()
                    .frame_size(),
                8
            );
        }
        for format in [
            RawVideoPixelFormat::GrayF16Le,
            RawVideoPixelFormat::GrayF16Be,
        ] {
            assert_eq!(
                RawVideoDemuxer::open(&[0; 4], 2, 1, format, Rational::new(1, 1).unwrap(),)
                    .unwrap()
                    .info()
                    .frame_size(),
                4
            );
        }
        for format in [
            RawVideoPixelFormat::GrayF32Le,
            RawVideoPixelFormat::GrayF32Be,
        ] {
            assert_eq!(
                RawVideoDemuxer::open(&[0; 8], 2, 1, format, Rational::new(1, 1).unwrap(),)
                    .unwrap()
                    .info()
                    .frame_size(),
                8
            );
        }
        assert_eq!(
            RawVideoDemuxer::open(
                &[0; 8],
                2,
                2,
                RawVideoPixelFormat::Ya8,
                Rational::new(1, 1).unwrap(),
            )
            .unwrap()
            .info()
            .frame_size(),
            8
        );
        for format in [RawVideoPixelFormat::Ya16Le, RawVideoPixelFormat::Ya16Be] {
            assert_eq!(
                RawVideoDemuxer::open(&[0; 8], 2, 1, format, Rational::new(1, 1).unwrap(),)
                    .unwrap()
                    .info()
                    .frame_size(),
                8
            );
        }
        for format in [
            RawVideoPixelFormat::Bgra,
            RawVideoPixelFormat::Argb,
            RawVideoPixelFormat::Abgr,
            RawVideoPixelFormat::ZeroRgb,
            RawVideoPixelFormat::Rgb0,
            RawVideoPixelFormat::ZeroBgr,
            RawVideoPixelFormat::Bgr0,
        ] {
            assert_eq!(
                RawVideoDemuxer::open(&[0; 4], 1, 1, format, Rational::new(1, 1).unwrap(),)
                    .unwrap()
                    .info()
                    .frame_size(),
                4
            );
        }
        assert_eq!(
            RawVideoDemuxer::open(
                &[0; 6],
                2,
                1,
                RawVideoPixelFormat::Gbrp,
                Rational::new(1, 1).unwrap(),
            )
            .unwrap()
            .info()
            .frame_size(),
            6
        );
        assert_eq!(
            RawVideoDemuxer::open(
                &[0; 12],
                2,
                1,
                RawVideoPixelFormat::Gbrp9Le,
                Rational::new(1, 1).unwrap(),
            )
            .unwrap()
            .info()
            .frame_size(),
            12
        );
        assert_eq!(
            RawVideoDemuxer::open(
                &[0; 12],
                2,
                1,
                RawVideoPixelFormat::Gbrp10Le,
                Rational::new(1, 1).unwrap(),
            )
            .unwrap()
            .info()
            .frame_size(),
            12
        );
        assert_eq!(
            RawVideoDemuxer::open(
                &[0; 12],
                2,
                1,
                RawVideoPixelFormat::Gbrp12Le,
                Rational::new(1, 1).unwrap(),
            )
            .unwrap()
            .info()
            .frame_size(),
            12
        );
        assert_eq!(
            RawVideoDemuxer::open(
                &[0; 12],
                2,
                1,
                RawVideoPixelFormat::Gbrp14Le,
                Rational::new(1, 1).unwrap(),
            )
            .unwrap()
            .info()
            .frame_size(),
            12
        );
        assert_eq!(
            RawVideoDemuxer::open(
                &[0; 12],
                2,
                1,
                RawVideoPixelFormat::Gbrp16Le,
                Rational::new(1, 1).unwrap(),
            )
            .unwrap()
            .info()
            .frame_size(),
            12
        );
        assert_eq!(
            RawVideoDemuxer::open(
                &[0; 8],
                2,
                1,
                RawVideoPixelFormat::Gbrap,
                Rational::new(1, 1).unwrap(),
            )
            .unwrap()
            .info()
            .frame_size(),
            8
        );
        for format in [
            RawVideoPixelFormat::Gbrap10Le,
            RawVideoPixelFormat::Gbrap10Be,
            RawVideoPixelFormat::Gbrap12Le,
            RawVideoPixelFormat::Gbrap12Be,
            RawVideoPixelFormat::Gbrap14Le,
            RawVideoPixelFormat::Gbrap14Be,
            RawVideoPixelFormat::Gbrap16Le,
            RawVideoPixelFormat::Gbrap16Be,
            RawVideoPixelFormat::GbrapF16Le,
            RawVideoPixelFormat::GbrapF16Be,
        ] {
            assert_eq!(
                RawVideoDemuxer::open(&[0; 16], 2, 1, format, Rational::new(1, 1).unwrap(),)
                    .unwrap()
                    .info()
                    .frame_size(),
                16
            );
        }
        for format in [
            RawVideoPixelFormat::Gbrap32Le,
            RawVideoPixelFormat::Gbrap32Be,
            RawVideoPixelFormat::GbrapF32Le,
            RawVideoPixelFormat::GbrapF32Be,
        ] {
            assert_eq!(
                RawVideoDemuxer::open(&[0; 32], 2, 1, format, Rational::new(1, 1).unwrap(),)
                    .unwrap()
                    .info()
                    .frame_size(),
                32
            );
        }
        for format in [
            RawVideoPixelFormat::Rgb48Le,
            RawVideoPixelFormat::Rgb48Be,
            RawVideoPixelFormat::Bgr48Le,
            RawVideoPixelFormat::Bgr48Be,
        ] {
            assert_eq!(
                RawVideoDemuxer::open(&[0; 6], 1, 1, format, Rational::new(1, 1).unwrap(),)
                    .unwrap()
                    .info()
                    .frame_size(),
                6
            );
        }
        for format in [
            RawVideoPixelFormat::Rgba64Le,
            RawVideoPixelFormat::Rgba64Be,
            RawVideoPixelFormat::Bgra64Le,
            RawVideoPixelFormat::Bgra64Be,
        ] {
            assert_eq!(
                RawVideoDemuxer::open(&[0; 8], 1, 1, format, Rational::new(1, 1).unwrap(),)
                    .unwrap()
                    .info()
                    .frame_size(),
                8
            );
        }
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
        assert_eq!(
            RawVideoDemuxer::open(
                &[0; 24],
                4,
                3,
                RawVideoPixelFormat::Yuv422p,
                Rational::new(1, 1).unwrap(),
            )
            .unwrap()
            .info()
            .frame_size(),
            24
        );
        assert_eq!(
            RawVideoDemuxer::open(
                &[0; 18],
                4,
                3,
                RawVideoPixelFormat::Yuv411p,
                Rational::new(1, 1).unwrap(),
            )
            .unwrap()
            .info()
            .frame_size(),
            18
        );
        assert_eq!(
            RawVideoDemuxer::open(
                &[0; 18],
                4,
                4,
                RawVideoPixelFormat::Yuv410p,
                Rational::new(1, 1).unwrap(),
            )
            .unwrap()
            .info()
            .frame_size(),
            18
        );
        assert_eq!(
            RawVideoDemuxer::open(
                &[0; 12],
                3,
                2,
                RawVideoPixelFormat::Yuv440p,
                Rational::new(1, 1).unwrap(),
            )
            .unwrap()
            .info()
            .frame_size(),
            12
        );
        assert_eq!(
            RawVideoDemuxer::open(
                &[0; 18],
                3,
                2,
                RawVideoPixelFormat::Yuv444p,
                Rational::new(1, 1).unwrap(),
            )
            .unwrap()
            .info()
            .frame_size(),
            18
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
            &[0; 8],
            3,
            2,
            RawVideoPixelFormat::Yuv422p,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoDemuxer::open(
            &[0; 18],
            3,
            2,
            RawVideoPixelFormat::Yuv411p,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoDemuxer::open(
            &[0; 18],
            4,
            3,
            RawVideoPixelFormat::Yuv411p,
            Rational::new(1, 1).unwrap(),
        )
        .is_ok());
        assert!(RawVideoDemuxer::open(
            &[0; 18],
            4,
            3,
            RawVideoPixelFormat::Yuv410p,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoDemuxer::open(
            &[0; 18],
            2,
            4,
            RawVideoPixelFormat::Yuv410p,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoDemuxer::open(
            &[0; 18],
            4,
            4,
            RawVideoPixelFormat::Yuv410p,
            Rational::new(1, 1).unwrap(),
        )
        .is_ok());
        assert!(RawVideoDemuxer::open(
            &[0; 18],
            3,
            3,
            RawVideoPixelFormat::Yuv440p,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoDemuxer::open(
            &[0; 12],
            3,
            2,
            RawVideoPixelFormat::Yuv440p,
            Rational::new(1, 1).unwrap(),
        )
        .is_ok());
        assert!(RawVideoDemuxer::open(
            &[0; 18],
            3,
            2,
            RawVideoPixelFormat::Yuv444p,
            Rational::new(1, 1).unwrap(),
        )
        .is_ok());
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

    #[test]
    fn muxer_concatenates_fixed_size_stream_zero_frames() {
        let mut muxer = RawVideoMuxer::new(
            1,
            2,
            RawVideoPixelFormat::Rgb24,
            Rational::new(30, 1).unwrap(),
        )
        .unwrap();
        let first = Packet::new(vec![0, 1, 2, 3, 4, 5], 0);
        let second = Packet::new(vec![6, 7, 8, 9, 10, 11], 0);

        muxer.write_packet(&first).unwrap();
        muxer.write_packet(&second).unwrap();

        assert_eq!(muxer.info().width(), 1);
        assert_eq!(muxer.info().height(), 2);
        assert_eq!(muxer.info().pixel_format(), RawVideoPixelFormat::Rgb24);
        assert_eq!(muxer.info().frame_rate(), Rational::new(30, 1).unwrap());
        assert_eq!(muxer.info().frame_size(), 6);
        assert_eq!(muxer.info().frame_count(), 2);
        assert_eq!(muxer.data_len(), 12);
        assert_eq!(muxer.render(), vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]);
    }

    #[test]
    fn muxer_supports_empty_output_when_geometry_is_valid() {
        let mut muxer = RawVideoMuxer::new(
            2,
            2,
            RawVideoPixelFormat::Gray8,
            Rational::new(24, 1).unwrap(),
        )
        .unwrap();

        let output = muxer.finish();

        assert!(muxer.is_finished());
        assert_eq!(muxer.info().frame_size(), 4);
        assert_eq!(muxer.info().frame_count(), 0);
        assert_eq!(muxer.data_len(), 0);
        assert!(output.is_empty());
    }

    #[test]
    fn muxer_computes_yuv420p_frame_size() {
        let muxer = RawVideoMuxer::new(
            4,
            2,
            RawVideoPixelFormat::Yuv420p,
            Rational::new(25, 1).unwrap(),
        )
        .unwrap();

        assert_eq!(muxer.info().frame_size(), 12);
    }

    #[test]
    fn muxer_computes_rgb0_frame_size() {
        let muxer = RawVideoMuxer::new(
            2,
            2,
            RawVideoPixelFormat::Rgb0,
            Rational::new(25, 1).unwrap(),
        )
        .unwrap();

        assert_eq!(muxer.info().frame_size(), 16);
    }

    #[test]
    fn muxer_computes_gray16_frame_size() {
        let muxer = RawVideoMuxer::new(
            3,
            2,
            RawVideoPixelFormat::Gray16Be,
            Rational::new(25, 1).unwrap(),
        )
        .unwrap();

        assert_eq!(muxer.info().frame_size(), 12);
    }

    #[test]
    fn muxer_computes_gray32_frame_size() {
        let muxer = RawVideoMuxer::new(
            3,
            2,
            RawVideoPixelFormat::Gray32Le,
            Rational::new(25, 1).unwrap(),
        )
        .unwrap();

        assert_eq!(muxer.info().frame_size(), 24);
    }

    #[test]
    fn muxer_computes_gray_float_frame_sizes() {
        let grayf16 = RawVideoMuxer::new(
            3,
            2,
            RawVideoPixelFormat::GrayF16Le,
            Rational::new(25, 1).unwrap(),
        )
        .unwrap();

        assert_eq!(grayf16.info().frame_size(), 12);

        let grayf32 = RawVideoMuxer::new(
            3,
            2,
            RawVideoPixelFormat::GrayF32Be,
            Rational::new(25, 1).unwrap(),
        )
        .unwrap();

        assert_eq!(grayf32.info().frame_size(), 24);
    }

    #[test]
    fn muxer_computes_gbrp_frame_size() {
        let muxer = RawVideoMuxer::new(
            3,
            2,
            RawVideoPixelFormat::Gbrp,
            Rational::new(25, 1).unwrap(),
        )
        .unwrap();

        assert_eq!(muxer.info().frame_size(), 18);
    }

    #[test]
    fn muxer_computes_gbrp9_frame_size() {
        let muxer = RawVideoMuxer::new(
            3,
            2,
            RawVideoPixelFormat::Gbrp9Be,
            Rational::new(25, 1).unwrap(),
        )
        .unwrap();

        assert_eq!(muxer.info().frame_size(), 36);
    }

    #[test]
    fn muxer_computes_gbrp10_frame_size() {
        let muxer = RawVideoMuxer::new(
            3,
            2,
            RawVideoPixelFormat::Gbrp10Be,
            Rational::new(25, 1).unwrap(),
        )
        .unwrap();

        assert_eq!(muxer.info().frame_size(), 36);
    }

    #[test]
    fn muxer_computes_gbrp12_frame_size() {
        let muxer = RawVideoMuxer::new(
            3,
            2,
            RawVideoPixelFormat::Gbrp12Be,
            Rational::new(25, 1).unwrap(),
        )
        .unwrap();

        assert_eq!(muxer.info().frame_size(), 36);
    }

    #[test]
    fn muxer_computes_gbrp14_frame_size() {
        let muxer = RawVideoMuxer::new(
            3,
            2,
            RawVideoPixelFormat::Gbrp14Be,
            Rational::new(25, 1).unwrap(),
        )
        .unwrap();

        assert_eq!(muxer.info().frame_size(), 36);
    }

    #[test]
    fn muxer_computes_gbrp16_frame_size() {
        let muxer = RawVideoMuxer::new(
            3,
            2,
            RawVideoPixelFormat::Gbrp16Be,
            Rational::new(25, 1).unwrap(),
        )
        .unwrap();

        assert_eq!(muxer.info().frame_size(), 36);
    }

    #[test]
    fn muxer_computes_gbrap_frame_sizes() {
        let gbrap = RawVideoMuxer::new(
            3,
            2,
            RawVideoPixelFormat::Gbrap,
            Rational::new(25, 1).unwrap(),
        )
        .unwrap();

        assert_eq!(gbrap.info().frame_size(), 24);

        let gbrap16 = RawVideoMuxer::new(
            3,
            2,
            RawVideoPixelFormat::Gbrap16Be,
            Rational::new(25, 1).unwrap(),
        )
        .unwrap();

        assert_eq!(gbrap16.info().frame_size(), 48);

        let gbrap32 = RawVideoMuxer::new(
            3,
            2,
            RawVideoPixelFormat::Gbrap32Le,
            Rational::new(25, 1).unwrap(),
        )
        .unwrap();

        assert_eq!(gbrap32.info().frame_size(), 96);

        let gbrapf16 = RawVideoMuxer::new(
            3,
            2,
            RawVideoPixelFormat::GbrapF16Le,
            Rational::new(25, 1).unwrap(),
        )
        .unwrap();

        assert_eq!(gbrapf16.info().frame_size(), 48);

        let gbrapf32 = RawVideoMuxer::new(
            3,
            2,
            RawVideoPixelFormat::GbrapF32Be,
            Rational::new(25, 1).unwrap(),
        )
        .unwrap();

        assert_eq!(gbrapf32.info().frame_size(), 96);
    }

    #[test]
    fn muxer_computes_ya8_frame_size() {
        let muxer = RawVideoMuxer::new(
            3,
            2,
            RawVideoPixelFormat::Ya8,
            Rational::new(25, 1).unwrap(),
        )
        .unwrap();

        assert_eq!(muxer.info().frame_size(), 12);
    }

    #[test]
    fn muxer_computes_ya16_frame_size() {
        let muxer = RawVideoMuxer::new(
            3,
            2,
            RawVideoPixelFormat::Ya16Le,
            Rational::new(25, 1).unwrap(),
        )
        .unwrap();

        assert_eq!(muxer.info().frame_size(), 24);
    }

    #[test]
    fn muxer_computes_rgb48_frame_size() {
        let muxer = RawVideoMuxer::new(
            2,
            2,
            RawVideoPixelFormat::Rgb48Le,
            Rational::new(25, 1).unwrap(),
        )
        .unwrap();

        assert_eq!(muxer.info().frame_size(), 24);
    }

    #[test]
    fn muxer_computes_rgba64_frame_size() {
        let muxer = RawVideoMuxer::new(
            2,
            2,
            RawVideoPixelFormat::Rgba64Le,
            Rational::new(25, 1).unwrap(),
        )
        .unwrap();

        assert_eq!(muxer.info().frame_size(), 32);
    }

    #[test]
    fn muxer_computes_yuv422p_frame_size() {
        let muxer = RawVideoMuxer::new(
            4,
            3,
            RawVideoPixelFormat::Yuv422p,
            Rational::new(25, 1).unwrap(),
        )
        .unwrap();

        assert_eq!(muxer.info().frame_size(), 24);
    }

    #[test]
    fn muxer_computes_yuv411p_frame_size() {
        let muxer = RawVideoMuxer::new(
            4,
            3,
            RawVideoPixelFormat::Yuv411p,
            Rational::new(25, 1).unwrap(),
        )
        .unwrap();

        assert_eq!(muxer.info().frame_size(), 18);
    }

    #[test]
    fn muxer_computes_yuv410p_frame_size() {
        let muxer = RawVideoMuxer::new(
            4,
            4,
            RawVideoPixelFormat::Yuv410p,
            Rational::new(25, 1).unwrap(),
        )
        .unwrap();

        assert_eq!(muxer.info().frame_size(), 18);
    }

    #[test]
    fn muxer_computes_yuv440p_frame_size() {
        let muxer = RawVideoMuxer::new(
            3,
            2,
            RawVideoPixelFormat::Yuv440p,
            Rational::new(25, 1).unwrap(),
        )
        .unwrap();

        assert_eq!(muxer.info().frame_size(), 12);
    }

    #[test]
    fn muxer_computes_yuv444p_frame_size() {
        let muxer = RawVideoMuxer::new(
            3,
            2,
            RawVideoPixelFormat::Yuv444p,
            Rational::new(25, 1).unwrap(),
        )
        .unwrap();

        assert_eq!(muxer.info().frame_size(), 18);
    }

    #[test]
    fn muxer_rejects_invalid_geometry_rate_stream_and_frame_size() {
        assert!(RawVideoMuxer::new(
            0,
            2,
            RawVideoPixelFormat::Gray8,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoMuxer::new(
            3,
            2,
            RawVideoPixelFormat::Yuv420p,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoMuxer::new(
            3,
            2,
            RawVideoPixelFormat::Yuv422p,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoMuxer::new(
            3,
            2,
            RawVideoPixelFormat::Yuv411p,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoMuxer::new(
            4,
            3,
            RawVideoPixelFormat::Yuv411p,
            Rational::new(1, 1).unwrap(),
        )
        .is_ok());
        assert!(RawVideoMuxer::new(
            4,
            3,
            RawVideoPixelFormat::Yuv410p,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoMuxer::new(
            2,
            4,
            RawVideoPixelFormat::Yuv410p,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoMuxer::new(
            4,
            4,
            RawVideoPixelFormat::Yuv410p,
            Rational::new(1, 1).unwrap(),
        )
        .is_ok());
        assert!(RawVideoMuxer::new(
            3,
            3,
            RawVideoPixelFormat::Yuv440p,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoMuxer::new(
            3,
            2,
            RawVideoPixelFormat::Yuv440p,
            Rational::new(1, 1).unwrap(),
        )
        .is_ok());
        assert!(RawVideoMuxer::new(
            3,
            2,
            RawVideoPixelFormat::Yuv444p,
            Rational::new(1, 1).unwrap(),
        )
        .is_ok());
        assert!(RawVideoMuxer::new(
            2,
            2,
            RawVideoPixelFormat::Gray8,
            Rational::new(0, 1).unwrap(),
        )
        .is_err());

        let mut muxer = RawVideoMuxer::new(
            2,
            2,
            RawVideoPixelFormat::Gray8,
            Rational::new(1, 1).unwrap(),
        )
        .unwrap();
        let wrong_stream = muxer.write_packet(&Packet::new(vec![0, 1, 2, 3], 1));
        assert_eq!(
            wrong_stream.unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
        let short_frame = muxer.write_packet(&Packet::new(vec![0, 1, 2], 0));
        assert_eq!(short_frame.unwrap_err().kind(), AvErrorKind::InvalidData);
        let long_frame = muxer.write_packet(&Packet::new(vec![0, 1, 2, 3, 4], 0));
        assert_eq!(long_frame.unwrap_err().kind(), AvErrorKind::InvalidData);
        assert_eq!(muxer.info().frame_count(), 0);
        assert_eq!(muxer.data_len(), 0);
    }

    #[test]
    fn muxer_finish_prevents_more_writes() {
        let mut muxer = RawVideoMuxer::new(
            2,
            2,
            RawVideoPixelFormat::Gray8,
            Rational::new(1, 1).unwrap(),
        )
        .unwrap();
        let packet = Packet::new(vec![0, 1, 2, 3], 0);

        muxer.write_packet(&packet).unwrap();
        let output = muxer.finish();
        let err = muxer.write_packet(&packet).unwrap_err();

        assert!(muxer.is_finished());
        assert_eq!(output, vec![0, 1, 2, 3]);
        assert_eq!(err.kind(), AvErrorKind::InvalidArgument);
        assert_eq!(muxer.info().frame_count(), 1);
    }
}
