use avutil::{AvError, AvErrorKind, AvResult, ByteReader, Packet};

const FTYP_ID: &[u8; 4] = b"ftyp";
const MOOV_ID: &[u8; 4] = b"moov";
const MVHD_ID: &[u8; 4] = b"mvhd";
const TRAK_ID: &[u8; 4] = b"trak";
const TKHD_ID: &[u8; 4] = b"tkhd";
const MDIA_ID: &[u8; 4] = b"mdia";
const MDHD_ID: &[u8; 4] = b"mdhd";
const MDAT_ID: &[u8; 4] = b"mdat";

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
pub struct MovTrackInfo {
    id: u32,
    duration: Option<u64>,
    width: Option<u32>,
    height: Option<u32>,
    media_timescale: u32,
    media_duration: Option<u64>,
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovDemuxer<'a> {
    info: MovInfo,
    _input: &'a [u8],
}

impl<'a> MovDemuxer<'a> {
    pub fn open(input: &'a [u8]) -> AvResult<Self> {
        let top_level = read_box_headers(input, 0, input.len(), "MOV/MP4 file")?;
        let mut ftyp = None;
        let mut movie_header = None;
        let mut tracks = Vec::new();
        let mut has_media_data = false;

        for header in top_level {
            let payload = &input[header.payload_start..header.payload_end];
            match &header.box_type {
                FTYP_ID => ftyp = Some(parse_ftyp(payload)?),
                MOOV_ID => {
                    let movie = parse_moov(input, &header)?;
                    movie_header = Some(movie.header);
                    tracks = movie.tracks;
                }
                MDAT_ID => has_media_data = true,
                _ => {}
            }
        }

        let ftyp = ftyp.ok_or_else(|| AvError::invalid_data("MOV/MP4 missing ftyp box"))?;
        let movie_header =
            movie_header.ok_or_else(|| AvError::invalid_data("MOV/MP4 missing moov box"))?;
        if tracks.is_empty() {
            return Err(AvError::invalid_data("MOV/MP4 missing trak boxes"));
        }

        Ok(Self {
            info: MovInfo {
                major_brand: ftyp.major_brand,
                minor_version: ftyp.minor_version,
                compatible_brands: ftyp.compatible_brands,
                timescale: movie_header.timescale,
                duration: movie_header.duration,
                tracks,
                has_media_data,
            },
            _input: input,
        })
    }

    pub fn info(&self) -> &MovInfo {
        &self.info
    }

    pub fn read_packet(&mut self) -> AvResult<Option<Packet>> {
        Err(AvError::unsupported(
            "MOV/MP4 sample table packet extraction is not implemented",
        ))
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
    tracks: Vec<MovTrackInfo>,
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

    for child in read_box_headers(input, moov.payload_start, moov.payload_end, "MOV/MP4 moov")? {
        match &child.box_type {
            MVHD_ID => header = Some(parse_mvhd(child.payload(input))?),
            TRAK_ID => tracks.push(parse_trak(input, &child)?),
            _ => {}
        }
    }

    let header = header.ok_or_else(|| AvError::invalid_data("MOV/MP4 missing mvhd box"))?;
    Ok(MovieData { header, tracks })
}

fn parse_trak(input: &[u8], trak: &BoxHeader) -> AvResult<MovTrackInfo> {
    let mut track_header = None;
    let mut media_header = None;

    for child in read_box_headers(input, trak.payload_start, trak.payload_end, "MOV/MP4 trak")? {
        match &child.box_type {
            TKHD_ID => track_header = Some(parse_tkhd(child.payload(input))?),
            MDIA_ID => media_header = Some(parse_mdia(input, &child)?),
            _ => {}
        }
    }

    let track_header =
        track_header.ok_or_else(|| AvError::invalid_data("MOV/MP4 track missing tkhd box"))?;
    let media_header =
        media_header.ok_or_else(|| AvError::invalid_data("MOV/MP4 track missing mdhd box"))?;

    Ok(MovTrackInfo {
        id: track_header.id,
        duration: track_header.duration,
        width: track_header.width,
        height: track_header.height,
        media_timescale: media_header.timescale,
        media_duration: media_header.duration,
    })
}

fn parse_mdia(input: &[u8], mdia: &BoxHeader) -> AvResult<MediaHeader> {
    let mut media_header = None;
    for child in read_box_headers(input, mdia.payload_start, mdia.payload_end, "MOV/MP4 mdia")? {
        if child.box_type == *MDHD_ID {
            media_header = Some(parse_mdhd(child.payload(input))?);
        }
    }
    media_header.ok_or_else(|| AvError::invalid_data("MOV/MP4 mdia missing mdhd box"))
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

    fn moov_v0_box_payload() -> Vec<u8> {
        [mvhd_v0(1_000, 5_000), trak_v0(1, 5_000, 90_000, 450_000)].concat()
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
