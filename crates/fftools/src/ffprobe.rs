use avformat::{
    register_mov_probe, AviDemuxer, AviInfo, MovDemuxer, MovInfo, ProbeRegistry, ProbeRequest,
};
use std::{fmt, fs, path::Path};

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
    input_url: String,
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
    codec_tag_string: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    time_base_num: u32,
    time_base_den: u32,
    time_base: String,
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

    pub fn codec_tag_string(&self) -> Option<&str> {
        self.codec_tag_string.as_deref()
    }

    pub fn width(&self) -> Option<u32> {
        self.width
    }

    pub fn height(&self) -> Option<u32> {
        self.height
    }

    pub fn time_base(&self) -> &str {
        &self.time_base
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
    let report = probe_local_file_inner(command.input_url.as_str(), command.show_packets)?;
    Ok(render_report(&command, &report))
}

pub fn probe_local_file(path: &str) -> Result<FfprobeReport, FfprobeError> {
    probe_local_file_inner(path, false)
}

fn probe_local_file_inner(
    path: &str,
    collect_packets: bool,
) -> Result<FfprobeReport, FfprobeError> {
    if path == "-" || path.starts_with("pipe:") {
        return Err(FfprobeError::unsupported(
            "ffprobe-rs currently supports only local seekable files",
        ));
    }

    let bytes = fs::read(path)
        .map_err(|err| FfprobeError::io(format!("failed to read `{path}`: {err}")))?;

    if is_avi_input(path, &bytes) {
        let mut demuxer = AviDemuxer::open(&bytes).map_err(|err| {
            FfprobeError::invalid_data(format!("failed to parse AVI input: {err}"))
        })?;
        let mut report = report_from_avi(path, demuxer.info());
        if collect_packets {
            report.packets = collect_avi_packets(&mut demuxer, &report.streams)?;
        }
        return Ok(report);
    }

    let mut registry = ProbeRegistry::new();
    register_mov_probe(&mut registry).map_err(|err| {
        FfprobeError::invalid_data(format!("failed to register MOV probe: {err}"))
    })?;
    let matched = registry
        .probe(ProbeRequest::new(&bytes).with_extension(path))
        .ok_or_else(|| FfprobeError::unsupported("unsupported input format"))?;
    if matched.descriptor().name() != MOV_FORMAT_NAME {
        return Err(FfprobeError::unsupported(format!(
            "unsupported input format `{}`",
            matched.descriptor().name()
        )));
    }

    let mut demuxer = MovDemuxer::open(&bytes).map_err(|err| {
        FfprobeError::invalid_data(format!("failed to parse MOV/MP4 input: {err}"))
    })?;
    let mut report = report_from_mov(path, matched.score().get(), demuxer.info());
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
        .map(|(index, track)| FfprobeStreamReport {
            index,
            id: track.id(),
            codec_tag_string: track.codec_tag().map(str::to_owned),
            width: track.width(),
            height: track.height(),
            time_base_num: 1,
            time_base_den: track.media_timescale(),
            time_base: format!("1/{}", track.media_timescale()),
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
                codec_tag_string: Some(stream.handler().to_owned()),
                width: Some(stream.width()),
                height: Some(stream.height()),
                time_base_num,
                time_base_den,
                time_base: stream.time_base().to_string(),
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
            if let Some(codec_tag_string) = &stream.codec_tag_string {
                out.push_str(&format!("codec_tag_string={codec_tag_string}\n"));
            }
            if let Some(width) = stream.width {
                out.push_str(&format!("width={width}\n"));
            }
            if let Some(height) = stream.height {
                out.push_str(&format!("height={height}\n"));
            }
            out.push_str(&format!("time_base={}\n", stream.time_base));
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
        json_string("time_base", &stream.time_base),
        json_number("nb_frames", stream.nb_frames),
    ];
    if let Some(codec_tag_string) = &stream.codec_tag_string {
        fields.push(json_string("codec_tag_string", codec_tag_string));
    }
    if let Some(width) = stream.width {
        fields.push(json_number("width", width));
    }
    if let Some(height) = stream.height {
        fields.push(json_number("height", height));
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

fn is_avi_input(path: &str, bytes: &[u8]) -> bool {
    has_avi_signature(bytes) || path_has_extension(path, "avi")
}

fn has_avi_signature(bytes: &[u8]) -> bool {
    bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"AVI "
}

fn path_has_extension(path: &str, expected: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case(expected))
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
            "clip.mp4",
        ]))
        .unwrap();

        assert!(command.show_streams);
        assert!(command.show_format);
        assert!(command.show_packets);
        assert_eq!(command.writer_format, WriterFormat::Json);
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
    fn renders_default_report_sections() {
        let command =
            parse_ffprobe_args(&strings(&["-show_streams", "-show_format", "clip.mp4"])).unwrap();
        let report = sample_report();

        let rendered = render_report(&command, &report);

        assert!(rendered.contains("[STREAM]\n"));
        assert!(rendered.contains("codec_tag_string=raw \n"));
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
        let path = write_temp_mov("show-streams", &minimal_mov_file());
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
        assert!(stdout.contains("\"width\": 1920"));
        assert!(stdout.contains("\"height\": 1080"));
        assert!(!stdout.contains("\"format\""));
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
        assert!(stdout.contains("\"codec_tag_string\": \"DIB \""));
        assert!(stdout.contains("\"width\": 2"));
        assert!(stdout.contains("\"height\": 1"));
        assert!(stdout.contains("\"time_base\": \"1/25\""));
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
                codec_tag_string: Some("raw ".to_string()),
                width: Some(1920),
                height: Some(1080),
                time_base_num: 1,
                time_base_den: 90_000,
                time_base: "1/90000".to_string(),
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
        let ftyp = ftyp_box();
        let sample_sizes = samples
            .iter()
            .map(|sample| u32::try_from(sample.len()).unwrap())
            .collect::<Vec<_>>();
        let placeholder_moov = box_(*b"moov", &moov_with_samples(0, &sample_sizes, durations));
        let chunk_offset = u32::try_from(ftyp.len() + placeholder_moov.len() + 8).unwrap();
        let moov = box_(
            MOOV_ID,
            &moov_with_samples(chunk_offset, &sample_sizes, durations),
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

    fn moov_with_samples(chunk_offset: u32, sample_sizes: &[u32], durations: &[u32]) -> Vec<u8> {
        let media_duration = durations.iter().copied().sum::<u32>();
        [
            mvhd_v0(1_000, media_duration),
            trak_with_sample_table(
                1,
                media_duration,
                90_000,
                sample_sizes,
                durations,
                chunk_offset,
            ),
        ]
        .concat()
    }

    fn trak_with_sample_table(
        track_id: u32,
        media_duration: u32,
        timescale: u32,
        sample_sizes: &[u32],
        durations: &[u32],
        chunk_offset: u32,
    ) -> Vec<u8> {
        let stbl = box_(
            STBL_ID,
            &[
                stsd_box(),
                stts_box(durations),
                stsc_box(u32::try_from(sample_sizes.len()).unwrap()),
                stsz_box(sample_sizes),
                stss_box(&[1]),
                stco_box(chunk_offset),
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
            &[tkhd_v0(track_id, media_duration, 1_920, 1_080), mdia].concat(),
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
