use avutil::{AvError, AvErrorKind, AvResult, ByteReader, Packet, SideData};

const FTYP_ID: &[u8; 4] = b"ftyp";
const MOOV_ID: &[u8; 4] = b"moov";
const MVHD_ID: &[u8; 4] = b"mvhd";
const MVEX_ID: &[u8; 4] = b"mvex";
const MOOF_ID: &[u8; 4] = b"moof";
const TRAK_ID: &[u8; 4] = b"trak";
const TKHD_ID: &[u8; 4] = b"tkhd";
const EDTS_ID: &[u8; 4] = b"edts";
const MDIA_ID: &[u8; 4] = b"mdia";
const MDHD_ID: &[u8; 4] = b"mdhd";
const MINF_ID: &[u8; 4] = b"minf";
const STBL_ID: &[u8; 4] = b"stbl";
const STSD_ID: &[u8; 4] = b"stsd";
const STTS_ID: &[u8; 4] = b"stts";
const CTTS_ID: &[u8; 4] = b"ctts";
const STSC_ID: &[u8; 4] = b"stsc";
const STSZ_ID: &[u8; 4] = b"stsz";
const STSS_ID: &[u8; 4] = b"stss";
const STCO_ID: &[u8; 4] = b"stco";
const CO64_ID: &[u8; 4] = b"co64";
const MDAT_ID: &[u8; 4] = b"mdat";
const AVCC_ID: &[u8; 4] = b"avcC";
const MAX_MOV_SAMPLE_COUNT: usize = 1_000_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovInfo {
    major_brand: String,
    minor_version: u32,
    compatible_brands: Vec<String>,
    timescale: u32,
    duration: Option<u64>,
    tracks: Vec<MovTrackInfo>,
    has_media_data: bool,
}

impl MovInfo {
    pub fn major_brand(&self) -> &str {
        &self.major_brand
    }

    pub fn minor_version(&self) -> u32 {
        self.minor_version
    }

    pub fn compatible_brands(&self) -> &[String] {
        &self.compatible_brands
    }

    pub fn timescale(&self) -> u32 {
        self.timescale
    }

    pub fn duration(&self) -> Option<u64> {
        self.duration
    }

    pub fn tracks(&self) -> &[MovTrackInfo] {
        &self.tracks
    }

    pub fn has_media_data(&self) -> bool {
        self.has_media_data
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovCodecParameters {
    codec_tag: String,
    data_reference_index: u16,
    extra_data: Vec<u8>,
    details: MovSampleEntryDetails,
}

impl MovCodecParameters {
    pub fn codec_tag(&self) -> &str {
        &self.codec_tag
    }

    pub fn data_reference_index(&self) -> u16 {
        self.data_reference_index
    }

    pub fn extra_data(&self) -> &[u8] {
        &self.extra_data
    }

    pub fn details(&self) -> &MovSampleEntryDetails {
        &self.details
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MovSampleEntryDetails {
    Generic,
    Video(MovVideoSampleEntry),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovVideoSampleEntry {
    width: u16,
    height: u16,
    frame_count: u16,
    compressor_name: String,
    depth: u16,
    child_boxes: Vec<MovSampleEntryChildBox>,
}

impl MovVideoSampleEntry {
    pub fn width(&self) -> u16 {
        self.width
    }

    pub fn height(&self) -> u16 {
        self.height
    }

    pub fn frame_count(&self) -> u16 {
        self.frame_count
    }

    pub fn compressor_name(&self) -> &str {
        &self.compressor_name
    }

    pub fn depth(&self) -> u16 {
        self.depth
    }

    pub fn child_boxes(&self) -> &[MovSampleEntryChildBox] {
        &self.child_boxes
    }

    pub fn avc_decoder_configuration_record(&self) -> Option<&[u8]> {
        self.child_boxes
            .iter()
            .find(|child| child.box_type.as_bytes() == AVCC_ID)
            .map(MovSampleEntryChildBox::payload)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovSampleEntryChildBox {
    box_type: String,
    payload: Vec<u8>,
}

impl MovSampleEntryChildBox {
    pub fn box_type(&self) -> &str {
        &self.box_type
    }

    pub fn payload(&self) -> &[u8] {
        &self.payload
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovTrackInfo {
    id: u32,
    duration: Option<u64>,
    width: Option<u32>,
    height: Option<u32>,
    media_timescale: u32,
    media_duration: Option<u64>,
    codec_parameters: Option<MovCodecParameters>,
    sample_count: usize,
}

impl MovTrackInfo {
    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn duration(&self) -> Option<u64> {
        self.duration
    }

    pub fn width(&self) -> Option<u32> {
        self.width
    }

    pub fn height(&self) -> Option<u32> {
        self.height
    }

    pub fn media_timescale(&self) -> u32 {
        self.media_timescale
    }

    pub fn media_duration(&self) -> Option<u64> {
        self.media_duration
    }

    pub fn codec_tag(&self) -> Option<&str> {
        self.codec_parameters
            .as_ref()
            .map(MovCodecParameters::codec_tag)
    }

    pub fn codec_parameters(&self) -> Option<&MovCodecParameters> {
        self.codec_parameters.as_ref()
    }

    pub fn sample_count(&self) -> usize {
        self.sample_count
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovDemuxer<'a> {
    info: MovInfo,
    packets: Option<Vec<Packet>>,
    next_packet: usize,
    _input: &'a [u8],
}

impl<'a> MovDemuxer<'a> {
    pub fn open(input: &'a [u8]) -> AvResult<Self> {
        let top_level = read_box_headers(input, 0, input.len(), "MOV/MP4 file")?;
        let mut ftyp = None;
        let mut movie_header = None;
        let mut tracks = Vec::new();
        let mut mdat_ranges = Vec::new();
        let mut has_movie_extends = false;
        let mut has_movie_fragment = false;

        for header in top_level {
            let payload = &input[header.payload_start..header.payload_end];
            match &header.box_type {
                FTYP_ID => ftyp = Some(parse_ftyp(payload)?),
                MOOV_ID => {
                    let movie = parse_moov(input, &header)?;
                    movie_header = Some(movie.header);
                    tracks = movie.tracks;
                    has_movie_extends = movie.has_movie_extends;
                }
                MDAT_ID => mdat_ranges.push((header.payload_start, header.payload_end)),
                MOOF_ID => has_movie_fragment = true,
                _ => {}
            }
        }

        let ftyp = ftyp.ok_or_else(|| AvError::invalid_data("MOV/MP4 missing ftyp box"))?;
        let movie_header =
            movie_header.ok_or_else(|| AvError::invalid_data("MOV/MP4 missing moov box"))?;
        if has_movie_extends || has_movie_fragment {
            return Err(AvError::unsupported(
                "fragmented MOV/MP4 files with mvex/moof boxes are not implemented",
            ));
        }
        if tracks.is_empty() {
            return Err(AvError::invalid_data("MOV/MP4 missing trak boxes"));
        }
        let packets = build_packets(input, &tracks, &mdat_ranges)?;

        let track_info = tracks.into_iter().map(|track| track.info).collect();
        Ok(Self {
            info: MovInfo {
                major_brand: ftyp.major_brand,
                minor_version: ftyp.minor_version,
                compatible_brands: ftyp.compatible_brands,
                timescale: movie_header.timescale,
                duration: movie_header.duration,
                tracks: track_info,
                has_media_data: !mdat_ranges.is_empty(),
            },
            packets,
            next_packet: 0,
            _input: input,
        })
    }

    pub fn info(&self) -> &MovInfo {
        &self.info
    }

    pub fn read_packet(&mut self) -> AvResult<Option<Packet>> {
        let Some(packets) = &self.packets else {
            return Err(AvError::unsupported(
                "MOV/MP4 sample table packet extraction is not implemented for this file",
            ));
        };
        let Some(packet) = packets.get(self.next_packet).cloned() else {
            return Ok(None);
        };
        self.next_packet += 1;
        Ok(Some(packet))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FtypInfo {
    major_brand: String,
    minor_version: u32,
    compatible_brands: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MovieHeader {
    timescale: u32,
    duration: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MovieData {
    header: MovieHeader,
    tracks: Vec<TrackData>,
    has_movie_extends: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TrackData {
    info: MovTrackInfo,
    sample_table: Option<SampleTable>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TrackHeader {
    id: u32,
    duration: Option<u64>,
    width: Option<u32>,
    height: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MediaHeader {
    timescale: u32,
    duration: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MediaData {
    header: MediaHeader,
    sample_table: Option<SampleTable>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SampleTable {
    codec_parameters: MovCodecParameters,
    sample_sizes: Vec<usize>,
    sample_durations: Vec<u32>,
    composition_offsets: Option<Vec<i64>>,
    sample_to_chunks: Vec<SampleToChunkEntry>,
    chunk_offsets: Vec<u64>,
    sync_samples: Option<Vec<bool>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SampleToChunkEntry {
    first_chunk: u32,
    samples_per_chunk: u32,
    sample_description_index: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BoxHeader {
    box_type: [u8; 4],
    payload_start: usize,
    payload_end: usize,
}

impl BoxHeader {
    fn payload<'a>(&self, input: &'a [u8]) -> &'a [u8] {
        &input[self.payload_start..self.payload_end]
    }
}

fn parse_ftyp(payload: &[u8]) -> AvResult<FtypInfo> {
    if payload.len() < 8 {
        return Err(AvError::new(
            AvErrorKind::EndOfFile,
            "MOV/MP4 ftyp box is shorter than 8 bytes",
        ));
    }
    if (payload.len() - 8) % 4 != 0 {
        return Err(AvError::invalid_data(
            "MOV/MP4 ftyp compatible brand list is not FourCC-aligned",
        ));
    }

    let mut reader = ByteReader::new(payload);
    let major_brand = fourcc_to_string(read_fourcc(&mut reader)?);
    let minor_version = reader.read_u32_be()?;
    let mut compatible_brands = Vec::new();
    while !reader.is_eof() {
        compatible_brands.push(fourcc_to_string(read_fourcc(&mut reader)?));
    }

    Ok(FtypInfo {
        major_brand,
        minor_version,
        compatible_brands,
    })
}

fn parse_moov(input: &[u8], moov: &BoxHeader) -> AvResult<MovieData> {
    let mut header = None;
    let mut tracks = Vec::new();
    let mut has_movie_extends = false;

    for child in read_box_headers(input, moov.payload_start, moov.payload_end, "MOV/MP4 moov")? {
        match &child.box_type {
            MVHD_ID => header = Some(parse_mvhd(child.payload(input))?),
            MVEX_ID => has_movie_extends = true,
            TRAK_ID => tracks.push(parse_trak(input, &child)?),
            _ => {}
        }
    }

    let header = header.ok_or_else(|| AvError::invalid_data("MOV/MP4 missing mvhd box"))?;
    Ok(MovieData {
        header,
        tracks,
        has_movie_extends,
    })
}

fn parse_trak(input: &[u8], trak: &BoxHeader) -> AvResult<TrackData> {
    let mut track_header = None;
    let mut media_data = None;

    for child in read_box_headers(input, trak.payload_start, trak.payload_end, "MOV/MP4 trak")? {
        match &child.box_type {
            TKHD_ID => track_header = Some(parse_tkhd(child.payload(input))?),
            EDTS_ID => {
                return Err(AvError::unsupported(
                    "MOV/MP4 edit lists with edts/elst boxes are not implemented",
                ));
            }
            MDIA_ID => media_data = Some(parse_mdia(input, &child)?),
            _ => {}
        }
    }

    let track_header =
        track_header.ok_or_else(|| AvError::invalid_data("MOV/MP4 track missing tkhd box"))?;
    let media_data =
        media_data.ok_or_else(|| AvError::invalid_data("MOV/MP4 track missing mdhd box"))?;
    let codec_parameters = media_data
        .sample_table
        .as_ref()
        .map(|table| table.codec_parameters.clone());
    let sample_count = media_data
        .sample_table
        .as_ref()
        .map_or(0, |table| table.sample_sizes.len());

    Ok(TrackData {
        info: MovTrackInfo {
            id: track_header.id,
            duration: track_header.duration,
            width: track_header.width,
            height: track_header.height,
            media_timescale: media_data.header.timescale,
            media_duration: media_data.header.duration,
            codec_parameters,
            sample_count,
        },
        sample_table: media_data.sample_table,
    })
}

fn parse_mdia(input: &[u8], mdia: &BoxHeader) -> AvResult<MediaData> {
    let mut media_header = None;
    let mut sample_table = None;
    for child in read_box_headers(input, mdia.payload_start, mdia.payload_end, "MOV/MP4 mdia")? {
        match &child.box_type {
            MDHD_ID => media_header = Some(parse_mdhd(child.payload(input))?),
            MINF_ID => sample_table = parse_minf(input, &child)?,
            _ => {}
        }
    }
    let header =
        media_header.ok_or_else(|| AvError::invalid_data("MOV/MP4 mdia missing mdhd box"))?;
    Ok(MediaData {
        header,
        sample_table,
    })
}

fn parse_minf(input: &[u8], minf: &BoxHeader) -> AvResult<Option<SampleTable>> {
    let mut sample_table = None;
    for child in read_box_headers(input, minf.payload_start, minf.payload_end, "MOV/MP4 minf")? {
        if child.box_type == *STBL_ID {
            sample_table = Some(parse_stbl(input, &child)?);
        }
    }
    Ok(sample_table)
}

fn parse_stbl(input: &[u8], stbl: &BoxHeader) -> AvResult<SampleTable> {
    let mut codec_parameters = None;
    let mut sample_durations = None;
    let mut composition_offsets = None;
    let mut sample_to_chunks = None;
    let mut sample_sizes = None;
    let mut chunk_offsets = None;
    let mut sync_sample_numbers = None;

    for child in read_box_headers(input, stbl.payload_start, stbl.payload_end, "MOV/MP4 stbl")? {
        match &child.box_type {
            STSD_ID => codec_parameters = Some(parse_stsd(child.payload(input))?),
            STTS_ID => sample_durations = Some(parse_stts(child.payload(input))?),
            CTTS_ID => composition_offsets = Some(parse_ctts(child.payload(input))?),
            STSC_ID => sample_to_chunks = Some(parse_stsc(child.payload(input))?),
            STSZ_ID => sample_sizes = Some(parse_stsz(child.payload(input))?),
            STSS_ID => sync_sample_numbers = Some(parse_stss(child.payload(input))?),
            STCO_ID => chunk_offsets = Some(parse_stco(child.payload(input))?),
            CO64_ID => chunk_offsets = Some(parse_co64(child.payload(input))?),
            _ => {}
        }
    }

    let codec_parameters =
        codec_parameters.ok_or_else(|| AvError::invalid_data("MOV/MP4 stbl missing stsd"))?;
    let sample_durations =
        sample_durations.ok_or_else(|| AvError::invalid_data("MOV/MP4 stbl missing stts"))?;
    let sample_to_chunks =
        sample_to_chunks.ok_or_else(|| AvError::invalid_data("MOV/MP4 stbl missing stsc"))?;
    let sample_sizes =
        sample_sizes.ok_or_else(|| AvError::invalid_data("MOV/MP4 stbl missing stsz"))?;
    let chunk_offsets =
        chunk_offsets.ok_or_else(|| AvError::invalid_data("MOV/MP4 stbl missing stco/co64"))?;
    if sample_durations.len() != sample_sizes.len() {
        return Err(AvError::invalid_data(format!(
            "MOV/MP4 stts describes {} samples but stsz describes {}",
            sample_durations.len(),
            sample_sizes.len()
        )));
    }
    if let Some(offsets) = &composition_offsets {
        if offsets.len() != sample_sizes.len() {
            return Err(AvError::invalid_data(format!(
                "MOV/MP4 ctts describes {} samples but stsz describes {}",
                offsets.len(),
                sample_sizes.len()
            )));
        }
    }
    let sync_samples = sync_sample_numbers
        .map(|sample_numbers| sync_sample_flags(sample_numbers, sample_sizes.len()))
        .transpose()?;

    Ok(SampleTable {
        codec_parameters,
        sample_sizes,
        sample_durations,
        composition_offsets,
        sample_to_chunks,
        chunk_offsets,
        sync_samples,
    })
}

fn parse_stsd(payload: &[u8]) -> AvResult<MovCodecParameters> {
    let mut reader = ByteReader::new(payload);
    read_full_box_header(&mut reader, "MOV/MP4 stsd")?;
    let entry_count = reader.read_u32_be()?;
    if entry_count == 0 {
        return Err(AvError::invalid_data(
            "MOV/MP4 stsd must contain at least one sample entry",
        ));
    }
    if entry_count != 1 {
        return Err(AvError::unsupported(
            "MOV/MP4 stsd with multiple sample entries is not implemented",
        ));
    }
    ensure_remaining(&reader, 16, "MOV/MP4 stsd sample entry")?;
    let entry_size = usize::try_from(reader.read_u32_be()?)
        .map_err(|_| AvError::invalid_data("MOV/MP4 stsd sample entry size is out of range"))?;
    if entry_size < 16 {
        return Err(AvError::invalid_data(
            "MOV/MP4 stsd sample entry is shorter than its base header",
        ));
    }
    let entry_type = read_fourcc(&mut reader)?;
    let remaining_entry = entry_size
        .checked_sub(8)
        .ok_or_else(|| AvError::invalid_data("MOV/MP4 stsd sample entry size underflow"))?;
    if remaining_entry > reader.remaining() {
        return Err(AvError::new(
            AvErrorKind::EndOfFile,
            "MOV/MP4 stsd sample entry exceeds box payload",
        ));
    }
    reader.skip(6)?;
    let data_reference_index = reader.read_u16_be()?;
    let extra_data = reader.read_exact(entry_size - 16)?.to_vec();
    if !reader.is_eof() {
        return Err(AvError::invalid_data(
            "MOV/MP4 stsd contains trailing sample entry data",
        ));
    }
    let codec_tag = fourcc_to_string(entry_type);
    let details = parse_sample_entry_details(&codec_tag, &extra_data)?;
    Ok(MovCodecParameters {
        codec_tag,
        data_reference_index,
        extra_data,
        details,
    })
}

fn parse_sample_entry_details(
    codec_tag: &str,
    extra_data: &[u8],
) -> AvResult<MovSampleEntryDetails> {
    match codec_tag.as_bytes() {
        b"avc1" | b"hvc1" | b"hev1" | b"mp4v" => {
            parse_visual_sample_entry(extra_data).map(MovSampleEntryDetails::Video)
        }
        _ => Ok(MovSampleEntryDetails::Generic),
    }
}

fn parse_visual_sample_entry(extra_data: &[u8]) -> AvResult<MovVideoSampleEntry> {
    let mut reader = ByteReader::new(extra_data);
    ensure_remaining(&reader, 70, "MOV/MP4 VisualSampleEntry")?;
    reader.skip(16)?;
    let width = reader.read_u16_be()?;
    let height = reader.read_u16_be()?;
    reader.skip(12)?;
    let frame_count = reader.read_u16_be()?;
    let compressor_name = parse_pascal_string_31(reader.read_exact(32)?);
    let depth = reader.read_u16_be()?;
    reader.skip(2)?;
    let child_boxes = parse_sample_entry_child_boxes(
        extra_data,
        reader.position(),
        extra_data.len(),
        "MOV/MP4 VisualSampleEntry children",
    )?;
    Ok(MovVideoSampleEntry {
        width,
        height,
        frame_count,
        compressor_name,
        depth,
        child_boxes,
    })
}

fn parse_sample_entry_child_boxes(
    input: &[u8],
    start: usize,
    end: usize,
    context: &str,
) -> AvResult<Vec<MovSampleEntryChildBox>> {
    read_box_headers(input, start, end, context)?
        .into_iter()
        .map(|header| {
            Ok(MovSampleEntryChildBox {
                box_type: fourcc_to_string(header.box_type),
                payload: header.payload(input).to_vec(),
            })
        })
        .collect()
}

fn parse_pascal_string_31(input: &[u8]) -> String {
    let declared_len = input.first().copied().unwrap_or(0) as usize;
    let len = declared_len.min(31).min(input.len().saturating_sub(1));
    String::from_utf8_lossy(&input[1..1 + len]).into_owned()
}

fn parse_stts(payload: &[u8]) -> AvResult<Vec<u32>> {
    let mut reader = ByteReader::new(payload);
    read_full_box_header(&mut reader, "MOV/MP4 stts")?;
    let entry_count = reader.read_u32_be()?;
    let mut durations = Vec::new();
    for _ in 0..entry_count {
        ensure_remaining(&reader, 8, "MOV/MP4 stts entry")?;
        let sample_count = checked_sample_count(reader.read_u32_be()?, "MOV/MP4 stts")?;
        let sample_delta = reader.read_u32_be()?;
        let new_len = durations
            .len()
            .checked_add(sample_count)
            .ok_or_else(|| AvError::invalid_data("MOV/MP4 stts sample count overflow"))?;
        if new_len > MAX_MOV_SAMPLE_COUNT {
            return Err(AvError::unsupported(
                "MOV/MP4 stts sample count exceeds current implementation limit",
            ));
        }
        durations.resize(new_len, sample_delta);
    }
    if durations.is_empty() {
        return Err(AvError::invalid_data("MOV/MP4 stts describes zero samples"));
    }
    ensure_box_consumed(&reader, "MOV/MP4 stts")?;
    Ok(durations)
}

fn parse_ctts(payload: &[u8]) -> AvResult<Vec<i64>> {
    let mut reader = ByteReader::new(payload);
    let (version, _) = read_full_box_header(&mut reader, "MOV/MP4 ctts")?;
    if version > 1 {
        return Err(AvError::unsupported(format!(
            "unsupported MOV/MP4 ctts version {version}"
        )));
    }

    let entry_count = reader.read_u32_be()?;
    let mut offsets = Vec::new();
    for _ in 0..entry_count {
        ensure_remaining(&reader, 8, "MOV/MP4 ctts entry")?;
        let sample_count = checked_sample_count(reader.read_u32_be()?, "MOV/MP4 ctts")?;
        let sample_offset = if version == 0 {
            i64::from(reader.read_u32_be()?)
        } else {
            i64::from(reader.read_i32_be()?)
        };
        let new_len = offsets
            .len()
            .checked_add(sample_count)
            .ok_or_else(|| AvError::invalid_data("MOV/MP4 ctts sample count overflow"))?;
        if new_len > MAX_MOV_SAMPLE_COUNT {
            return Err(AvError::unsupported(
                "MOV/MP4 ctts sample count exceeds current implementation limit",
            ));
        }
        offsets.resize(new_len, sample_offset);
    }
    if offsets.is_empty() {
        return Err(AvError::invalid_data("MOV/MP4 ctts describes zero samples"));
    }
    ensure_box_consumed(&reader, "MOV/MP4 ctts")?;
    Ok(offsets)
}

fn parse_stsc(payload: &[u8]) -> AvResult<Vec<SampleToChunkEntry>> {
    let mut reader = ByteReader::new(payload);
    read_full_box_header(&mut reader, "MOV/MP4 stsc")?;
    let entry_count = reader.read_u32_be()?;
    if entry_count == 0 {
        return Err(AvError::invalid_data("MOV/MP4 stsc has no entries"));
    }

    let mut entries = Vec::new();
    let mut previous_first_chunk = 0;
    for _ in 0..entry_count {
        ensure_remaining(&reader, 12, "MOV/MP4 stsc entry")?;
        let first_chunk = reader.read_u32_be()?;
        let samples_per_chunk = reader.read_u32_be()?;
        let sample_description_index = reader.read_u32_be()?;
        if first_chunk == 0 || first_chunk <= previous_first_chunk {
            return Err(AvError::invalid_data(
                "MOV/MP4 stsc first_chunk values must be positive and increasing",
            ));
        }
        if samples_per_chunk == 0 {
            return Err(AvError::invalid_data(
                "MOV/MP4 stsc samples_per_chunk must be non-zero",
            ));
        }
        if sample_description_index != 1 {
            return Err(AvError::unsupported(
                "MOV/MP4 stsc sample_description_index values other than 1 are not implemented",
            ));
        }
        previous_first_chunk = first_chunk;
        entries.push(SampleToChunkEntry {
            first_chunk,
            samples_per_chunk,
            sample_description_index,
        });
    }
    ensure_box_consumed(&reader, "MOV/MP4 stsc")?;
    Ok(entries)
}

fn parse_stsz(payload: &[u8]) -> AvResult<Vec<usize>> {
    let mut reader = ByteReader::new(payload);
    read_full_box_header(&mut reader, "MOV/MP4 stsz")?;
    let default_sample_size = reader.read_u32_be()?;
    let sample_count = checked_sample_count(reader.read_u32_be()?, "MOV/MP4 stsz")?;
    if sample_count == 0 {
        return Err(AvError::invalid_data("MOV/MP4 stsz describes zero samples"));
    }

    let sample_sizes = if default_sample_size != 0 {
        let size = checked_sample_size(default_sample_size, "MOV/MP4 stsz default sample size")?;
        vec![size; sample_count]
    } else {
        let mut sizes = Vec::with_capacity(sample_count);
        for _ in 0..sample_count {
            ensure_remaining(&reader, 4, "MOV/MP4 stsz sample size")?;
            sizes.push(checked_sample_size(
                reader.read_u32_be()?,
                "MOV/MP4 stsz sample size",
            )?);
        }
        sizes
    };
    ensure_box_consumed(&reader, "MOV/MP4 stsz")?;
    Ok(sample_sizes)
}

fn parse_stss(payload: &[u8]) -> AvResult<Vec<u32>> {
    let mut reader = ByteReader::new(payload);
    read_full_box_header(&mut reader, "MOV/MP4 stss")?;
    let entry_count = usize::try_from(reader.read_u32_be()?)
        .map_err(|_| AvError::invalid_data("MOV/MP4 stss entry count is out of range"))?;
    if entry_count > MAX_MOV_SAMPLE_COUNT {
        return Err(AvError::unsupported(
            "MOV/MP4 stss entry count exceeds current implementation limit",
        ));
    }
    let mut sample_numbers = Vec::with_capacity(entry_count);
    let mut previous_sample_number = 0;
    for _ in 0..entry_count {
        ensure_remaining(&reader, 4, "MOV/MP4 stss entry")?;
        let sample_number = reader.read_u32_be()?;
        if sample_number == 0 || sample_number <= previous_sample_number {
            return Err(AvError::invalid_data(
                "MOV/MP4 stss sample numbers must be positive and increasing",
            ));
        }
        previous_sample_number = sample_number;
        sample_numbers.push(sample_number);
    }
    ensure_box_consumed(&reader, "MOV/MP4 stss")?;
    Ok(sample_numbers)
}

fn parse_stco(payload: &[u8]) -> AvResult<Vec<u64>> {
    let mut reader = ByteReader::new(payload);
    read_full_box_header(&mut reader, "MOV/MP4 stco")?;
    let entry_count = checked_table_entry_count(reader.read_u32_be()?, "MOV/MP4 stco")?;
    let mut offsets = Vec::with_capacity(entry_count);
    for _ in 0..entry_count {
        ensure_remaining(&reader, 4, "MOV/MP4 stco entry")?;
        offsets.push(u64::from(reader.read_u32_be()?));
    }
    ensure_box_consumed(&reader, "MOV/MP4 stco")?;
    Ok(offsets)
}

fn parse_co64(payload: &[u8]) -> AvResult<Vec<u64>> {
    let mut reader = ByteReader::new(payload);
    read_full_box_header(&mut reader, "MOV/MP4 co64")?;
    let entry_count = checked_table_entry_count(reader.read_u32_be()?, "MOV/MP4 co64")?;
    let mut offsets = Vec::with_capacity(entry_count);
    for _ in 0..entry_count {
        ensure_remaining(&reader, 8, "MOV/MP4 co64 entry")?;
        offsets.push(reader.read_u64_be()?);
    }
    ensure_box_consumed(&reader, "MOV/MP4 co64")?;
    Ok(offsets)
}

fn parse_mvhd(payload: &[u8]) -> AvResult<MovieHeader> {
    let mut reader = ByteReader::new(payload);
    let (version, _) = read_full_box_header(&mut reader, "MOV/MP4 mvhd")?;
    let (timescale, duration) = match version {
        0 => {
            ensure_remaining(&reader, 16, "MOV/MP4 mvhd version 0")?;
            reader.skip(8)?;
            let timescale = reader.read_u32_be()?;
            let duration = unknown_u32_duration(reader.read_u32_be()?);
            (timescale, duration)
        }
        1 => {
            ensure_remaining(&reader, 28, "MOV/MP4 mvhd version 1")?;
            reader.skip(16)?;
            let timescale = reader.read_u32_be()?;
            let duration = unknown_u64_duration(reader.read_u64_be()?);
            (timescale, duration)
        }
        _ => {
            return Err(AvError::unsupported(format!(
                "unsupported MOV/MP4 mvhd version {version}"
            )));
        }
    };
    validate_timescale(timescale, "MOV/MP4 movie timescale")?;
    Ok(MovieHeader {
        timescale,
        duration,
    })
}

fn parse_tkhd(payload: &[u8]) -> AvResult<TrackHeader> {
    let mut reader = ByteReader::new(payload);
    let (version, _) = read_full_box_header(&mut reader, "MOV/MP4 tkhd")?;
    let (id, duration) = match version {
        0 => {
            ensure_remaining(&reader, 80, "MOV/MP4 tkhd version 0")?;
            reader.skip(8)?;
            let id = reader.read_u32_be()?;
            reader.skip(4)?;
            let duration = unknown_u32_duration(reader.read_u32_be()?);
            reader.skip(52)?;
            (id, duration)
        }
        1 => {
            ensure_remaining(&reader, 92, "MOV/MP4 tkhd version 1")?;
            reader.skip(16)?;
            let id = reader.read_u32_be()?;
            reader.skip(4)?;
            let duration = unknown_u64_duration(reader.read_u64_be()?);
            reader.skip(52)?;
            (id, duration)
        }
        _ => {
            return Err(AvError::unsupported(format!(
                "unsupported MOV/MP4 tkhd version {version}"
            )));
        }
    };
    if id == 0 {
        return Err(AvError::invalid_data(
            "MOV/MP4 tkhd track ID must be non-zero",
        ));
    }

    let width = fixed_16_16_to_dimension(reader.read_u32_be()?);
    let height = fixed_16_16_to_dimension(reader.read_u32_be()?);
    Ok(TrackHeader {
        id,
        duration,
        width,
        height,
    })
}

fn parse_mdhd(payload: &[u8]) -> AvResult<MediaHeader> {
    let mut reader = ByteReader::new(payload);
    let (version, _) = read_full_box_header(&mut reader, "MOV/MP4 mdhd")?;
    let (timescale, duration) = match version {
        0 => {
            ensure_remaining(&reader, 16, "MOV/MP4 mdhd version 0")?;
            reader.skip(8)?;
            let timescale = reader.read_u32_be()?;
            let duration = unknown_u32_duration(reader.read_u32_be()?);
            (timescale, duration)
        }
        1 => {
            ensure_remaining(&reader, 28, "MOV/MP4 mdhd version 1")?;
            reader.skip(16)?;
            let timescale = reader.read_u32_be()?;
            let duration = unknown_u64_duration(reader.read_u64_be()?);
            (timescale, duration)
        }
        _ => {
            return Err(AvError::unsupported(format!(
                "unsupported MOV/MP4 mdhd version {version}"
            )));
        }
    };
    validate_timescale(timescale, "MOV/MP4 media timescale")?;
    Ok(MediaHeader {
        timescale,
        duration,
    })
}

fn build_packets(
    input: &[u8],
    tracks: &[TrackData],
    mdat_ranges: &[(usize, usize)],
) -> AvResult<Option<Vec<Packet>>> {
    let tracks_with_tables = tracks
        .iter()
        .enumerate()
        .filter_map(|(stream_index, track)| {
            track
                .sample_table
                .as_ref()
                .map(|table| (stream_index, track, table))
        })
        .collect::<Vec<_>>();
    if tracks_with_tables.is_empty() {
        return Ok(None);
    }
    if tracks_with_tables.len() != 1 {
        return Err(AvError::unsupported(
            "MOV/MP4 packet extraction for multiple populated tracks is not implemented",
        ));
    }
    if mdat_ranges.is_empty() {
        return Err(AvError::invalid_data(
            "MOV/MP4 sample table references media data but no mdat box is present",
        ));
    }

    let (stream_index, track, table) = tracks_with_tables[0];
    let sample_spans = build_sample_spans(table)?;
    let mut packets = Vec::with_capacity(sample_spans.len());
    let mut dts = 0_i64;

    for (sample_index, span) in sample_spans.into_iter().enumerate() {
        let start = usize::try_from(span.offset)
            .map_err(|_| AvError::invalid_data("MOV/MP4 sample offset is out of range"))?;
        let end = start
            .checked_add(span.size)
            .ok_or_else(|| AvError::invalid_data("MOV/MP4 sample range overflow"))?;
        if end > input.len() || !range_within_mdat(start, end, mdat_ranges) {
            return Err(AvError::new(
                AvErrorKind::EndOfFile,
                "MOV/MP4 sample payload exceeds mdat bounds",
            ));
        }

        let mut packet = Packet::new(input[start..end].to_vec(), stream_index);
        let pts = dts
            .checked_add(composition_offset(table, sample_index))
            .ok_or_else(|| AvError::invalid_data("MOV/MP4 packet PTS overflow"))?;
        packet.set_pts(Some(pts));
        packet.set_dts(Some(dts));
        packet.set_duration(i64::from(span.duration))?;
        packet.set_key(is_sync_sample(table, sample_index));
        packet.push_side_data(SideData::new(
            "mov_track_id",
            track.info.id.to_be_bytes().to_vec(),
        )?);
        packet.push_side_data(SideData::new(
            "mov_codec_tag",
            table.codec_parameters.codec_tag.as_bytes().to_vec(),
        )?);
        dts = dts
            .checked_add(i64::from(span.duration))
            .ok_or_else(|| AvError::invalid_data("MOV/MP4 packet DTS overflow"))?;
        packets.push(packet);
    }

    Ok(Some(packets))
}

fn is_sync_sample(table: &SampleTable, sample_index: usize) -> bool {
    table
        .sync_samples
        .as_ref()
        .map_or(true, |sync_samples| sync_samples[sample_index])
}

fn composition_offset(table: &SampleTable, sample_index: usize) -> i64 {
    table
        .composition_offsets
        .as_ref()
        .map_or(0, |offsets| offsets[sample_index])
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SampleSpan {
    offset: u64,
    size: usize,
    duration: u32,
}

fn build_sample_spans(table: &SampleTable) -> AvResult<Vec<SampleSpan>> {
    if table.sample_to_chunks[0].first_chunk != 1 {
        return Err(AvError::invalid_data(
            "MOV/MP4 first stsc entry must start at chunk 1",
        ));
    }

    let mut spans = Vec::with_capacity(table.sample_sizes.len());
    let mut sample_index = 0;
    for (chunk_index_zero, chunk_offset) in table.chunk_offsets.iter().copied().enumerate() {
        if sample_index == table.sample_sizes.len() {
            break;
        }

        let chunk_index = u32::try_from(chunk_index_zero + 1)
            .map_err(|_| AvError::unsupported("MOV/MP4 chunk index exceeds u32 range"))?;
        let entry = sample_to_chunk_entry(&table.sample_to_chunks, chunk_index)?;
        if entry.sample_description_index != 1 {
            return Err(AvError::unsupported(
                "MOV/MP4 sample descriptions other than index 1 are not implemented",
            ));
        }

        let samples_in_chunk = usize::try_from(entry.samples_per_chunk).map_err(|_| {
            AvError::unsupported("MOV/MP4 samples_per_chunk exceeds addressable memory")
        })?;
        let mut relative_offset = 0_u64;
        for _ in 0..samples_in_chunk {
            if sample_index == table.sample_sizes.len() {
                break;
            }
            let size = table.sample_sizes[sample_index];
            let offset = chunk_offset
                .checked_add(relative_offset)
                .ok_or_else(|| AvError::invalid_data("MOV/MP4 sample offset overflow"))?;
            spans.push(SampleSpan {
                offset,
                size,
                duration: table.sample_durations[sample_index],
            });
            relative_offset = relative_offset
                .checked_add(
                    u64::try_from(size).map_err(|_| {
                        AvError::invalid_data("MOV/MP4 sample size does not fit u64")
                    })?,
                )
                .ok_or_else(|| AvError::invalid_data("MOV/MP4 chunk payload offset overflow"))?;
            sample_index += 1;
        }
    }

    if sample_index != table.sample_sizes.len() {
        return Err(AvError::invalid_data(
            "MOV/MP4 stsc/stco tables do not map every sample",
        ));
    }
    Ok(spans)
}

fn sample_to_chunk_entry(
    entries: &[SampleToChunkEntry],
    chunk_index: u32,
) -> AvResult<SampleToChunkEntry> {
    let mut selected = None;
    for entry in entries {
        if entry.first_chunk > chunk_index {
            break;
        }
        selected = Some(*entry);
    }
    selected.ok_or_else(|| AvError::invalid_data("MOV/MP4 chunk has no stsc mapping"))
}

fn range_within_mdat(start: usize, end: usize, mdat_ranges: &[(usize, usize)]) -> bool {
    mdat_ranges
        .iter()
        .any(|(mdat_start, mdat_end)| start >= *mdat_start && end <= *mdat_end)
}

fn sync_sample_flags(sample_numbers: Vec<u32>, sample_count: usize) -> AvResult<Vec<bool>> {
    let mut flags = vec![false; sample_count];
    for sample_number in sample_numbers {
        let sample_index = usize::try_from(sample_number - 1)
            .map_err(|_| AvError::invalid_data("MOV/MP4 stss sample number is out of range"))?;
        let Some(flag) = flags.get_mut(sample_index) else {
            return Err(AvError::invalid_data(
                "MOV/MP4 stss sample number exceeds stsz sample count",
            ));
        };
        *flag = true;
    }
    Ok(flags)
}

fn read_box_headers(
    input: &[u8],
    start: usize,
    end: usize,
    context: &str,
) -> AvResult<Vec<BoxHeader>> {
    if start > end || end > input.len() {
        return Err(AvError::invalid_argument(format!(
            "{context} box range is out of bounds"
        )));
    }

    let mut headers = Vec::new();
    let mut position = start;
    while position < end {
        if end - position < 8 {
            return Err(AvError::new(
                AvErrorKind::EndOfFile,
                format!("{context} box header is truncated"),
            ));
        }

        let mut reader = ByteReader::new(&input[position..end]);
        let size32 = reader.read_u32_be()?;
        let box_type = read_fourcc(&mut reader)?;
        let (header_size, box_size) = match size32 {
            0 => (8_usize, end - position),
            1 => {
                let extended = reader.read_u64_be()?;
                let size = usize::try_from(extended).map_err(|_| {
                    AvError::invalid_data(format!("{context} extended box size is out of range"))
                })?;
                (16, size)
            }
            size => {
                let size = usize::try_from(size).map_err(|_| {
                    AvError::invalid_data(format!("{context} box size is out of range"))
                })?;
                (8, size)
            }
        };
        if box_size < header_size {
            return Err(AvError::invalid_data(format!(
                "{context} `{}` box size is smaller than its header",
                fourcc_to_string(box_type)
            )));
        }

        let box_end = position
            .checked_add(box_size)
            .ok_or_else(|| AvError::invalid_data(format!("{context} box size overflow")))?;
        if box_end > end {
            return Err(AvError::new(
                AvErrorKind::EndOfFile,
                format!(
                    "{context} `{}` box exceeds parent bounds",
                    fourcc_to_string(box_type)
                ),
            ));
        }

        headers.push(BoxHeader {
            box_type,
            payload_start: position + header_size,
            payload_end: box_end,
        });
        position = box_end;
    }
    Ok(headers)
}

fn read_full_box_header(reader: &mut ByteReader<'_>, context: &str) -> AvResult<(u8, [u8; 3])> {
    ensure_remaining(reader, 4, context)?;
    let version = reader.read_u8()?;
    let flags = reader.read_exact(3)?;
    Ok((version, [flags[0], flags[1], flags[2]]))
}

fn ensure_remaining(reader: &ByteReader<'_>, count: usize, context: &str) -> AvResult<()> {
    if reader.remaining() < count {
        return Err(AvError::new(
            AvErrorKind::EndOfFile,
            format!("{context} box payload is truncated"),
        ));
    }
    Ok(())
}

fn ensure_box_consumed(reader: &ByteReader<'_>, context: &str) -> AvResult<()> {
    if !reader.is_eof() {
        return Err(AvError::invalid_data(format!(
            "{context} box contains trailing bytes"
        )));
    }
    Ok(())
}

fn checked_sample_count(value: u32, context: &str) -> AvResult<usize> {
    if value == 0 {
        return Err(AvError::invalid_data(format!(
            "{context} sample count must be non-zero"
        )));
    }
    let value = usize::try_from(value)
        .map_err(|_| AvError::invalid_data(format!("{context} sample count is out of range")))?;
    if value > MAX_MOV_SAMPLE_COUNT {
        return Err(AvError::unsupported(format!(
            "{context} sample count exceeds current implementation limit"
        )));
    }
    Ok(value)
}

fn checked_table_entry_count(value: u32, context: &str) -> AvResult<usize> {
    if value == 0 {
        return Err(AvError::invalid_data(format!(
            "{context} entry count must be non-zero"
        )));
    }
    usize::try_from(value)
        .map_err(|_| AvError::invalid_data(format!("{context} entry count is out of range")))
}

fn checked_sample_size(value: u32, context: &str) -> AvResult<usize> {
    if value == 0 {
        return Err(AvError::invalid_data(format!("{context} must be non-zero")));
    }
    usize::try_from(value).map_err(|_| AvError::invalid_data(format!("{context} is out of range")))
}

fn validate_timescale(timescale: u32, name: &str) -> AvResult<()> {
    if timescale == 0 {
        return Err(AvError::invalid_data(format!("{name} must be non-zero")));
    }
    Ok(())
}

fn unknown_u32_duration(duration: u32) -> Option<u64> {
    if duration == u32::MAX {
        None
    } else {
        Some(u64::from(duration))
    }
}

fn unknown_u64_duration(duration: u64) -> Option<u64> {
    if duration == u64::MAX {
        None
    } else {
        Some(duration)
    }
}

fn fixed_16_16_to_dimension(value: u32) -> Option<u32> {
    let whole = value >> 16;
    if whole == 0 {
        None
    } else {
        Some(whole)
    }
}

fn read_fourcc(reader: &mut ByteReader<'_>) -> AvResult<[u8; 4]> {
    let bytes = reader.read_exact(4)?;
    Ok([bytes[0], bytes[1], bytes[2], bytes[3]])
}

fn fourcc_to_string(fourcc: [u8; 4]) -> String {
    String::from_utf8_lossy(&fourcc).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use avutil::ByteWriter;

    #[test]
    fn parses_ftyp_movie_and_track_metadata() {
        let bytes = mp4_v0_fixture();
        let mut demuxer = MovDemuxer::open(&bytes).unwrap();

        assert_eq!(demuxer.info().major_brand(), "isom");
        assert_eq!(demuxer.info().minor_version(), 512);
        assert_eq!(
            demuxer.info().compatible_brands(),
            &["isom".to_string(), "iso2".to_string(), "avc1".to_string()]
        );
        assert_eq!(demuxer.info().timescale(), 1_000);
        assert_eq!(demuxer.info().duration(), Some(5_000));
        assert!(demuxer.info().has_media_data());
        assert_eq!(demuxer.info().tracks().len(), 1);

        let track = &demuxer.info().tracks()[0];
        assert_eq!(track.id(), 1);
        assert_eq!(track.duration(), Some(5_000));
        assert_eq!(track.width(), Some(1_920));
        assert_eq!(track.height(), Some(1_080));
        assert_eq!(track.media_timescale(), 90_000);
        assert_eq!(track.media_duration(), Some(450_000));
        assert_eq!(track.codec_tag(), None);
        assert_eq!(track.sample_count(), 0);

        assert_eq!(
            demuxer.read_packet().unwrap_err().kind(),
            AvErrorKind::Unsupported
        );
    }

    #[test]
    fn supports_extended_size_boxes_and_version_one_headers() {
        let bytes = mp4_v1_fixture_with_extended_free_box();
        let demuxer = MovDemuxer::open(&bytes).unwrap();
        let track = &demuxer.info().tracks()[0];

        assert_eq!(demuxer.info().duration(), Some(7_000_000_000));
        assert_eq!(track.id(), 7);
        assert_eq!(track.duration(), None);
        assert_eq!(track.width(), Some(3840));
        assert_eq!(track.height(), Some(2160));
        assert_eq!(track.media_timescale(), 48_000);
        assert_eq!(track.media_duration(), Some(96_000));
    }

    #[test]
    fn accepts_size_zero_top_level_box_as_remaining_payload() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&ftyp_box());
        bytes.extend_from_slice(&box_size_zero(*MOOV_ID, &moov_v0_box_payload()));

        let demuxer = MovDemuxer::open(&bytes).unwrap();

        assert!(!demuxer.info().has_media_data());
        assert_eq!(demuxer.info().tracks()[0].id(), 1);
    }

    #[test]
    fn rejects_bad_box_bounds_and_required_metadata() {
        assert_eq!(
            MovDemuxer::open(b"not mp4").unwrap_err().kind(),
            AvErrorKind::EndOfFile
        );

        let mut oversized = box_(*FTYP_ID, &ftyp_payload());
        oversized[0..4].copy_from_slice(&999_u32.to_be_bytes());
        assert_eq!(
            MovDemuxer::open(&oversized).unwrap_err().kind(),
            AvErrorKind::EndOfFile
        );

        let mut undersized = Vec::new();
        undersized.extend_from_slice(&4_u32.to_be_bytes());
        undersized.extend_from_slice(FTYP_ID);
        assert_eq!(
            MovDemuxer::open(&undersized).unwrap_err().kind(),
            AvErrorKind::InvalidData
        );

        assert_eq!(
            MovDemuxer::open(&box_(*MOOV_ID, &moov_v0_box_payload()))
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
        assert_eq!(
            MovDemuxer::open(&ftyp_box()).unwrap_err().kind(),
            AvErrorKind::InvalidData
        );
        assert_eq!(
            MovDemuxer::open(&mp4_without_track_header())
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
    }

    #[test]
    fn rejects_invalid_full_boxes_and_ftyp_payloads() {
        let mut bad_ftyp_payload = ftyp_payload();
        bad_ftyp_payload.push(b'x');
        let bad_ftyp = box_(*FTYP_ID, &bad_ftyp_payload);
        assert_eq!(
            MovDemuxer::open(&bad_ftyp).unwrap_err().kind(),
            AvErrorKind::InvalidData
        );

        assert_eq!(
            MovDemuxer::open(&mp4_with_mvhd_timescale(0))
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
        assert_eq!(
            MovDemuxer::open(&mp4_with_track_id(0)).unwrap_err().kind(),
            AvErrorKind::InvalidData
        );
        assert_eq!(
            MovDemuxer::open(&mp4_with_mvhd_version(2))
                .unwrap_err()
                .kind(),
            AvErrorKind::Unsupported
        );
    }

    #[test]
    fn rejects_fragmented_movie_boxes_explicitly() {
        assert_eq!(
            MovDemuxer::open(&mp4_with_movie_extends())
                .unwrap_err()
                .kind(),
            AvErrorKind::Unsupported
        );
        assert_eq!(
            MovDemuxer::open(&mp4_with_movie_fragment())
                .unwrap_err()
                .kind(),
            AvErrorKind::Unsupported
        );
    }

    #[test]
    fn rejects_edit_lists_explicitly() {
        assert_eq!(
            MovDemuxer::open(&mp4_with_edit_list()).unwrap_err().kind(),
            AvErrorKind::Unsupported
        );
    }

    #[test]
    fn rejects_multiple_populated_tracks_explicitly() {
        assert_eq!(
            MovDemuxer::open(&mp4_with_two_populated_tracks())
                .unwrap_err()
                .kind(),
            AvErrorKind::Unsupported
        );
    }

    #[test]
    fn reads_packets_from_simple_sample_table() {
        let bytes = mp4_with_samples(
            false,
            &[b"abc".as_slice(), b"defg".as_slice()],
            &[1_000, 2_000],
        );
        let mut demuxer = MovDemuxer::open(&bytes).unwrap();

        let track = &demuxer.info().tracks()[0];
        assert_eq!(track.codec_tag(), Some("raw "));
        let codec_parameters = track.codec_parameters().unwrap();
        assert_eq!(codec_parameters.codec_tag(), "raw ");
        assert_eq!(codec_parameters.data_reference_index(), 1);
        assert_eq!(codec_parameters.extra_data(), b"");
        assert_eq!(track.sample_count(), 2);

        let first = demuxer.read_packet().unwrap().unwrap();
        assert_eq!(first.stream_index(), 0);
        assert_eq!(first.data(), b"abc");
        assert_eq!(first.pts(), Some(0));
        assert_eq!(first.dts(), Some(0));
        assert_eq!(first.duration(), 1_000);
        assert!(first.flags().contains(avutil::PacketFlags::KEY));
        assert_eq!(first.side_data()[0].kind(), "mov_track_id");
        assert_eq!(first.side_data()[0].data(), &1_u32.to_be_bytes());
        assert_eq!(first.side_data()[1].kind(), "mov_codec_tag");
        assert_eq!(first.side_data()[1].data(), b"raw ");

        let second = demuxer.read_packet().unwrap().unwrap();
        assert_eq!(second.data(), b"defg");
        assert_eq!(second.pts(), Some(1_000));
        assert_eq!(second.dts(), Some(1_000));
        assert_eq!(second.duration(), 2_000);
        assert!(demuxer.read_packet().unwrap().is_none());
    }

    #[test]
    fn parses_sample_description_codec_parameters() {
        let bytes = mp4_with_sample_description_entry(b"zzzz", 2, b"\x01\x64\x00\x1f");
        let demuxer = MovDemuxer::open(&bytes).unwrap();
        let track = &demuxer.info().tracks()[0];
        let codec_parameters = track.codec_parameters().unwrap();
        assert_eq!(track.codec_tag(), Some("zzzz"));
        assert_eq!(codec_parameters.codec_tag(), "zzzz");
        assert_eq!(codec_parameters.data_reference_index(), 2);
        assert_eq!(codec_parameters.extra_data(), b"\x01\x64\x00\x1f");
        assert_eq!(codec_parameters.details(), &MovSampleEntryDetails::Generic);
    }

    #[test]
    fn parses_visual_sample_entry_codec_parameters() {
        let child_box = box_(*b"avcC", b"\x01\x64\x00\x1f");
        let extra_data = visual_sample_entry_extra_data(640, 360, "Rust AVC", 24, &child_box);
        let bytes = mp4_with_sample_description_entry(b"avc1", 2, &extra_data);
        let demuxer = MovDemuxer::open(&bytes).unwrap();
        let codec_parameters = demuxer.info().tracks()[0].codec_parameters().unwrap();

        assert_eq!(codec_parameters.codec_tag(), "avc1");
        assert_eq!(codec_parameters.data_reference_index(), 2);
        assert_eq!(codec_parameters.extra_data(), extra_data.as_slice());
        let MovSampleEntryDetails::Video(video) = codec_parameters.details() else {
            panic!("expected visual sample entry details");
        };
        assert_eq!(video.width(), 640);
        assert_eq!(video.height(), 360);
        assert_eq!(video.frame_count(), 1);
        assert_eq!(video.compressor_name(), "Rust AVC");
        assert_eq!(video.depth(), 24);
        assert_eq!(video.child_boxes().len(), 1);
        assert_eq!(video.child_boxes()[0].box_type(), "avcC");
        assert_eq!(video.child_boxes()[0].payload(), b"\x01\x64\x00\x1f");
        assert_eq!(
            video.avc_decoder_configuration_record(),
            Some(b"\x01\x64\x00\x1f".as_slice())
        );
    }

    #[test]
    fn rejects_malformed_visual_sample_entry_child_box() {
        let child_box = box_with_declared_size(*AVCC_ID, 12, b"\x01");
        let extra_data = visual_sample_entry_extra_data(640, 360, "Rust AVC", 24, &child_box);
        let err = MovDemuxer::open(&mp4_with_sample_description_entry(b"avc1", 1, &extra_data))
            .unwrap_err();

        assert_eq!(err.kind(), AvErrorKind::EndOfFile);
    }

    #[test]
    fn rejects_sample_description_indexes_other_than_one() {
        assert_eq!(
            MovDemuxer::open(&mp4_with_sample_description_index(2))
                .unwrap_err()
                .kind(),
            AvErrorKind::Unsupported
        );
    }

    #[test]
    fn rejects_multiple_sample_description_entries_explicitly() {
        assert_eq!(
            MovDemuxer::open(&mp4_with_multiple_sample_description_entries())
                .unwrap_err()
                .kind(),
            AvErrorKind::Unsupported
        );
    }

    #[test]
    fn reads_packets_from_co64_offsets() {
        let bytes = mp4_with_samples(true, &[b"xy".as_slice()], &[42]);
        let mut demuxer = MovDemuxer::open(&bytes).unwrap();

        let packet = demuxer.read_packet().unwrap().unwrap();

        assert_eq!(demuxer.info().tracks()[0].sample_count(), 1);
        assert_eq!(packet.data(), b"xy");
        assert_eq!(packet.duration(), 42);
        assert!(demuxer.read_packet().unwrap().is_none());
    }

    #[test]
    fn reads_packets_from_multiple_chunks_and_stsc_entries() {
        let bytes = mp4_with_chunk_layout(
            false,
            &[
                b"aa".as_slice(),
                b"bbb".as_slice(),
                b"c".as_slice(),
                b"dddd".as_slice(),
                b"ee".as_slice(),
            ],
            &[10, 11, 12, 13, 14],
            &[(1, 2, 1), (3, 1, 1)],
            &[2, 2, 1],
            &[b"gap".as_slice(), b"x".as_slice()],
        );
        let mut demuxer = MovDemuxer::open(&bytes).unwrap();

        assert_eq!(demuxer.info().tracks()[0].sample_count(), 5);
        for (data, pts, duration) in [
            (b"aa".as_slice(), 0, 10),
            (b"bbb".as_slice(), 10, 11),
            (b"c".as_slice(), 21, 12),
            (b"dddd".as_slice(), 33, 13),
            (b"ee".as_slice(), 46, 14),
        ] {
            let packet = demuxer.read_packet().unwrap().unwrap();
            assert_eq!(packet.data(), data);
            assert_eq!(packet.pts(), Some(pts));
            assert_eq!(packet.dts(), Some(pts));
            assert_eq!(packet.duration(), duration);
        }
        assert!(demuxer.read_packet().unwrap().is_none());
    }

    #[test]
    fn reads_packets_from_multiple_mdat_ranges() {
        let bytes = mp4_with_multiple_mdat_ranges();
        let mut demuxer = MovDemuxer::open(&bytes).unwrap();

        assert!(demuxer.info().has_media_data());
        assert_eq!(demuxer.info().tracks()[0].sample_count(), 2);

        let first = demuxer.read_packet().unwrap().unwrap();
        let second = demuxer.read_packet().unwrap().unwrap();

        assert_eq!(first.data(), b"aa");
        assert_eq!(first.pts(), Some(0));
        assert_eq!(first.duration(), 10);
        assert_eq!(second.data(), b"bbb");
        assert_eq!(second.pts(), Some(10));
        assert_eq!(second.duration(), 11);
        assert!(demuxer.read_packet().unwrap().is_none());
    }

    #[test]
    fn reads_sync_sample_table_for_key_flags() {
        let bytes = mp4_with_samples_and_sync(
            false,
            &[b"aa".as_slice(), b"bb".as_slice(), b"cc".as_slice()],
            &[10, 20, 30],
            &[1, 3],
        );
        let mut demuxer = MovDemuxer::open(&bytes).unwrap();

        let first = demuxer.read_packet().unwrap().unwrap();
        let second = demuxer.read_packet().unwrap().unwrap();
        let third = demuxer.read_packet().unwrap().unwrap();

        assert!(first.flags().contains(avutil::PacketFlags::KEY));
        assert!(!second.flags().contains(avutil::PacketFlags::KEY));
        assert!(third.flags().contains(avutil::PacketFlags::KEY));
        assert_eq!(third.pts(), Some(30));
        assert_eq!(third.duration(), 30);
        assert!(demuxer.read_packet().unwrap().is_none());
    }

    #[test]
    fn reads_composition_offsets_for_packet_pts() {
        let bytes = mp4_with_samples_and_ctts(
            false,
            &[b"aa".as_slice(), b"bb".as_slice(), b"cc".as_slice()],
            &[10, 20, 30],
            0,
            &[(2, 5), (1, 7)],
        );
        let mut demuxer = MovDemuxer::open(&bytes).unwrap();

        let first = demuxer.read_packet().unwrap().unwrap();
        let second = demuxer.read_packet().unwrap().unwrap();
        let third = demuxer.read_packet().unwrap().unwrap();

        assert_eq!(first.dts(), Some(0));
        assert_eq!(first.pts(), Some(5));
        assert_eq!(second.dts(), Some(10));
        assert_eq!(second.pts(), Some(15));
        assert_eq!(third.dts(), Some(30));
        assert_eq!(third.pts(), Some(37));
        assert_eq!(third.duration(), 30);
        assert!(demuxer.read_packet().unwrap().is_none());
    }

    #[test]
    fn reads_signed_composition_offsets_for_packet_pts() {
        let bytes = mp4_with_samples_and_ctts(
            false,
            &[b"aa".as_slice(), b"bb".as_slice()],
            &[10, 10],
            1,
            &[(1, -2), (1, 0)],
        );
        let mut demuxer = MovDemuxer::open(&bytes).unwrap();

        let first = demuxer.read_packet().unwrap().unwrap();
        let second = demuxer.read_packet().unwrap().unwrap();

        assert_eq!(first.dts(), Some(0));
        assert_eq!(first.pts(), Some(-2));
        assert_eq!(second.dts(), Some(10));
        assert_eq!(second.pts(), Some(10));
        assert!(demuxer.read_packet().unwrap().is_none());
    }

    #[test]
    fn rejects_invalid_sample_tables_and_truncated_mdat_payloads() {
        assert_eq!(
            MovDemuxer::open(&mp4_with_mismatched_sample_counts())
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
        assert_eq!(
            MovDemuxer::open(&mp4_with_sample_description_index(2))
                .unwrap_err()
                .kind(),
            AvErrorKind::Unsupported
        );
        assert_eq!(
            MovDemuxer::open(&mp4_with_short_mdat_payload())
                .unwrap_err()
                .kind(),
            AvErrorKind::EndOfFile
        );
        assert_eq!(
            MovDemuxer::open(&mp4_with_sync_samples(&[3]))
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
        assert_eq!(
            MovDemuxer::open(&mp4_with_sync_samples(&[2, 2]))
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
        assert_eq!(
            MovDemuxer::open(&mp4_with_composition_offsets(0, &[(1, 0)]))
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
        assert_eq!(
            MovDemuxer::open(&mp4_with_composition_offsets(0, &[(0, 0)]))
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
        assert_eq!(
            MovDemuxer::open(&mp4_with_composition_offsets(2, &[(2, 0)]))
                .unwrap_err()
                .kind(),
            AvErrorKind::Unsupported
        );
        assert_eq!(
            MovDemuxer::open(&mp4_with_chunk_layout(
                false,
                &[b"aa".as_slice(), b"bb".as_slice()],
                &[1_000, 1_000],
                &[(1, 1, 1)],
                &[1],
                &[],
            ))
            .unwrap_err()
            .kind(),
            AvErrorKind::InvalidData
        );
    }

    fn mp4_v0_fixture() -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&ftyp_box());
        out.extend_from_slice(&box_(*MOOV_ID, &moov_v0_box_payload()));
        out.extend_from_slice(&box_(*MDAT_ID, &[]));
        out
    }

    fn mp4_v1_fixture_with_extended_free_box() -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&box_extended(*b"free", b"skip"));
        out.extend_from_slice(&ftyp_box());
        out.extend_from_slice(&box_(
            *MOOV_ID,
            &moov_v1_box_payload(7_000_000_000, 7, u64::MAX, 96_000),
        ));
        out
    }

    fn mp4_without_track_header() -> Vec<u8> {
        let mut out = Vec::new();
        let moov_payload = [
            mvhd_v0(1_000, 5_000),
            box_(*TRAK_ID, &box_(*MDIA_ID, &mdhd_v0(1, 1))),
        ]
        .concat();
        out.extend_from_slice(&ftyp_box());
        out.extend_from_slice(&box_(*MOOV_ID, &moov_payload));
        out
    }

    fn mp4_with_mvhd_timescale(timescale: u32) -> Vec<u8> {
        let mut out = Vec::new();
        let moov_payload = [
            mvhd_v0(timescale, 5_000),
            trak_v0(1, 5_000, 90_000, 450_000),
        ]
        .concat();
        out.extend_from_slice(&ftyp_box());
        out.extend_from_slice(&box_(*MOOV_ID, &moov_payload));
        out
    }

    fn mp4_with_track_id(track_id: u32) -> Vec<u8> {
        let mut out = Vec::new();
        let moov_payload = [
            mvhd_v0(1_000, 5_000),
            trak_v0(track_id, 5_000, 90_000, 450_000),
        ]
        .concat();
        out.extend_from_slice(&ftyp_box());
        out.extend_from_slice(&box_(*MOOV_ID, &moov_payload));
        out
    }

    fn mp4_with_mvhd_version(version: u8) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.push(version);
        payload.extend_from_slice(&[0, 0, 0]);
        payload.extend_from_slice(&[0; 16]);

        let mut out = Vec::new();
        let moov_payload = [box_(*MVHD_ID, &payload), trak_v0(1, 5_000, 90_000, 450_000)].concat();
        out.extend_from_slice(&ftyp_box());
        out.extend_from_slice(&box_(*MOOV_ID, &moov_payload));
        out
    }

    fn mp4_with_movie_extends() -> Vec<u8> {
        let mut out = Vec::new();
        let moov_payload = [
            mvhd_v0(1_000, 5_000),
            box_(*MVEX_ID, &[]),
            trak_v0(1, 5_000, 90_000, 450_000),
        ]
        .concat();
        out.extend_from_slice(&ftyp_box());
        out.extend_from_slice(&box_(*MOOV_ID, &moov_payload));
        out
    }

    fn mp4_with_movie_fragment() -> Vec<u8> {
        let mut out = mp4_with_samples(false, &[b"aa".as_slice()], &[10]);
        out.extend_from_slice(&box_(*MOOF_ID, &box_(*b"traf", &[])));
        out
    }

    fn mp4_with_edit_list() -> Vec<u8> {
        let mut edit_entry = Vec::new();
        edit_entry.extend_from_slice(&1_u32.to_be_bytes());
        edit_entry.extend_from_slice(&1_000_u32.to_be_bytes());
        edit_entry.extend_from_slice(&0_i32.to_be_bytes());
        edit_entry.extend_from_slice(&1_i16.to_be_bytes());
        edit_entry.extend_from_slice(&0_u16.to_be_bytes());
        let edts = box_(*EDTS_ID, &box_(*b"elst", &full_box(0, &edit_entry)));
        let mdia = box_(*MDIA_ID, &mdhd_v0(90_000, 450_000));
        let trak = box_(
            *TRAK_ID,
            &[tkhd_v0(1, 5_000, 1_920, 1_080), edts, mdia].concat(),
        );
        let moov_payload = [mvhd_v0(1_000, 5_000), trak].concat();

        let mut out = Vec::new();
        out.extend_from_slice(&ftyp_box());
        out.extend_from_slice(&box_(*MOOV_ID, &moov_payload));
        out
    }

    fn mp4_with_two_populated_tracks() -> Vec<u8> {
        let mut out = Vec::new();
        let sample_sizes = [2];
        let durations = [1_000];
        let moov_payload = [
            mvhd_v0(1_000, 1_000),
            trak_v0_with_stbl(
                1,
                1_000,
                90_000,
                1_000,
                0,
                &sample_sizes,
                &durations,
                false,
                1,
                None,
                None,
            ),
            trak_v0_with_stbl(
                2,
                1_000,
                48_000,
                1_000,
                0,
                &sample_sizes,
                &durations,
                false,
                1,
                None,
                None,
            ),
        ]
        .concat();
        out.extend_from_slice(&ftyp_box());
        out.extend_from_slice(&box_(*MOOV_ID, &moov_payload));
        out
    }

    fn mp4_with_samples(use_co64: bool, samples: &[&[u8]], durations: &[u32]) -> Vec<u8> {
        mp4_with_sample_tables(use_co64, samples, durations, samples, 1, None, None)
    }

    fn mp4_with_samples_and_sync(
        use_co64: bool,
        samples: &[&[u8]],
        durations: &[u32],
        sync_sample_numbers: &[u32],
    ) -> Vec<u8> {
        mp4_with_sample_tables(
            use_co64,
            samples,
            durations,
            samples,
            1,
            Some(sync_sample_numbers),
            None,
        )
    }

    fn mp4_with_samples_and_ctts(
        use_co64: bool,
        samples: &[&[u8]],
        durations: &[u32],
        ctts_version: u8,
        composition_entries: &[(u32, i32)],
    ) -> Vec<u8> {
        mp4_with_sample_tables(
            use_co64,
            samples,
            durations,
            samples,
            1,
            None,
            Some((ctts_version, composition_entries)),
        )
    }

    fn mp4_with_chunk_layout(
        use_co64: bool,
        samples: &[&[u8]],
        durations: &[u32],
        stsc_entries: &[(u32, u32, u32)],
        chunk_sample_counts: &[u32],
        chunk_gaps: &[&[u8]],
    ) -> Vec<u8> {
        let ftyp = ftyp_box();
        let declared_sizes = samples
            .iter()
            .map(|sample| u32::try_from(sample.len()).unwrap())
            .collect::<Vec<_>>();
        let placeholder_offsets = vec![0; chunk_sample_counts.len()];
        let placeholder_moov = box_(
            *MOOV_ID,
            &moov_v0_with_chunk_layout_payload(
                &placeholder_offsets,
                &declared_sizes,
                durations,
                use_co64,
                stsc_entries,
            ),
        );
        let mdat_start = ftyp.len() + placeholder_moov.len() + 8;
        let (chunk_offsets, mdat_payload) =
            chunked_mdat_payload(samples, chunk_sample_counts, chunk_gaps, mdat_start);
        let moov = box_(
            *MOOV_ID,
            &moov_v0_with_chunk_layout_payload(
                &chunk_offsets,
                &declared_sizes,
                durations,
                use_co64,
                stsc_entries,
            ),
        );

        let mut out = Vec::new();
        out.extend_from_slice(&ftyp);
        out.extend_from_slice(&moov);
        out.extend_from_slice(&box_(*MDAT_ID, &mdat_payload));
        out
    }

    fn mp4_with_multiple_mdat_ranges() -> Vec<u8> {
        let ftyp = ftyp_box();
        let samples = [b"aa".as_slice(), b"bbb".as_slice()];
        let durations = [10, 11];
        let declared_sizes = samples
            .iter()
            .map(|sample| u32::try_from(sample.len()).unwrap())
            .collect::<Vec<_>>();
        let stsc_entries = [(1, 1, 1)];
        let placeholder_offsets = [0, 0];
        let placeholder_moov = box_(
            *MOOV_ID,
            &moov_v0_with_chunk_layout_payload(
                &placeholder_offsets,
                &declared_sizes,
                &durations,
                false,
                &stsc_entries,
            ),
        );
        let first_mdat = box_(*MDAT_ID, samples[0]);
        let separator = box_(*b"free", b"skip");
        let first_offset = u64::try_from(ftyp.len() + placeholder_moov.len() + 8).unwrap();
        let second_offset = u64::try_from(
            ftyp.len() + placeholder_moov.len() + first_mdat.len() + separator.len() + 8,
        )
        .unwrap();
        let moov = box_(
            *MOOV_ID,
            &moov_v0_with_chunk_layout_payload(
                &[first_offset, second_offset],
                &declared_sizes,
                &durations,
                false,
                &stsc_entries,
            ),
        );
        let second_mdat = box_(*MDAT_ID, samples[1]);

        let mut out = Vec::new();
        out.extend_from_slice(&ftyp);
        out.extend_from_slice(&moov);
        out.extend_from_slice(&first_mdat);
        out.extend_from_slice(&separator);
        out.extend_from_slice(&second_mdat);
        out
    }

    fn mp4_with_mismatched_sample_counts() -> Vec<u8> {
        mp4_with_sample_tables(
            false,
            &[b"aa".as_slice(), b"bb".as_slice()],
            &[1_000],
            &[b"aa".as_slice(), b"bb".as_slice()],
            1,
            None,
            None,
        )
    }

    fn mp4_with_sample_description_index(sample_description_index: u32) -> Vec<u8> {
        mp4_with_sample_tables(
            false,
            &[b"aa".as_slice()],
            &[1_000],
            &[b"aa".as_slice()],
            sample_description_index,
            None,
            None,
        )
    }

    fn mp4_with_sample_description_entry(
        codec_tag: &[u8; 4],
        data_reference_index: u16,
        extra_data: &[u8],
    ) -> Vec<u8> {
        let ftyp = ftyp_box();
        let sample_sizes = [2];
        let durations = [1_000];
        let sample_description = stsd_box_with_entry(codec_tag, data_reference_index, extra_data);
        let placeholder_moov_payload = moov_v0_with_custom_stsd_payload(
            0,
            &sample_sizes,
            &durations,
            sample_description.clone(),
        );
        let placeholder_moov = box_(*MOOV_ID, &placeholder_moov_payload);
        let chunk_offset = u64::try_from(ftyp.len() + placeholder_moov.len() + 8).unwrap();
        let moov = moov_v0_with_custom_stsd_payload(
            chunk_offset,
            &sample_sizes,
            &durations,
            sample_description,
        );

        let mut out = Vec::new();
        out.extend_from_slice(&ftyp);
        out.extend_from_slice(&box_(*MOOV_ID, &moov));
        out.extend_from_slice(&box_(*MDAT_ID, b"aa"));
        out
    }

    fn mp4_with_multiple_sample_description_entries() -> Vec<u8> {
        let moov =
            moov_v0_with_custom_stsd_payload(0, &[2], &[1_000], stsd_box_with_multiple_entries());
        let mut out = Vec::new();
        out.extend_from_slice(&ftyp_box());
        out.extend_from_slice(&box_(*MOOV_ID, &moov));
        out
    }

    fn mp4_with_short_mdat_payload() -> Vec<u8> {
        mp4_with_sample_tables(
            false,
            &[b"abcd".as_slice()],
            &[1_000],
            &[b"abc".as_slice()],
            1,
            None,
            None,
        )
    }

    fn mp4_with_sync_samples(sync_sample_numbers: &[u32]) -> Vec<u8> {
        mp4_with_sample_tables(
            false,
            &[b"aa".as_slice(), b"bb".as_slice()],
            &[1_000, 1_000],
            &[b"aa".as_slice(), b"bb".as_slice()],
            1,
            Some(sync_sample_numbers),
            None,
        )
    }

    fn mp4_with_composition_offsets(
        ctts_version: u8,
        composition_entries: &[(u32, i32)],
    ) -> Vec<u8> {
        mp4_with_sample_tables(
            false,
            &[b"aa".as_slice(), b"bb".as_slice()],
            &[1_000, 1_000],
            &[b"aa".as_slice(), b"bb".as_slice()],
            1,
            None,
            Some((ctts_version, composition_entries)),
        )
    }

    fn mp4_with_sample_tables(
        use_co64: bool,
        declared_samples: &[&[u8]],
        durations: &[u32],
        mdat_samples: &[&[u8]],
        sample_description_index: u32,
        sync_sample_numbers: Option<&[u32]>,
        composition_offsets: Option<(u8, &[(u32, i32)])>,
    ) -> Vec<u8> {
        let ftyp = ftyp_box();
        let mdat_payload = mdat_samples.concat();
        let declared_sizes = declared_samples
            .iter()
            .map(|sample| u32::try_from(sample.len()).unwrap())
            .collect::<Vec<_>>();
        let placeholder_moov = box_(
            *MOOV_ID,
            &moov_v0_with_stbl_payload(
                0,
                &declared_sizes,
                durations,
                use_co64,
                sample_description_index,
                sync_sample_numbers,
                composition_offsets,
            ),
        );
        let chunk_offset = u64::try_from(ftyp.len() + placeholder_moov.len() + 8).unwrap();
        let moov = box_(
            *MOOV_ID,
            &moov_v0_with_stbl_payload(
                chunk_offset,
                &declared_sizes,
                durations,
                use_co64,
                sample_description_index,
                sync_sample_numbers,
                composition_offsets,
            ),
        );

        let mut out = Vec::new();
        out.extend_from_slice(&ftyp);
        out.extend_from_slice(&moov);
        out.extend_from_slice(&box_(*MDAT_ID, &mdat_payload));
        out
    }

    fn moov_v0_box_payload() -> Vec<u8> {
        [mvhd_v0(1_000, 5_000), trak_v0(1, 5_000, 90_000, 450_000)].concat()
    }

    fn moov_v0_with_stbl_payload(
        chunk_offset: u64,
        sample_sizes: &[u32],
        durations: &[u32],
        use_co64: bool,
        sample_description_index: u32,
        sync_sample_numbers: Option<&[u32]>,
        composition_offsets: Option<(u8, &[(u32, i32)])>,
    ) -> Vec<u8> {
        let media_duration = durations.iter().copied().sum::<u32>();
        [
            mvhd_v0(1_000, media_duration),
            trak_v0_with_stbl(
                1,
                media_duration,
                90_000,
                media_duration,
                chunk_offset,
                sample_sizes,
                durations,
                use_co64,
                sample_description_index,
                sync_sample_numbers,
                composition_offsets,
            ),
        ]
        .concat()
    }

    fn moov_v0_with_custom_stsd_payload(
        chunk_offset: u64,
        sample_sizes: &[u32],
        durations: &[u32],
        sample_description_box: Vec<u8>,
    ) -> Vec<u8> {
        let media_duration = durations.iter().copied().sum::<u32>();
        let stbl = stbl_box_with_stsd(
            sample_description_box,
            chunk_offset,
            sample_sizes,
            durations,
            false,
            1,
            None,
            None,
        );
        let minf = box_(*MINF_ID, &stbl);
        let mdia = box_(*MDIA_ID, &[mdhd_v0(90_000, media_duration), minf].concat());
        let trak = box_(
            *TRAK_ID,
            &[tkhd_v0(1, media_duration, 1_920, 1_080), mdia].concat(),
        );
        [mvhd_v0(1_000, media_duration), trak].concat()
    }

    fn moov_v0_with_chunk_layout_payload(
        chunk_offsets: &[u64],
        sample_sizes: &[u32],
        durations: &[u32],
        use_co64: bool,
        stsc_entries: &[(u32, u32, u32)],
    ) -> Vec<u8> {
        let media_duration = durations.iter().copied().sum::<u32>();
        [
            mvhd_v0(1_000, media_duration),
            trak_v0_with_chunk_layout(
                chunk_offsets,
                sample_sizes,
                durations,
                use_co64,
                stsc_entries,
            ),
        ]
        .concat()
    }

    fn moov_v1_box_payload(
        movie_duration: u64,
        track_id: u32,
        track_duration: u64,
        media_duration: u64,
    ) -> Vec<u8> {
        [
            mvhd_v1(1_000, movie_duration),
            trak_v1(track_id, track_duration, 48_000, media_duration),
        ]
        .concat()
    }

    fn trak_v0(track_id: u32, track_duration: u32, timescale: u32, media_duration: u32) -> Vec<u8> {
        let payload = [
            tkhd_v0(track_id, track_duration, 1_920, 1_080),
            box_(*MDIA_ID, &mdhd_v0(timescale, media_duration)),
        ]
        .concat();
        box_(*TRAK_ID, &payload)
    }

    fn trak_v0_with_chunk_layout(
        chunk_offsets: &[u64],
        sample_sizes: &[u32],
        durations: &[u32],
        use_co64: bool,
        stsc_entries: &[(u32, u32, u32)],
    ) -> Vec<u8> {
        let media_duration = durations.iter().copied().sum::<u32>();
        let stbl = stbl_box_with_chunk_layout(
            chunk_offsets,
            sample_sizes,
            durations,
            use_co64,
            stsc_entries,
        );
        let minf = box_(*MINF_ID, &stbl);
        let mdia = box_(*MDIA_ID, &[mdhd_v0(90_000, media_duration), minf].concat());
        let payload = [tkhd_v0(1, media_duration, 1_920, 1_080), mdia].concat();
        box_(*TRAK_ID, &payload)
    }

    #[allow(clippy::too_many_arguments)]
    fn trak_v0_with_stbl(
        track_id: u32,
        track_duration: u32,
        timescale: u32,
        media_duration: u32,
        chunk_offset: u64,
        sample_sizes: &[u32],
        durations: &[u32],
        use_co64: bool,
        sample_description_index: u32,
        sync_sample_numbers: Option<&[u32]>,
        composition_offsets: Option<(u8, &[(u32, i32)])>,
    ) -> Vec<u8> {
        let stbl = stbl_box(
            chunk_offset,
            sample_sizes,
            durations,
            use_co64,
            sample_description_index,
            sync_sample_numbers,
            composition_offsets,
        );
        let minf = box_(*MINF_ID, &stbl);
        let mdia = box_(
            *MDIA_ID,
            &[mdhd_v0(timescale, media_duration), minf].concat(),
        );
        let payload = [tkhd_v0(track_id, track_duration, 1_920, 1_080), mdia].concat();
        box_(*TRAK_ID, &payload)
    }

    fn trak_v1(track_id: u32, track_duration: u64, timescale: u32, media_duration: u64) -> Vec<u8> {
        let payload = [
            tkhd_v1(track_id, track_duration, 3_840, 2_160),
            box_(*MDIA_ID, &mdhd_v1(timescale, media_duration)),
        ]
        .concat();
        box_(*TRAK_ID, &payload)
    }

    fn ftyp_box() -> Vec<u8> {
        box_(*FTYP_ID, &ftyp_payload())
    }

    fn ftyp_payload() -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"isom");
        out.extend_from_slice(&512_u32.to_be_bytes());
        out.extend_from_slice(b"isom");
        out.extend_from_slice(b"iso2");
        out.extend_from_slice(b"avc1");
        out
    }

    fn stbl_box(
        chunk_offset: u64,
        sample_sizes: &[u32],
        durations: &[u32],
        use_co64: bool,
        sample_description_index: u32,
        sync_sample_numbers: Option<&[u32]>,
        composition_offsets: Option<(u8, &[(u32, i32)])>,
    ) -> Vec<u8> {
        stbl_box_with_stsd(
            stsd_box(),
            chunk_offset,
            sample_sizes,
            durations,
            use_co64,
            sample_description_index,
            sync_sample_numbers,
            composition_offsets,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn stbl_box_with_stsd(
        sample_description_box: Vec<u8>,
        chunk_offset: u64,
        sample_sizes: &[u32],
        durations: &[u32],
        use_co64: bool,
        sample_description_index: u32,
        sync_sample_numbers: Option<&[u32]>,
        composition_offsets: Option<(u8, &[(u32, i32)])>,
    ) -> Vec<u8> {
        let chunk_offset_box = if use_co64 {
            co64_box(chunk_offset)
        } else {
            stco_box(u32::try_from(chunk_offset).unwrap())
        };
        let mut boxes = vec![
            sample_description_box,
            stts_box(durations),
            stsc_box(
                u32::try_from(sample_sizes.len()).unwrap(),
                sample_description_index,
            ),
            stsz_box(sample_sizes),
        ];
        if let Some((version, composition_entries)) = composition_offsets {
            boxes.push(ctts_box(version, composition_entries));
        }
        if let Some(sync_sample_numbers) = sync_sample_numbers {
            boxes.push(stss_box(sync_sample_numbers));
        }
        boxes.push(chunk_offset_box);
        box_(*STBL_ID, &boxes.concat())
    }

    fn stbl_box_with_chunk_layout(
        chunk_offsets: &[u64],
        sample_sizes: &[u32],
        durations: &[u32],
        use_co64: bool,
        stsc_entries: &[(u32, u32, u32)],
    ) -> Vec<u8> {
        let chunk_offset_box = if use_co64 {
            co64_offsets_box(chunk_offsets)
        } else {
            stco_offsets_box(chunk_offsets)
        };
        box_(
            *STBL_ID,
            &[
                stsd_box(),
                stts_box(durations),
                stsc_entries_box(stsc_entries),
                stsz_box(sample_sizes),
                chunk_offset_box,
            ]
            .concat(),
        )
    }

    fn stsd_box() -> Vec<u8> {
        stsd_box_with_entry(b"raw ", 1, &[])
    }

    fn stsd_box_with_entry(
        codec_tag: &[u8; 4],
        data_reference_index: u16,
        extra_data: &[u8],
    ) -> Vec<u8> {
        let sample_entry = sample_entry(codec_tag, data_reference_index, extra_data);
        let mut body = Vec::new();
        body.extend_from_slice(&1_u32.to_be_bytes());
        body.extend_from_slice(&sample_entry);
        box_(*STSD_ID, &full_box(0, &body))
    }

    fn stsd_box_with_multiple_entries() -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&2_u32.to_be_bytes());
        body.extend_from_slice(&sample_entry(b"raw ", 1, &[]));
        body.extend_from_slice(&sample_entry(b"avc1", 1, b"\x01\x64"));
        box_(*STSD_ID, &full_box(0, &body))
    }

    fn sample_entry(codec_tag: &[u8; 4], data_reference_index: u16, extra_data: &[u8]) -> Vec<u8> {
        let mut sample_entry = Vec::new();
        let entry_size = u32::try_from(16 + extra_data.len()).unwrap();
        sample_entry.extend_from_slice(&entry_size.to_be_bytes());
        sample_entry.extend_from_slice(codec_tag);
        sample_entry.extend_from_slice(&[0; 6]);
        sample_entry.extend_from_slice(&data_reference_index.to_be_bytes());
        sample_entry.extend_from_slice(extra_data);
        sample_entry
    }

    fn visual_sample_entry_extra_data(
        width: u16,
        height: u16,
        compressor_name: &str,
        depth: u16,
        child_boxes: &[u8],
    ) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&[0; 16]);
        out.extend_from_slice(&width.to_be_bytes());
        out.extend_from_slice(&height.to_be_bytes());
        out.extend_from_slice(&0x0048_0000_u32.to_be_bytes());
        out.extend_from_slice(&0x0048_0000_u32.to_be_bytes());
        out.extend_from_slice(&0_u32.to_be_bytes());
        out.extend_from_slice(&1_u16.to_be_bytes());

        let name_bytes = compressor_name.as_bytes();
        let name_len = name_bytes.len().min(31);
        out.push(u8::try_from(name_len).unwrap());
        out.extend_from_slice(&name_bytes[..name_len]);
        out.resize(out.len() + (31 - name_len), 0);

        out.extend_from_slice(&depth.to_be_bytes());
        out.extend_from_slice(&u16::MAX.to_be_bytes());
        out.extend_from_slice(child_boxes);
        out
    }

    fn stts_box(durations: &[u32]) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&(u32::try_from(durations.len()).unwrap()).to_be_bytes());
        for duration in durations {
            body.extend_from_slice(&1_u32.to_be_bytes());
            body.extend_from_slice(&duration.to_be_bytes());
        }
        box_(*STTS_ID, &full_box(0, &body))
    }

    fn ctts_box(version: u8, composition_entries: &[(u32, i32)]) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&(u32::try_from(composition_entries.len()).unwrap()).to_be_bytes());
        for (sample_count, sample_offset) in composition_entries {
            body.extend_from_slice(&sample_count.to_be_bytes());
            if version == 0 {
                body.extend_from_slice(
                    &u32::try_from(*sample_offset)
                        .expect("version 0 ctts fixture offsets must be non-negative")
                        .to_be_bytes(),
                );
            } else {
                body.extend_from_slice(&sample_offset.to_be_bytes());
            }
        }
        box_(*CTTS_ID, &full_box(version, &body))
    }

    fn stsc_box(samples_per_chunk: u32, sample_description_index: u32) -> Vec<u8> {
        stsc_entries_box(&[(1, samples_per_chunk, sample_description_index)])
    }

    fn stsc_entries_box(entries: &[(u32, u32, u32)]) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&(u32::try_from(entries.len()).unwrap()).to_be_bytes());
        for (first_chunk, samples_per_chunk, sample_description_index) in entries {
            body.extend_from_slice(&first_chunk.to_be_bytes());
            body.extend_from_slice(&samples_per_chunk.to_be_bytes());
            body.extend_from_slice(&sample_description_index.to_be_bytes());
        }
        box_(*STSC_ID, &full_box(0, &body))
    }

    fn stsz_box(sample_sizes: &[u32]) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&0_u32.to_be_bytes());
        body.extend_from_slice(&(u32::try_from(sample_sizes.len()).unwrap()).to_be_bytes());
        for sample_size in sample_sizes {
            body.extend_from_slice(&sample_size.to_be_bytes());
        }
        box_(*STSZ_ID, &full_box(0, &body))
    }

    fn stss_box(sync_sample_numbers: &[u32]) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&(u32::try_from(sync_sample_numbers.len()).unwrap()).to_be_bytes());
        for sample_number in sync_sample_numbers {
            body.extend_from_slice(&sample_number.to_be_bytes());
        }
        box_(*STSS_ID, &full_box(0, &body))
    }

    fn stco_box(chunk_offset: u32) -> Vec<u8> {
        stco_offsets_box(&[u64::from(chunk_offset)])
    }

    fn stco_offsets_box(chunk_offsets: &[u64]) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&(u32::try_from(chunk_offsets.len()).unwrap()).to_be_bytes());
        for chunk_offset in chunk_offsets {
            body.extend_from_slice(&u32::try_from(*chunk_offset).unwrap().to_be_bytes());
        }
        box_(*STCO_ID, &full_box(0, &body))
    }

    fn co64_box(chunk_offset: u64) -> Vec<u8> {
        co64_offsets_box(&[chunk_offset])
    }

    fn co64_offsets_box(chunk_offsets: &[u64]) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&(u32::try_from(chunk_offsets.len()).unwrap()).to_be_bytes());
        for chunk_offset in chunk_offsets {
            body.extend_from_slice(&chunk_offset.to_be_bytes());
        }
        box_(*CO64_ID, &full_box(0, &body))
    }

    fn chunked_mdat_payload(
        samples: &[&[u8]],
        chunk_sample_counts: &[u32],
        chunk_gaps: &[&[u8]],
        mdat_start: usize,
    ) -> (Vec<u64>, Vec<u8>) {
        let mut chunk_offsets = Vec::new();
        let mut payload = Vec::new();
        let mut sample_index = 0;

        for (chunk_index, sample_count) in chunk_sample_counts.iter().copied().enumerate() {
            chunk_offsets.push(u64::try_from(mdat_start + payload.len()).unwrap());
            for _ in 0..sample_count {
                payload.extend_from_slice(samples[sample_index]);
                sample_index += 1;
            }
            if let Some(gap) = chunk_gaps.get(chunk_index) {
                payload.extend_from_slice(gap);
            }
        }

        (chunk_offsets, payload)
    }

    fn mvhd_v0(timescale: u32, duration: u32) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&0_u32.to_be_bytes());
        body.extend_from_slice(&0_u32.to_be_bytes());
        body.extend_from_slice(&timescale.to_be_bytes());
        body.extend_from_slice(&duration.to_be_bytes());
        box_(*MVHD_ID, &full_box(0, &body))
    }

    fn mvhd_v1(timescale: u32, duration: u64) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&0_u64.to_be_bytes());
        body.extend_from_slice(&0_u64.to_be_bytes());
        body.extend_from_slice(&timescale.to_be_bytes());
        body.extend_from_slice(&duration.to_be_bytes());
        box_(*MVHD_ID, &full_box(1, &body))
    }

    fn tkhd_v0(track_id: u32, duration: u32, width: u32, height: u32) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&0_u32.to_be_bytes());
        body.extend_from_slice(&0_u32.to_be_bytes());
        body.extend_from_slice(&track_id.to_be_bytes());
        body.extend_from_slice(&0_u32.to_be_bytes());
        body.extend_from_slice(&duration.to_be_bytes());
        write_tkhd_tail(&mut body, width, height);
        box_(*TKHD_ID, &full_box(0, &body))
    }

    fn tkhd_v1(track_id: u32, duration: u64, width: u32, height: u32) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&0_u64.to_be_bytes());
        body.extend_from_slice(&0_u64.to_be_bytes());
        body.extend_from_slice(&track_id.to_be_bytes());
        body.extend_from_slice(&0_u32.to_be_bytes());
        body.extend_from_slice(&duration.to_be_bytes());
        write_tkhd_tail(&mut body, width, height);
        box_(*TKHD_ID, &full_box(1, &body))
    }

    fn write_tkhd_tail(body: &mut Vec<u8>, width: u32, height: u32) {
        body.extend_from_slice(&0_u64.to_be_bytes());
        body.extend_from_slice(&0_i16.to_be_bytes());
        body.extend_from_slice(&0_i16.to_be_bytes());
        body.extend_from_slice(&0_i16.to_be_bytes());
        body.extend_from_slice(&0_u16.to_be_bytes());
        for _ in 0..9 {
            body.extend_from_slice(&0_u32.to_be_bytes());
        }
        body.extend_from_slice(&(width << 16).to_be_bytes());
        body.extend_from_slice(&(height << 16).to_be_bytes());
    }

    fn mdhd_v0(timescale: u32, duration: u32) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&0_u32.to_be_bytes());
        body.extend_from_slice(&0_u32.to_be_bytes());
        body.extend_from_slice(&timescale.to_be_bytes());
        body.extend_from_slice(&duration.to_be_bytes());
        box_(*MDHD_ID, &full_box(0, &body))
    }

    fn mdhd_v1(timescale: u32, duration: u64) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&0_u64.to_be_bytes());
        body.extend_from_slice(&0_u64.to_be_bytes());
        body.extend_from_slice(&timescale.to_be_bytes());
        body.extend_from_slice(&duration.to_be_bytes());
        box_(*MDHD_ID, &full_box(1, &body))
    }

    fn full_box(version: u8, body: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.push(version);
        out.extend_from_slice(&[0, 0, 0]);
        out.extend_from_slice(body);
        out
    }

    fn box_(kind: [u8; 4], payload: &[u8]) -> Vec<u8> {
        let size = u32::try_from(8 + payload.len()).unwrap();
        let mut out = ByteWriter::new();
        out.write_u32_be(size);
        out.write_all(&kind);
        out.write_all(payload);
        out.into_inner()
    }

    fn box_with_declared_size(kind: [u8; 4], declared_size: u32, payload: &[u8]) -> Vec<u8> {
        let mut out = ByteWriter::new();
        out.write_u32_be(declared_size);
        out.write_all(&kind);
        out.write_all(payload);
        out.into_inner()
    }

    fn box_extended(kind: [u8; 4], payload: &[u8]) -> Vec<u8> {
        let size = u64::try_from(16 + payload.len()).unwrap();
        let mut out = ByteWriter::new();
        out.write_u32_be(1);
        out.write_all(&kind);
        out.write_u64_be(size);
        out.write_all(payload);
        out.into_inner()
    }

    fn box_size_zero(kind: [u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut out = ByteWriter::new();
        out.write_u32_be(0);
        out.write_all(&kind);
        out.write_all(payload);
        out.into_inner()
    }
}
