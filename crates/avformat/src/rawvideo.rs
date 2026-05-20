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
    fn slices_pal8_index_frames_with_format_side_data() {
        let input = vec![0, 1, 2, 3, 4, 5, 6, 7];
        let mut demuxer = RawVideoDemuxer::open(
            &input,
            2,
            2,
            RawVideoPixelFormat::Pal8,
            Rational::new(25, 1).unwrap(),
        )
        .unwrap();

        assert_eq!(demuxer.info().pixel_format(), RawVideoPixelFormat::Pal8);
        assert_eq!(demuxer.info().frame_size(), 4);
        assert_eq!(demuxer.info().frame_count(), 2);

        let first = demuxer.read_packet().unwrap().unwrap();
        assert_eq!(first.data(), &[0, 1, 2, 3]);
        assert_eq!(first.side_data()[0].kind(), "rawvideo_pix_fmt");
        assert_eq!(first.side_data()[0].data(), b"pal8");

        let second = demuxer.read_packet().unwrap().unwrap();
        assert_eq!(second.data(), &[4, 5, 6, 7]);
        assert!(demuxer.read_packet().unwrap().is_none());
    }

    #[test]
    fn slices_xv_packed_yuv_frames_with_format_side_data() {
        let input = (0_u8..16).collect::<Vec<_>>();
        let mut demuxer = RawVideoDemuxer::open(
            &input,
            2,
            1,
            RawVideoPixelFormat::Xv30Le,
            Rational::new(24, 1).unwrap(),
        )
        .unwrap();

        assert_eq!(demuxer.info().pixel_format(), RawVideoPixelFormat::Xv30Le);
        assert_eq!(demuxer.info().frame_size(), 8);
        assert_eq!(demuxer.info().frame_count(), 2);

        let first = demuxer.read_packet().unwrap().unwrap();
        assert_eq!(first.data(), &[0, 1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(first.side_data()[0].kind(), "rawvideo_pix_fmt");
        assert_eq!(first.side_data()[0].data(), b"xv30le");

        let second = demuxer.read_packet().unwrap().unwrap();
        assert_eq!(second.data(), &[8, 9, 10, 11, 12, 13, 14, 15]);
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
        for format in [
            RawVideoPixelFormat::X2Rgb10Le,
            RawVideoPixelFormat::X2Rgb10Be,
            RawVideoPixelFormat::X2Bgr10Le,
            RawVideoPixelFormat::X2Bgr10Be,
        ] {
            assert_eq!(
                RawVideoDemuxer::open(&[0; 24], 3, 2, format, Rational::new(1, 1).unwrap(),)
                    .unwrap()
                    .info()
                    .frame_size(),
                24
            );
        }
        for (format, width, height, payload_len, expected_size) in [
            (RawVideoPixelFormat::Xv30Le, 3, 2, 24, 24),
            (RawVideoPixelFormat::Xv30Be, 3, 2, 24, 24),
            (RawVideoPixelFormat::Xv36Le, 3, 2, 36, 36),
            (RawVideoPixelFormat::Xv36Be, 3, 2, 36, 36),
            (RawVideoPixelFormat::Xv48Le, 3, 2, 48, 48),
            (RawVideoPixelFormat::Xv48Be, 3, 2, 48, 48),
            (RawVideoPixelFormat::V30xLe, 3, 2, 24, 24),
            (RawVideoPixelFormat::V30xBe, 3, 2, 24, 24),
        ] {
            assert_eq!(
                RawVideoDemuxer::open(
                    &vec![0; payload_len],
                    width,
                    height,
                    format,
                    Rational::new(1, 1).unwrap(),
                )
                .unwrap()
                .info()
                .frame_size(),
                expected_size
            );
        }
        for format in [
            RawVideoPixelFormat::Vuya,
            RawVideoPixelFormat::Vuyx,
            RawVideoPixelFormat::Ayuv,
            RawVideoPixelFormat::Uyva,
        ] {
            assert_eq!(
                RawVideoDemuxer::open(&[0; 24], 3, 2, format, Rational::new(1, 1).unwrap(),)
                    .unwrap()
                    .info()
                    .frame_size(),
                24
            );
        }
        assert_eq!(
            RawVideoDemuxer::open(
                &[0; 18],
                3,
                2,
                RawVideoPixelFormat::Vyu444,
                Rational::new(1, 1).unwrap(),
            )
            .unwrap()
            .info()
            .frame_size(),
            18
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
        for format in [
            RawVideoPixelFormat::Pal8,
            RawVideoPixelFormat::Rgb8,
            RawVideoPixelFormat::Bgr8,
            RawVideoPixelFormat::Rgb4Byte,
            RawVideoPixelFormat::Bgr4Byte,
        ] {
            assert_eq!(
                RawVideoDemuxer::open(&[0; 4], 2, 2, format, Rational::new(1, 1).unwrap(),)
                    .unwrap()
                    .info()
                    .frame_size(),
                4
            );
        }
        for format in [RawVideoPixelFormat::Rgb4, RawVideoPixelFormat::Bgr4] {
            assert_eq!(
                RawVideoDemuxer::open(&[0; 4], 3, 2, format, Rational::new(1, 1).unwrap(),)
                    .unwrap()
                    .info()
                    .frame_size(),
                4
            );
        }
        for (format, width, height, payload_len, expected_size) in [
            (RawVideoPixelFormat::MonoWhite, 9, 2, 4, 4),
            (RawVideoPixelFormat::MonoBlack, 16, 1, 2, 2),
        ] {
            assert_eq!(
                RawVideoDemuxer::open(
                    &vec![0; payload_len],
                    width,
                    height,
                    format,
                    Rational::new(1, 1).unwrap(),
                )
                .unwrap()
                .info()
                .frame_size(),
                expected_size
            );
        }
        for format in [
            RawVideoPixelFormat::Rgb565Be,
            RawVideoPixelFormat::Rgb565Le,
            RawVideoPixelFormat::Rgb555Be,
            RawVideoPixelFormat::Rgb555Le,
            RawVideoPixelFormat::Bgr565Be,
            RawVideoPixelFormat::Bgr565Le,
            RawVideoPixelFormat::Bgr555Be,
            RawVideoPixelFormat::Bgr555Le,
            RawVideoPixelFormat::Rgb444Le,
            RawVideoPixelFormat::Rgb444Be,
            RawVideoPixelFormat::Bgr444Le,
            RawVideoPixelFormat::Bgr444Be,
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
            RawVideoPixelFormat::Yuyv422,
            RawVideoPixelFormat::Uyvy422,
            RawVideoPixelFormat::Yvyu422,
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
            RawVideoPixelFormat::Y210Le,
            RawVideoPixelFormat::Y210Be,
            RawVideoPixelFormat::Y212Le,
            RawVideoPixelFormat::Y212Be,
            RawVideoPixelFormat::Y216Le,
            RawVideoPixelFormat::Y216Be,
        ] {
            assert_eq!(
                RawVideoDemuxer::open(&[0; 16], 2, 2, format, Rational::new(1, 1).unwrap(),)
                    .unwrap()
                    .info()
                    .frame_size(),
                16
            );
        }
        for format in [RawVideoPixelFormat::Nv12, RawVideoPixelFormat::Nv21] {
            assert_eq!(
                RawVideoDemuxer::open(&[0; 12], 4, 2, format, Rational::new(1, 1).unwrap(),)
                    .unwrap()
                    .info()
                    .frame_size(),
                12
            );
        }
        for (format, width, height, payload_len, expected_size) in [
            (RawVideoPixelFormat::Nv16, 4, 3, 24, 24),
            (RawVideoPixelFormat::Nv20Le, 4, 3, 48, 48),
            (RawVideoPixelFormat::Nv20Be, 4, 3, 48, 48),
            (RawVideoPixelFormat::Nv24, 3, 2, 18, 18),
            (RawVideoPixelFormat::Nv42, 3, 2, 18, 18),
            (RawVideoPixelFormat::P010Le, 4, 2, 24, 24),
            (RawVideoPixelFormat::P010Be, 4, 2, 24, 24),
            (RawVideoPixelFormat::P012Le, 4, 2, 24, 24),
            (RawVideoPixelFormat::P012Be, 4, 2, 24, 24),
            (RawVideoPixelFormat::P016Le, 4, 2, 24, 24),
            (RawVideoPixelFormat::P016Be, 4, 2, 24, 24),
            (RawVideoPixelFormat::P210Le, 4, 3, 48, 48),
            (RawVideoPixelFormat::P210Be, 4, 3, 48, 48),
            (RawVideoPixelFormat::P212Le, 4, 3, 48, 48),
            (RawVideoPixelFormat::P212Be, 4, 3, 48, 48),
            (RawVideoPixelFormat::P216Le, 4, 3, 48, 48),
            (RawVideoPixelFormat::P216Be, 4, 3, 48, 48),
            (RawVideoPixelFormat::P410Le, 3, 2, 36, 36),
            (RawVideoPixelFormat::P410Be, 3, 2, 36, 36),
            (RawVideoPixelFormat::P412Le, 3, 2, 36, 36),
            (RawVideoPixelFormat::P412Be, 3, 2, 36, 36),
            (RawVideoPixelFormat::P416Le, 3, 2, 36, 36),
            (RawVideoPixelFormat::P416Be, 3, 2, 36, 36),
        ] {
            assert_eq!(
                RawVideoDemuxer::open(
                    &vec![0; payload_len],
                    width,
                    height,
                    format,
                    Rational::new(1, 1).unwrap(),
                )
                .unwrap()
                .info()
                .frame_size(),
                expected_size
            );
        }
        for (format, width, height, payload_len, expected_size) in [
            (RawVideoPixelFormat::Yuv420p9Le, 4, 2, 24, 24),
            (RawVideoPixelFormat::Yuv420p9Be, 4, 2, 24, 24),
            (RawVideoPixelFormat::Yuv422p9Le, 4, 3, 48, 48),
            (RawVideoPixelFormat::Yuv422p9Be, 4, 3, 48, 48),
            (RawVideoPixelFormat::Yuv444p9Le, 3, 2, 36, 36),
            (RawVideoPixelFormat::Yuv444p9Be, 3, 2, 36, 36),
            (RawVideoPixelFormat::Yuv420p10Le, 4, 2, 24, 24),
            (RawVideoPixelFormat::Yuv420p10Be, 4, 2, 24, 24),
            (RawVideoPixelFormat::Yuv422p10Le, 4, 3, 48, 48),
            (RawVideoPixelFormat::Yuv422p10Be, 4, 3, 48, 48),
            (RawVideoPixelFormat::Yuv440p10Le, 3, 2, 24, 24),
            (RawVideoPixelFormat::Yuv440p10Be, 3, 2, 24, 24),
            (RawVideoPixelFormat::Yuv444p10Le, 3, 2, 36, 36),
            (RawVideoPixelFormat::Yuv444p10Be, 3, 2, 36, 36),
            (RawVideoPixelFormat::Yuv420p12Le, 4, 2, 24, 24),
            (RawVideoPixelFormat::Yuv420p12Be, 4, 2, 24, 24),
            (RawVideoPixelFormat::Yuv422p12Le, 4, 3, 48, 48),
            (RawVideoPixelFormat::Yuv422p12Be, 4, 3, 48, 48),
            (RawVideoPixelFormat::Yuv440p12Le, 3, 2, 24, 24),
            (RawVideoPixelFormat::Yuv440p12Be, 3, 2, 24, 24),
            (RawVideoPixelFormat::Yuv444p12Le, 3, 2, 36, 36),
            (RawVideoPixelFormat::Yuv444p12Be, 3, 2, 36, 36),
            (RawVideoPixelFormat::Yuv420p14Le, 4, 2, 24, 24),
            (RawVideoPixelFormat::Yuv420p14Be, 4, 2, 24, 24),
            (RawVideoPixelFormat::Yuv422p14Le, 4, 3, 48, 48),
            (RawVideoPixelFormat::Yuv422p14Be, 4, 3, 48, 48),
            (RawVideoPixelFormat::Yuv444p14Le, 3, 2, 36, 36),
            (RawVideoPixelFormat::Yuv444p14Be, 3, 2, 36, 36),
            (RawVideoPixelFormat::Yuv420p16Le, 4, 2, 24, 24),
            (RawVideoPixelFormat::Yuv420p16Be, 4, 2, 24, 24),
            (RawVideoPixelFormat::Yuv422p16Le, 4, 3, 48, 48),
            (RawVideoPixelFormat::Yuv422p16Be, 4, 3, 48, 48),
            (RawVideoPixelFormat::Yuv444p16Le, 3, 2, 36, 36),
            (RawVideoPixelFormat::Yuv444p16Be, 3, 2, 36, 36),
            (RawVideoPixelFormat::Yuva420p, 4, 2, 20, 20),
            (RawVideoPixelFormat::Yuva422p, 4, 3, 36, 36),
            (RawVideoPixelFormat::Yuva444p, 3, 2, 24, 24),
            (RawVideoPixelFormat::Yuva420p9Le, 4, 2, 40, 40),
            (RawVideoPixelFormat::Yuva420p9Be, 4, 2, 40, 40),
            (RawVideoPixelFormat::Yuva422p9Le, 4, 3, 72, 72),
            (RawVideoPixelFormat::Yuva422p9Be, 4, 3, 72, 72),
            (RawVideoPixelFormat::Yuva444p9Le, 3, 2, 48, 48),
            (RawVideoPixelFormat::Yuva444p9Be, 3, 2, 48, 48),
            (RawVideoPixelFormat::Yuva420p10Le, 4, 2, 40, 40),
            (RawVideoPixelFormat::Yuva420p10Be, 4, 2, 40, 40),
            (RawVideoPixelFormat::Yuva422p10Le, 4, 3, 72, 72),
            (RawVideoPixelFormat::Yuva422p10Be, 4, 3, 72, 72),
            (RawVideoPixelFormat::Yuva444p10Le, 3, 2, 48, 48),
            (RawVideoPixelFormat::Yuva444p10Be, 3, 2, 48, 48),
            (RawVideoPixelFormat::Yuva422p12Le, 4, 3, 72, 72),
            (RawVideoPixelFormat::Yuva422p12Be, 4, 3, 72, 72),
            (RawVideoPixelFormat::Yuva444p12Le, 3, 2, 48, 48),
            (RawVideoPixelFormat::Yuva444p12Be, 3, 2, 48, 48),
            (RawVideoPixelFormat::Yuva420p16Le, 4, 2, 40, 40),
            (RawVideoPixelFormat::Yuva420p16Be, 4, 2, 40, 40),
            (RawVideoPixelFormat::Yuva422p16Le, 4, 3, 72, 72),
            (RawVideoPixelFormat::Yuva422p16Be, 4, 3, 72, 72),
            (RawVideoPixelFormat::Yuva444p16Le, 3, 2, 48, 48),
            (RawVideoPixelFormat::Yuva444p16Be, 3, 2, 48, 48),
        ] {
            assert_eq!(
                RawVideoDemuxer::open(
                    &vec![0; payload_len],
                    width,
                    height,
                    format,
                    Rational::new(1, 1).unwrap(),
                )
                .unwrap()
                .info()
                .frame_size(),
                expected_size
            );
        }
        for format in [RawVideoPixelFormat::Gray16Le, RawVideoPixelFormat::Gray16Be] {
            assert_eq!(
                RawVideoDemuxer::open(&[0; 4], 2, 1, format, Rational::new(1, 1).unwrap(),)
                    .unwrap()
                    .info()
                    .frame_size(),
                4
            );
        }
        for format in [
            RawVideoPixelFormat::Gray9Le,
            RawVideoPixelFormat::Gray9Be,
            RawVideoPixelFormat::Gray10Le,
            RawVideoPixelFormat::Gray10Be,
            RawVideoPixelFormat::Gray12Le,
            RawVideoPixelFormat::Gray12Be,
            RawVideoPixelFormat::Gray14Le,
            RawVideoPixelFormat::Gray14Be,
        ] {
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
        for (format, width, height, frame_size) in [
            (RawVideoPixelFormat::Yaf16Le, 2, 2, 16),
            (RawVideoPixelFormat::Yaf16Be, 1, 2, 8),
            (RawVideoPixelFormat::Yaf32Le, 2, 2, 32),
            (RawVideoPixelFormat::Yaf32Be, 1, 2, 16),
        ] {
            assert_eq!(
                RawVideoDemuxer::open(
                    &vec![0; frame_size],
                    width,
                    height,
                    format,
                    Rational::new(1, 1).unwrap(),
                )
                .unwrap()
                .info()
                .frame_size(),
                frame_size
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
            RawVideoPixelFormat::Ayuv64Le,
            RawVideoPixelFormat::Ayuv64Be,
        ] {
            assert_eq!(
                RawVideoDemuxer::open(&[0; 8], 1, 1, format, Rational::new(1, 1).unwrap(),)
                    .unwrap()
                    .info()
                    .frame_size(),
                8
            );
        }
        for format in [RawVideoPixelFormat::Xyz12Le, RawVideoPixelFormat::Xyz12Be] {
            assert_eq!(
                RawVideoDemuxer::open(&[0; 12], 1, 2, format, Rational::new(1, 1).unwrap(),)
                    .unwrap()
                    .info()
                    .frame_size(),
                12
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
        for (format, width, height, payload_len, expected_size) in [
            (RawVideoPixelFormat::YuvJ420p, 4, 2, 12, 12),
            (RawVideoPixelFormat::YuvJ422p, 4, 3, 24, 24),
            (RawVideoPixelFormat::YuvJ411p, 4, 3, 18, 18),
            (RawVideoPixelFormat::YuvJ440p, 3, 2, 12, 12),
            (RawVideoPixelFormat::YuvJ444p, 3, 2, 18, 18),
        ] {
            assert_eq!(
                RawVideoDemuxer::open(
                    &vec![0; payload_len],
                    width,
                    height,
                    format,
                    Rational::new(1, 1).unwrap(),
                )
                .unwrap()
                .info()
                .frame_size(),
                expected_size
            );
        }
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
            &[0; 12],
            3,
            2,
            RawVideoPixelFormat::YuvJ420p,
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
            &[0; 8],
            3,
            2,
            RawVideoPixelFormat::YuvJ422p,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoDemuxer::open(
            &[0; 24],
            3,
            2,
            RawVideoPixelFormat::Yuv420p9Le,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoDemuxer::open(
            &[0; 24],
            4,
            3,
            RawVideoPixelFormat::Yuv420p9Be,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoDemuxer::open(
            &[0; 48],
            3,
            2,
            RawVideoPixelFormat::Yuv422p9Le,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoDemuxer::open(
            &[0; 36],
            3,
            2,
            RawVideoPixelFormat::Yuv444p9Be,
            Rational::new(1, 1).unwrap(),
        )
        .is_ok());
        assert!(RawVideoDemuxer::open(
            &[0; 24],
            3,
            2,
            RawVideoPixelFormat::Yuv420p14Le,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoDemuxer::open(
            &[0; 24],
            4,
            3,
            RawVideoPixelFormat::Yuv420p16Be,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoDemuxer::open(
            &[0; 48],
            3,
            2,
            RawVideoPixelFormat::Yuv422p14Le,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoDemuxer::open(
            &[0; 36],
            3,
            2,
            RawVideoPixelFormat::Yuv444p16Be,
            Rational::new(1, 1).unwrap(),
        )
        .is_ok());
        assert!(RawVideoDemuxer::open(
            &[0; 12],
            3,
            2,
            RawVideoPixelFormat::Yuyv422,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoDemuxer::open(
            &[0; 12],
            2,
            3,
            RawVideoPixelFormat::Yuyv422,
            Rational::new(1, 1).unwrap(),
        )
        .is_ok());
        assert!(RawVideoDemuxer::open(
            &[0; 12],
            3,
            2,
            RawVideoPixelFormat::Nv12,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoDemuxer::open(
            &[0; 12],
            2,
            3,
            RawVideoPixelFormat::Nv12,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoDemuxer::open(
            &[0; 6],
            2,
            2,
            RawVideoPixelFormat::Nv12,
            Rational::new(1, 1).unwrap(),
        )
        .is_ok());
        assert!(RawVideoDemuxer::open(
            &[0; 24],
            3,
            2,
            RawVideoPixelFormat::Nv16,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoDemuxer::open(
            &[0; 48],
            3,
            2,
            RawVideoPixelFormat::Nv20Be,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoDemuxer::open(
            &[0; 18],
            3,
            2,
            RawVideoPixelFormat::Nv24,
            Rational::new(1, 1).unwrap(),
        )
        .is_ok());
        assert!(RawVideoDemuxer::open(
            &[0; 24],
            3,
            2,
            RawVideoPixelFormat::P010Le,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoDemuxer::open(
            &[0; 24],
            4,
            3,
            RawVideoPixelFormat::P010Be,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoDemuxer::open(
            &[0; 48],
            3,
            2,
            RawVideoPixelFormat::P210Be,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoDemuxer::open(
            &[0; 36],
            3,
            2,
            RawVideoPixelFormat::P410Le,
            Rational::new(1, 1).unwrap(),
        )
        .is_ok());
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
            3,
            2,
            RawVideoPixelFormat::YuvJ411p,
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
            &[0; 18],
            3,
            3,
            RawVideoPixelFormat::YuvJ440p,
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
            &[0; 24],
            3,
            3,
            RawVideoPixelFormat::Yuv440p10Le,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoDemuxer::open(
            &[0; 24],
            3,
            2,
            RawVideoPixelFormat::Yuv440p10Le,
            Rational::new(1, 1).unwrap(),
        )
        .is_ok());
        assert!(RawVideoDemuxer::open(
            &[0; 24],
            3,
            3,
            RawVideoPixelFormat::Yuv440p12Be,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoDemuxer::open(
            &[0; 24],
            3,
            2,
            RawVideoPixelFormat::Yuv440p12Be,
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
            &[0; 18],
            3,
            2,
            RawVideoPixelFormat::YuvJ444p,
            Rational::new(1, 1).unwrap(),
        )
        .is_ok());
        assert!(RawVideoDemuxer::open(
            &[0; 40],
            3,
            2,
            RawVideoPixelFormat::Yuva420p9Le,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoDemuxer::open(
            &[0; 40],
            4,
            3,
            RawVideoPixelFormat::Yuva420p10Be,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoDemuxer::open(
            &[0; 72],
            3,
            2,
            RawVideoPixelFormat::Yuva422p16Le,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoDemuxer::open(
            &[0; 48],
            3,
            2,
            RawVideoPixelFormat::Yuva444p12Be,
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
    fn muxer_computes_low_bit_depth_rgb_byte_frame_sizes() {
        for format in [
            RawVideoPixelFormat::Pal8,
            RawVideoPixelFormat::Rgb8,
            RawVideoPixelFormat::Bgr8,
            RawVideoPixelFormat::Rgb4Byte,
            RawVideoPixelFormat::Bgr4Byte,
        ] {
            let muxer = RawVideoMuxer::new(3, 2, format, Rational::new(25, 1).unwrap()).unwrap();

            assert_eq!(muxer.info().frame_size(), 6);
        }
    }

    #[test]
    fn muxer_computes_packed_4bit_rgb_frame_sizes() {
        for format in [RawVideoPixelFormat::Rgb4, RawVideoPixelFormat::Bgr4] {
            let muxer = RawVideoMuxer::new(3, 2, format, Rational::new(25, 1).unwrap()).unwrap();

            assert_eq!(muxer.info().frame_size(), 4);
        }
    }

    #[test]
    fn muxer_computes_mono_bitstream_frame_sizes() {
        for (format, width, height, expected_size) in [
            (RawVideoPixelFormat::MonoWhite, 9, 2, 4),
            (RawVideoPixelFormat::MonoBlack, 16, 1, 2),
        ] {
            let muxer =
                RawVideoMuxer::new(width, height, format, Rational::new(25, 1).unwrap()).unwrap();

            assert_eq!(muxer.info().frame_size(), expected_size);
        }
    }

    #[test]
    fn muxer_computes_packed_16bit_rgb_frame_sizes() {
        for format in [
            RawVideoPixelFormat::Rgb565Le,
            RawVideoPixelFormat::Rgb555Be,
            RawVideoPixelFormat::Bgr565Le,
            RawVideoPixelFormat::Bgr555Be,
            RawVideoPixelFormat::Rgb444Le,
            RawVideoPixelFormat::Bgr444Be,
        ] {
            let muxer = RawVideoMuxer::new(3, 2, format, Rational::new(25, 1).unwrap()).unwrap();

            assert_eq!(muxer.info().frame_size(), 12);
        }
    }

    #[test]
    fn muxer_computes_x2rgb10_frame_sizes() {
        for format in [
            RawVideoPixelFormat::X2Rgb10Le,
            RawVideoPixelFormat::X2Rgb10Be,
            RawVideoPixelFormat::X2Bgr10Le,
            RawVideoPixelFormat::X2Bgr10Be,
        ] {
            let muxer = RawVideoMuxer::new(3, 2, format, Rational::new(25, 1).unwrap()).unwrap();

            assert_eq!(muxer.info().frame_size(), 24);
        }
    }

    #[test]
    fn muxer_computes_xv_packed_yuv_frame_sizes() {
        for (format, width, height, expected_size) in [
            (RawVideoPixelFormat::Xv30Le, 3, 2, 24),
            (RawVideoPixelFormat::Xv30Be, 3, 2, 24),
            (RawVideoPixelFormat::Xv36Le, 3, 2, 36),
            (RawVideoPixelFormat::Xv36Be, 3, 2, 36),
            (RawVideoPixelFormat::Xv48Le, 3, 2, 48),
            (RawVideoPixelFormat::Xv48Be, 3, 2, 48),
            (RawVideoPixelFormat::V30xLe, 3, 2, 24),
            (RawVideoPixelFormat::V30xBe, 3, 2, 24),
        ] {
            let muxer =
                RawVideoMuxer::new(width, height, format, Rational::new(25, 1).unwrap()).unwrap();

            assert_eq!(muxer.info().frame_size(), expected_size);
        }
    }

    #[test]
    fn muxer_computes_packed_yuv422_frame_sizes() {
        for format in [
            RawVideoPixelFormat::Yuyv422,
            RawVideoPixelFormat::Uyvy422,
            RawVideoPixelFormat::Yvyu422,
        ] {
            let muxer = RawVideoMuxer::new(4, 2, format, Rational::new(25, 1).unwrap()).unwrap();

            assert_eq!(muxer.info().frame_size(), 16);
        }
    }

    #[test]
    fn muxer_computes_semiplanar_yuv420_frame_sizes() {
        for format in [RawVideoPixelFormat::Nv12, RawVideoPixelFormat::Nv21] {
            let muxer = RawVideoMuxer::new(4, 2, format, Rational::new(25, 1).unwrap()).unwrap();

            assert_eq!(muxer.info().frame_size(), 12);
        }
    }

    #[test]
    fn muxer_computes_semiplanar_yuv422_and_yuv444_frame_sizes() {
        for (format, width, height, expected_size) in [
            (RawVideoPixelFormat::Nv16, 4, 3, 24),
            (RawVideoPixelFormat::Nv20Le, 4, 3, 48),
            (RawVideoPixelFormat::Nv20Be, 4, 3, 48),
            (RawVideoPixelFormat::Nv24, 3, 2, 18),
            (RawVideoPixelFormat::Nv42, 3, 2, 18),
            (RawVideoPixelFormat::P010Le, 4, 2, 24),
            (RawVideoPixelFormat::P010Be, 4, 2, 24),
            (RawVideoPixelFormat::P012Le, 4, 2, 24),
            (RawVideoPixelFormat::P012Be, 4, 2, 24),
            (RawVideoPixelFormat::P016Le, 4, 2, 24),
            (RawVideoPixelFormat::P016Be, 4, 2, 24),
            (RawVideoPixelFormat::P210Le, 4, 3, 48),
            (RawVideoPixelFormat::P210Be, 4, 3, 48),
            (RawVideoPixelFormat::P212Le, 4, 3, 48),
            (RawVideoPixelFormat::P212Be, 4, 3, 48),
            (RawVideoPixelFormat::P216Le, 4, 3, 48),
            (RawVideoPixelFormat::P216Be, 4, 3, 48),
            (RawVideoPixelFormat::P410Le, 3, 2, 36),
            (RawVideoPixelFormat::P410Be, 3, 2, 36),
            (RawVideoPixelFormat::P412Le, 3, 2, 36),
            (RawVideoPixelFormat::P412Be, 3, 2, 36),
            (RawVideoPixelFormat::P416Le, 3, 2, 36),
            (RawVideoPixelFormat::P416Be, 3, 2, 36),
        ] {
            let muxer =
                RawVideoMuxer::new(width, height, format, Rational::new(25, 1).unwrap()).unwrap();

            assert_eq!(muxer.info().frame_size(), expected_size);
        }
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
    fn muxer_computes_high_bit_depth_gray_frame_sizes() {
        for format in [
            RawVideoPixelFormat::Gray9Le,
            RawVideoPixelFormat::Gray10Be,
            RawVideoPixelFormat::Gray12Le,
            RawVideoPixelFormat::Gray14Be,
        ] {
            let muxer = RawVideoMuxer::new(3, 2, format, Rational::new(25, 1).unwrap()).unwrap();

            assert_eq!(muxer.info().frame_size(), 12);
        }
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
    fn muxer_computes_yaf_frame_sizes() {
        for (format, width, height, expected_size) in [
            (RawVideoPixelFormat::Yaf16Le, 3, 2, 24),
            (RawVideoPixelFormat::Yaf16Be, 1, 2, 8),
            (RawVideoPixelFormat::Yaf32Le, 3, 2, 48),
            (RawVideoPixelFormat::Yaf32Be, 1, 2, 16),
        ] {
            let muxer =
                RawVideoMuxer::new(width, height, format, Rational::new(25, 1).unwrap()).unwrap();

            assert_eq!(muxer.info().frame_size(), expected_size);
        }
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
    fn muxer_computes_ayuv64_frame_size() {
        for format in [RawVideoPixelFormat::Ayuv64Le, RawVideoPixelFormat::Ayuv64Be] {
            let muxer = RawVideoMuxer::new(2, 2, format, Rational::new(25, 1).unwrap()).unwrap();

            assert_eq!(muxer.info().frame_size(), 32);
        }
    }

    #[test]
    fn muxer_computes_xyz12_frame_size() {
        for format in [RawVideoPixelFormat::Xyz12Le, RawVideoPixelFormat::Xyz12Be] {
            let muxer = RawVideoMuxer::new(2, 2, format, Rational::new(25, 1).unwrap()).unwrap();

            assert_eq!(muxer.info().frame_size(), 24);
        }
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
    fn muxer_computes_packed_8bit_yuv_frame_sizes() {
        for (format, width, height, expected_size) in [
            (RawVideoPixelFormat::Vuya, 3, 2, 24),
            (RawVideoPixelFormat::Vuyx, 3, 2, 24),
            (RawVideoPixelFormat::Ayuv, 3, 2, 24),
            (RawVideoPixelFormat::Uyva, 3, 2, 24),
            (RawVideoPixelFormat::Vyu444, 3, 2, 18),
        ] {
            let muxer =
                RawVideoMuxer::new(width, height, format, Rational::new(25, 1).unwrap()).unwrap();

            assert_eq!(muxer.info().frame_size(), expected_size);
        }
    }

    #[test]
    fn muxer_computes_yuva_frame_sizes() {
        for (format, width, height, expected_size) in [
            (RawVideoPixelFormat::Yuva420p, 4, 2, 20),
            (RawVideoPixelFormat::Yuva422p, 4, 3, 36),
            (RawVideoPixelFormat::Yuva444p, 3, 2, 24),
        ] {
            let muxer =
                RawVideoMuxer::new(width, height, format, Rational::new(25, 1).unwrap()).unwrap();

            assert_eq!(muxer.info().frame_size(), expected_size);
        }
    }

    #[test]
    fn muxer_computes_high_bit_depth_yuva_frame_sizes() {
        for (format, width, height, expected_size) in [
            (RawVideoPixelFormat::Yuva420p9Le, 4, 2, 40),
            (RawVideoPixelFormat::Yuva420p9Be, 4, 2, 40),
            (RawVideoPixelFormat::Yuva422p9Le, 4, 3, 72),
            (RawVideoPixelFormat::Yuva422p9Be, 4, 3, 72),
            (RawVideoPixelFormat::Yuva444p9Le, 3, 2, 48),
            (RawVideoPixelFormat::Yuva444p9Be, 3, 2, 48),
            (RawVideoPixelFormat::Yuva420p10Le, 4, 2, 40),
            (RawVideoPixelFormat::Yuva420p10Be, 4, 2, 40),
            (RawVideoPixelFormat::Yuva422p10Le, 4, 3, 72),
            (RawVideoPixelFormat::Yuva422p10Be, 4, 3, 72),
            (RawVideoPixelFormat::Yuva444p10Le, 3, 2, 48),
            (RawVideoPixelFormat::Yuva444p10Be, 3, 2, 48),
            (RawVideoPixelFormat::Yuva422p12Le, 4, 3, 72),
            (RawVideoPixelFormat::Yuva422p12Be, 4, 3, 72),
            (RawVideoPixelFormat::Yuva444p12Le, 3, 2, 48),
            (RawVideoPixelFormat::Yuva444p12Be, 3, 2, 48),
            (RawVideoPixelFormat::Yuva420p16Le, 4, 2, 40),
            (RawVideoPixelFormat::Yuva420p16Be, 4, 2, 40),
            (RawVideoPixelFormat::Yuva422p16Le, 4, 3, 72),
            (RawVideoPixelFormat::Yuva422p16Be, 4, 3, 72),
            (RawVideoPixelFormat::Yuva444p16Le, 3, 2, 48),
            (RawVideoPixelFormat::Yuva444p16Be, 3, 2, 48),
        ] {
            let muxer =
                RawVideoMuxer::new(width, height, format, Rational::new(25, 1).unwrap()).unwrap();

            assert_eq!(muxer.info().frame_size(), expected_size);
        }
    }

    #[test]
    fn muxer_computes_yuvj_frame_sizes() {
        for (format, width, height, expected_size) in [
            (RawVideoPixelFormat::YuvJ420p, 4, 2, 12),
            (RawVideoPixelFormat::YuvJ422p, 4, 3, 24),
            (RawVideoPixelFormat::YuvJ411p, 4, 3, 18),
            (RawVideoPixelFormat::YuvJ440p, 3, 2, 12),
            (RawVideoPixelFormat::YuvJ444p, 3, 2, 18),
        ] {
            let muxer =
                RawVideoMuxer::new(width, height, format, Rational::new(25, 1).unwrap()).unwrap();

            assert_eq!(muxer.info().frame_size(), expected_size);
        }
    }

    #[test]
    fn muxer_computes_high_bit_depth_planar_yuv_frame_sizes() {
        for (format, width, height, expected_size) in [
            (RawVideoPixelFormat::Yuv420p9Le, 4, 2, 24),
            (RawVideoPixelFormat::Yuv420p9Be, 4, 2, 24),
            (RawVideoPixelFormat::Yuv422p9Le, 4, 3, 48),
            (RawVideoPixelFormat::Yuv422p9Be, 4, 3, 48),
            (RawVideoPixelFormat::Yuv444p9Le, 3, 2, 36),
            (RawVideoPixelFormat::Yuv444p9Be, 3, 2, 36),
            (RawVideoPixelFormat::Yuv420p10Le, 4, 2, 24),
            (RawVideoPixelFormat::Yuv420p10Be, 4, 2, 24),
            (RawVideoPixelFormat::Yuv422p10Le, 4, 3, 48),
            (RawVideoPixelFormat::Yuv422p10Be, 4, 3, 48),
            (RawVideoPixelFormat::Yuv440p10Le, 3, 2, 24),
            (RawVideoPixelFormat::Yuv440p10Be, 3, 2, 24),
            (RawVideoPixelFormat::Yuv444p10Le, 3, 2, 36),
            (RawVideoPixelFormat::Yuv444p10Be, 3, 2, 36),
            (RawVideoPixelFormat::Yuv420p12Le, 4, 2, 24),
            (RawVideoPixelFormat::Yuv420p12Be, 4, 2, 24),
            (RawVideoPixelFormat::Yuv422p12Le, 4, 3, 48),
            (RawVideoPixelFormat::Yuv422p12Be, 4, 3, 48),
            (RawVideoPixelFormat::Yuv440p12Le, 3, 2, 24),
            (RawVideoPixelFormat::Yuv440p12Be, 3, 2, 24),
            (RawVideoPixelFormat::Yuv444p12Le, 3, 2, 36),
            (RawVideoPixelFormat::Yuv444p12Be, 3, 2, 36),
            (RawVideoPixelFormat::Yuv420p14Le, 4, 2, 24),
            (RawVideoPixelFormat::Yuv420p14Be, 4, 2, 24),
            (RawVideoPixelFormat::Yuv422p14Le, 4, 3, 48),
            (RawVideoPixelFormat::Yuv422p14Be, 4, 3, 48),
            (RawVideoPixelFormat::Yuv444p14Le, 3, 2, 36),
            (RawVideoPixelFormat::Yuv444p14Be, 3, 2, 36),
            (RawVideoPixelFormat::Yuv420p16Le, 4, 2, 24),
            (RawVideoPixelFormat::Yuv420p16Be, 4, 2, 24),
            (RawVideoPixelFormat::Yuv422p16Le, 4, 3, 48),
            (RawVideoPixelFormat::Yuv422p16Be, 4, 3, 48),
            (RawVideoPixelFormat::Yuv444p16Le, 3, 2, 36),
            (RawVideoPixelFormat::Yuv444p16Be, 3, 2, 36),
        ] {
            let muxer =
                RawVideoMuxer::new(width, height, format, Rational::new(25, 1).unwrap()).unwrap();

            assert_eq!(muxer.info().frame_size(), expected_size);
        }
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
            RawVideoPixelFormat::YuvJ420p,
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
            RawVideoPixelFormat::YuvJ422p,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoMuxer::new(
            3,
            2,
            RawVideoPixelFormat::Yuv420p9Le,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoMuxer::new(
            4,
            3,
            RawVideoPixelFormat::Yuv420p9Be,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoMuxer::new(
            3,
            2,
            RawVideoPixelFormat::Yuv422p9Le,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoMuxer::new(
            3,
            2,
            RawVideoPixelFormat::Yuv444p9Be,
            Rational::new(1, 1).unwrap(),
        )
        .is_ok());
        assert!(RawVideoMuxer::new(
            3,
            2,
            RawVideoPixelFormat::Yuv420p10Le,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoMuxer::new(
            4,
            3,
            RawVideoPixelFormat::Yuv420p12Be,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoMuxer::new(
            3,
            2,
            RawVideoPixelFormat::Yuv422p10Be,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoMuxer::new(
            4,
            3,
            RawVideoPixelFormat::Yuv422p12Be,
            Rational::new(1, 1).unwrap(),
        )
        .is_ok());
        assert!(RawVideoMuxer::new(
            3,
            2,
            RawVideoPixelFormat::Yuv444p10Le,
            Rational::new(1, 1).unwrap(),
        )
        .is_ok());
        assert!(RawVideoMuxer::new(
            3,
            2,
            RawVideoPixelFormat::Yuv420p14Le,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoMuxer::new(
            4,
            3,
            RawVideoPixelFormat::Yuv420p16Be,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoMuxer::new(
            3,
            2,
            RawVideoPixelFormat::Yuv422p14Le,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoMuxer::new(
            3,
            2,
            RawVideoPixelFormat::Yuv444p16Be,
            Rational::new(1, 1).unwrap(),
        )
        .is_ok());
        assert!(RawVideoMuxer::new(
            3,
            2,
            RawVideoPixelFormat::Yuyv422,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoMuxer::new(
            2,
            3,
            RawVideoPixelFormat::Yuyv422,
            Rational::new(1, 1).unwrap(),
        )
        .is_ok());
        assert!(RawVideoMuxer::new(
            3,
            2,
            RawVideoPixelFormat::Y210Le,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoMuxer::new(
            2,
            2,
            RawVideoPixelFormat::Y210Be,
            Rational::new(1, 1).unwrap(),
        )
        .is_ok());
        assert!(RawVideoMuxer::new(
            3,
            2,
            RawVideoPixelFormat::Y212Le,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoMuxer::new(
            2,
            2,
            RawVideoPixelFormat::Y212Be,
            Rational::new(1, 1).unwrap(),
        )
        .is_ok());
        assert!(RawVideoMuxer::new(
            3,
            2,
            RawVideoPixelFormat::Y216Le,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoMuxer::new(
            2,
            2,
            RawVideoPixelFormat::Y216Be,
            Rational::new(1, 1).unwrap(),
        )
        .is_ok());
        assert!(RawVideoMuxer::new(
            3,
            2,
            RawVideoPixelFormat::Nv12,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoMuxer::new(
            2,
            3,
            RawVideoPixelFormat::Nv12,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoMuxer::new(
            2,
            2,
            RawVideoPixelFormat::Nv12,
            Rational::new(1, 1).unwrap(),
        )
        .is_ok());
        assert!(RawVideoMuxer::new(
            3,
            2,
            RawVideoPixelFormat::Nv16,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoMuxer::new(
            3,
            2,
            RawVideoPixelFormat::Nv20Be,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoMuxer::new(
            3,
            2,
            RawVideoPixelFormat::Nv24,
            Rational::new(1, 1).unwrap(),
        )
        .is_ok());
        assert!(RawVideoMuxer::new(
            3,
            2,
            RawVideoPixelFormat::P010Le,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoMuxer::new(
            4,
            3,
            RawVideoPixelFormat::P010Be,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoMuxer::new(
            3,
            2,
            RawVideoPixelFormat::P210Be,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoMuxer::new(
            3,
            2,
            RawVideoPixelFormat::P410Le,
            Rational::new(1, 1).unwrap(),
        )
        .is_ok());
        assert!(RawVideoMuxer::new(
            3,
            2,
            RawVideoPixelFormat::Yuv411p,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoMuxer::new(
            3,
            2,
            RawVideoPixelFormat::YuvJ411p,
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
            3,
            RawVideoPixelFormat::YuvJ440p,
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
            3,
            RawVideoPixelFormat::Yuv440p10Le,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoMuxer::new(
            3,
            2,
            RawVideoPixelFormat::Yuv440p10Le,
            Rational::new(1, 1).unwrap(),
        )
        .is_ok());
        assert!(RawVideoMuxer::new(
            3,
            3,
            RawVideoPixelFormat::Yuv440p12Be,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoMuxer::new(
            3,
            2,
            RawVideoPixelFormat::Yuv440p12Be,
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
            3,
            2,
            RawVideoPixelFormat::Yuva420p,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoMuxer::new(
            4,
            3,
            RawVideoPixelFormat::Yuva420p,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoMuxer::new(
            3,
            2,
            RawVideoPixelFormat::Yuva422p,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoMuxer::new(
            4,
            3,
            RawVideoPixelFormat::Yuva422p,
            Rational::new(1, 1).unwrap(),
        )
        .is_ok());
        assert!(RawVideoMuxer::new(
            3,
            2,
            RawVideoPixelFormat::Yuva444p,
            Rational::new(1, 1).unwrap(),
        )
        .is_ok());
        assert!(RawVideoMuxer::new(
            3,
            2,
            RawVideoPixelFormat::Yuva420p9Le,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoMuxer::new(
            4,
            3,
            RawVideoPixelFormat::Yuva420p10Be,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoMuxer::new(
            3,
            2,
            RawVideoPixelFormat::Yuva422p16Le,
            Rational::new(1, 1).unwrap(),
        )
        .is_err());
        assert!(RawVideoMuxer::new(
            3,
            2,
            RawVideoPixelFormat::Yuva444p12Be,
            Rational::new(1, 1).unwrap(),
        )
        .is_ok());
        assert!(RawVideoMuxer::new(
            3,
            2,
            RawVideoPixelFormat::YuvJ444p,
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
