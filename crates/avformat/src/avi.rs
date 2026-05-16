use avutil::{AvError, AvErrorKind, AvResult, ByteReader, Packet, Rational, SideData};

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
