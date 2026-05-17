use crate::{
    probe::{ProbeDescriptor, ProbeRegistry},
    VideoStreamParameters,
};
use avutil::{
    AvError, AvErrorKind, AvResult, ByteReader, ByteWriter, Packet, PixelFormat, Rational, SideData,
};

const RIFF_ID: &[u8; 4] = b"RIFF";
const LIST_ID: &[u8; 4] = b"LIST";
const AVI_FORM: &[u8; 4] = b"AVI ";
const HDRL_LIST: &[u8; 4] = b"hdrl";
const STRL_LIST: &[u8; 4] = b"strl";
const MOVI_LIST: &[u8; 4] = b"movi";
const REC_LIST: &[u8; 4] = b"rec ";
const AVIH_ID: &[u8; 4] = b"avih";
const STRH_ID: &[u8; 4] = b"strh";
const STRF_ID: &[u8; 4] = b"strf";
const VIDEO_STREAM_TYPE: &[u8; 4] = b"vids";
const AVI_PROBE_NAME: &str = "avi";
const AVI_PROBE_EXTENSIONS: &[&str] = &["avi"];
const AVI_PROBE_MIME_TYPES: &[&str] = &["video/x-msvideo", "video/avi", "video/msvideo"];

pub fn avi_probe_descriptor() -> AvResult<ProbeDescriptor> {
    ProbeDescriptor::new_with_offset_signatures(
        AVI_PROBE_NAME,
        AVI_PROBE_EXTENSIONS,
        AVI_PROBE_MIME_TYPES,
        &[],
        &[(8, AVI_FORM.as_slice())],
    )
}

pub fn register_avi_probe(registry: &mut ProbeRegistry) -> AvResult<()> {
    registry.register(avi_probe_descriptor()?)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AviMediaType {
    Video,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AviStreamInfo {
    index: usize,
    media_type: AviMediaType,
    handler: String,
    time_base: Rational,
    frame_rate: Rational,
    length: u32,
    sample_size: u32,
    width: u32,
    height: u32,
    bit_count: u16,
    compression: String,
}

impl AviStreamInfo {
    pub fn index(&self) -> usize {
        self.index
    }

    pub fn media_type(&self) -> AviMediaType {
        self.media_type
    }

    pub fn handler(&self) -> &str {
        &self.handler
    }

    pub fn time_base(&self) -> Rational {
        self.time_base
    }

    pub fn frame_rate(&self) -> Rational {
        self.frame_rate
    }

    pub fn length(&self) -> u32 {
        self.length
    }

    pub fn sample_size(&self) -> u32 {
        self.sample_size
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn bit_count(&self) -> u16 {
        self.bit_count
    }

    pub fn compression(&self) -> &str {
        &self.compression
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AviInfo {
    microseconds_per_frame: u32,
    max_bytes_per_second: u32,
    total_frames: u32,
    width: u32,
    height: u32,
    streams: Vec<AviStreamInfo>,
    packet_count: usize,
}

impl AviInfo {
    pub fn microseconds_per_frame(&self) -> u32 {
        self.microseconds_per_frame
    }

    pub fn max_bytes_per_second(&self) -> u32 {
        self.max_bytes_per_second
    }

    pub fn total_frames(&self) -> u32 {
        self.total_frames
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn streams(&self) -> &[AviStreamInfo] {
        &self.streams
    }

    pub fn packet_count(&self) -> usize {
        self.packet_count
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AviDemuxer {
    info: AviInfo,
    packets: Vec<Packet>,
    next_packet: usize,
}

impl AviDemuxer {
    pub fn open(input: &[u8]) -> AvResult<Self> {
        let mut reader = ByteReader::new(input);
        expect_fourcc(&mut reader, RIFF_ID)?;
        let riff_size = usize::try_from(reader.read_u32_le()?)
            .map_err(|_| AvError::invalid_data("AVI RIFF size is out of range"))?;
        expect_fourcc(&mut reader, AVI_FORM)?;

        let riff_end = riff_size
            .checked_add(8)
            .ok_or_else(|| AvError::invalid_data("AVI RIFF size overflow"))?;
        if riff_end > input.len() {
            return Err(AvError::new(
                AvErrorKind::EndOfFile,
                "AVI RIFF size exceeds input length",
            ));
        }

        let mut main_header = None;
        let mut streams = Vec::new();
        let mut movi_payload = None;

        for chunk in read_chunks(&input[12..riff_end], "AVI RIFF")? {
            if chunk.id == *LIST_ID {
                let (list_type, payload) = split_list_payload(chunk.payload, "AVI LIST")?;
                match &list_type {
                    HDRL_LIST => {
                        let (header, parsed_streams) = parse_hdrl(payload)?;
                        main_header = Some(header);
                        streams = parsed_streams;
                    }
                    MOVI_LIST => movi_payload = Some(payload),
                    _ => {}
                }
            }
        }

        let main_header =
            main_header.ok_or_else(|| AvError::invalid_data("AVI missing hdrl list"))?;
        if streams.is_empty() {
            return Err(AvError::invalid_data("AVI missing stream headers"));
        }
        if main_header.streams != u32::try_from(streams.len()).unwrap_or(u32::MAX) {
            return Err(AvError::invalid_data(format!(
                "AVI main header declares {} streams but {} were parsed",
                main_header.streams,
                streams.len()
            )));
        }
        let movi_payload =
            movi_payload.ok_or_else(|| AvError::invalid_data("AVI missing movi list"))?;
        let packets = parse_movi(movi_payload, &streams)?;
        let packet_count = packets.len();

        Ok(Self {
            info: AviInfo {
                microseconds_per_frame: main_header.microseconds_per_frame,
                max_bytes_per_second: main_header.max_bytes_per_second,
                total_frames: main_header.total_frames,
                width: main_header.width,
                height: main_header.height,
                streams,
                packet_count,
            },
            packets,
            next_packet: 0,
        })
    }

    pub fn info(&self) -> &AviInfo {
        &self.info
    }

    pub fn read_packet(&mut self) -> AvResult<Option<Packet>> {
        let Some(packet) = self.packets.get(self.next_packet).cloned() else {
            return Ok(None);
        };
        self.next_packet += 1;
        Ok(Some(packet))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AviMuxer {
    video: VideoStreamParameters,
    frame_rate: Rational,
    packets: Vec<Vec<u8>>,
    finished: bool,
}

impl AviMuxer {
    pub fn new_rgb24(width: u32, height: u32, frame_rate: Rational) -> AvResult<Self> {
        let video = VideoStreamParameters::from_u32_with_context(
            width,
            height,
            PixelFormat::Rgb24,
            "AVI video",
        )?;
        validate_video_geometry(video)?;
        validate_positive_frame_rate(frame_rate)?;

        Ok(Self {
            video,
            frame_rate,
            packets: Vec::new(),
            finished: false,
        })
    }

    pub fn width(&self) -> u32 {
        video_width_u32(self.video).expect("AVI muxer stores u32 width")
    }

    pub fn height(&self) -> u32 {
        video_height_u32(self.video).expect("AVI muxer stores u32 height")
    }

    pub fn frame_rate(&self) -> Rational {
        self.frame_rate
    }

    pub fn frame_size(&self) -> usize {
        self.video.frame_size()
    }

    pub fn packet_count(&self) -> usize {
        self.packets.len()
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }

    pub fn write_packet(&mut self, packet: &Packet) -> AvResult<()> {
        if self.finished {
            return Err(AvError::invalid_argument(
                "cannot write packet after AVI muxer is finished",
            ));
        }
        if packet.stream_index() != 0 {
            return Err(AvError::invalid_argument(format!(
                "AVI muxer only accepts stream 0, got stream {}",
                packet.stream_index()
            )));
        }
        self.video.validate_frame_payload_len(
            packet.data().len(),
            AvErrorKind::InvalidData,
            "AVI packet",
        )?;
        if self.packets.len() == usize::try_from(u32::MAX).unwrap_or(usize::MAX) {
            return Err(AvError::invalid_argument(
                "AVI frame count exceeds classic RIFF header range",
            ));
        }

        self.packets.push(packet.data().to_vec());
        Ok(())
    }

    pub fn render(&self) -> AvResult<Vec<u8>> {
        let frame_count = u32::try_from(self.packets.len()).map_err(|_| {
            AvError::invalid_argument("AVI frame count does not fit classic RIFF headers")
        })?;
        let width = video_width_u32(self.video)?;
        let height = video_height_u32(self.video)?;
        let frame_size = self.video.frame_size();
        let frame_size_u32 = u32_len(frame_size, "AVI frame size")?;
        let frame_rate_num = positive_u32_from_i32(self.frame_rate.num(), "AVI frame rate")?;
        let frame_rate_den = positive_u32_from_i32(self.frame_rate.den(), "AVI frame rate")?;
        let microseconds_per_frame = microseconds_per_frame(self.frame_rate)?;
        let max_bytes_per_second = max_bytes_per_second(frame_size, self.frame_rate)?;

        let hdrl = write_list(
            *HDRL_LIST,
            &[
                write_chunk(
                    *AVIH_ID,
                    &main_header_payload(
                        microseconds_per_frame,
                        max_bytes_per_second,
                        frame_count,
                        width,
                        height,
                        frame_size_u32,
                    ),
                )?,
                write_list(
                    *STRL_LIST,
                    &[
                        write_chunk(
                            *STRH_ID,
                            &stream_header_payload(
                                *b"DIB ",
                                frame_rate_den,
                                frame_rate_num,
                                frame_count,
                                frame_size_u32,
                                width,
                                height,
                            )?,
                        )?,
                        write_chunk(
                            *STRF_ID,
                            &bitmap_info_payload(width, height, 24, frame_size_u32)?,
                        )?,
                    ],
                )?,
            ],
        )?;

        let movi_chunks = self
            .packets
            .iter()
            .map(|packet| write_chunk(*b"00db", packet))
            .collect::<AvResult<Vec<_>>>()?;
        let movi = write_list(*MOVI_LIST, &movi_chunks)?;
        write_riff_avi(&[hdrl, movi])
    }

    pub fn finish(&mut self) -> AvResult<Vec<u8>> {
        let output = self.render()?;
        self.finished = true;
        Ok(output)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AviMainHeader {
    microseconds_per_frame: u32,
    max_bytes_per_second: u32,
    total_frames: u32,
    streams: u32,
    width: u32,
    height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AviStreamHeader {
    media_type: AviMediaType,
    handler: String,
    scale: u32,
    rate: u32,
    length: u32,
    sample_size: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AviBitmapInfo {
    width: u32,
    height: u32,
    bit_count: u16,
    compression: [u8; 4],
}

#[derive(Debug, Clone, Copy)]
struct RiffChunk<'a> {
    id: [u8; 4],
    payload: &'a [u8],
}

fn parse_hdrl(payload: &[u8]) -> AvResult<(AviMainHeader, Vec<AviStreamInfo>)> {
    let mut main_header = None;
    let mut streams = Vec::new();

    for chunk in read_chunks(payload, "AVI hdrl")? {
        if chunk.id == *AVIH_ID {
            main_header = Some(parse_main_header(chunk.payload)?);
        } else if chunk.id == *LIST_ID {
            let (list_type, payload) = split_list_payload(chunk.payload, "AVI hdrl LIST")?;
            if list_type == *STRL_LIST {
                let stream = parse_stream_list(payload, streams.len())?;
                streams.push(stream);
            }
        }
    }

    let main_header =
        main_header.ok_or_else(|| AvError::invalid_data("AVI missing avih header"))?;
    Ok((main_header, streams))
}

fn write_riff_avi(chunks: &[Vec<u8>]) -> AvResult<Vec<u8>> {
    let mut payload = Vec::new();
    payload.extend_from_slice(AVI_FORM);
    for chunk in chunks {
        payload.extend_from_slice(chunk);
    }

    let mut out = ByteWriter::new();
    out.write_all(RIFF_ID);
    out.write_u32_le(u32_len(payload.len(), "AVI RIFF payload")?);
    out.write_all(&payload);
    Ok(out.into_inner())
}

fn write_list(kind: [u8; 4], chunks: &[Vec<u8>]) -> AvResult<Vec<u8>> {
    let mut payload = Vec::new();
    payload.extend_from_slice(&kind);
    for chunk in chunks {
        payload.extend_from_slice(chunk);
    }
    write_chunk(*LIST_ID, &payload)
}

fn write_chunk(id: [u8; 4], payload: &[u8]) -> AvResult<Vec<u8>> {
    let mut out = ByteWriter::new();
    out.write_all(&id);
    out.write_u32_le(u32_len(payload.len(), "AVI chunk payload")?);
    out.write_all(payload);
    if payload.len() % 2 == 1 {
        out.write_u8(0);
    }
    Ok(out.into_inner())
}

fn main_header_payload(
    microseconds_per_frame: u32,
    max_bytes_per_second: u32,
    total_frames: u32,
    width: u32,
    height: u32,
    suggested_buffer_size: u32,
) -> Vec<u8> {
    let mut out = ByteWriter::new();
    out.write_u32_le(microseconds_per_frame);
    out.write_u32_le(max_bytes_per_second);
    out.write_u32_le(0);
    out.write_u32_le(0x10);
    out.write_u32_le(total_frames);
    out.write_u32_le(0);
    out.write_u32_le(1);
    out.write_u32_le(suggested_buffer_size);
    out.write_u32_le(width);
    out.write_u32_le(height);
    for _ in 0..4 {
        out.write_u32_le(0);
    }
    out.into_inner()
}

fn stream_header_payload(
    handler: [u8; 4],
    scale: u32,
    rate: u32,
    length: u32,
    suggested_buffer_size: u32,
    width: u32,
    height: u32,
) -> AvResult<Vec<u8>> {
    let width = i32_from_u32(width, "AVI stream frame width")?;
    let height = i32_from_u32(height, "AVI stream frame height")?;

    let mut out = ByteWriter::new();
    out.write_all(VIDEO_STREAM_TYPE);
    out.write_all(&handler);
    out.write_u32_le(0);
    out.write_u16_le(0);
    out.write_u16_le(0);
    out.write_u32_le(0);
    out.write_u32_le(scale);
    out.write_u32_le(rate);
    out.write_u32_le(0);
    out.write_u32_le(length);
    out.write_u32_le(suggested_buffer_size);
    out.write_u32_le(u32::MAX);
    out.write_u32_le(0);
    out.write_i32_le(0);
    out.write_i32_le(0);
    out.write_i32_le(width);
    out.write_i32_le(height);
    Ok(out.into_inner())
}

fn bitmap_info_payload(
    width: u32,
    height: u32,
    bit_count: u16,
    image_size: u32,
) -> AvResult<Vec<u8>> {
    let width = i32::try_from(width)
        .map_err(|_| AvError::invalid_argument("AVI width does not fit BITMAPINFOHEADER"))?;
    let height = i32::try_from(height)
        .map_err(|_| AvError::invalid_argument("AVI height does not fit BITMAPINFOHEADER"))?;

    let mut out = ByteWriter::new();
    out.write_u32_le(40);
    out.write_i32_le(width);
    out.write_i32_le(height);
    out.write_u16_le(1);
    out.write_u16_le(bit_count);
    out.write_u32_le(0);
    out.write_u32_le(image_size);
    out.write_i32_le(0);
    out.write_i32_le(0);
    out.write_u32_le(0);
    out.write_u32_le(0);
    Ok(out.into_inner())
}

fn validate_video_geometry(video: VideoStreamParameters) -> AvResult<()> {
    i32_from_u32(video_width_u32(video)?, "AVI video width")?;
    i32_from_u32(video_height_u32(video)?, "AVI video height")?;
    u32_len(video.frame_size(), "AVI RGB24 frame size")?;
    Ok(())
}

fn validate_positive_frame_rate(frame_rate: Rational) -> AvResult<()> {
    positive_u32_from_i32(frame_rate.num(), "AVI frame rate numerator")?;
    positive_u32_from_i32(frame_rate.den(), "AVI frame rate denominator")?;
    Ok(())
}

fn microseconds_per_frame(frame_rate: Rational) -> AvResult<u32> {
    let rate = u128::from(positive_u32_from_i32(
        frame_rate.num(),
        "AVI frame rate numerator",
    )?);
    let scale = u128::from(positive_u32_from_i32(
        frame_rate.den(),
        "AVI frame rate denominator",
    )?);
    let microseconds = 1_000_000_u128
        .checked_mul(scale)
        .ok_or_else(|| AvError::invalid_argument("AVI frame duration overflow"))?
        / rate;
    if microseconds == 0 {
        return Err(AvError::invalid_argument(
            "AVI frame rate is too high for microsecond header precision",
        ));
    }
    u32::try_from(microseconds)
        .map_err(|_| AvError::invalid_argument("AVI frame duration is out of range"))
}

fn max_bytes_per_second(frame_size: usize, frame_rate: Rational) -> AvResult<u32> {
    let rate = u128::from(positive_u32_from_i32(
        frame_rate.num(),
        "AVI frame rate numerator",
    )?);
    let scale = u128::from(positive_u32_from_i32(
        frame_rate.den(),
        "AVI frame rate denominator",
    )?);
    let bytes = (frame_size as u128)
        .checked_mul(rate)
        .ok_or_else(|| AvError::invalid_argument("AVI max bytes per second overflow"))?;
    let rounded = bytes
        .checked_add(scale - 1)
        .ok_or_else(|| AvError::invalid_argument("AVI max bytes per second overflow"))?
        / scale;
    u32::try_from(rounded)
        .map_err(|_| AvError::invalid_argument("AVI max bytes per second is out of range"))
}

fn positive_u32_from_i32(value: i32, name: &str) -> AvResult<u32> {
    if value <= 0 {
        return Err(AvError::invalid_argument(format!(
            "{name} must be positive"
        )));
    }
    u32::try_from(value).map_err(|_| AvError::invalid_argument(format!("{name} is out of range")))
}

fn i32_from_u32(value: u32, name: &str) -> AvResult<i32> {
    i32::try_from(value).map_err(|_| AvError::invalid_argument(format!("{name} is out of range")))
}

fn u32_len(len: usize, context: &str) -> AvResult<u32> {
    u32::try_from(len).map_err(|_| AvError::invalid_argument(format!("{context} exceeds 32 bits")))
}

fn video_width_u32(video: VideoStreamParameters) -> AvResult<u32> {
    u32::try_from(video.width())
        .map_err(|_| AvError::invalid_argument("AVI video width exceeds 32 bits"))
}

fn video_height_u32(video: VideoStreamParameters) -> AvResult<u32> {
    u32::try_from(video.height())
        .map_err(|_| AvError::invalid_argument("AVI video height exceeds 32 bits"))
}

fn parse_main_header(data: &[u8]) -> AvResult<AviMainHeader> {
    if data.len() < 56 {
        return Err(AvError::new(
            AvErrorKind::EndOfFile,
            "AVI avih chunk is shorter than 56 bytes",
        ));
    }

    let mut reader = ByteReader::new(data);
    let microseconds_per_frame = reader.read_u32_le()?;
    let max_bytes_per_second = reader.read_u32_le()?;
    reader.skip(8)?;
    let total_frames = reader.read_u32_le()?;
    reader.skip(4)?;
    let streams = reader.read_u32_le()?;
    reader.skip(4)?;
    let width = reader.read_u32_le()?;
    let height = reader.read_u32_le()?;

    if streams == 0 {
        return Err(AvError::invalid_data("AVI avih declares zero streams"));
    }
    if width == 0 || height == 0 {
        return Err(AvError::invalid_data(
            "AVI avih dimensions must be non-zero",
        ));
    }

    Ok(AviMainHeader {
        microseconds_per_frame,
        max_bytes_per_second,
        total_frames,
        streams,
        width,
        height,
    })
}

fn parse_stream_list(payload: &[u8], index: usize) -> AvResult<AviStreamInfo> {
    let mut stream_header = None;
    let mut bitmap_info = None;

    for chunk in read_chunks(payload, "AVI strl")? {
        match &chunk.id {
            STRH_ID => stream_header = Some(parse_stream_header(chunk.payload)?),
            STRF_ID => bitmap_info = Some(parse_bitmap_info(chunk.payload)?),
            _ => {}
        }
    }

    let stream_header =
        stream_header.ok_or_else(|| AvError::invalid_data("AVI stream missing strh"))?;
    let bitmap_info =
        bitmap_info.ok_or_else(|| AvError::invalid_data("AVI video stream missing strf"))?;
    if stream_header.rate == 0 || stream_header.scale == 0 {
        return Err(AvError::invalid_data(
            "AVI video stream rate and scale must be non-zero",
        ));
    }

    Ok(AviStreamInfo {
        index,
        media_type: stream_header.media_type,
        handler: stream_header.handler,
        time_base: rational_from_u32(stream_header.scale, stream_header.rate, "AVI time base")?,
        frame_rate: rational_from_u32(stream_header.rate, stream_header.scale, "AVI frame rate")?,
        length: stream_header.length,
        sample_size: stream_header.sample_size,
        width: bitmap_info.width,
        height: bitmap_info.height,
        bit_count: bitmap_info.bit_count,
        compression: compression_name(bitmap_info.compression),
    })
}

fn parse_stream_header(data: &[u8]) -> AvResult<AviStreamHeader> {
    if data.len() < 56 {
        return Err(AvError::new(
            AvErrorKind::EndOfFile,
            "AVI strh chunk is shorter than 56 bytes",
        ));
    }

    let mut reader = ByteReader::new(data);
    let stream_type = read_fourcc(&mut reader)?;
    let handler = read_fourcc(&mut reader)?;
    reader.skip(12)?;
    let scale = reader.read_u32_le()?;
    let rate = reader.read_u32_le()?;
    reader.skip(4)?;
    let length = reader.read_u32_le()?;
    reader.skip(8)?;
    let sample_size = reader.read_u32_le()?;

    if stream_type != *VIDEO_STREAM_TYPE {
        return Err(AvError::unsupported(format!(
            "unsupported AVI stream type `{}`",
            fourcc_to_string(stream_type)
        )));
    }

    Ok(AviStreamHeader {
        media_type: AviMediaType::Video,
        handler: fourcc_to_string(handler),
        scale,
        rate,
        length,
        sample_size,
    })
}

fn parse_bitmap_info(data: &[u8]) -> AvResult<AviBitmapInfo> {
    if data.len() < 40 {
        return Err(AvError::new(
            AvErrorKind::EndOfFile,
            "AVI BITMAPINFOHEADER is shorter than 40 bytes",
        ));
    }

    let mut reader = ByteReader::new(data);
    let size = reader.read_u32_le()?;
    if size < 40 {
        return Err(AvError::invalid_data(
            "AVI BITMAPINFOHEADER size is smaller than 40",
        ));
    }
    let width = reader.read_i32_le()?;
    let height = reader.read_i32_le()?;
    let planes = reader.read_u16_le()?;
    let bit_count = reader.read_u16_le()?;
    let compression_value = reader.read_u32_le()?;

    if width <= 0 || height <= 0 {
        return Err(AvError::unsupported(
            "AVI video stream requires positive bottom-up dimensions",
        ));
    }
    if planes != 1 {
        return Err(AvError::invalid_data("AVI video stream planes must be 1"));
    }

    Ok(AviBitmapInfo {
        width: u32::try_from(width)
            .map_err(|_| AvError::invalid_data("AVI video width is out of range"))?,
        height: u32::try_from(height)
            .map_err(|_| AvError::invalid_data("AVI video height is out of range"))?,
        bit_count,
        compression: compression_value.to_le_bytes(),
    })
}

fn parse_movi(payload: &[u8], streams: &[AviStreamInfo]) -> AvResult<Vec<Packet>> {
    let mut packets = Vec::new();
    let mut next_pts = vec![0_i64; streams.len()];
    parse_movi_payload(payload, streams, &mut next_pts, &mut packets)?;
    Ok(packets)
}

fn parse_movi_payload(
    payload: &[u8],
    streams: &[AviStreamInfo],
    next_pts: &mut [i64],
    packets: &mut Vec<Packet>,
) -> AvResult<()> {
    for chunk in read_chunks(payload, "AVI movi")? {
        if chunk.id == *LIST_ID {
            let (list_type, payload) = split_list_payload(chunk.payload, "AVI movi LIST")?;
            if list_type == *REC_LIST {
                parse_movi_payload(payload, streams, next_pts, packets)?;
            }
            continue;
        }

        let Some((stream_index, chunk_kind)) = parse_movi_chunk_id(chunk.id) else {
            continue;
        };
        let Some(stream) = streams.get(stream_index) else {
            return Err(AvError::invalid_data(format!(
                "AVI packet references unknown stream {stream_index}"
            )));
        };
        if stream.media_type != AviMediaType::Video || (chunk_kind != b"db" && chunk_kind != b"dc")
        {
            return Err(AvError::unsupported(format!(
                "unsupported AVI packet chunk `{}`",
                fourcc_to_string(chunk.id)
            )));
        }

        let mut packet = Packet::new(chunk.payload.to_vec(), stream_index);
        let pts = next_pts[stream_index];
        packet.set_pts(Some(pts));
        packet.set_dts(Some(pts));
        packet.set_duration(1)?;
        packet.push_side_data(SideData::new("avi_chunk_id", chunk.id.to_vec())?);
        next_pts[stream_index] = pts
            .checked_add(1)
            .ok_or_else(|| AvError::invalid_data("AVI packet PTS overflow"))?;
        packets.push(packet);
    }
    Ok(())
}

fn parse_movi_chunk_id(id: [u8; 4]) -> Option<(usize, &'static [u8; 2])> {
    let first = id[0];
    let second = id[1];
    if !first.is_ascii_digit() || !second.is_ascii_digit() {
        return None;
    }
    let stream_index = usize::from(first - b'0') * 10 + usize::from(second - b'0');
    match &id[2..4] {
        b"db" => Some((stream_index, b"db")),
        b"dc" => Some((stream_index, b"dc")),
        b"wb" => Some((stream_index, b"wb")),
        _ => None,
    }
}

fn read_chunks<'a>(data: &'a [u8], context: &str) -> AvResult<Vec<RiffChunk<'a>>> {
    let mut chunks = Vec::new();
    let mut position = 0;
    while position < data.len() {
        if data.len() - position < 8 {
            return Err(AvError::new(
                AvErrorKind::EndOfFile,
                format!("{context} chunk header is truncated"),
            ));
        }

        let id = fourcc_at(data, position)?;
        let size = usize::try_from(u32::from_le_bytes([
            data[position + 4],
            data[position + 5],
            data[position + 6],
            data[position + 7],
        ]))
        .map_err(|_| AvError::invalid_data(format!("{context} chunk size is out of range")))?;
        let payload_start = position
            .checked_add(8)
            .ok_or_else(|| AvError::invalid_data(format!("{context} chunk offset overflow")))?;
        let payload_end = payload_start
            .checked_add(size)
            .ok_or_else(|| AvError::invalid_data(format!("{context} chunk size overflow")))?;
        if payload_end > data.len() {
            return Err(AvError::new(
                AvErrorKind::EndOfFile,
                format!("{context} chunk exceeds parent bounds"),
            ));
        }

        chunks.push(RiffChunk {
            id,
            payload: &data[payload_start..payload_end],
        });
        position = payload_end;
        if size % 2 == 1 && position < data.len() {
            position += 1;
        }
    }
    Ok(chunks)
}

fn split_list_payload<'a>(payload: &'a [u8], context: &str) -> AvResult<([u8; 4], &'a [u8])> {
    if payload.len() < 4 {
        return Err(AvError::new(
            AvErrorKind::EndOfFile,
            format!("{context} is missing list type"),
        ));
    }
    Ok((fourcc_at(payload, 0)?, &payload[4..]))
}

fn expect_fourcc(reader: &mut ByteReader<'_>, expected: &[u8; 4]) -> AvResult<()> {
    let actual = read_fourcc(reader)?;
    if &actual != expected {
        return Err(AvError::invalid_data(format!(
            "expected FourCC `{}`, found `{}`",
            fourcc_to_string(*expected),
            fourcc_to_string(actual)
        )));
    }
    Ok(())
}

fn read_fourcc(reader: &mut ByteReader<'_>) -> AvResult<[u8; 4]> {
    let bytes = reader.read_exact(4)?;
    Ok([bytes[0], bytes[1], bytes[2], bytes[3]])
}

fn fourcc_at(data: &[u8], offset: usize) -> AvResult<[u8; 4]> {
    let end = offset
        .checked_add(4)
        .ok_or_else(|| AvError::invalid_data("FourCC offset overflow"))?;
    if end > data.len() {
        return Err(AvError::new(
            AvErrorKind::EndOfFile,
            "FourCC exceeds buffer bounds",
        ));
    }
    Ok([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

fn fourcc_to_string(fourcc: [u8; 4]) -> String {
    String::from_utf8_lossy(&fourcc).into_owned()
}

fn compression_name(compression: [u8; 4]) -> String {
    if compression == [0, 0, 0, 0] {
        "BI_RGB".to_string()
    } else {
        fourcc_to_string(compression)
    }
}

fn rational_from_u32(num: u32, den: u32, name: &str) -> AvResult<Rational> {
    let num = i32::try_from(num)
        .map_err(|_| AvError::invalid_data(format!("{name} numerator is out of range")))?;
    let den = i32::try_from(den)
        .map_err(|_| AvError::invalid_data(format!("{name} denominator is out of range")))?;
    Rational::new(num, den)
}

#[cfg(test)]
mod tests {
    use super::*;
    use avutil::ByteWriter;

    #[test]
    fn parses_video_metadata_and_movi_packets() {
        let first = frame_bytes(12, 0x10);
        let second = frame_bytes(12, 0x80);
        let bytes = avi_fixture(&[&first, &second]);
        let mut demuxer = AviDemuxer::open(&bytes).unwrap();

        assert_eq!(demuxer.info().microseconds_per_frame(), 40_000);
        assert_eq!(demuxer.info().max_bytes_per_second(), 1_000_000);
        assert_eq!(demuxer.info().total_frames(), 2);
        assert_eq!(demuxer.info().width(), 2);
        assert_eq!(demuxer.info().height(), 2);
        assert_eq!(demuxer.info().packet_count(), 2);
        assert_eq!(demuxer.info().streams().len(), 1);

        let stream = &demuxer.info().streams()[0];
        assert_eq!(stream.index(), 0);
        assert_eq!(stream.media_type(), AviMediaType::Video);
        assert_eq!(stream.handler(), "DIB ");
        assert_eq!(stream.time_base(), Rational::new(1, 25).unwrap());
        assert_eq!(stream.frame_rate(), Rational::new(25, 1).unwrap());
        assert_eq!(stream.length(), 2);
        assert_eq!(stream.width(), 2);
        assert_eq!(stream.height(), 2);
        assert_eq!(stream.bit_count(), 24);
        assert_eq!(stream.compression(), "BI_RGB");

        let first_packet = demuxer.read_packet().unwrap().unwrap();
        assert_eq!(first_packet.stream_index(), 0);
        assert_eq!(first_packet.data(), first);
        assert_eq!(first_packet.pts(), Some(0));
        assert_eq!(first_packet.dts(), Some(0));
        assert_eq!(first_packet.duration(), 1);
        assert_eq!(first_packet.side_data()[0].kind(), "avi_chunk_id");
        assert_eq!(first_packet.side_data()[0].data(), b"00db");

        let second_packet = demuxer.read_packet().unwrap().unwrap();
        assert_eq!(second_packet.data(), second);
        assert_eq!(second_packet.pts(), Some(1));
        assert!(demuxer.read_packet().unwrap().is_none());
    }

    #[test]
    fn skips_unknown_chunks_and_reads_rec_lists() {
        let frame = frame_bytes(12, 0x40);
        let bytes = avi_fixture_with_movi_payload(&[
            chunk(*b"JUNK", b"abc"),
            list(*REC_LIST, &[chunk(*b"00db", &frame)]),
        ]);
        let mut demuxer = AviDemuxer::open(&bytes).unwrap();

        let packet = demuxer.read_packet().unwrap().unwrap();
        assert_eq!(packet.data(), frame);
        assert!(demuxer.read_packet().unwrap().is_none());
    }

    #[test]
    fn rejects_bad_riff_headers_and_chunk_bounds() {
        assert_eq!(
            AviDemuxer::open(b"not an avi").unwrap_err().kind(),
            AvErrorKind::InvalidData
        );

        let mut truncated = avi_fixture(&[&frame_bytes(12, 0x10)]);
        truncated.pop();
        assert_eq!(
            AviDemuxer::open(&truncated).unwrap_err().kind(),
            AvErrorKind::EndOfFile
        );
    }

    #[test]
    fn rejects_missing_required_headers_and_stream_mismatch() {
        assert!(AviDemuxer::open(&riff_avi(&[list(*MOVI_LIST, &[])])).is_err());

        let mut bytes = avi_fixture(&[&frame_bytes(12, 0x10)]);
        let streams_offset = find_bytes(&bytes, &1_u32.to_le_bytes()).unwrap();
        bytes[streams_offset..streams_offset + 4].copy_from_slice(&2_u32.to_le_bytes());

        assert_eq!(
            AviDemuxer::open(&bytes).unwrap_err().kind(),
            AvErrorKind::InvalidData
        );
    }

    #[test]
    fn rejects_unsupported_streams_and_unknown_packet_streams() {
        let audio_strh = stream_header(*b"auds", *b"\0\0\0\0", 1, 48_000, 1, 4);
        let hdrl = list(
            *HDRL_LIST,
            &[
                chunk(*AVIH_ID, &main_header(1, 2, 2, 1)),
                list(
                    *STRL_LIST,
                    &[
                        chunk(*STRH_ID, &audio_strh),
                        chunk(*STRF_ID, &bitmap_info(2, 2, 24, 0)),
                    ],
                ),
            ],
        );
        let movi = list(*MOVI_LIST, &[chunk(*b"00wb", b"abcd")]);
        assert_eq!(
            AviDemuxer::open(&riff_avi(&[hdrl, movi]))
                .unwrap_err()
                .kind(),
            AvErrorKind::Unsupported
        );

        let bytes = avi_fixture_with_movi_payload(&[chunk(*b"01db", &frame_bytes(12, 0x22))]);
        assert_eq!(
            AviDemuxer::open(&bytes).unwrap_err().kind(),
            AvErrorKind::InvalidData
        );
    }

    #[test]
    fn registers_avi_probe_descriptor_for_signature_extension_and_mime() {
        let mut registry = ProbeRegistry::new();
        register_avi_probe(&mut registry).unwrap();

        let descriptor = avi_probe_descriptor().unwrap();
        assert_eq!(descriptor.name(), AVI_PROBE_NAME);
        let expected_extensions = AVI_PROBE_EXTENSIONS
            .iter()
            .map(|extension| extension.to_string())
            .collect::<Vec<_>>();
        assert_eq!(descriptor.extensions(), expected_extensions.as_slice());

        let bytes = avi_fixture(&[&frame_bytes(12, 0x10)]);
        let signature_match = registry
            .probe(crate::probe::ProbeRequest::new(&bytes).with_extension("clip.bin"))
            .unwrap();
        assert_eq!(signature_match.descriptor().name(), AVI_PROBE_NAME);
        assert_eq!(signature_match.score(), crate::probe::ProbeScore::SIGNATURE);

        let extension_match = registry
            .probe(crate::probe::ProbeRequest::new(b"not avi").with_extension("clip.AVI"))
            .unwrap();
        assert_eq!(extension_match.descriptor().name(), AVI_PROBE_NAME);
        assert_eq!(extension_match.score(), crate::probe::ProbeScore::EXTENSION);

        let mime_match = registry
            .probe(crate::probe::ProbeRequest::new(b"").with_mime_type("Video/X-MsVideo"))
            .unwrap();
        assert_eq!(mime_match.descriptor().name(), AVI_PROBE_NAME);
        assert_eq!(mime_match.score(), crate::probe::ProbeScore::MIME_TYPE);

        assert!(registry
            .probe(crate::probe::ProbeRequest::new(b"RIFF....WAVE").with_extension("clip.bin"))
            .is_none());
    }

    #[test]
    fn muxer_writes_headers_and_round_trips_through_demuxer() {
        let first = frame_bytes(12, 0x10);
        let second = frame_bytes(12, 0x80);
        let mut muxer = AviMuxer::new_rgb24(2, 2, Rational::new(25, 1).unwrap()).unwrap();

        assert_eq!(muxer.width(), 2);
        assert_eq!(muxer.height(), 2);
        assert_eq!(muxer.frame_rate(), Rational::new(25, 1).unwrap());
        assert_eq!(muxer.frame_size(), 12);
        assert_eq!(muxer.packet_count(), 0);

        muxer.write_packet(&Packet::new(first.clone(), 0)).unwrap();
        muxer.write_packet(&Packet::new(second.clone(), 0)).unwrap();
        assert_eq!(muxer.packet_count(), 2);

        let bytes = muxer.finish().unwrap();
        assert!(muxer.is_finished());

        let mut demuxer = AviDemuxer::open(&bytes).unwrap();
        assert_eq!(demuxer.info().microseconds_per_frame(), 40_000);
        assert_eq!(demuxer.info().max_bytes_per_second(), 300);
        assert_eq!(demuxer.info().total_frames(), 2);
        assert_eq!(demuxer.info().width(), 2);
        assert_eq!(demuxer.info().height(), 2);
        assert_eq!(demuxer.info().packet_count(), 2);

        let stream = &demuxer.info().streams()[0];
        assert_eq!(stream.handler(), "DIB ");
        assert_eq!(stream.time_base(), Rational::new(1, 25).unwrap());
        assert_eq!(stream.frame_rate(), Rational::new(25, 1).unwrap());
        assert_eq!(stream.length(), 2);
        assert_eq!(stream.sample_size(), 0);
        assert_eq!(stream.width(), 2);
        assert_eq!(stream.height(), 2);
        assert_eq!(stream.bit_count(), 24);
        assert_eq!(stream.compression(), "BI_RGB");

        let first_packet = demuxer.read_packet().unwrap().unwrap();
        assert_eq!(first_packet.data(), first);
        assert_eq!(first_packet.pts(), Some(0));
        assert_eq!(first_packet.dts(), Some(0));
        assert_eq!(first_packet.duration(), 1);
        assert_eq!(first_packet.side_data()[0].data(), b"00db");

        let second_packet = demuxer.read_packet().unwrap().unwrap();
        assert_eq!(second_packet.data(), second);
        assert_eq!(second_packet.pts(), Some(1));
        assert!(demuxer.read_packet().unwrap().is_none());
    }

    #[test]
    fn muxer_rejects_invalid_parameters_and_packets() {
        let rate = Rational::new(25, 1).unwrap();

        assert_eq!(
            AviMuxer::new_rgb24(0, 2, rate).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(
            AviMuxer::new_rgb24(2, 0, rate).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(
            AviMuxer::new_rgb24(2, 2, Rational::ZERO)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(
            AviMuxer::new_rgb24(i32::MAX as u32 + 1, 1, rate)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidArgument
        );

        let mut muxer = AviMuxer::new_rgb24(2, 2, rate).unwrap();
        assert_eq!(
            muxer
                .write_packet(&Packet::new(frame_bytes(12, 0x01), 1))
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(muxer.packet_count(), 0);

        assert_eq!(
            muxer
                .write_packet(&Packet::new(frame_bytes(11, 0x01), 0))
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
        assert_eq!(
            muxer
                .write_packet(&Packet::new(frame_bytes(13, 0x01), 0))
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
        assert_eq!(muxer.packet_count(), 0);
    }

    #[test]
    fn muxer_finish_prevents_more_writes() {
        let mut muxer = AviMuxer::new_rgb24(2, 2, Rational::new(25, 1).unwrap()).unwrap();
        muxer
            .write_packet(&Packet::new(frame_bytes(12, 0x10), 0))
            .unwrap();

        let output = muxer.finish().unwrap();
        assert!(!output.is_empty());
        assert!(muxer.is_finished());
        assert_eq!(
            muxer
                .write_packet(&Packet::new(frame_bytes(12, 0x20), 0))
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(muxer.packet_count(), 1);
    }

    #[test]
    fn muxer_pads_odd_sized_chunks_without_exposing_padding_as_payload() {
        let frame = vec![1, 2, 3];
        let mut muxer = AviMuxer::new_rgb24(1, 1, Rational::new(1, 1).unwrap()).unwrap();
        muxer.write_packet(&Packet::new(frame.clone(), 0)).unwrap();

        let bytes = muxer.finish().unwrap();
        assert_eq!(bytes.len() % 2, 0);

        let mut demuxer = AviDemuxer::open(&bytes).unwrap();
        let packet = demuxer.read_packet().unwrap().unwrap();
        assert_eq!(packet.data(), frame);
        assert!(demuxer.read_packet().unwrap().is_none());
    }

    fn avi_fixture(frames: &[&[u8]]) -> Vec<u8> {
        let movi_chunks = frames
            .iter()
            .map(|frame| chunk(*b"00db", frame))
            .collect::<Vec<_>>();
        avi_fixture_with_movi_payload(&movi_chunks)
    }

    fn avi_fixture_with_movi_payload(movi_chunks: &[Vec<u8>]) -> Vec<u8> {
        let hdrl = list(
            *HDRL_LIST,
            &[
                chunk(*AVIH_ID, &main_header(2, 2, 2, 1)),
                list(
                    *STRL_LIST,
                    &[
                        chunk(*STRH_ID, &stream_header(*b"vids", *b"DIB ", 1, 25, 2, 0)),
                        chunk(*STRF_ID, &bitmap_info(2, 2, 24, 0)),
                    ],
                ),
            ],
        );
        let movi = list(*MOVI_LIST, movi_chunks);
        riff_avi(&[hdrl, movi])
    }

    fn riff_avi(chunks: &[Vec<u8>]) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(AVI_FORM);
        for chunk in chunks {
            payload.extend_from_slice(chunk);
        }

        let mut out = ByteWriter::new();
        out.write_all(RIFF_ID);
        out.write_u32_le(u32::try_from(payload.len()).unwrap());
        out.write_all(&payload);
        out.into_inner()
    }

    fn list(kind: [u8; 4], chunks: &[Vec<u8>]) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&kind);
        for chunk in chunks {
            payload.extend_from_slice(chunk);
        }
        chunk(*LIST_ID, &payload)
    }

    fn chunk(id: [u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut out = ByteWriter::new();
        out.write_all(&id);
        out.write_u32_le(u32::try_from(payload.len()).unwrap());
        out.write_all(payload);
        if payload.len() % 2 == 1 {
            out.write_u8(0);
        }
        out.into_inner()
    }

    fn main_header(total_frames: u32, width: u32, height: u32, streams: u32) -> Vec<u8> {
        let mut out = ByteWriter::new();
        out.write_u32_le(40_000);
        out.write_u32_le(1_000_000);
        out.write_u32_le(0);
        out.write_u32_le(0x10);
        out.write_u32_le(total_frames);
        out.write_u32_le(0);
        out.write_u32_le(streams);
        out.write_u32_le(12);
        out.write_u32_le(width);
        out.write_u32_le(height);
        for _ in 0..4 {
            out.write_u32_le(0);
        }
        out.into_inner()
    }

    fn stream_header(
        stream_type: [u8; 4],
        handler: [u8; 4],
        scale: u32,
        rate: u32,
        length: u32,
        sample_size: u32,
    ) -> Vec<u8> {
        let mut out = ByteWriter::new();
        out.write_all(&stream_type);
        out.write_all(&handler);
        out.write_u32_le(0);
        out.write_u16_le(0);
        out.write_u16_le(0);
        out.write_u32_le(0);
        out.write_u32_le(scale);
        out.write_u32_le(rate);
        out.write_u32_le(0);
        out.write_u32_le(length);
        out.write_u32_le(12);
        out.write_u32_le(u32::MAX);
        out.write_u32_le(sample_size);
        out.write_i32_le(0);
        out.write_i32_le(0);
        out.write_i32_le(2);
        out.write_i32_le(2);
        out.into_inner()
    }

    fn bitmap_info(width: i32, height: i32, bit_count: u16, compression: u32) -> Vec<u8> {
        let mut out = ByteWriter::new();
        out.write_u32_le(40);
        out.write_i32_le(width);
        out.write_i32_le(height);
        out.write_u16_le(1);
        out.write_u16_le(bit_count);
        out.write_u32_le(compression);
        out.write_u32_le(12);
        out.write_i32_le(0);
        out.write_i32_le(0);
        out.write_u32_le(0);
        out.write_u32_le(0);
        out.into_inner()
    }

    fn frame_bytes(len: usize, start: u8) -> Vec<u8> {
        (0..len)
            .map(|offset| start.wrapping_add(offset as u8))
            .collect()
    }

    fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack
            .windows(needle.len())
            .position(|window| window == needle)
    }
}
