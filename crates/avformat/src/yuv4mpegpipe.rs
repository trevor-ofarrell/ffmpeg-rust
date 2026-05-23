use crate::VideoStreamParameters;
use avutil::{AvError, AvErrorKind, AvResult, Packet, PixelFormat, Rational};

const Y4M_MAGIC: &str = "YUV4MPEG2";
const FRAME_MAGIC: &str = "FRAME";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Yuv4MpegChroma {
    C420Jpeg,
}

impl Yuv4MpegChroma {
    pub fn name(self) -> &'static str {
        match self {
            Self::C420Jpeg => "420jpeg",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Yuv4MpegInterlace {
    Progressive,
}

impl Yuv4MpegInterlace {
    pub fn tag(self) -> &'static str {
        match self {
            Self::Progressive => "p",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Yuv4MpegInfo {
    video: VideoStreamParameters,
    frame_rate: Rational,
    sample_aspect_ratio: Option<Rational>,
    interlace: Yuv4MpegInterlace,
    chroma: Yuv4MpegChroma,
}

impl Yuv4MpegInfo {
    pub fn width(&self) -> u32 {
        video_width_u32(self.video).expect("YUV4MPEG2 stores u32 width")
    }

    pub fn height(&self) -> u32 {
        video_height_u32(self.video).expect("YUV4MPEG2 stores u32 height")
    }

    pub fn pixel_format(&self) -> PixelFormat {
        self.video.pixel_format()
    }

    pub fn frame_rate(&self) -> Rational {
        self.frame_rate
    }

    pub fn sample_aspect_ratio(&self) -> Option<Rational> {
        self.sample_aspect_ratio
    }

    pub fn interlace(&self) -> Yuv4MpegInterlace {
        self.interlace
    }

    pub fn chroma(&self) -> Yuv4MpegChroma {
        self.chroma
    }

    pub fn frame_size(&self) -> usize {
        self.video.frame_size()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Yuv4MpegDemuxer<'a> {
    info: Yuv4MpegInfo,
    input: &'a [u8],
    position: usize,
    next_pts: i64,
}

impl<'a> Yuv4MpegDemuxer<'a> {
    pub fn open(input: &'a [u8]) -> AvResult<Self> {
        let mut position = 0;
        let header = read_required_line(input, &mut position, "YUV4MPEG2 header")?;
        let info = parse_header(header)?;

        Ok(Self {
            info,
            input,
            position,
            next_pts: 0,
        })
    }

    pub fn info(&self) -> &Yuv4MpegInfo {
        &self.info
    }

    pub fn read_packet(&mut self) -> AvResult<Option<Packet>> {
        if self.position == self.input.len() {
            return Ok(None);
        }

        let frame_header =
            read_required_line(self.input, &mut self.position, "YUV4MPEG2 frame header")?;
        parse_frame_header(frame_header)?;

        let frame_end = self
            .position
            .checked_add(self.info.frame_size())
            .ok_or_else(|| AvError::invalid_data("YUV4MPEG2 frame size overflow"))?;
        if frame_end > self.input.len() {
            return Err(AvError::new(
                AvErrorKind::EndOfFile,
                "YUV4MPEG2 frame payload is truncated",
            ));
        }

        let mut packet = Packet::new(self.input[self.position..frame_end].to_vec(), 0);
        packet.set_pts(Some(self.next_pts));
        packet.set_dts(Some(self.next_pts));
        packet.set_duration(1)?;
        self.position = frame_end;
        self.next_pts = self
            .next_pts
            .checked_add(1)
            .ok_or_else(|| AvError::invalid_data("YUV4MPEG2 PTS overflow"))?;
        Ok(Some(packet))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Yuv4MpegMuxer {
    info: Yuv4MpegInfo,
    payload: Vec<u8>,
    frame_count: usize,
    finished: bool,
}

impl Yuv4MpegMuxer {
    pub fn new(
        width: u32,
        height: u32,
        frame_rate: Rational,
        sample_aspect_ratio: Option<Rational>,
    ) -> AvResult<Self> {
        let video = yuv420_user_video_parameters(width, height)?;
        validate_positive_rational(frame_rate, "YUV4MPEG2 frame rate")?;
        if let Some(sample_aspect_ratio) = sample_aspect_ratio {
            validate_positive_rational(sample_aspect_ratio, "YUV4MPEG2 sample aspect ratio")?;
        }

        Ok(Self {
            info: Yuv4MpegInfo {
                video,
                frame_rate,
                sample_aspect_ratio,
                interlace: Yuv4MpegInterlace::Progressive,
                chroma: Yuv4MpegChroma::C420Jpeg,
            },
            payload: Vec::new(),
            frame_count: 0,
            finished: false,
        })
    }

    pub fn info(&self) -> &Yuv4MpegInfo {
        &self.info
    }

    pub fn frame_count(&self) -> usize {
        self.frame_count
    }

    pub fn payload_len(&self) -> usize {
        self.payload.len()
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }

    pub fn header(&self) -> String {
        let mut header = format!(
            "{Y4M_MAGIC} W{} H{} F{}:{} I{}",
            self.info.width(),
            self.info.height(),
            self.info.frame_rate.num(),
            self.info.frame_rate.den(),
            self.info.interlace.tag()
        );
        match self.info.sample_aspect_ratio {
            Some(sample_aspect_ratio) => {
                header.push_str(&format!(
                    " A{}:{}",
                    sample_aspect_ratio.num(),
                    sample_aspect_ratio.den()
                ));
            }
            None => header.push_str(" A0:0"),
        }
        header.push_str(&format!(" C{} XYSCSS=420JPEG\n", self.info.chroma.name()));
        header
    }

    pub fn write_packet(&mut self, packet: &Packet) -> AvResult<()> {
        if self.finished {
            return Err(AvError::invalid_argument(
                "cannot write packet after YUV4MPEG2 muxer is finished",
            ));
        }
        if packet.stream_index() != 0 {
            return Err(AvError::invalid_argument(format!(
                "YUV4MPEG2 muxer only accepts stream 0, got stream {}",
                packet.stream_index()
            )));
        }
        self.info.video.validate_frame_payload_len(
            packet.data().len(),
            AvErrorKind::InvalidData,
            "YUV4MPEG2 packet",
        )?;

        let new_payload_len = self
            .payload
            .len()
            .checked_add(packet.data().len())
            .ok_or_else(|| AvError::invalid_argument("YUV4MPEG2 payload size overflow"))?;
        let new_frame_count = self
            .frame_count
            .checked_add(1)
            .ok_or_else(|| AvError::invalid_argument("YUV4MPEG2 frame count overflow"))?;

        self.payload.reserve(new_payload_len - self.payload.len());
        self.payload.extend_from_slice(packet.data());
        self.frame_count = new_frame_count;
        Ok(())
    }

    pub fn render(&self) -> Vec<u8> {
        let mut output = self.header().into_bytes();
        for frame in self.payload.chunks_exact(self.info.frame_size()) {
            output.extend_from_slice(b"FRAME\n");
            output.extend_from_slice(frame);
        }
        output
    }

    pub fn finish(&mut self) -> Vec<u8> {
        self.finished = true;
        self.render()
    }
}

fn parse_header(line: &str) -> AvResult<Yuv4MpegInfo> {
    if line != Y4M_MAGIC && !line.starts_with("YUV4MPEG2 ") {
        return Err(AvError::invalid_data("YUV4MPEG2 header missing magic"));
    }

    let mut width = None;
    let mut height = None;
    let mut frame_rate = None;
    let mut sample_aspect_ratio = None;
    let mut interlace = Yuv4MpegInterlace::Progressive;
    let mut chroma = Yuv4MpegChroma::C420Jpeg;

    for field in line[Y4M_MAGIC.len()..].split_ascii_whitespace() {
        let (tag, value) = field.split_at(1);
        if value.is_empty() {
            return Err(AvError::invalid_data(format!(
                "YUV4MPEG2 header field `{field}` has no value"
            )));
        }

        match tag {
            "W" => width = Some(parse_positive_u32(value, "YUV4MPEG2 width")?),
            "H" => height = Some(parse_positive_u32(value, "YUV4MPEG2 height")?),
            "F" => frame_rate = Some(parse_positive_rational(value, "YUV4MPEG2 frame rate")?),
            "A" => sample_aspect_ratio = parse_sample_aspect_ratio(value)?,
            "I" => {
                if value != Yuv4MpegInterlace::Progressive.tag() {
                    return Err(AvError::unsupported(format!(
                        "unsupported YUV4MPEG2 interlace mode `{value}`"
                    )));
                }
                interlace = Yuv4MpegInterlace::Progressive;
            }
            "C" => {
                if value != Yuv4MpegChroma::C420Jpeg.name() {
                    return Err(AvError::unsupported(format!(
                        "unsupported YUV4MPEG2 chroma mode `{value}`"
                    )));
                }
                chroma = Yuv4MpegChroma::C420Jpeg;
            }
            "X" => {}
            _ => {
                return Err(AvError::unsupported(format!(
                    "unsupported YUV4MPEG2 header field `{field}`"
                )))
            }
        }
    }

    let width = width.ok_or_else(|| AvError::invalid_data("YUV4MPEG2 missing width"))?;
    let height = height.ok_or_else(|| AvError::invalid_data("YUV4MPEG2 missing height"))?;
    let frame_rate =
        frame_rate.ok_or_else(|| AvError::invalid_data("YUV4MPEG2 missing frame rate"))?;
    let video = yuv420_container_video_parameters(width, height)?;

    Ok(Yuv4MpegInfo {
        video,
        frame_rate,
        sample_aspect_ratio,
        interlace,
        chroma,
    })
}

fn parse_sample_aspect_ratio(value: &str) -> AvResult<Option<Rational>> {
    if value == "0:0" {
        return Ok(None);
    }
    Ok(Some(parse_positive_rational(
        value,
        "YUV4MPEG2 sample aspect ratio",
    )?))
}

fn parse_frame_header(line: &str) -> AvResult<()> {
    if line != FRAME_MAGIC && !line.starts_with("FRAME ") {
        return Err(AvError::invalid_data(
            "YUV4MPEG2 frame header missing FRAME",
        ));
    }

    for field in line[FRAME_MAGIC.len()..].split_ascii_whitespace() {
        if !field.starts_with('X') {
            return Err(AvError::unsupported(format!(
                "unsupported YUV4MPEG2 frame field `{field}`"
            )));
        }
    }

    Ok(())
}

fn parse_positive_u32(value: &str, name: &str) -> AvResult<u32> {
    let value = value
        .parse::<u32>()
        .map_err(|_| AvError::invalid_data(format!("{name} is not a positive integer")))?;
    validate_positive_dimension(value, name)
        .map_err(|err| AvError::invalid_data(err.message().to_string()))?;
    Ok(value)
}

fn validate_positive_dimension(value: u32, name: &str) -> AvResult<()> {
    if value == 0 {
        return Err(AvError::invalid_argument(format!(
            "{name} must be non-zero"
        )));
    }
    Ok(())
}

fn validate_positive_rational(value: Rational, name: &str) -> AvResult<()> {
    if value.num() <= 0 || value.den() <= 0 {
        return Err(AvError::invalid_argument(format!(
            "{name} numerator and denominator must be positive"
        )));
    }
    Ok(())
}

fn parse_positive_rational(value: &str, name: &str) -> AvResult<Rational> {
    let (num, den) = value
        .split_once(':')
        .ok_or_else(|| AvError::invalid_data(format!("{name} must use N:D syntax")))?;
    let num = parse_i32_component(num, name)?;
    let den = parse_i32_component(den, name)?;
    if num <= 0 || den <= 0 {
        return Err(AvError::invalid_data(format!(
            "{name} numerator and denominator must be positive"
        )));
    }
    Rational::new(num, den)
}

fn parse_i32_component(value: &str, name: &str) -> AvResult<i32> {
    value
        .parse::<i32>()
        .map_err(|_| AvError::invalid_data(format!("{name} component is out of range")))
}

fn yuv420_user_video_parameters(width: u32, height: u32) -> AvResult<VideoStreamParameters> {
    validate_yuv420_dimensions(width, height)?;
    VideoStreamParameters::from_u32_with_context(width, height, PixelFormat::Yuv420p, "YUV4MPEG2")
}

fn yuv420_container_video_parameters(width: u32, height: u32) -> AvResult<VideoStreamParameters> {
    validate_yuv420_dimensions(width, height)?;
    VideoStreamParameters::from_u32_container(width, height, PixelFormat::Yuv420p, "YUV4MPEG2")
}

fn validate_yuv420_dimensions(width: u32, height: u32) -> AvResult<()> {
    if width != 0 && height != 0 && (width % 2 != 0 || height % 2 != 0) {
        return Err(AvError::unsupported(
            "YUV4MPEG2 4:2:0 requires even width and height",
        ));
    }
    Ok(())
}

fn video_width_u32(video: VideoStreamParameters) -> AvResult<u32> {
    u32::try_from(video.width())
        .map_err(|_| AvError::invalid_data("YUV4MPEG2 width exceeds 32 bits"))
}

fn video_height_u32(video: VideoStreamParameters) -> AvResult<u32> {
    u32::try_from(video.height())
        .map_err(|_| AvError::invalid_data("YUV4MPEG2 height exceeds 32 bits"))
}

fn read_required_line<'a>(
    input: &'a [u8],
    position: &mut usize,
    context: &str,
) -> AvResult<&'a str> {
    let start = *position;
    let line_end = input[start..]
        .iter()
        .position(|byte| *byte == b'\n')
        .map(|relative| start + relative)
        .ok_or_else(|| AvError::new(AvErrorKind::EndOfFile, format!("{context} is incomplete")))?;
    *position = line_end + 1;

    let mut line = &input[start..line_end];
    if line.ends_with(b"\r") {
        line = &line[..line.len() - 1];
    }
    std::str::from_utf8(line)
        .map_err(|_| AvError::invalid_data(format!("{context} is not valid UTF-8")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_header_and_reads_multiple_frames() {
        let first = frame_bytes(12, 0x10);
        let second = frame_bytes(12, 0x80);
        let input = y4m_bytes("W4 H2 F30000:1001 Ip A1:1 C420jpeg", &[&first, &second]);
        let mut demuxer = Yuv4MpegDemuxer::open(&input).unwrap();

        assert_eq!(demuxer.info().width(), 4);
        assert_eq!(demuxer.info().height(), 2);
        assert_eq!(demuxer.info().pixel_format(), PixelFormat::Yuv420p);
        assert_eq!(
            demuxer.info().frame_rate(),
            Rational::new(30000, 1001).unwrap()
        );
        assert_eq!(demuxer.info().sample_aspect_ratio(), Some(Rational::ONE));
        assert_eq!(demuxer.info().interlace(), Yuv4MpegInterlace::Progressive);
        assert_eq!(demuxer.info().chroma(), Yuv4MpegChroma::C420Jpeg);
        assert_eq!(demuxer.info().frame_size(), 12);

        let first_packet = demuxer.read_packet().unwrap().unwrap();
        assert_eq!(first_packet.data(), first.as_slice());
        assert_eq!(first_packet.pts(), Some(0));
        assert_eq!(first_packet.dts(), Some(0));
        assert_eq!(first_packet.duration(), 1);

        let second_packet = demuxer.read_packet().unwrap().unwrap();
        assert_eq!(second_packet.data(), second.as_slice());
        assert_eq!(second_packet.pts(), Some(1));
        assert_eq!(second_packet.duration(), 1);
        assert!(demuxer.read_packet().unwrap().is_none());
    }

    #[test]
    fn defaults_missing_chroma_and_allows_clean_eof_after_header() {
        let input = b"YUV4MPEG2 W2 H2 F25:1 Ip\n";
        let mut demuxer = Yuv4MpegDemuxer::open(input).unwrap();

        assert_eq!(demuxer.info().chroma(), Yuv4MpegChroma::C420Jpeg);
        assert_eq!(demuxer.info().frame_size(), 6);
        assert!(demuxer.read_packet().unwrap().is_none());
    }

    #[test]
    fn parses_ffmpeg_unspecified_sample_aspect_and_xyscss_extension() {
        let input = b"YUV4MPEG2 W2 H2 F25:1 Ip A0:0 C420jpeg XYSCSS=420JPEG\nFRAME\nabcdef";
        let mut demuxer = Yuv4MpegDemuxer::open(input).unwrap();

        assert_eq!(demuxer.info().sample_aspect_ratio(), None);
        assert_eq!(demuxer.info().chroma(), Yuv4MpegChroma::C420Jpeg);
        let packet = demuxer.read_packet().unwrap().unwrap();
        assert_eq!(packet.data(), b"abcdef");
        assert!(demuxer.read_packet().unwrap().is_none());
    }

    #[test]
    fn rejects_bad_or_incomplete_stream_headers() {
        assert_eq!(
            Yuv4MpegDemuxer::open(b"not y4m\n").unwrap_err().kind(),
            AvErrorKind::InvalidData
        );
        assert!(Yuv4MpegDemuxer::open(b"YUV4MPEG2 H2 F25:1 Ip C420jpeg\n").is_err());
        assert!(Yuv4MpegDemuxer::open(b"YUV4MPEG2 W0 H2 F25:1 Ip C420jpeg\n").is_err());
        assert_eq!(
            Yuv4MpegDemuxer::open(b"YUV4MPEG2 W3 H2 F25:1 Ip C420jpeg\n")
                .unwrap_err()
                .kind(),
            AvErrorKind::Unsupported
        );
        assert!(Yuv4MpegDemuxer::open(b"YUV4MPEG2 W2 H2 F0:1 Ip C420jpeg\n").is_err());
        assert!(Yuv4MpegDemuxer::open(b"YUV4MPEG2 W2 H2 F25:0 Ip C420jpeg\n").is_err());
        assert!(Yuv4MpegDemuxer::open(b"YUV4MPEG2 W2 H2 Fabc Ip C420jpeg\n").is_err());
        assert!(Yuv4MpegDemuxer::open(b"YUV4MPEG2 W2 H2 F25:1 It C420jpeg\n").is_err());
        assert!(Yuv4MpegDemuxer::open(b"YUV4MPEG2 W2 H2 F25:1 Ip C422\n").is_err());
    }

    #[test]
    fn rejects_bad_frame_headers_and_truncated_payloads() {
        let truncated = b"YUV4MPEG2 W2 H2 F25:1 Ip C420jpeg\nFRAME\nabc";
        let mut demuxer = Yuv4MpegDemuxer::open(truncated).unwrap();
        assert_eq!(
            demuxer.read_packet().unwrap_err().kind(),
            AvErrorKind::EndOfFile
        );

        let bad_header = b"YUV4MPEG2 W2 H2 F25:1 Ip C420jpeg\nFIELD\nabcdef";
        let mut demuxer = Yuv4MpegDemuxer::open(bad_header).unwrap();
        assert_eq!(
            demuxer.read_packet().unwrap_err().kind(),
            AvErrorKind::InvalidData
        );

        let unsupported_frame_field = b"YUV4MPEG2 W2 H2 F25:1 Ip C420jpeg\nFRAME Iu\nabcdef";
        let mut demuxer = Yuv4MpegDemuxer::open(unsupported_frame_field).unwrap();
        assert_eq!(
            demuxer.read_packet().unwrap_err().kind(),
            AvErrorKind::Unsupported
        );
    }

    #[test]
    fn muxer_writes_header_frames_and_round_trips_through_demuxer() {
        let first = frame_bytes(6, 0x10);
        let second = frame_bytes(6, 0x80);
        let mut muxer =
            Yuv4MpegMuxer::new(2, 2, Rational::new(25, 1).unwrap(), Some(Rational::ONE)).unwrap();

        muxer.write_packet(&Packet::new(first.clone(), 0)).unwrap();
        muxer.write_packet(&Packet::new(second.clone(), 0)).unwrap();
        let output = muxer.finish();

        assert!(muxer.is_finished());
        assert_eq!(muxer.info().width(), 2);
        assert_eq!(muxer.info().height(), 2);
        assert_eq!(muxer.info().pixel_format(), PixelFormat::Yuv420p);
        assert_eq!(muxer.info().frame_size(), 6);
        assert_eq!(muxer.frame_count(), 2);
        assert_eq!(muxer.payload_len(), 12);
        assert!(
            output.starts_with(b"YUV4MPEG2 W2 H2 F25:1 Ip A1:1 C420jpeg XYSCSS=420JPEG\nFRAME\n")
        );

        let mut demuxer = Yuv4MpegDemuxer::open(&output).unwrap();
        assert_eq!(demuxer.info().sample_aspect_ratio(), Some(Rational::ONE));
        assert_eq!(demuxer.read_packet().unwrap().unwrap().data(), first);
        assert_eq!(demuxer.read_packet().unwrap().unwrap().data(), second);
        assert!(demuxer.read_packet().unwrap().is_none());
    }

    #[test]
    fn muxer_writes_ffmpeg_unspecified_sample_aspect_and_allows_empty_output() {
        let mut muxer =
            Yuv4MpegMuxer::new(4, 2, Rational::new(30000, 1001).unwrap(), None).unwrap();

        assert_eq!(
            muxer.header(),
            "YUV4MPEG2 W4 H2 F30000:1001 Ip A0:0 C420jpeg XYSCSS=420JPEG\n"
        );
        let output = muxer.finish();

        assert_eq!(
            output,
            b"YUV4MPEG2 W4 H2 F30000:1001 Ip A0:0 C420jpeg XYSCSS=420JPEG\n"
        );
        assert_eq!(muxer.frame_count(), 0);
        assert_eq!(muxer.payload_len(), 0);
        let mut demuxer = Yuv4MpegDemuxer::open(&output).unwrap();
        assert!(demuxer.read_packet().unwrap().is_none());
    }

    #[test]
    fn muxer_rejects_invalid_parameters_streams_and_packet_sizes() {
        assert!(Yuv4MpegMuxer::new(0, 2, Rational::new(25, 1).unwrap(), None).is_err());
        assert_eq!(
            Yuv4MpegMuxer::new(3, 2, Rational::new(25, 1).unwrap(), None)
                .unwrap_err()
                .kind(),
            AvErrorKind::Unsupported
        );
        assert!(Yuv4MpegMuxer::new(2, 2, Rational::new(0, 1).unwrap(), None).is_err());
        assert!(
            Yuv4MpegMuxer::new(2, 2, Rational::new(25, 1).unwrap(), Some(Rational::ZERO)).is_err()
        );

        let mut muxer = Yuv4MpegMuxer::new(2, 2, Rational::new(25, 1).unwrap(), None).unwrap();
        let wrong_stream = muxer.write_packet(&Packet::new(frame_bytes(6, 0), 1));
        assert_eq!(
            wrong_stream.unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
        let short_frame = muxer.write_packet(&Packet::new(frame_bytes(5, 0), 0));
        assert_eq!(short_frame.unwrap_err().kind(), AvErrorKind::InvalidData);
        let long_frame = muxer.write_packet(&Packet::new(frame_bytes(7, 0), 0));
        assert_eq!(long_frame.unwrap_err().kind(), AvErrorKind::InvalidData);
        assert_eq!(muxer.frame_count(), 0);
        assert_eq!(muxer.payload_len(), 0);
    }

    #[test]
    fn muxer_finish_prevents_more_writes() {
        let frame = frame_bytes(6, 0x20);
        let mut muxer = Yuv4MpegMuxer::new(2, 2, Rational::new(24, 1).unwrap(), None).unwrap();

        muxer.write_packet(&Packet::new(frame.clone(), 0)).unwrap();
        let output = muxer.finish();
        let err = muxer.write_packet(&Packet::new(frame, 0)).unwrap_err();

        assert_eq!(err.kind(), AvErrorKind::InvalidArgument);
        assert!(output.ends_with(b"FRAME\n !\"#$%"));
        assert_eq!(muxer.frame_count(), 1);
        assert_eq!(muxer.payload_len(), 6);
    }

    fn y4m_bytes(header_fields: &str, frames: &[&[u8]]) -> Vec<u8> {
        let mut out = format!("{Y4M_MAGIC} {header_fields}\n").into_bytes();
        for frame in frames {
            out.extend_from_slice(b"FRAME\n");
            out.extend_from_slice(frame);
        }
        out
    }

    fn frame_bytes(len: usize, start: u8) -> Vec<u8> {
        (0..len)
            .map(|offset| start.wrapping_add(offset as u8))
            .collect()
    }
}
