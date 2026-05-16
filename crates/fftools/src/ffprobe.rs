use avformat::mov::{MovSampleEntryDetails, MovVideoSampleEntry};
use avformat::{
    register_avi_probe, register_mov_probe, AviDemuxer, AviInfo, AviMediaType, MovDemuxer, MovInfo,
    MovTrackInfo, ProbeRegistry, ProbeRequest,
};
use std::{fmt, fs};

const MOV_FORMAT_NAME: &str = "mov,mp4,m4a,3gp,3g2,mj2";
const MOV_FORMAT_LONG_NAME: &str = "QuickTime / MOV";
const AVI_FORMAT_NAME: &str = "avi";
const AVI_FORMAT_LONG_NAME: &str = "AVI (Audio Video Interleaved)";
const AVI_PROBE_SCORE: u8 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WriterFormat {
    Default,
    Json,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FfprobeCommand {
    show_format: bool,
    show_streams: bool,
    show_packets: bool,
    writer_format: WriterFormat,
    input_format: Option<ForcedInputFormat>,
    input_url: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ForcedInputFormat {
    Avi,
    Mov,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FfprobeReport {
    filename: String,
    format_name: String,
    format_long_name: String,
    probe_score: u8,
    nb_streams: usize,
    duration_ts: Option<u64>,
    duration: Option<String>,
    time_base: String,
    tags: Vec<(String, String)>,
    streams: Vec<FfprobeStreamReport>,
    packets: Vec<FfprobePacketReport>,
}

impl FfprobeReport {
    pub fn filename(&self) -> &str {
        &self.filename
    }

    pub fn format_name(&self) -> &str {
        &self.format_name
    }

    pub fn format_long_name(&self) -> &str {
        &self.format_long_name
    }

    pub fn probe_score(&self) -> u8 {
        self.probe_score
    }

    pub fn nb_streams(&self) -> usize {
        self.nb_streams
    }

    pub fn duration_ts(&self) -> Option<u64> {
        self.duration_ts
    }

    pub fn duration(&self) -> Option<&str> {
        self.duration.as_deref()
    }

    pub fn time_base(&self) -> &str {
        &self.time_base
    }

    pub fn tags(&self) -> &[(String, String)] {
        &self.tags
    }

    pub fn streams(&self) -> &[FfprobeStreamReport] {
        &self.streams
    }

    pub fn packets(&self) -> &[FfprobePacketReport] {
        &self.packets
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FfprobeStreamReport {
    index: usize,
    id: u32,
    codec_name: Option<String>,
    codec_long_name: Option<String>,
    profile: Option<String>,
    level: Option<u32>,
    codec_type: String,
    codec_tag_string: Option<String>,
    codec_tag: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    bits_per_raw_sample: Option<u16>,
    extradata_size: Option<usize>,
    is_avc: Option<bool>,
    nal_length_size: Option<u8>,
    sample_aspect_ratio: Option<String>,
    display_aspect_ratio: Option<String>,
    color_range: Option<String>,
    color_space: Option<String>,
    color_transfer: Option<String>,
    color_primaries: Option<String>,
    time_base_num: u32,
    time_base_den: u32,
    time_base: String,
    r_frame_rate: Option<String>,
    avg_frame_rate: Option<String>,
    duration_ts: Option<u64>,
    duration: Option<String>,
    nb_frames: usize,
    tags: Vec<(String, String)>,
}

impl FfprobeStreamReport {
    pub fn index(&self) -> usize {
        self.index
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn codec_name(&self) -> Option<&str> {
        self.codec_name.as_deref()
    }

    pub fn codec_long_name(&self) -> Option<&str> {
        self.codec_long_name.as_deref()
    }

    pub fn profile(&self) -> Option<&str> {
        self.profile.as_deref()
    }

    pub fn level(&self) -> Option<u32> {
        self.level
    }

    pub fn codec_type(&self) -> &str {
        &self.codec_type
    }

    pub fn codec_tag_string(&self) -> Option<&str> {
        self.codec_tag_string.as_deref()
    }

    pub fn codec_tag(&self) -> Option<&str> {
        self.codec_tag.as_deref()
    }

    pub fn width(&self) -> Option<u32> {
        self.width
    }

    pub fn height(&self) -> Option<u32> {
        self.height
    }

    pub fn bits_per_raw_sample(&self) -> Option<u16> {
        self.bits_per_raw_sample
    }

    pub fn extradata_size(&self) -> Option<usize> {
        self.extradata_size
    }

    pub fn is_avc(&self) -> Option<bool> {
        self.is_avc
    }

    pub fn nal_length_size(&self) -> Option<u8> {
        self.nal_length_size
    }

    pub fn sample_aspect_ratio(&self) -> Option<&str> {
        self.sample_aspect_ratio.as_deref()
    }

    pub fn display_aspect_ratio(&self) -> Option<&str> {
        self.display_aspect_ratio.as_deref()
    }

    pub fn color_range(&self) -> Option<&str> {
        self.color_range.as_deref()
    }

    pub fn color_space(&self) -> Option<&str> {
        self.color_space.as_deref()
    }

    pub fn color_transfer(&self) -> Option<&str> {
        self.color_transfer.as_deref()
    }

    pub fn color_primaries(&self) -> Option<&str> {
        self.color_primaries.as_deref()
    }

    pub fn time_base(&self) -> &str {
        &self.time_base
    }

    pub fn r_frame_rate(&self) -> Option<&str> {
        self.r_frame_rate.as_deref()
    }

    pub fn avg_frame_rate(&self) -> Option<&str> {
        self.avg_frame_rate.as_deref()
    }

    pub fn duration_ts(&self) -> Option<u64> {
        self.duration_ts
    }

    pub fn duration(&self) -> Option<&str> {
        self.duration.as_deref()
    }

    pub fn nb_frames(&self) -> usize {
        self.nb_frames
    }

    pub fn tags(&self) -> &[(String, String)] {
        &self.tags
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FfprobePacketReport {
    index: usize,
    stream_index: usize,
    pts: Option<i64>,
    pts_time: Option<String>,
    dts: Option<i64>,
    dts_time: Option<String>,
    duration: i64,
    duration_time: String,
    size: usize,
    flags: String,
}

impl FfprobePacketReport {
    pub fn index(&self) -> usize {
        self.index
    }

    pub fn stream_index(&self) -> usize {
        self.stream_index
    }

    pub fn pts(&self) -> Option<i64> {
        self.pts
    }

    pub fn pts_time(&self) -> Option<&str> {
        self.pts_time.as_deref()
    }

    pub fn dts(&self) -> Option<i64> {
        self.dts
    }

    pub fn dts_time(&self) -> Option<&str> {
        self.dts_time.as_deref()
    }

    pub fn duration(&self) -> i64 {
        self.duration
    }

    pub fn duration_time(&self) -> &str {
        &self.duration_time
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn flags(&self) -> &str {
        &self.flags
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FfprobeError {
    kind: FfprobeErrorKind,
    message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FfprobeErrorKind {
    Usage,
    Io,
    Unsupported,
    InvalidData,
}

impl FfprobeError {
    fn usage(message: impl Into<String>) -> Self {
        Self::new(FfprobeErrorKind::Usage, message)
    }

    fn io(message: impl Into<String>) -> Self {
        Self::new(FfprobeErrorKind::Io, message)
    }

    fn unsupported(message: impl Into<String>) -> Self {
        Self::new(FfprobeErrorKind::Unsupported, message)
    }

    fn invalid_data(message: impl Into<String>) -> Self {
        Self::new(FfprobeErrorKind::InvalidData, message)
    }

    fn new(kind: FfprobeErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    fn exit_code(&self) -> i32 {
        match self.kind {
            FfprobeErrorKind::Usage => 1,
            FfprobeErrorKind::Io | FfprobeErrorKind::InvalidData => 1,
            FfprobeErrorKind::Unsupported => 2,
        }
    }
}

impl fmt::Display for FfprobeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for FfprobeError {}

pub fn run_ffprobe_tool(args: &[String]) -> i32 {
    match ffprobe_output(args) {
        Ok(output) => {
            print!("{output}");
            0
        }
        Err(err) => {
            eprintln!("ffprobe: {err}");
            err.exit_code()
        }
    }
}

pub fn ffprobe_output(args: &[String]) -> Result<String, FfprobeError> {
    if args
        .iter()
        .any(|arg| arg == "-version" || arg == "--version")
    {
        return Ok(crate::version_banner("ffprobe"));
    }

    let command = parse_ffprobe_args(args)?;
    let report = probe_local_file_inner(
        command.input_url.as_str(),
        command.show_packets,
        command.input_format,
    )?;
    Ok(render_report(&command, &report))
}

pub fn probe_local_file(path: &str) -> Result<FfprobeReport, FfprobeError> {
    probe_local_file_inner(path, false, None)
}

fn probe_local_file_inner(
    path: &str,
    collect_packets: bool,
    forced_format: Option<ForcedInputFormat>,
) -> Result<FfprobeReport, FfprobeError> {
    if path == "-" || path.starts_with("pipe:") {
        return Err(FfprobeError::unsupported(
            "ffprobe-rs currently supports only local seekable files",
        ));
    }

    let bytes = fs::read(path)
        .map_err(|err| FfprobeError::io(format!("failed to read `{path}`: {err}")))?;

    if let Some(forced_format) = forced_format {
        return match forced_format {
            ForcedInputFormat::Avi => probe_avi_bytes(path, &bytes, collect_packets),
            ForcedInputFormat::Mov => probe_mov_bytes(path, &bytes, 100, collect_packets),
        };
    }

    let mut registry = ProbeRegistry::new();
    register_avi_probe(&mut registry).map_err(|err| {
        FfprobeError::invalid_data(format!("failed to register AVI probe: {err}"))
    })?;
    register_mov_probe(&mut registry).map_err(|err| {
        FfprobeError::invalid_data(format!("failed to register MOV probe: {err}"))
    })?;
    let matched = registry
        .probe(ProbeRequest::new(&bytes).with_extension(path))
        .ok_or_else(|| FfprobeError::unsupported("unsupported input format"))?;
    match matched.descriptor().name() {
        AVI_FORMAT_NAME => probe_avi_bytes(path, &bytes, collect_packets),
        MOV_FORMAT_NAME => probe_mov_bytes(path, &bytes, matched.score().get(), collect_packets),
        name => Err(FfprobeError::unsupported(format!(
            "unsupported input format `{name}`"
        ))),
    }
}

fn probe_avi_bytes(
    path: &str,
    bytes: &[u8],
    collect_packets: bool,
) -> Result<FfprobeReport, FfprobeError> {
    let mut demuxer = AviDemuxer::open(bytes)
        .map_err(|err| FfprobeError::invalid_data(format!("failed to parse AVI input: {err}")))?;
    let mut report = report_from_avi(path, demuxer.info());
    if collect_packets {
        report.packets = collect_avi_packets(&mut demuxer, &report.streams)?;
    }
    Ok(report)
}

fn probe_mov_bytes(
    path: &str,
    bytes: &[u8],
    probe_score: u8,
    collect_packets: bool,
) -> Result<FfprobeReport, FfprobeError> {
    let mut demuxer = MovDemuxer::open(bytes).map_err(|err| {
        FfprobeError::invalid_data(format!("failed to parse MOV/MP4 input: {err}"))
    })?;
    let mut report = report_from_mov(path, probe_score, demuxer.info());
    if collect_packets {
        report.packets = collect_mov_packets(&mut demuxer, &report.streams)?;
    }
    Ok(report)
}

fn parse_ffprobe_args(args: &[String]) -> Result<FfprobeCommand, FfprobeError> {
    let mut show_format = false;
    let mut show_streams = false;
    let mut show_packets = false;
    let mut writer_format = WriterFormat::Default;
    let mut input_format = None;
    let mut input_url = None;
    let mut index = 0;

    while index < args.len() {
        let arg = &args[index];
        match arg.as_str() {
            "-hide_banner" => index += 1,
            "-show_format" => {
                show_format = true;
                index += 1;
            }
            "-show_streams" => {
                show_streams = true;
                index += 1;
            }
            "-show_packets" => {
                show_packets = true;
                index += 1;
            }
            "-of" | "-print_format" => {
                let value = take_value(args, index, arg)?;
                writer_format = parse_writer_format(value)?;
                index += 2;
            }
            "-f" => {
                let value = take_value(args, index, arg)?;
                set_input_format(&mut input_format, parse_forced_input_format(value)?)?;
                index += 2;
            }
            "-v" | "-loglevel" => {
                take_value(args, index, arg)?;
                index += 2;
            }
            "-i" => {
                let value = take_value(args, index, arg)?;
                set_input_url(&mut input_url, value)?;
                index += 2;
            }
            _ if arg.starts_with('-') => {
                return Err(FfprobeError::usage(format!("unknown option `{arg}`")));
            }
            _ => {
                set_input_url(&mut input_url, arg)?;
                index += 1;
            }
        }
    }

    if !show_format && !show_streams && !show_packets {
        return Err(FfprobeError::usage("missing command"));
    }

    let input_url = input_url.ok_or_else(|| FfprobeError::usage("missing input file"))?;
    Ok(FfprobeCommand {
        show_format,
        show_streams,
        show_packets,
        writer_format,
        input_format,
        input_url,
    })
}

fn take_value<'a>(
    args: &'a [String],
    option_index: usize,
    option: &str,
) -> Result<&'a str, FfprobeError> {
    let value_index = option_index + 1;
    let value = args
        .get(value_index)
        .ok_or_else(|| FfprobeError::usage(format!("missing value for option `{option}`")))?;
    if value.starts_with('-') && value != "-" {
        return Err(FfprobeError::usage(format!(
            "missing value for option `{option}` before `{value}`"
        )));
    }
    Ok(value)
}

fn parse_writer_format(value: &str) -> Result<WriterFormat, FfprobeError> {
    let writer_name = value.split_once('=').map_or(value, |(name, _)| name);
    match writer_name {
        "default" => Ok(WriterFormat::Default),
        "json" => Ok(WriterFormat::Json),
        _ => Err(FfprobeError::unsupported(format!(
            "unsupported writer format `{value}`"
        ))),
    }
}

fn parse_forced_input_format(value: &str) -> Result<ForcedInputFormat, FfprobeError> {
    match value {
        "avi" => Ok(ForcedInputFormat::Avi),
        "mov" | "mp4" | MOV_FORMAT_NAME => Ok(ForcedInputFormat::Mov),
        _ => Err(FfprobeError::unsupported(format!(
            "unsupported input format `{value}`"
        ))),
    }
}

fn set_input_format(
    input_format: &mut Option<ForcedInputFormat>,
    value: ForcedInputFormat,
) -> Result<(), FfprobeError> {
    if input_format.is_some() {
        return Err(FfprobeError::usage(
            "ffprobe-rs currently supports one forced input format",
        ));
    }
    *input_format = Some(value);
    Ok(())
}

fn set_input_url(input_url: &mut Option<String>, value: &str) -> Result<(), FfprobeError> {
    if input_url.is_some() {
        return Err(FfprobeError::usage(
            "ffprobe-rs currently supports one input file",
        ));
    }
    *input_url = Some(value.to_owned());
    Ok(())
}

fn report_from_mov(path: &str, probe_score: u8, info: &MovInfo) -> FfprobeReport {
    let streams = info
        .tracks()
        .iter()
        .enumerate()
        .map(|(index, track)| {
            let codec_tag_string = track.codec_tag().map(str::to_owned);
            let video_sample_entry = mov_video_sample_entry(track);
            let color_information =
                video_sample_entry.and_then(MovVideoSampleEntry::color_information);
            let frame_rate = average_frame_rate(
                track.sample_count(),
                track.media_duration(),
                track.media_timescale(),
            );
            FfprobeStreamReport {
                index,
                id: track.id(),
                codec_name: codec_tag_string
                    .as_deref()
                    .and_then(codec_name_for_tag)
                    .map(str::to_owned),
                codec_long_name: codec_tag_string
                    .as_deref()
                    .and_then(codec_long_name_for_tag)
                    .map(str::to_owned),
                profile: video_sample_entry.and_then(mov_codec_profile),
                level: video_sample_entry.and_then(mov_codec_level),
                codec_type: mov_codec_type(track).to_owned(),
                codec_tag: codec_tag_string.as_deref().and_then(fourcc_codec_tag),
                codec_tag_string,
                width: track.width(),
                height: track.height(),
                bits_per_raw_sample: mov_bits_per_raw_sample(track.codec_tag(), video_sample_entry),
                extradata_size: mov_extradata_size(track),
                is_avc: mov_is_avc(video_sample_entry),
                nal_length_size: mov_nal_length_size(video_sample_entry),
                sample_aspect_ratio: video_sample_entry.and_then(mov_sample_aspect_ratio),
                display_aspect_ratio: mov_display_aspect_ratio(
                    track.width(),
                    track.height(),
                    video_sample_entry,
                ),
                color_range: color_information
                    .and_then(mov_color_range)
                    .map(str::to_owned),
                color_space: color_information
                    .and_then(avformat::MovColorInformation::matrix_coefficients)
                    .map(color_space_name),
                color_transfer: color_information
                    .and_then(avformat::MovColorInformation::transfer_characteristics)
                    .map(color_transfer_name),
                color_primaries: color_information
                    .and_then(avformat::MovColorInformation::color_primaries)
                    .map(color_primaries_name),
                time_base_num: 1,
                time_base_den: track.media_timescale(),
                time_base: format!("1/{}", track.media_timescale()),
                r_frame_rate: frame_rate.clone(),
                avg_frame_rate: frame_rate,
                duration_ts: track.media_duration(),
                duration: track
                    .media_duration()
                    .map(|duration| format_duration(duration, track.media_timescale())),
                nb_frames: track.sample_count(),
                tags: track
                    .metadata()
                    .entries()
                    .iter()
                    .map(|entry| (entry.key().to_owned(), entry.value().to_owned()))
                    .collect(),
            }
        })
        .collect::<Vec<_>>();

    FfprobeReport {
        filename: path.to_owned(),
        format_name: MOV_FORMAT_NAME.to_owned(),
        format_long_name: MOV_FORMAT_LONG_NAME.to_owned(),
        probe_score,
        nb_streams: streams.len(),
        duration_ts: info.duration(),
        duration: info
            .duration()
            .map(|duration| format_duration(duration, info.timescale())),
        time_base: format!("1/{}", info.timescale()),
        tags: info
            .metadata()
            .entries()
            .iter()
            .map(|entry| (entry.key().to_owned(), entry.value().to_owned()))
            .collect(),
        streams,
        packets: Vec::new(),
    }
}

fn report_from_avi(path: &str, info: &AviInfo) -> FfprobeReport {
    let streams = info
        .streams()
        .iter()
        .map(|stream| {
            let (time_base_num, time_base_den) = rational_parts(stream.time_base());
            FfprobeStreamReport {
                index: stream.index(),
                id: u32::try_from(stream.index()).unwrap_or(u32::MAX),
                codec_name: codec_name_for_tag(stream.handler()).map(str::to_owned),
                codec_long_name: codec_long_name_for_tag(stream.handler()).map(str::to_owned),
                profile: None,
                level: None,
                codec_type: avi_codec_type(stream.media_type()).to_owned(),
                codec_tag: fourcc_codec_tag(stream.handler()),
                codec_tag_string: Some(stream.handler().to_owned()),
                width: Some(stream.width()),
                height: Some(stream.height()),
                bits_per_raw_sample: Some(stream.bit_count()),
                extradata_size: None,
                is_avc: None,
                nal_length_size: None,
                sample_aspect_ratio: None,
                display_aspect_ratio: None,
                color_range: None,
                color_space: None,
                color_transfer: None,
                color_primaries: None,
                time_base_num,
                time_base_den,
                time_base: stream.time_base().to_string(),
                r_frame_rate: Some(stream.frame_rate().to_string()),
                avg_frame_rate: Some(stream.frame_rate().to_string()),
                duration_ts: Some(u64::from(stream.length())),
                duration: Some(format_rational_duration(
                    u64::from(stream.length()),
                    time_base_num,
                    time_base_den,
                )),
                nb_frames: stream.length() as usize,
                tags: Vec::new(),
            }
        })
        .collect::<Vec<_>>();
    let duration_stream = streams.first();

    FfprobeReport {
        filename: path.to_owned(),
        format_name: AVI_FORMAT_NAME.to_owned(),
        format_long_name: AVI_FORMAT_LONG_NAME.to_owned(),
        probe_score: AVI_PROBE_SCORE,
        nb_streams: streams.len(),
        duration_ts: Some(u64::from(info.total_frames())),
        duration: duration_stream.map(|stream| {
            format_rational_duration(
                u64::from(info.total_frames()),
                stream.time_base_num,
                stream.time_base_den,
            )
        }),
        time_base: duration_stream.map_or_else(
            || "1/1000000".to_string(),
            |stream| stream.time_base.clone(),
        ),
        tags: Vec::new(),
        streams,
        packets: Vec::new(),
    }
}

fn collect_mov_packets(
    demuxer: &mut MovDemuxer<'_>,
    streams: &[FfprobeStreamReport],
) -> Result<Vec<FfprobePacketReport>, FfprobeError> {
    let mut packets = Vec::new();
    while let Some(packet) = demuxer.read_packet().map_err(|err| {
        FfprobeError::invalid_data(format!("failed to read MOV/MP4 packet: {err}"))
    })? {
        let stream = streams
            .get(packet.stream_index())
            .ok_or_else(|| FfprobeError::invalid_data("packet references an unknown stream"))?;
        let pts = packet.pts();
        let dts = packet.dts();
        let duration = packet.duration();
        packets.push(FfprobePacketReport {
            index: packets.len(),
            stream_index: packet.stream_index(),
            pts,
            pts_time: pts.map(|pts| {
                format_rational_signed_time(pts, stream.time_base_num, stream.time_base_den)
            }),
            dts,
            dts_time: dts.map(|dts| {
                format_rational_signed_time(dts, stream.time_base_num, stream.time_base_den)
            }),
            duration,
            duration_time: format_rational_signed_time(
                duration,
                stream.time_base_num,
                stream.time_base_den,
            ),
            size: packet.data().len(),
            flags: packet_flags(packet.flags().bits()),
        });
    }
    Ok(packets)
}

fn collect_avi_packets(
    demuxer: &mut AviDemuxer,
    streams: &[FfprobeStreamReport],
) -> Result<Vec<FfprobePacketReport>, FfprobeError> {
    let mut packets = Vec::new();
    while let Some(packet) = demuxer
        .read_packet()
        .map_err(|err| FfprobeError::invalid_data(format!("failed to read AVI packet: {err}")))?
    {
        let stream = streams
            .get(packet.stream_index())
            .ok_or_else(|| FfprobeError::invalid_data("packet references an unknown stream"))?;
        let pts = packet.pts();
        let dts = packet.dts();
        let duration = packet.duration();
        packets.push(FfprobePacketReport {
            index: packets.len(),
            stream_index: packet.stream_index(),
            pts,
            pts_time: pts.map(|pts| {
                format_rational_signed_time(pts, stream.time_base_num, stream.time_base_den)
            }),
            dts,
            dts_time: dts.map(|dts| {
                format_rational_signed_time(dts, stream.time_base_num, stream.time_base_den)
            }),
            duration,
            duration_time: format_rational_signed_time(
                duration,
                stream.time_base_num,
                stream.time_base_den,
            ),
            size: packet.data().len(),
            flags: packet_flags(packet.flags().bits()),
        });
    }
    Ok(packets)
}

fn mov_video_sample_entry(track: &MovTrackInfo) -> Option<&MovVideoSampleEntry> {
    match track.codec_parameters()?.details() {
        MovSampleEntryDetails::Generic => None,
        MovSampleEntryDetails::Video(video) => Some(video.as_ref()),
    }
}

fn mov_sample_aspect_ratio(video: &MovVideoSampleEntry) -> Option<String> {
    let pixel_aspect_ratio = video.pixel_aspect_ratio()?;
    Some(format_reduced_u128_colon(
        u128::from(pixel_aspect_ratio.horizontal_spacing()),
        u128::from(pixel_aspect_ratio.vertical_spacing()),
    ))
}

fn mov_display_aspect_ratio(
    width: Option<u32>,
    height: Option<u32>,
    video: Option<&MovVideoSampleEntry>,
) -> Option<String> {
    let width = u128::from(width?);
    let height = u128::from(height?);
    if width == 0 || height == 0 {
        return None;
    }
    let pixel_aspect_ratio = video?.pixel_aspect_ratio()?;
    Some(format_reduced_u128_colon(
        width * u128::from(pixel_aspect_ratio.horizontal_spacing()),
        height * u128::from(pixel_aspect_ratio.vertical_spacing()),
    ))
}

fn mov_codec_profile(video: &MovVideoSampleEntry) -> Option<String> {
    if let Some(configuration) = video.avc_decoder_configuration() {
        return Some(avc_profile_name(configuration.profile_indication()).to_owned());
    }
    if let Some(configuration) = video.hevc_decoder_configuration() {
        return Some(hevc_profile_name(configuration.general_profile_idc()).to_owned());
    }
    None
}

fn mov_codec_level(video: &MovVideoSampleEntry) -> Option<u32> {
    if let Some(configuration) = video.avc_decoder_configuration() {
        return Some(u32::from(configuration.level_indication()));
    }
    if let Some(configuration) = video.hevc_decoder_configuration() {
        return Some(u32::from(configuration.general_level_idc()));
    }
    None
}

fn mov_bits_per_raw_sample(
    codec_tag: Option<&str>,
    video: Option<&MovVideoSampleEntry>,
) -> Option<u16> {
    if codec_tag == Some("raw ") {
        return Some(video?.depth());
    }
    None
}

fn mov_extradata_size(track: &MovTrackInfo) -> Option<usize> {
    let size = track.codec_parameters()?.extra_data().len();
    if size == 0 {
        None
    } else {
        Some(size)
    }
}

fn mov_is_avc(video: Option<&MovVideoSampleEntry>) -> Option<bool> {
    video?.avc_decoder_configuration().map(|_| true)
}

fn mov_nal_length_size(video: Option<&MovVideoSampleEntry>) -> Option<u8> {
    let video = video?;
    if let Some(configuration) = video.avc_decoder_configuration() {
        return Some(configuration.nal_length_size());
    }
    if let Some(configuration) = video.hevc_decoder_configuration() {
        return Some(configuration.nal_length_size());
    }
    None
}

fn avc_profile_name(profile_indication: u8) -> &'static str {
    match profile_indication {
        44 => "CAVLC 4:4:4",
        66 => "Baseline",
        77 => "Main",
        88 => "Extended",
        100 => "High",
        110 => "High 10",
        122 => "High 4:2:2",
        244 => "High 4:4:4 Predictive",
        _ => "unknown",
    }
}

fn hevc_profile_name(general_profile_idc: u8) -> &'static str {
    match general_profile_idc {
        1 => "Main",
        2 => "Main 10",
        3 => "Main Still Picture",
        4 => "Range Extension",
        _ => "unknown",
    }
}

fn mov_color_range(color: &avformat::MovColorInformation) -> Option<&'static str> {
    color
        .full_range()
        .map(|full_range| if full_range { "pc" } else { "tv" })
}

fn color_space_name(value: u16) -> String {
    match value {
        1 => "bt709",
        4 => "fcc",
        5 => "bt470bg",
        6 => "smpte170m",
        7 => "smpte240m",
        8 => "ycgco",
        9 => "bt2020nc",
        10 => "bt2020c",
        11 => "smpte2085",
        12 => "chroma-derived-nc",
        13 => "chroma-derived-c",
        14 => "ictcp",
        _ => "unknown",
    }
    .to_owned()
}

fn color_transfer_name(value: u16) -> String {
    match value {
        1 => "bt709",
        4 => "gamma22",
        5 => "gamma28",
        6 => "smpte170m",
        7 => "smpte240m",
        8 => "linear",
        9 => "log",
        10 => "log_sqrt",
        11 => "iec61966-2-4",
        12 => "bt1361e",
        13 => "iec61966-2-1",
        14 => "bt2020-10",
        15 => "bt2020-12",
        16 => "smpte2084",
        17 => "smpte428",
        18 => "arib-std-b67",
        _ => "unknown",
    }
    .to_owned()
}

fn color_primaries_name(value: u16) -> String {
    match value {
        1 => "bt709",
        4 => "bt470m",
        5 => "bt470bg",
        6 => "smpte170m",
        7 => "smpte240m",
        8 => "film",
        9 => "bt2020",
        10 => "smpte428",
        11 => "smpte431",
        12 => "smpte432",
        22 => "ebu3213",
        _ => "unknown",
    }
    .to_owned()
}

fn mov_codec_type(track: &MovTrackInfo) -> &'static str {
    if track.width().is_some() || track.height().is_some() {
        "video"
    } else {
        "unknown"
    }
}

fn avi_codec_type(media_type: AviMediaType) -> &'static str {
    match media_type {
        AviMediaType::Video => "video",
    }
}

fn codec_name_for_tag(tag: &str) -> Option<&'static str> {
    match tag {
        "DIB " | "raw " => Some("rawvideo"),
        "avc1" | "avc3" => Some("h264"),
        "hvc1" | "hev1" => Some("hevc"),
        "mp4v" => Some("mpeg4"),
        _ => None,
    }
}

fn codec_long_name_for_tag(tag: &str) -> Option<&'static str> {
    match tag {
        "DIB " | "raw " => Some("raw video"),
        "avc1" | "avc3" => Some("H.264 / AVC / MPEG-4 AVC / MPEG-4 part 10"),
        "hvc1" | "hev1" => Some("H.265 / HEVC (High Efficiency Video Coding)"),
        "mp4v" => Some("MPEG-4 part 2"),
        _ => None,
    }
}

fn fourcc_codec_tag(tag: &str) -> Option<String> {
    let bytes: [u8; 4] = tag.as_bytes().try_into().ok()?;
    Some(format!("0x{:08x}", u32::from_le_bytes(bytes)))
}

fn average_frame_rate(
    sample_count: usize,
    duration_ts: Option<u64>,
    time_base_den: u32,
) -> Option<String> {
    let duration_ts = duration_ts?;
    if sample_count == 0 || duration_ts == 0 || time_base_den == 0 {
        return None;
    }
    let numerator = u128::try_from(sample_count).ok()? * u128::from(time_base_den);
    Some(format_reduced_u128_ratio(
        numerator,
        u128::from(duration_ts),
    ))
}

fn format_reduced_u128_ratio(numerator: u128, denominator: u128) -> String {
    let divisor = gcd_u128(numerator, denominator);
    format!("{}/{}", numerator / divisor, denominator / divisor)
}

fn format_reduced_u128_colon(numerator: u128, denominator: u128) -> String {
    let divisor = gcd_u128(numerator, denominator);
    format!("{}:{}", numerator / divisor, denominator / divisor)
}

fn gcd_u128(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left.max(1)
}

fn packet_flags(bits: u32) -> String {
    if bits & 1 != 0 {
        "K_".to_string()
    } else {
        "__".to_string()
    }
}

fn render_report(command: &FfprobeCommand, report: &FfprobeReport) -> String {
    match command.writer_format {
        WriterFormat::Default => render_default(command, report),
        WriterFormat::Json => render_json(command, report),
    }
}

fn render_default(command: &FfprobeCommand, report: &FfprobeReport) -> String {
    let mut out = String::new();
    if command.show_packets {
        for packet in &report.packets {
            out.push_str("[PACKET]\n");
            out.push_str(&format!("stream_index={}\n", packet.stream_index));
            out.push_str(&format!("pts={}\n", optional_i64(packet.pts)));
            out.push_str(&format!("pts_time={}\n", optional_str(&packet.pts_time)));
            out.push_str(&format!("dts={}\n", optional_i64(packet.dts)));
            out.push_str(&format!("dts_time={}\n", optional_str(&packet.dts_time)));
            out.push_str(&format!("duration={}\n", packet.duration));
            out.push_str(&format!("duration_time={}\n", packet.duration_time));
            out.push_str(&format!("size={}\n", packet.size));
            out.push_str(&format!("flags={}\n", packet.flags));
            out.push_str("[/PACKET]\n");
        }
    }

    if command.show_streams {
        for stream in &report.streams {
            out.push_str("[STREAM]\n");
            out.push_str(&format!("index={}\n", stream.index));
            out.push_str(&format!("id={}\n", stream.id));
            if let Some(codec_name) = &stream.codec_name {
                out.push_str(&format!("codec_name={codec_name}\n"));
            }
            if let Some(codec_long_name) = &stream.codec_long_name {
                out.push_str(&format!("codec_long_name={codec_long_name}\n"));
            }
            if let Some(profile) = &stream.profile {
                out.push_str(&format!("profile={profile}\n"));
            }
            out.push_str(&format!("codec_type={}\n", stream.codec_type));
            if let Some(codec_tag_string) = &stream.codec_tag_string {
                out.push_str(&format!("codec_tag_string={codec_tag_string}\n"));
            }
            if let Some(codec_tag) = &stream.codec_tag {
                out.push_str(&format!("codec_tag={codec_tag}\n"));
            }
            if let Some(width) = stream.width {
                out.push_str(&format!("width={width}\n"));
            }
            if let Some(height) = stream.height {
                out.push_str(&format!("height={height}\n"));
            }
            if let Some(bits_per_raw_sample) = stream.bits_per_raw_sample {
                out.push_str(&format!("bits_per_raw_sample={bits_per_raw_sample}\n"));
            }
            if let Some(extradata_size) = stream.extradata_size {
                out.push_str(&format!("extradata_size={extradata_size}\n"));
            }
            if let Some(is_avc) = stream.is_avc {
                out.push_str(&format!("is_avc={}\n", bool_string(is_avc)));
            }
            if let Some(nal_length_size) = stream.nal_length_size {
                out.push_str(&format!("nal_length_size={nal_length_size}\n"));
            }
            if let Some(sample_aspect_ratio) = &stream.sample_aspect_ratio {
                out.push_str(&format!("sample_aspect_ratio={sample_aspect_ratio}\n"));
            }
            if let Some(display_aspect_ratio) = &stream.display_aspect_ratio {
                out.push_str(&format!("display_aspect_ratio={display_aspect_ratio}\n"));
            }
            if let Some(color_range) = &stream.color_range {
                out.push_str(&format!("color_range={color_range}\n"));
            }
            if let Some(color_space) = &stream.color_space {
                out.push_str(&format!("color_space={color_space}\n"));
            }
            if let Some(color_transfer) = &stream.color_transfer {
                out.push_str(&format!("color_transfer={color_transfer}\n"));
            }
            if let Some(color_primaries) = &stream.color_primaries {
                out.push_str(&format!("color_primaries={color_primaries}\n"));
            }
            if let Some(level) = stream.level {
                out.push_str(&format!("level={level}\n"));
            }
            out.push_str(&format!("time_base={}\n", stream.time_base));
            if let Some(r_frame_rate) = &stream.r_frame_rate {
                out.push_str(&format!("r_frame_rate={r_frame_rate}\n"));
            }
            if let Some(avg_frame_rate) = &stream.avg_frame_rate {
                out.push_str(&format!("avg_frame_rate={avg_frame_rate}\n"));
            }
            if let Some(duration_ts) = stream.duration_ts {
                out.push_str(&format!("duration_ts={duration_ts}\n"));
            }
            if let Some(duration) = &stream.duration {
                out.push_str(&format!("duration={duration}\n"));
            }
            out.push_str(&format!("nb_frames={}\n", stream.nb_frames));
            for (key, value) in &stream.tags {
                out.push_str(&format!("TAG:{key}={value}\n"));
            }
            out.push_str("[/STREAM]\n");
        }
    }

    if command.show_format {
        out.push_str("[FORMAT]\n");
        out.push_str(&format!("filename={}\n", report.filename));
        out.push_str(&format!("nb_streams={}\n", report.nb_streams));
        out.push_str(&format!("format_name={}\n", report.format_name));
        out.push_str(&format!("format_long_name={}\n", report.format_long_name));
        out.push_str(&format!("time_base={}\n", report.time_base));
        if let Some(duration_ts) = report.duration_ts {
            out.push_str(&format!("duration_ts={duration_ts}\n"));
        }
        if let Some(duration) = &report.duration {
            out.push_str(&format!("duration={duration}\n"));
        }
        out.push_str(&format!("probe_score={}\n", report.probe_score));
        for (key, value) in &report.tags {
            out.push_str(&format!("TAG:{key}={value}\n"));
        }
        out.push_str("[/FORMAT]\n");
    }
    out
}

fn render_json(command: &FfprobeCommand, report: &FfprobeReport) -> String {
    let mut sections = Vec::new();
    if command.show_packets {
        let packets = report
            .packets
            .iter()
            .map(render_packet_json)
            .collect::<Vec<_>>()
            .join(",\n    ");
        sections.push(format!("  \"packets\": [\n    {packets}\n  ]"));
    }
    if command.show_streams {
        let streams = report
            .streams
            .iter()
            .map(render_stream_json)
            .collect::<Vec<_>>()
            .join(",\n    ");
        sections.push(format!("  \"streams\": [\n    {streams}\n  ]"));
    }
    if command.show_format {
        sections.push(format!("  \"format\": {}", render_format_json(report)));
    }
    format!("{{\n{}\n}}\n", sections.join(",\n"))
}

fn render_packet_json(packet: &FfprobePacketReport) -> String {
    let fields = vec![
        json_number("stream_index", packet.stream_index),
        json_optional_number("pts", packet.pts),
        json_optional_string("pts_time", packet.pts_time.as_deref()),
        json_optional_number("dts", packet.dts),
        json_optional_string("dts_time", packet.dts_time.as_deref()),
        json_number("duration", packet.duration),
        json_string("duration_time", &packet.duration_time),
        json_number("size", packet.size),
        json_string("flags", &packet.flags),
    ];
    format!("{{{}}}", fields.join(", "))
}

fn render_stream_json(stream: &FfprobeStreamReport) -> String {
    let mut fields = vec![
        json_number("index", stream.index),
        json_number("id", stream.id),
        json_string("codec_type", &stream.codec_type),
        json_string("time_base", &stream.time_base),
        json_number("nb_frames", stream.nb_frames),
    ];
    if let Some(codec_name) = &stream.codec_name {
        fields.push(json_string("codec_name", codec_name));
    }
    if let Some(codec_long_name) = &stream.codec_long_name {
        fields.push(json_string("codec_long_name", codec_long_name));
    }
    if let Some(profile) = &stream.profile {
        fields.push(json_string("profile", profile));
    }
    if let Some(codec_tag_string) = &stream.codec_tag_string {
        fields.push(json_string("codec_tag_string", codec_tag_string));
    }
    if let Some(codec_tag) = &stream.codec_tag {
        fields.push(json_string("codec_tag", codec_tag));
    }
    if let Some(width) = stream.width {
        fields.push(json_number("width", width));
    }
    if let Some(height) = stream.height {
        fields.push(json_number("height", height));
    }
    if let Some(bits_per_raw_sample) = stream.bits_per_raw_sample {
        fields.push(json_number("bits_per_raw_sample", bits_per_raw_sample));
    }
    if let Some(extradata_size) = stream.extradata_size {
        fields.push(json_number("extradata_size", extradata_size));
    }
    if let Some(is_avc) = stream.is_avc {
        fields.push(json_string("is_avc", bool_string(is_avc)));
    }
    if let Some(nal_length_size) = stream.nal_length_size {
        fields.push(json_string("nal_length_size", &nal_length_size.to_string()));
    }
    if let Some(sample_aspect_ratio) = &stream.sample_aspect_ratio {
        fields.push(json_string("sample_aspect_ratio", sample_aspect_ratio));
    }
    if let Some(display_aspect_ratio) = &stream.display_aspect_ratio {
        fields.push(json_string("display_aspect_ratio", display_aspect_ratio));
    }
    if let Some(color_range) = &stream.color_range {
        fields.push(json_string("color_range", color_range));
    }
    if let Some(color_space) = &stream.color_space {
        fields.push(json_string("color_space", color_space));
    }
    if let Some(color_transfer) = &stream.color_transfer {
        fields.push(json_string("color_transfer", color_transfer));
    }
    if let Some(color_primaries) = &stream.color_primaries {
        fields.push(json_string("color_primaries", color_primaries));
    }
    if let Some(level) = stream.level {
        fields.push(json_number("level", level));
    }
    if let Some(r_frame_rate) = &stream.r_frame_rate {
        fields.push(json_string("r_frame_rate", r_frame_rate));
    }
    if let Some(avg_frame_rate) = &stream.avg_frame_rate {
        fields.push(json_string("avg_frame_rate", avg_frame_rate));
    }
    if let Some(duration_ts) = stream.duration_ts {
        fields.push(json_number("duration_ts", duration_ts));
    }
    if let Some(duration) = &stream.duration {
        fields.push(json_string("duration", duration));
    }
    if !stream.tags.is_empty() {
        fields.push(json_object("tags", &stream.tags));
    }
    format!("{{{}}}", fields.join(", "))
}

fn render_format_json(report: &FfprobeReport) -> String {
    let mut fields = vec![
        json_string("filename", &report.filename),
        json_number("nb_streams", report.nb_streams),
        json_string("format_name", &report.format_name),
        json_string("format_long_name", &report.format_long_name),
        json_string("time_base", &report.time_base),
        json_number("probe_score", report.probe_score),
    ];
    if let Some(duration_ts) = report.duration_ts {
        fields.push(json_number("duration_ts", duration_ts));
    }
    if let Some(duration) = &report.duration {
        fields.push(json_string("duration", duration));
    }
    if !report.tags.is_empty() {
        fields.push(json_object("tags", &report.tags));
    }
    format!("{{{}}}", fields.join(", "))
}

fn bool_string(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}

fn json_string(key: &str, value: &str) -> String {
    format!("\"{}\": \"{}\"", escape_json(key), escape_json(value))
}

fn json_number(key: &str, value: impl fmt::Display) -> String {
    format!("\"{}\": {value}", escape_json(key))
}

fn json_optional_number(key: &str, value: Option<i64>) -> String {
    match value {
        Some(value) => json_number(key, value),
        None => json_null(key),
    }
}

fn json_optional_string(key: &str, value: Option<&str>) -> String {
    match value {
        Some(value) => json_string(key, value),
        None => json_null(key),
    }
}

fn json_null(key: &str) -> String {
    format!("\"{}\": null", escape_json(key))
}

fn json_object(key: &str, pairs: &[(String, String)]) -> String {
    let fields = pairs
        .iter()
        .map(|(key, value)| json_string(key, value))
        .collect::<Vec<_>>()
        .join(", ");
    format!("\"{}\": {{{fields}}}", escape_json(key))
}

fn escape_json(value: &str) -> String {
    let mut escaped = String::new();
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            character if character.is_control() => {
                escaped.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => escaped.push(character),
        }
    }
    escaped
}

fn rational_parts(rational: avutil::Rational) -> (u32, u32) {
    (
        positive_i32_to_u32(rational.num()),
        positive_i32_to_u32(rational.den()),
    )
}

fn positive_i32_to_u32(value: i32) -> u32 {
    u32::try_from(value).unwrap_or(1).max(1)
}

fn format_duration(duration_ts: u64, timescale: u32) -> String {
    format_rational_duration(duration_ts, 1, timescale)
}

fn format_rational_duration(duration_ts: u64, time_base_num: u32, time_base_den: u32) -> String {
    let micros = (u128::from(duration_ts) * u128::from(time_base_num) * 1_000_000)
        / u128::from(time_base_den);
    format!("{}.{:06}", micros / 1_000_000, micros % 1_000_000)
}

fn format_rational_signed_time(value: i64, time_base_num: u32, time_base_den: u32) -> String {
    let micros =
        (i128::from(value) * i128::from(time_base_num) * 1_000_000) / i128::from(time_base_den);
    let sign = if micros < 0 { "-" } else { "" };
    let abs = micros.abs();
    format!("{sign}{}.{:06}", abs / 1_000_000, abs % 1_000_000)
}

fn optional_i64(value: Option<i64>) -> String {
    value.map_or_else(|| "N/A".to_string(), |value| value.to_string())
}

fn optional_str(value: &Option<String>) -> &str {
    value.as_deref().unwrap_or("N/A")
}

#[cfg(test)]
mod tests {
    use super::*;
    use avutil::{Packet, Rational};
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    const FTYP_ID: [u8; 4] = *b"ftyp";
    const MOOV_ID: [u8; 4] = *b"moov";
    const MVHD_ID: [u8; 4] = *b"mvhd";
    const TRAK_ID: [u8; 4] = *b"trak";
    const TKHD_ID: [u8; 4] = *b"tkhd";
    const MDIA_ID: [u8; 4] = *b"mdia";
    const MDHD_ID: [u8; 4] = *b"mdhd";
    const MINF_ID: [u8; 4] = *b"minf";
    const STBL_ID: [u8; 4] = *b"stbl";
    const STSD_ID: [u8; 4] = *b"stsd";
    const STTS_ID: [u8; 4] = *b"stts";
    const STSC_ID: [u8; 4] = *b"stsc";
    const STSZ_ID: [u8; 4] = *b"stsz";
    const STSS_ID: [u8; 4] = *b"stss";
    const STCO_ID: [u8; 4] = *b"stco";
    const MDAT_ID: [u8; 4] = *b"mdat";
    const AVCC_ID: [u8; 4] = *b"avcC";
    const PASP_ID: [u8; 4] = *b"pasp";
    const COLR_ID: [u8; 4] = *b"colr";
    const NCLX_ID: [u8; 4] = *b"nclx";

    #[test]
    fn parses_ffprobe_show_options_and_input() {
        let command = parse_ffprobe_args(&strings(&[
            "-hide_banner",
            "-v",
            "error",
            "-show_streams",
            "-show_packets",
            "-show_format",
            "-of",
            "json",
            "-f",
            "avi",
            "clip.mp4",
        ]))
        .unwrap();

        assert!(command.show_streams);
        assert!(command.show_format);
        assert!(command.show_packets);
        assert_eq!(command.writer_format, WriterFormat::Json);
        assert_eq!(command.input_format, Some(ForcedInputFormat::Avi));
        assert_eq!(command.input_url, "clip.mp4");
    }

    #[test]
    fn rejects_missing_command_or_input() {
        assert!(parse_ffprobe_args(&strings(&["-hide_banner"]))
            .unwrap_err()
            .message()
            .contains("missing command"));
        assert!(parse_ffprobe_args(&strings(&["-show_format"]))
            .unwrap_err()
            .message()
            .contains("missing input"));
    }

    #[test]
    fn parses_and_rejects_forced_input_formats() {
        let command =
            parse_ffprobe_args(&strings(&["-show_format", "-f", "mp4", "clip.bin"])).unwrap();

        assert_eq!(command.input_format, Some(ForcedInputFormat::Mov));
        assert_eq!(command.input_url, "clip.bin");

        assert!(
            parse_ffprobe_args(&strings(&["-show_format", "-f", "matroska", "clip.mkv"]))
                .unwrap_err()
                .message()
                .contains("unsupported input format `matroska`")
        );
        assert!(parse_ffprobe_args(&strings(&[
            "-show_format",
            "-f",
            "avi",
            "-f",
            "mp4",
            "clip.bin"
        ]))
        .unwrap_err()
        .message()
        .contains("one forced input format"));
    }

    #[test]
    fn renders_default_report_sections() {
        let command =
            parse_ffprobe_args(&strings(&["-show_streams", "-show_format", "clip.mp4"])).unwrap();
        let report = sample_report();

        let rendered = render_report(&command, &report);

        assert!(rendered.contains("[STREAM]\n"));
        assert!(rendered.contains("codec_name=rawvideo\n"));
        assert!(rendered.contains("codec_long_name=raw video\n"));
        assert!(rendered.contains("codec_type=video\n"));
        assert!(rendered.contains("codec_tag_string=raw \n"));
        assert!(rendered.contains("codec_tag=0x20776172\n"));
        assert!(rendered.contains("bits_per_raw_sample=24\n"));
        assert!(rendered.contains("extradata_size=70\n"));
        assert!(rendered.contains("sample_aspect_ratio=1:1\n"));
        assert!(rendered.contains("display_aspect_ratio=16:9\n"));
        assert!(rendered.contains("color_range=tv\n"));
        assert!(rendered.contains("color_space=bt709\n"));
        assert!(rendered.contains("color_transfer=bt709\n"));
        assert!(rendered.contains("color_primaries=bt709\n"));
        assert!(rendered.contains("r_frame_rate=30/1\n"));
        assert!(rendered.contains("avg_frame_rate=30/1\n"));
        assert!(rendered.contains("[FORMAT]\n"));
        assert!(rendered.contains("format_name=mov,mp4,m4a,3gp,3g2,mj2\n"));
        assert!(rendered.contains("duration=5.000000\n"));
    }

    #[test]
    fn renders_default_packet_sections() {
        let command =
            parse_ffprobe_args(&strings(&["-show_packets", "-show_format", "clip.mp4"])).unwrap();
        let report = sample_report();

        let rendered = render_report(&command, &report);

        assert!(rendered.starts_with("[PACKET]\n"));
        assert!(rendered.contains("stream_index=0\n"));
        assert!(rendered.contains("pts=0\n"));
        assert!(rendered.contains("pts_time=0.000000\n"));
        assert!(rendered.contains("duration_time=0.011111\n"));
        assert!(rendered.contains("size=3\n"));
        assert!(rendered.contains("flags=K_\n"));
        assert!(rendered.contains("[FORMAT]\n"));
    }

    #[test]
    fn renders_json_report_sections() {
        let command =
            parse_ffprobe_args(&strings(&["-show_format", "-of", "json", "clip.mp4"])).unwrap();
        let report = sample_report();

        let rendered = render_report(&command, &report);

        assert!(rendered.contains("\"format\""));
        assert!(rendered.contains("\"filename\": \"clip.mp4\""));
        assert!(rendered.contains("\"title\": \"Rust \\\"MOV\\\"\""));
        assert!(!rendered.contains("\"streams\""));
        assert!(!rendered.contains("\"packets\""));
    }

    #[test]
    fn ffprobe_output_prints_version_banner() {
        let stdout = ffprobe_output(&strings(&["-version"])).unwrap();

        assert!(stdout.starts_with("ffprobe version 8.1.1-rust target FFmpeg 8.1.1"));
        assert!(stdout.contains("libavformat"));
    }

    #[test]
    fn ffprobe_output_accepts_hide_banner_with_version() {
        let stdout = ffprobe_output(&strings(&["-hide_banner", "-version"])).unwrap();

        assert!(stdout.starts_with("ffprobe version 8.1.1-rust target FFmpeg 8.1.1"));
    }

    #[test]
    fn hide_banner_alone_returns_missing_command() {
        let err = ffprobe_output(&strings(&["-hide_banner"])).unwrap_err();

        assert!(err.message().contains("missing command"));
    }

    #[test]
    fn opens_local_mov_file_for_show_format() {
        let path = write_temp_mov("show-format", &minimal_mov_file());
        let path_arg = path.to_string_lossy().into_owned();

        let stdout = ffprobe_output(&strings(&[
            "-hide_banner",
            "-show_format",
            path_arg.as_str(),
        ]))
        .expect("ffprobe command path should execute");

        let _ = fs::remove_file(&path);

        assert!(stdout.contains("[FORMAT]\n"));
        assert!(stdout.contains("format_name=mov,mp4,m4a,3gp,3g2,mj2\n"));
        assert!(stdout.contains("nb_streams=1\n"));
        assert!(stdout.contains("duration_ts=5000\n"));
        assert!(stdout.contains("duration=5.000000\n"));
        assert!(stdout.contains("probe_score=100\n"));
    }

    #[test]
    fn outputs_mov_stream_json() {
        let expected_extradata_size =
            visual_sample_entry_extra_data(1_920, 1_080, "Rust AVC", 24, &[]).len();
        let stsd = visual_stsd_box(*b"raw ", 1_920, 1_080, &[]);
        let path = write_temp_mov(
            "show-streams",
            &sampled_mov_file_with_stsd(&[b"abc".as_slice()], &[3_000], 1_920, 1_080, &stsd),
        );
        let path_arg = path.to_string_lossy().into_owned();

        let stdout = ffprobe_output(&strings(&[
            "-show_streams",
            "-of",
            "json",
            path_arg.as_str(),
        ]))
        .expect("ffprobe command path should execute");

        let _ = fs::remove_file(&path);

        assert!(stdout.contains("\"streams\""));
        assert!(stdout.contains("\"index\": 0"));
        assert!(stdout.contains("\"id\": 1"));
        assert!(stdout.contains("\"codec_name\": \"rawvideo\""));
        assert!(stdout.contains("\"codec_long_name\": \"raw video\""));
        assert!(stdout.contains("\"codec_type\": \"video\""));
        assert!(stdout.contains("\"codec_tag_string\": \"raw \""));
        assert!(stdout.contains("\"codec_tag\": \"0x20776172\""));
        assert!(stdout.contains("\"width\": 1920"));
        assert!(stdout.contains("\"height\": 1080"));
        assert!(stdout.contains("\"bits_per_raw_sample\": 24"));
        assert!(stdout.contains(&format!("\"extradata_size\": {expected_extradata_size}")));
        assert!(stdout.contains("\"r_frame_rate\": \"30/1\""));
        assert!(stdout.contains("\"avg_frame_rate\": \"30/1\""));
        assert!(!stdout.contains("\"format\""));
    }

    #[test]
    fn outputs_mov_visual_stream_aspect_and_color_json() {
        let child_boxes = [
            box_(
                AVCC_ID,
                &avcc_payload(100, 0, 31, 4, &[b"\x67".as_slice()], &[b"\x68".as_slice()]),
            ),
            pasp_box(4, 3),
            colr_nclx_box(1, 13, 6, true),
        ]
        .concat();
        let expected_extradata_size =
            visual_sample_entry_extra_data(720, 576, "Rust AVC", 24, &child_boxes).len();
        let stsd = visual_stsd_box(*b"avc1", 720, 576, &child_boxes);
        let path = write_temp_mov(
            "show-streams-visual",
            &sampled_mov_file_with_stsd(&[b"abcd".as_slice()], &[3_000], 720, 576, &stsd),
        );
        let path_arg = path.to_string_lossy().into_owned();

        let stdout = ffprobe_output(&strings(&[
            "-show_streams",
            "-of",
            "json",
            path_arg.as_str(),
        ]))
        .expect("ffprobe command path should execute");

        let _ = fs::remove_file(&path);

        assert!(stdout.contains("\"codec_name\": \"h264\""));
        assert!(
            stdout.contains("\"codec_long_name\": \"H.264 / AVC / MPEG-4 AVC / MPEG-4 part 10\"")
        );
        assert!(stdout.contains("\"profile\": \"High\""));
        assert!(stdout.contains("\"codec_tag_string\": \"avc1\""));
        assert!(stdout.contains("\"codec_tag\": \"0x31637661\""));
        assert!(stdout.contains("\"width\": 720"));
        assert!(stdout.contains("\"height\": 576"));
        assert!(stdout.contains(&format!("\"extradata_size\": {expected_extradata_size}")));
        assert!(stdout.contains("\"is_avc\": \"true\""));
        assert!(stdout.contains("\"nal_length_size\": \"4\""));
        assert!(stdout.contains("\"sample_aspect_ratio\": \"4:3\""));
        assert!(stdout.contains("\"display_aspect_ratio\": \"5:3\""));
        assert!(stdout.contains("\"color_range\": \"pc\""));
        assert!(stdout.contains("\"color_space\": \"smpte170m\""));
        assert!(stdout.contains("\"color_transfer\": \"iec61966-2-1\""));
        assert!(stdout.contains("\"color_primaries\": \"bt709\""));
        assert!(stdout.contains("\"level\": 31"));
        assert!(stdout.contains("\"r_frame_rate\": \"30/1\""));
        assert!(stdout.contains("\"avg_frame_rate\": \"30/1\""));
    }

    #[test]
    fn outputs_mov_packet_sections() {
        let path = write_temp_mov(
            "show-packets",
            &sampled_mov_file(&[b"abc".as_slice(), b"defg".as_slice()], &[1_000, 2_000]),
        );
        let path_arg = path.to_string_lossy().into_owned();

        let stdout = ffprobe_output(&strings(&["-show_packets", path_arg.as_str()]))
            .expect("ffprobe command path should execute");

        let _ = fs::remove_file(&path);

        assert!(stdout.contains("[PACKET]\n"));
        assert!(stdout.contains("stream_index=0\n"));
        assert!(stdout.contains("pts=0\n"));
        assert!(stdout.contains("dts=0\n"));
        assert!(stdout.contains("duration=1000\n"));
        assert!(stdout.contains("duration_time=0.011111\n"));
        assert!(stdout.contains("size=3\n"));
        assert!(stdout.contains("flags=K_\n"));
        assert!(stdout.contains("pts=1000\n"));
        assert!(stdout.contains("duration=2000\n"));
        assert!(stdout.contains("size=4\n"));
        assert!(stdout.contains("flags=__\n"));
    }

    #[test]
    fn outputs_mov_packet_json_without_stream_or_format_sections() {
        let path = write_temp_mov(
            "show-packets-json",
            &sampled_mov_file(&[b"abc".as_slice(), b"defg".as_slice()], &[1_000, 2_000]),
        );
        let path_arg = path.to_string_lossy().into_owned();

        let stdout = ffprobe_output(&strings(&[
            "-show_packets",
            "-of",
            "json",
            path_arg.as_str(),
        ]))
        .expect("ffprobe command path should execute");

        let _ = fs::remove_file(&path);

        assert!(stdout.contains("\"packets\""));
        assert!(stdout.contains("\"stream_index\": 0"));
        assert!(stdout.contains("\"pts\": 1000"));
        assert!(stdout.contains("\"pts_time\": \"0.011111\""));
        assert!(stdout.contains("\"duration_time\": \"0.022222\""));
        assert!(stdout.contains("\"flags\": \"__\""));
        assert!(!stdout.contains("\"streams\""));
        assert!(!stdout.contains("\"format\""));
    }

    #[test]
    fn opens_local_avi_file_for_show_format() {
        let first = [0, 1, 2, 3, 4, 5];
        let second = [6, 7, 8, 9, 10, 11];
        let path = write_temp_avi(
            "avi-show-format",
            &avi_file_bytes(2, 1, Rational::new(25, 1).unwrap(), &[&first, &second]),
        );
        let path_arg = path.to_string_lossy().into_owned();

        let stdout = ffprobe_output(&strings(&["-show_format", path_arg.as_str()]))
            .expect("ffprobe AVI command path should execute");

        let _ = fs::remove_file(&path);

        assert!(stdout.contains("[FORMAT]\n"));
        assert!(stdout.contains("format_name=avi\n"));
        assert!(stdout.contains("format_long_name=AVI (Audio Video Interleaved)\n"));
        assert!(stdout.contains("nb_streams=1\n"));
        assert!(stdout.contains("time_base=1/25\n"));
        assert!(stdout.contains("duration_ts=2\n"));
        assert!(stdout.contains("duration=0.080000\n"));
        assert!(stdout.contains("probe_score=100\n"));
    }

    #[test]
    fn forced_avi_format_opens_non_avi_extension() {
        let frame = [0, 1, 2, 3, 4, 5];
        let path = write_temp_bytes(
            "forced-avi",
            "bin",
            &avi_file_bytes(2, 1, Rational::new(30, 1).unwrap(), &[&frame]),
        );
        let path_arg = path.to_string_lossy().into_owned();

        let stdout = ffprobe_output(&strings(&["-show_format", "-f", "avi", path_arg.as_str()]))
            .expect("forced AVI ffprobe command path should execute");

        let _ = fs::remove_file(&path);

        assert!(stdout.contains("format_name=avi\n"));
        assert!(stdout.contains("duration=0.033333\n"));
    }

    #[test]
    fn forced_mov_format_opens_non_mov_extension() {
        let path = write_temp_bytes("forced-mov", "bin", &minimal_mov_file());
        let path_arg = path.to_string_lossy().into_owned();

        let stdout = ffprobe_output(&strings(&["-show_format", "-f", "mp4", path_arg.as_str()]))
            .expect("forced MOV/MP4 ffprobe command path should execute");

        let _ = fs::remove_file(&path);

        assert!(stdout.contains("format_name=mov,mp4,m4a,3gp,3g2,mj2\n"));
        assert!(stdout.contains("duration=5.000000\n"));
        assert!(stdout.contains("probe_score=100\n"));
    }

    #[test]
    fn forced_avi_format_rejects_mismatched_input() {
        let path = write_temp_bytes("forced-avi-bad", "bin", &minimal_mov_file());
        let path_arg = path.to_string_lossy().into_owned();

        let err = ffprobe_output(&strings(&["-show_format", "-f", "avi", path_arg.as_str()]))
            .expect_err("forced AVI should use AVI demuxer and reject MOV bytes");

        let _ = fs::remove_file(&path);

        assert!(err.message().contains("failed to parse AVI input"));
    }

    #[test]
    fn outputs_avi_stream_json() {
        let frame = [0, 1, 2, 3, 4, 5];
        let path = write_temp_avi(
            "avi-show-streams",
            &avi_file_bytes(2, 1, Rational::new(25, 1).unwrap(), &[&frame]),
        );
        let path_arg = path.to_string_lossy().into_owned();

        let stdout = ffprobe_output(&strings(&[
            "-show_streams",
            "-of",
            "json",
            path_arg.as_str(),
        ]))
        .expect("ffprobe AVI command path should execute");

        let _ = fs::remove_file(&path);

        assert!(stdout.contains("\"streams\""));
        assert!(stdout.contains("\"index\": 0"));
        assert!(stdout.contains("\"id\": 0"));
        assert!(stdout.contains("\"codec_name\": \"rawvideo\""));
        assert!(stdout.contains("\"codec_long_name\": \"raw video\""));
        assert!(stdout.contains("\"codec_type\": \"video\""));
        assert!(stdout.contains("\"codec_tag_string\": \"DIB \""));
        assert!(stdout.contains("\"codec_tag\": \"0x20424944\""));
        assert!(stdout.contains("\"width\": 2"));
        assert!(stdout.contains("\"height\": 1"));
        assert!(stdout.contains("\"bits_per_raw_sample\": 24"));
        assert!(stdout.contains("\"time_base\": \"1/25\""));
        assert!(stdout.contains("\"r_frame_rate\": \"25/1\""));
        assert!(stdout.contains("\"avg_frame_rate\": \"25/1\""));
        assert!(stdout.contains("\"duration_ts\": 1"));
        assert!(!stdout.contains("\"format\""));
    }

    #[test]
    fn outputs_avi_packet_sections() {
        let first = [0, 1, 2, 3, 4, 5];
        let second = [6, 7, 8, 9, 10, 11];
        let path = write_temp_avi(
            "avi-show-packets",
            &avi_file_bytes(2, 1, Rational::new(25, 1).unwrap(), &[&first, &second]),
        );
        let path_arg = path.to_string_lossy().into_owned();

        let stdout = ffprobe_output(&strings(&["-show_packets", path_arg.as_str()]))
            .expect("ffprobe AVI command path should execute");

        let _ = fs::remove_file(&path);

        assert!(stdout.contains("[PACKET]\n"));
        assert!(stdout.contains("stream_index=0\n"));
        assert!(stdout.contains("pts=0\n"));
        assert!(stdout.contains("pts_time=0.000000\n"));
        assert!(stdout.contains("dts=0\n"));
        assert!(stdout.contains("dts_time=0.000000\n"));
        assert!(stdout.contains("duration=1\n"));
        assert!(stdout.contains("duration_time=0.040000\n"));
        assert!(stdout.contains("size=6\n"));
        assert!(stdout.contains("flags=__\n"));
        assert!(stdout.contains("pts=1\n"));
        assert!(stdout.contains("dts=1\n"));
        assert!(stdout.contains("pts_time=0.040000\n"));
    }

    #[test]
    fn rejects_bad_avi_input() {
        let path = write_temp_bytes("avi-bad-input", "avi", b"not an avi");
        let path_arg = path.to_string_lossy().into_owned();

        let err = ffprobe_output(&strings(&["-show_format", path_arg.as_str()]))
            .expect_err("malformed AVI input should fail");

        let _ = fs::remove_file(&path);

        assert!(err.message().contains("failed to parse AVI input"));
    }

    #[test]
    fn rejects_unmatched_local_input_format() {
        let path = write_temp_bytes("not-mov", "bin", b"not a movie");
        let path_arg = path.to_string_lossy().into_owned();

        let err = ffprobe_output(&strings(&["-show_format", path_arg.as_str()]))
            .expect_err("unmatched input should fail");

        let _ = fs::remove_file(&path);

        assert!(err.message().contains("unsupported input format"));
    }

    fn sample_report() -> FfprobeReport {
        FfprobeReport {
            filename: "clip.mp4".to_string(),
            format_name: MOV_FORMAT_NAME.to_string(),
            format_long_name: MOV_FORMAT_LONG_NAME.to_string(),
            probe_score: 100,
            nb_streams: 1,
            duration_ts: Some(5_000),
            duration: Some("5.000000".to_string()),
            time_base: "1/1000".to_string(),
            tags: vec![("title".to_string(), "Rust \"MOV\"".to_string())],
            streams: vec![FfprobeStreamReport {
                index: 0,
                id: 1,
                codec_name: Some("rawvideo".to_string()),
                codec_long_name: Some("raw video".to_string()),
                profile: None,
                level: None,
                codec_type: "video".to_string(),
                codec_tag_string: Some("raw ".to_string()),
                codec_tag: Some("0x20776172".to_string()),
                width: Some(1920),
                height: Some(1080),
                bits_per_raw_sample: Some(24),
                extradata_size: Some(70),
                is_avc: None,
                nal_length_size: None,
                sample_aspect_ratio: Some("1:1".to_string()),
                display_aspect_ratio: Some("16:9".to_string()),
                color_range: Some("tv".to_string()),
                color_space: Some("bt709".to_string()),
                color_transfer: Some("bt709".to_string()),
                color_primaries: Some("bt709".to_string()),
                time_base_num: 1,
                time_base_den: 90_000,
                time_base: "1/90000".to_string(),
                r_frame_rate: Some("30/1".to_string()),
                avg_frame_rate: Some("30/1".to_string()),
                duration_ts: Some(450_000),
                duration: Some("5.000000".to_string()),
                nb_frames: 0,
                tags: Vec::new(),
            }],
            packets: vec![FfprobePacketReport {
                index: 0,
                stream_index: 0,
                pts: Some(0),
                pts_time: Some("0.000000".to_string()),
                dts: Some(0),
                dts_time: Some("0.000000".to_string()),
                duration: 1_000,
                duration_time: "0.011111".to_string(),
                size: 3,
                flags: "K_".to_string(),
            }],
        }
    }

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    fn write_temp_mov(label: &str, bytes: &[u8]) -> PathBuf {
        write_temp_bytes(label, "mp4", bytes)
    }

    fn write_temp_avi(label: &str, bytes: &[u8]) -> PathBuf {
        write_temp_bytes(label, "avi", bytes)
    }

    fn write_temp_bytes(label: &str, extension: &str, bytes: &[u8]) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "ffmpegrust-{}-{label}-{unique}.{extension}",
            std::process::id()
        ));
        fs::write(&path, bytes).expect("temp media file should be writable");
        path
    }

    struct MovSampleTableFixture<'a> {
        sample_sizes: &'a [u32],
        durations: &'a [u32],
        chunk_offset: u32,
        track_width: u32,
        track_height: u32,
        stsd: &'a [u8],
    }

    fn minimal_mov_file() -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&ftyp_box());
        out.extend_from_slice(&box_(
            MOOV_ID,
            &[mvhd_v0(1_000, 5_000), trak_v0(1, 5_000, 90_000, 450_000)].concat(),
        ));
        out.extend_from_slice(&box_(MDAT_ID, &[]));
        out
    }

    fn sampled_mov_file(samples: &[&[u8]], durations: &[u32]) -> Vec<u8> {
        let stsd = stsd_box();
        sampled_mov_file_with_stsd(samples, durations, 1_920, 1_080, &stsd)
    }

    fn sampled_mov_file_with_stsd(
        samples: &[&[u8]],
        durations: &[u32],
        track_width: u32,
        track_height: u32,
        stsd: &[u8],
    ) -> Vec<u8> {
        let ftyp = ftyp_box();
        let sample_sizes = samples
            .iter()
            .map(|sample| u32::try_from(sample.len()).unwrap())
            .collect::<Vec<_>>();
        let placeholder_moov = box_(
            MOOV_ID,
            &moov_with_samples_and_stsd(
                0,
                &sample_sizes,
                durations,
                track_width,
                track_height,
                stsd,
            ),
        );
        let chunk_offset = u32::try_from(ftyp.len() + placeholder_moov.len() + 8).unwrap();
        let moov = box_(
            MOOV_ID,
            &moov_with_samples_and_stsd(
                chunk_offset,
                &sample_sizes,
                durations,
                track_width,
                track_height,
                stsd,
            ),
        );
        let mut out = Vec::new();
        out.extend_from_slice(&ftyp);
        out.extend_from_slice(&moov);
        out.extend_from_slice(&box_(MDAT_ID, &samples.concat()));
        out
    }

    fn avi_file_bytes(width: u32, height: u32, frame_rate: Rational, frames: &[&[u8]]) -> Vec<u8> {
        let mut muxer = avformat::AviMuxer::new_rgb24(width, height, frame_rate).unwrap();
        for frame in frames {
            muxer
                .write_packet(&Packet::new((*frame).to_vec(), 0))
                .unwrap();
        }
        muxer.finish().unwrap()
    }

    fn moov_with_samples_and_stsd(
        chunk_offset: u32,
        sample_sizes: &[u32],
        durations: &[u32],
        track_width: u32,
        track_height: u32,
        stsd: &[u8],
    ) -> Vec<u8> {
        let media_duration = durations.iter().copied().sum::<u32>();
        let fixture = MovSampleTableFixture {
            sample_sizes,
            durations,
            chunk_offset,
            track_width,
            track_height,
            stsd,
        };
        [
            mvhd_v0(1_000, media_duration),
            trak_with_sample_table_and_stsd(1, media_duration, 90_000, &fixture),
        ]
        .concat()
    }

    fn trak_with_sample_table_and_stsd(
        track_id: u32,
        media_duration: u32,
        timescale: u32,
        fixture: &MovSampleTableFixture<'_>,
    ) -> Vec<u8> {
        let stbl = box_(
            STBL_ID,
            &[
                fixture.stsd.to_vec(),
                stts_box(fixture.durations),
                stsc_box(u32::try_from(fixture.sample_sizes.len()).unwrap()),
                stsz_box(fixture.sample_sizes),
                stss_box(&[1]),
                stco_box(fixture.chunk_offset),
            ]
            .concat(),
        );
        let minf = box_(MINF_ID, &stbl);
        let mdia = box_(
            MDIA_ID,
            &[mdhd_v0(timescale, media_duration), minf].concat(),
        );
        box_(
            TRAK_ID,
            &[
                tkhd_v0(
                    track_id,
                    media_duration,
                    fixture.track_width,
                    fixture.track_height,
                ),
                mdia,
            ]
            .concat(),
        )
    }

    fn stsd_box() -> Vec<u8> {
        let mut sample_entry = Vec::new();
        sample_entry.extend_from_slice(&16_u32.to_be_bytes());
        sample_entry.extend_from_slice(b"raw ");
        sample_entry.extend_from_slice(&[0; 6]);
        sample_entry.extend_from_slice(&1_u16.to_be_bytes());

        let mut body = Vec::new();
        body.extend_from_slice(&1_u32.to_be_bytes());
        body.extend_from_slice(&sample_entry);
        box_(STSD_ID, &full_box(0, &body))
    }

    fn visual_stsd_box(codec_tag: [u8; 4], width: u16, height: u16, child_boxes: &[u8]) -> Vec<u8> {
        let extra_data = visual_sample_entry_extra_data(width, height, "Rust AVC", 24, child_boxes);
        let mut sample_entry = Vec::new();
        sample_entry
            .extend_from_slice(&u32::try_from(16 + extra_data.len()).unwrap().to_be_bytes());
        sample_entry.extend_from_slice(&codec_tag);
        sample_entry.extend_from_slice(&[0; 6]);
        sample_entry.extend_from_slice(&1_u16.to_be_bytes());
        sample_entry.extend_from_slice(&extra_data);

        let mut body = Vec::new();
        body.extend_from_slice(&1_u32.to_be_bytes());
        body.extend_from_slice(&sample_entry);
        box_(STSD_ID, &full_box(0, &body))
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

    fn avcc_payload(
        profile_indication: u8,
        profile_compatibility: u8,
        level_indication: u8,
        nal_length_size: u8,
        sequence_parameter_sets: &[&[u8]],
        picture_parameter_sets: &[&[u8]],
    ) -> Vec<u8> {
        let mut out = vec![
            1,
            profile_indication,
            profile_compatibility,
            level_indication,
            0b1111_1100 | (nal_length_size - 1),
            0b1110_0000 | u8::try_from(sequence_parameter_sets.len()).unwrap(),
        ];
        for parameter_set in sequence_parameter_sets {
            out.extend_from_slice(&u16::try_from(parameter_set.len()).unwrap().to_be_bytes());
            out.extend_from_slice(parameter_set);
        }
        out.push(u8::try_from(picture_parameter_sets.len()).unwrap());
        for parameter_set in picture_parameter_sets {
            out.extend_from_slice(&u16::try_from(parameter_set.len()).unwrap().to_be_bytes());
            out.extend_from_slice(parameter_set);
        }
        out
    }

    fn pasp_box(horizontal_spacing: u32, vertical_spacing: u32) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&horizontal_spacing.to_be_bytes());
        body.extend_from_slice(&vertical_spacing.to_be_bytes());
        box_(PASP_ID, &body)
    }

    fn colr_nclx_box(
        color_primaries: u16,
        transfer_characteristics: u16,
        matrix_coefficients: u16,
        full_range: bool,
    ) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&NCLX_ID);
        body.extend_from_slice(&color_primaries.to_be_bytes());
        body.extend_from_slice(&transfer_characteristics.to_be_bytes());
        body.extend_from_slice(&matrix_coefficients.to_be_bytes());
        body.push(if full_range { 0x80 } else { 0 });
        box_(COLR_ID, &body)
    }

    fn stts_box(durations: &[u32]) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&u32::try_from(durations.len()).unwrap().to_be_bytes());
        for duration in durations {
            body.extend_from_slice(&1_u32.to_be_bytes());
            body.extend_from_slice(&duration.to_be_bytes());
        }
        box_(STTS_ID, &full_box(0, &body))
    }

    fn stsc_box(samples_per_chunk: u32) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&1_u32.to_be_bytes());
        body.extend_from_slice(&1_u32.to_be_bytes());
        body.extend_from_slice(&samples_per_chunk.to_be_bytes());
        body.extend_from_slice(&1_u32.to_be_bytes());
        box_(STSC_ID, &full_box(0, &body))
    }

    fn stsz_box(sample_sizes: &[u32]) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&0_u32.to_be_bytes());
        body.extend_from_slice(&u32::try_from(sample_sizes.len()).unwrap().to_be_bytes());
        for sample_size in sample_sizes {
            body.extend_from_slice(&sample_size.to_be_bytes());
        }
        box_(STSZ_ID, &full_box(0, &body))
    }

    fn stss_box(sync_sample_numbers: &[u32]) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(
            &u32::try_from(sync_sample_numbers.len())
                .unwrap()
                .to_be_bytes(),
        );
        for sample_number in sync_sample_numbers {
            body.extend_from_slice(&sample_number.to_be_bytes());
        }
        box_(STSS_ID, &full_box(0, &body))
    }

    fn stco_box(chunk_offset: u32) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&1_u32.to_be_bytes());
        body.extend_from_slice(&chunk_offset.to_be_bytes());
        box_(STCO_ID, &full_box(0, &body))
    }

    fn ftyp_box() -> Vec<u8> {
        box_(
            FTYP_ID,
            &[
                b"isom".as_slice(),
                &512_u32.to_be_bytes(),
                b"isom".as_slice(),
                b"iso2".as_slice(),
                b"avc1".as_slice(),
            ]
            .concat(),
        )
    }

    fn trak_v0(track_id: u32, track_duration: u32, timescale: u32, media_duration: u32) -> Vec<u8> {
        box_(
            TRAK_ID,
            &[
                tkhd_v0(track_id, track_duration, 1_920, 1_080),
                box_(MDIA_ID, &mdhd_v0(timescale, media_duration)),
            ]
            .concat(),
        )
    }

    fn mvhd_v0(timescale: u32, duration: u32) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&0_u32.to_be_bytes());
        body.extend_from_slice(&0_u32.to_be_bytes());
        body.extend_from_slice(&timescale.to_be_bytes());
        body.extend_from_slice(&duration.to_be_bytes());
        box_(MVHD_ID, &full_box(0, &body))
    }

    fn tkhd_v0(track_id: u32, duration: u32, width: u32, height: u32) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&0_u32.to_be_bytes());
        body.extend_from_slice(&0_u32.to_be_bytes());
        body.extend_from_slice(&track_id.to_be_bytes());
        body.extend_from_slice(&0_u32.to_be_bytes());
        body.extend_from_slice(&duration.to_be_bytes());
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
        box_(TKHD_ID, &full_box(0, &body))
    }

    fn mdhd_v0(timescale: u32, duration: u32) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&0_u32.to_be_bytes());
        body.extend_from_slice(&0_u32.to_be_bytes());
        body.extend_from_slice(&timescale.to_be_bytes());
        body.extend_from_slice(&duration.to_be_bytes());
        box_(MDHD_ID, &full_box(0, &body))
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
        let mut out = Vec::new();
        out.extend_from_slice(&size.to_be_bytes());
        out.extend_from_slice(&kind);
        out.extend_from_slice(payload);
        out
    }
}
