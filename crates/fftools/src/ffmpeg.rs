use crate::{
    build_io_plan, parse_ffmpeg_args, version_banner, CliOption, Endpoint, IoPlan, PlannedFile,
};
use avformat::mov::MovCodecParameters;
use avformat::{
    register_mov_probe, AviDemuxer, AviInfo, AviMediaType, AviMuxer, FrameCrcMuxer, FrameHashMuxer,
    HashAlgorithm, HashMuxer, Image2Demuxer, Image2Entry, Image2Muxer, Image2Pattern, MovDemuxer,
    MovInfo, MovSampleEntryDetails, MovTrackInfo, NullMuxer, PcmS16leDemuxer, PcmS16leMuxer,
    ProbeRegistry, ProbeRequest, RawVideoDemuxer, RawVideoMuxer, RawVideoPixelFormat,
    StreamHashMuxer, StreamHashStreamType, WavDemuxer, WavMuxer, Yuv4MpegDemuxer, Yuv4MpegMuxer,
};
use avutil::{Packet, Rational};
use std::{fmt, fs, io::Write, path::Path};

const MOV_FORMAT_NAME: &str = "mov,mp4,m4a,3gp,3g2,mj2";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FfmpegOutput {
    stdout: String,
    stderr: String,
    output_format: Option<String>,
    packet_count: u64,
    byte_count: u64,
}

impl FfmpegOutput {
    fn version(stdout: String) -> Self {
        Self {
            stdout,
            stderr: String::new(),
            output_format: None,
            packet_count: 0,
            byte_count: 0,
        }
    }

    fn media(
        stdout: String,
        stderr: String,
        output_format: OutputMuxer,
        packet_count: u64,
        byte_count: u64,
    ) -> Self {
        Self {
            stdout,
            stderr,
            output_format: Some(output_format.name().to_string()),
            packet_count,
            byte_count,
        }
    }

    pub fn stdout(&self) -> &str {
        &self.stdout
    }

    pub fn stderr(&self) -> &str {
        &self.stderr
    }

    pub fn output_format(&self) -> Option<&str> {
        self.output_format.as_deref()
    }

    pub fn packet_count(&self) -> u64 {
        self.packet_count
    }

    pub fn byte_count(&self) -> u64 {
        self.byte_count
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FfmpegError {
    kind: FfmpegErrorKind,
    message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FfmpegErrorKind {
    Usage,
    Io,
    Unsupported,
    InvalidData,
}

impl FfmpegError {
    fn usage(message: impl Into<String>) -> Self {
        Self::new(FfmpegErrorKind::Usage, message)
    }

    fn io(message: impl Into<String>) -> Self {
        Self::new(FfmpegErrorKind::Io, message)
    }

    fn unsupported(message: impl Into<String>) -> Self {
        Self::new(FfmpegErrorKind::Unsupported, message)
    }

    fn invalid_data(message: impl Into<String>) -> Self {
        Self::new(FfmpegErrorKind::InvalidData, message)
    }

    fn new(kind: FfmpegErrorKind, message: impl Into<String>) -> Self {
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
            FfmpegErrorKind::Usage | FfmpegErrorKind::Io | FfmpegErrorKind::InvalidData => 1,
            FfmpegErrorKind::Unsupported => 2,
        }
    }
}

impl fmt::Display for FfmpegError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for FfmpegError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputMuxer {
    Avi,
    Null,
    FrameCrc,
    FrameHash(FrameHashOutputMuxer),
    Hash(HashOutputMuxer),
    Image2,
    PcmS16le,
    RawVideo,
    StreamHash(HashAlgorithm),
    Wav,
    Yuv4MpegPipe,
}

impl OutputMuxer {
    fn name(self) -> &'static str {
        match self {
            Self::Avi => "avi",
            Self::Null => "null",
            Self::FrameCrc => "framecrc",
            Self::FrameHash(hash) => hash.name(),
            Self::Hash(hash) => hash.name(),
            Self::Image2 => "image2",
            Self::PcmS16le => "s16le",
            Self::RawVideo => "rawvideo",
            Self::StreamHash(_) => "streamhash",
            Self::Wav => "wav",
            Self::Yuv4MpegPipe => "yuv4mpegpipe",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StreamHashStreamTypeMap {
    stream_types: Vec<StreamHashStreamType>,
}

impl StreamHashStreamTypeMap {
    fn single(stream_type: StreamHashStreamType) -> Self {
        Self {
            stream_types: vec![stream_type],
        }
    }

    fn from_indexed_streams<I>(streams: I) -> Self
    where
        I: IntoIterator<Item = (usize, StreamHashStreamType)>,
    {
        let mut stream_types = Vec::new();
        for (index, stream_type) in streams {
            if stream_types.len() <= index {
                stream_types.resize(index + 1, StreamHashStreamType::Unknown);
            }
            stream_types[index] = stream_type;
        }
        Self { stream_types }
    }

    fn stream_type_for_index(
        &self,
        stream_index: usize,
    ) -> Result<StreamHashStreamType, FfmpegError> {
        self.stream_types.get(stream_index).copied().ok_or_else(|| {
            FfmpegError::invalid_data(format!(
                "missing streamhash stream metadata for stream {stream_index}"
            ))
        })
    }

    fn stream_type_for_packet(&self, packet: &Packet) -> Result<StreamHashStreamType, FfmpegError> {
        self.stream_type_for_index(packet.stream_index())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FrameHashOutputMuxer {
    FrameHash(HashAlgorithm),
    FrameMd5,
}

impl FrameHashOutputMuxer {
    fn name(self) -> &'static str {
        match self {
            Self::FrameHash(_) => "framehash",
            Self::FrameMd5 => "framemd5",
        }
    }

    fn algorithm(self) -> HashAlgorithm {
        match self {
            Self::FrameHash(algorithm) => algorithm,
            Self::FrameMd5 => HashAlgorithm::Md5,
        }
    }

    fn allows_hash_option(self) -> bool {
        matches!(self, Self::FrameHash(_))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HashOutputMuxer {
    Hash(HashAlgorithm),
    Md5,
}

impl HashOutputMuxer {
    fn name(self) -> &'static str {
        match self {
            Self::Hash(_) => "hash",
            Self::Md5 => "md5",
        }
    }

    fn algorithm(self) -> HashAlgorithm {
        match self {
            Self::Hash(algorithm) => algorithm,
            Self::Md5 => HashAlgorithm::Md5,
        }
    }

    fn allows_hash_option(self) -> bool {
        matches!(self, Self::Hash(_))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputFormat {
    Avi,
    Image2 {
        frame_rate: Rational,
        start_number: i64,
    },
    Mov,
    PcmS16le {
        sample_rate: u32,
        channels: u16,
    },
    RawVideo {
        width: usize,
        height: usize,
        pixel_format: RawVideoPixelFormat,
        frame_rate: Rational,
    },
    Wav,
    Yuv4MpegPipe,
}

impl InputFormat {
    fn name(self) -> &'static str {
        match self {
            Self::Avi => "AVI",
            Self::Image2 { .. } => "image2",
            Self::Mov => "MOV/MP4",
            Self::PcmS16le { .. } => "pcm_s16le",
            Self::RawVideo { .. } => "rawvideo",
            Self::Wav => "WAV",
            Self::Yuv4MpegPipe => "yuv4mpegpipe",
        }
    }
}

pub fn run_ffmpeg_tool(args: &[String]) -> i32 {
    match ffmpeg_output(args) {
        Ok(output) => {
            print!("{}", output.stdout());
            eprint!("{}", output.stderr());
            0
        }
        Err(err) => {
            eprint!(
                "{}",
                crate::cli_logging::tool_error_stderr("ffmpeg", args, &err)
            );
            err.exit_code()
        }
    }
}

pub fn ffmpeg_output(args: &[String]) -> Result<FfmpegOutput, FfmpegError> {
    if args
        .iter()
        .any(|arg| arg == "-version" || arg == "--version")
    {
        return Ok(FfmpegOutput::version(version_banner("ffmpeg")));
    }

    if args.is_empty() {
        return Err(FfmpegError::usage("missing command"));
    }

    let command = parse_ffmpeg_args(args)
        .map_err(|err| FfmpegError::usage(format!("failed to parse options: {err}")))?;
    let plan = build_io_plan(&command)
        .map_err(|err| FfmpegError::usage(format!("invalid input/output plan: {err}")))?;

    execute_plan(&plan)
}

fn execute_plan(plan: &IoPlan) -> Result<FfmpegOutput, FfmpegError> {
    if plan.inputs().len() != 1 {
        return Err(FfmpegError::unsupported(
            "ffmpeg-rs currently supports exactly one input",
        ));
    }
    if plan.outputs().len() != 1 {
        return Err(FfmpegError::unsupported(
            "ffmpeg-rs currently supports exactly one output",
        ));
    }

    let input = &plan.inputs()[0];
    let output = &plan.outputs()[0];
    let output_muxer = parse_output_muxer(output)?;
    validate_output_options(output_muxer, output)?;
    validate_output_endpoint(output_muxer, output)?;

    let input_path = local_input_path(input)?;
    let explicit_input = explicit_input_format(input)?;
    if let Some(InputFormat::Image2 {
        frame_rate,
        start_number,
    }) = explicit_input
    {
        return run_image2_input(input_path, frame_rate, start_number, output, output_muxer);
    }

    let bytes = fs::read(input_path)
        .map_err(|err| FfmpegError::io(format!("failed to read `{input_path}`: {err}")))?;
    let input_format = detect_input_format(explicit_input, input_path, &bytes)?;

    match input_format {
        InputFormat::Avi => {
            let mut demuxer = AviDemuxer::open(&bytes).map_err(|err| {
                FfmpegError::invalid_data(format!("failed to parse AVI input: {err}"))
            })?;
            let streamhash_types = streamhash_types_from_avi_info(demuxer.info());
            run_output_muxer(output_muxer, streamhash_types, || {
                demuxer.read_packet().map_err(|err| {
                    FfmpegError::invalid_data(format!("failed to read AVI packet: {err}"))
                })
            })
        }
        InputFormat::Image2 {
            frame_rate,
            start_number,
        } => run_image2_input(input_path, frame_rate, start_number, output, output_muxer),
        InputFormat::Mov => {
            let mut demuxer = MovDemuxer::open(&bytes).map_err(|err| {
                FfmpegError::invalid_data(format!("failed to parse MOV/MP4 input: {err}"))
            })?;
            let streamhash_types = streamhash_types_from_mov_info(demuxer.info());
            run_output_muxer(output_muxer, streamhash_types, || {
                demuxer.read_packet().map_err(|err| {
                    FfmpegError::invalid_data(format!("failed to read MOV/MP4 packet: {err}"))
                })
            })
        }
        InputFormat::PcmS16le {
            sample_rate,
            channels,
        } => {
            let mut demuxer =
                PcmS16leDemuxer::open(&bytes, sample_rate, channels, 1024).map_err(|err| {
                    FfmpegError::invalid_data(format!("failed to parse pcm_s16le input: {err}"))
                })?;
            let read_packet = || {
                demuxer.read_packet().map_err(|err| {
                    FfmpegError::invalid_data(format!("failed to read pcm_s16le packet: {err}"))
                })
            };
            match output_muxer {
                OutputMuxer::PcmS16le => {
                    run_pcm_s16le_file_muxer(output, sample_rate, channels, read_packet)
                }
                OutputMuxer::Wav => run_wav_file_muxer(output, sample_rate, channels, read_packet),
                OutputMuxer::Null
                | OutputMuxer::FrameCrc
                | OutputMuxer::FrameHash(_)
                | OutputMuxer::StreamHash(_)
                | OutputMuxer::Hash(_) => run_output_muxer(
                    output_muxer,
                    StreamHashStreamTypeMap::single(StreamHashStreamType::Audio),
                    read_packet,
                ),
                OutputMuxer::Avi
                | OutputMuxer::Image2
                | OutputMuxer::RawVideo
                | OutputMuxer::Yuv4MpegPipe => run_output_muxer(
                    output_muxer,
                    StreamHashStreamTypeMap::single(StreamHashStreamType::Audio),
                    read_packet,
                ),
            }
        }
        InputFormat::RawVideo {
            width,
            height,
            pixel_format,
            frame_rate,
        } => {
            let mut demuxer =
                RawVideoDemuxer::open(&bytes, width, height, pixel_format, frame_rate).map_err(
                    |err| {
                        FfmpegError::invalid_data(format!("failed to parse rawvideo input: {err}"))
                    },
                )?;
            let read_packet = || {
                demuxer.read_packet().map_err(|err| {
                    FfmpegError::invalid_data(format!("failed to read rawvideo packet: {err}"))
                })
            };
            match output_muxer {
                OutputMuxer::RawVideo => run_rawvideo_file_muxer(
                    output,
                    width,
                    height,
                    pixel_format,
                    frame_rate,
                    read_packet,
                ),
                OutputMuxer::Null
                | OutputMuxer::FrameCrc
                | OutputMuxer::FrameHash(_)
                | OutputMuxer::StreamHash(_)
                | OutputMuxer::Hash(_) => run_output_muxer(
                    output_muxer,
                    StreamHashStreamTypeMap::single(StreamHashStreamType::Video),
                    read_packet,
                ),
                OutputMuxer::PcmS16le => run_output_muxer(
                    output_muxer,
                    StreamHashStreamTypeMap::single(StreamHashStreamType::Video),
                    read_packet,
                ),
                OutputMuxer::Wav => run_output_muxer(
                    output_muxer,
                    StreamHashStreamTypeMap::single(StreamHashStreamType::Video),
                    read_packet,
                ),
                OutputMuxer::Image2 => run_output_muxer(
                    output_muxer,
                    StreamHashStreamTypeMap::single(StreamHashStreamType::Video),
                    read_packet,
                ),
                OutputMuxer::Avi => {
                    run_avi_file_muxer(output, width, height, pixel_format, frame_rate, read_packet)
                }
                OutputMuxer::Yuv4MpegPipe => run_yuv4mpegpipe_file_muxer(
                    output,
                    width,
                    height,
                    pixel_format,
                    frame_rate,
                    read_packet,
                ),
            }
        }
        InputFormat::Wav => {
            let mut demuxer = WavDemuxer::open(&bytes).map_err(|err| {
                FfmpegError::invalid_data(format!("failed to parse WAV input: {err}"))
            })?;
            run_output_muxer(
                output_muxer,
                StreamHashStreamTypeMap::single(StreamHashStreamType::Audio),
                || {
                    demuxer.read_packet().map_err(|err| {
                        FfmpegError::invalid_data(format!("failed to read WAV packet: {err}"))
                    })
                },
            )
        }
        InputFormat::Yuv4MpegPipe => {
            let mut demuxer = Yuv4MpegDemuxer::open(&bytes).map_err(|err| {
                FfmpegError::invalid_data(format!("failed to parse YUV4MPEG2 input: {err}"))
            })?;
            run_output_muxer(
                output_muxer,
                StreamHashStreamTypeMap::single(StreamHashStreamType::Video),
                || {
                    demuxer.read_packet().map_err(|err| {
                        FfmpegError::invalid_data(format!("failed to read YUV4MPEG2 packet: {err}"))
                    })
                },
            )
        }
    }
}

fn local_input_path(input: &PlannedFile) -> Result<&str, FfmpegError> {
    match input.endpoint() {
        Endpoint::File(path) => Ok(path),
        Endpoint::Pipe { .. } => Err(FfmpegError::unsupported(
            "ffmpeg-rs currently supports only local seekable file inputs",
        )),
        Endpoint::Protocol { name, .. } => Err(FfmpegError::unsupported(format!(
            "ffmpeg-rs input protocol `{name}` is not implemented"
        ))),
    }
}

fn validate_stdout_output(output: &PlannedFile) -> Result<(), FfmpegError> {
    match output.endpoint() {
        Endpoint::Pipe { fd: None } | Endpoint::Pipe { fd: Some(1) } => Ok(()),
        Endpoint::Pipe { fd: Some(fd) } => Err(FfmpegError::unsupported(format!(
            "ffmpeg-rs currently supports only stdout pipe output, got pipe:{fd}"
        ))),
        Endpoint::File(_) => Err(FfmpegError::unsupported(
            "ffmpeg-rs currently supports only stdout pipe output `-`",
        )),
        Endpoint::Protocol { name, .. } => Err(FfmpegError::unsupported(format!(
            "ffmpeg-rs output protocol `{name}` is not implemented"
        ))),
    }
}

fn validate_output_endpoint(muxer: OutputMuxer, output: &PlannedFile) -> Result<(), FfmpegError> {
    match muxer {
        OutputMuxer::Null
        | OutputMuxer::FrameCrc
        | OutputMuxer::FrameHash(_)
        | OutputMuxer::StreamHash(_)
        | OutputMuxer::Hash(_) => validate_stdout_output(output),
        OutputMuxer::Avi => {
            local_output_path(output, "avi")?;
            Ok(())
        }
        OutputMuxer::Image2 => {
            local_output_path(output, "image2")?;
            Ok(())
        }
        OutputMuxer::PcmS16le => {
            local_output_path(output, "pcm_s16le")?;
            Ok(())
        }
        OutputMuxer::RawVideo => {
            local_output_path(output, "rawvideo")?;
            Ok(())
        }
        OutputMuxer::Wav => {
            local_output_path(output, "wav")?;
            Ok(())
        }
        OutputMuxer::Yuv4MpegPipe => {
            local_output_path(output, "yuv4mpegpipe")?;
            Ok(())
        }
    }
}

fn local_output_path<'a>(
    output: &'a PlannedFile,
    format_name: &str,
) -> Result<&'a str, FfmpegError> {
    match output.endpoint() {
        Endpoint::File(path) => Ok(path),
        Endpoint::Pipe { .. } => Err(FfmpegError::unsupported(format!(
            "ffmpeg-rs {format_name} output currently supports only local file outputs"
        ))),
        Endpoint::Protocol { name, .. } => Err(FfmpegError::unsupported(format!(
            "ffmpeg-rs output protocol `{name}` is not implemented"
        ))),
    }
}

fn run_image2_input(
    path: &str,
    frame_rate: Rational,
    start_number: i64,
    output: &PlannedFile,
    output_muxer: OutputMuxer,
) -> Result<FfmpegOutput, FfmpegError> {
    let entries = image2_entries_for_path(path, start_number)?;
    let mut demuxer = Image2Demuxer::open(path, entries, start_number, frame_rate)
        .map_err(|err| FfmpegError::invalid_data(format!("failed to parse image2 input: {err}")))?;
    let read_packet = || {
        demuxer.read_packet().map_err(|err| {
            FfmpegError::invalid_data(format!("failed to read image2 packet: {err}"))
        })
    };

    match output_muxer {
        OutputMuxer::Image2 => run_image2_file_muxer(output, frame_rate, read_packet),
        _ => run_output_muxer(
            output_muxer,
            StreamHashStreamTypeMap::single(StreamHashStreamType::Video),
            read_packet,
        ),
    }
}

fn image2_entries_for_path(path: &str, start_number: i64) -> Result<Vec<Image2Entry>, FfmpegError> {
    let pattern = Image2Pattern::parse(path).map_err(|err| {
        FfmpegError::invalid_data(format!("failed to parse image2 pattern: {err}"))
    })?;

    if pattern.is_sequence() {
        return discover_image2_sequence_entries(&pattern, start_number);
    }

    let actual_path = pattern.path_for_frame_number(0).map_err(|err| {
        FfmpegError::invalid_data(format!("failed to resolve image2 input path: {err}"))
    })?;
    let entry = read_image2_entry(actual_path.clone(), Path::new(&actual_path))?;
    Ok(vec![entry])
}

fn discover_image2_sequence_entries(
    pattern: &Image2Pattern,
    start_number: i64,
) -> Result<Vec<Image2Entry>, FfmpegError> {
    let first_path = pattern.path_for_frame_number(start_number).map_err(|err| {
        FfmpegError::invalid_data(format!("failed to resolve image2 sequence path: {err}"))
    })?;
    let parent = Path::new(&first_path)
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let entries = fs::read_dir(parent).map_err(|err| {
        FfmpegError::io(format!(
            "failed to read image2 sequence directory `{}`: {err}",
            parent.display()
        ))
    })?;

    let mut image_entries = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|err| {
            FfmpegError::io(format!(
                "failed to read image2 sequence directory entry in `{}`: {err}",
                parent.display()
            ))
        })?;
        let file_type = entry.file_type().map_err(|err| {
            FfmpegError::io(format!(
                "failed to inspect image2 sequence path `{}`: {err}",
                entry.path().display()
            ))
        })?;
        if !file_type.is_file() {
            continue;
        }

        if let Some(matched_path) = matched_image2_path(pattern, &entry.path()) {
            image_entries.push(read_image2_entry(matched_path, &entry.path())?);
        }
    }

    if image_entries.is_empty() {
        return Err(FfmpegError::invalid_data(format!(
            "no image2 files matched pattern `{}`",
            pattern.raw()
        )));
    }

    Ok(image_entries)
}

fn matched_image2_path(pattern: &Image2Pattern, path: &Path) -> Option<String> {
    let mut candidates = Vec::with_capacity(3);
    let full_path = path.to_string_lossy().into_owned();
    candidates.push(full_path.clone());

    let slash_path = full_path.replace('\\', "/");
    if slash_path != full_path {
        candidates.push(slash_path);
    }

    if let Some(file_name) = path.file_name().and_then(|value| value.to_str()) {
        candidates.push(file_name.to_string());
    }

    candidates
        .into_iter()
        .find(|candidate| pattern.frame_number_for_path(candidate).is_some())
}

fn read_image2_entry(entry_path: String, read_path: &Path) -> Result<Image2Entry, FfmpegError> {
    let bytes = fs::read(read_path).map_err(|err| {
        FfmpegError::io(format!(
            "failed to read image2 input `{}`: {err}",
            read_path.display()
        ))
    })?;
    Image2Entry::new(entry_path, bytes)
        .map_err(|err| FfmpegError::invalid_data(format!("failed to prepare image2 input: {err}")))
}

fn detect_input_format(
    explicit: Option<InputFormat>,
    path: &str,
    bytes: &[u8],
) -> Result<InputFormat, FfmpegError> {
    if let Some(format) = explicit {
        validate_input_signature(format, path, bytes)?;
        return Ok(format);
    }

    if is_wav_like(path, bytes) {
        return Ok(InputFormat::Wav);
    }

    if is_avi_like(path, bytes) {
        return Ok(InputFormat::Avi);
    }

    if is_yuv4mpegpipe_like(path, bytes) {
        return Ok(InputFormat::Yuv4MpegPipe);
    }

    if is_mov_like(path, bytes)? {
        return Ok(InputFormat::Mov);
    }

    Err(FfmpegError::unsupported("unsupported input format"))
}

fn explicit_input_format(input: &PlannedFile) -> Result<Option<InputFormat>, FfmpegError> {
    for option in input.options() {
        if !matches!(
            option.name(),
            "f" | "ar" | "ac" | "s" | "r" | "framerate" | "pix_fmt" | "start_number"
        ) {
            return Err(FfmpegError::unsupported(format!(
                "input option `-{}` is not implemented",
                option.name()
            )));
        }
    }

    let Some(format) = last_option_value(input.options(), "f") else {
        if let Some(option) = input.options().first() {
            return Err(FfmpegError::unsupported(format!(
                "input option `-{}` requires an explicit input format",
                option.name()
            )));
        }
        return Ok(None);
    };

    match format.to_ascii_lowercase().as_str() {
        "avi" => {
            reject_stream_parameter_options(input, "AVI")?;
            Ok(Some(InputFormat::Avi))
        }
        "image2" => parse_image2_input(input).map(Some),
        "mov" | "mp4" | "m4a" | "3gp" | "3g2" | "mj2" => {
            reject_stream_parameter_options(input, "MOV/MP4")?;
            Ok(Some(InputFormat::Mov))
        }
        "s16le" | "pcm_s16le" => parse_pcm_s16le_input(input).map(Some),
        "rawvideo" => parse_rawvideo_input(input).map(Some),
        "wav" | "wave" => {
            reject_stream_parameter_options(input, "WAV")?;
            Ok(Some(InputFormat::Wav))
        }
        "yuv4mpegpipe" => {
            reject_stream_parameter_options(input, "yuv4mpegpipe")?;
            Ok(Some(InputFormat::Yuv4MpegPipe))
        }
        _ => Err(FfmpegError::unsupported(format!(
            "input format `{format}` is not implemented"
        ))),
    }
}

fn reject_stream_parameter_options(
    input: &PlannedFile,
    format_name: &str,
) -> Result<(), FfmpegError> {
    for option in input.options() {
        if option.name() != "f" {
            return Err(FfmpegError::unsupported(format!(
                "input option `-{}` is not implemented for {format_name}",
                option.name()
            )));
        }
    }
    Ok(())
}

fn parse_pcm_s16le_input(input: &PlannedFile) -> Result<InputFormat, FfmpegError> {
    reject_options_except(input, "pcm_s16le", &["f", "ar", "ac"])?;
    let sample_rate = parse_u32_option(input, "ar", "pcm_s16le sample rate")?;
    let channels = parse_u16_option(input, "ac", "pcm_s16le channel count")?;
    Ok(InputFormat::PcmS16le {
        sample_rate,
        channels,
    })
}

fn parse_image2_input(input: &PlannedFile) -> Result<InputFormat, FfmpegError> {
    reject_options_except(input, "image2", &["f", "r", "framerate", "start_number"])?;
    let frame_rate = parse_image2_frame_rate_option(input)?;
    let start_number = parse_optional_i64_option(input, "start_number", "image2 start number", 0)?;
    Ok(InputFormat::Image2 {
        frame_rate,
        start_number,
    })
}

fn parse_rawvideo_input(input: &PlannedFile) -> Result<InputFormat, FfmpegError> {
    reject_options_except(input, "rawvideo", &["f", "s", "r", "pix_fmt"])?;
    let (width, height) = parse_video_size_option(input)?;
    let pixel_format = parse_rawvideo_pixel_format(input)?;
    let frame_rate = parse_frame_rate_option(input)?;
    Ok(InputFormat::RawVideo {
        width,
        height,
        pixel_format,
        frame_rate,
    })
}

fn reject_options_except(
    input: &PlannedFile,
    format_name: &str,
    allowed: &[&str],
) -> Result<(), FfmpegError> {
    for option in input.options() {
        if !allowed.contains(&option.name()) {
            return Err(FfmpegError::unsupported(format!(
                "input option `-{}` is not implemented for {format_name}",
                option.name()
            )));
        }
    }
    Ok(())
}

fn parse_u32_option(file: &PlannedFile, name: &str, description: &str) -> Result<u32, FfmpegError> {
    let value = last_option_value(file.options(), name)
        .ok_or_else(|| FfmpegError::usage(format!("{description} requires `-{name}`")))?;
    let parsed = value
        .parse::<u32>()
        .map_err(|_| FfmpegError::usage(format!("invalid {description} `{value}`")))?;
    if parsed == 0 {
        return Err(FfmpegError::usage(format!(
            "{description} must be non-zero"
        )));
    }
    Ok(parsed)
}

fn parse_u16_option(file: &PlannedFile, name: &str, description: &str) -> Result<u16, FfmpegError> {
    let value = parse_u32_option(file, name, description)?;
    u16::try_from(value).map_err(|_| FfmpegError::usage(format!("{description} is out of range")))
}

fn parse_optional_i64_option(
    file: &PlannedFile,
    name: &str,
    description: &str,
    default: i64,
) -> Result<i64, FfmpegError> {
    let Some(value) = last_option_value(file.options(), name) else {
        return Ok(default);
    };
    let parsed = value
        .parse::<i64>()
        .map_err(|_| FfmpegError::usage(format!("invalid {description} `{value}`")))?;
    if parsed < 0 {
        return Err(FfmpegError::usage(format!(
            "{description} must be non-negative"
        )));
    }
    Ok(parsed)
}

fn parse_video_size_option(file: &PlannedFile) -> Result<(usize, usize), FfmpegError> {
    let value = last_option_value(file.options(), "s")
        .ok_or_else(|| FfmpegError::usage("rawvideo dimensions require `-s`"))?;
    let (width, height) = value
        .split_once(['x', 'X'])
        .ok_or_else(|| FfmpegError::usage(format!("invalid rawvideo dimensions `{value}`")))?;
    let width = parse_nonzero_usize(width, "rawvideo width")?;
    let height = parse_nonzero_usize(height, "rawvideo height")?;
    Ok((width, height))
}

fn parse_nonzero_usize(value: &str, description: &str) -> Result<usize, FfmpegError> {
    let parsed = value
        .parse::<usize>()
        .map_err(|_| FfmpegError::usage(format!("invalid {description} `{value}`")))?;
    if parsed == 0 {
        return Err(FfmpegError::usage(format!(
            "{description} must be non-zero"
        )));
    }
    Ok(parsed)
}

fn parse_rawvideo_pixel_format(file: &PlannedFile) -> Result<RawVideoPixelFormat, FfmpegError> {
    let value = last_option_value(file.options(), "pix_fmt")
        .ok_or_else(|| FfmpegError::usage("rawvideo pixel format requires `-pix_fmt`"))?;
    RawVideoPixelFormat::from_name(&value.to_ascii_lowercase()).ok_or_else(|| {
        FfmpegError::unsupported(format!(
            "rawvideo pixel format `{value}` is not implemented"
        ))
    })
}

fn parse_image2_frame_rate_option(file: &PlannedFile) -> Result<Rational, FfmpegError> {
    let value = last_option_value(file.options(), "framerate")
        .or_else(|| last_option_value(file.options(), "r"))
        .ok_or_else(|| FfmpegError::usage("image2 frame rate requires `-framerate` or `-r`"))?;
    parse_positive_rate(value, "image2 frame rate")
}

fn parse_frame_rate_option(file: &PlannedFile) -> Result<Rational, FfmpegError> {
    let value = last_option_value(file.options(), "r")
        .ok_or_else(|| FfmpegError::usage("rawvideo frame rate requires `-r`"))?;
    parse_positive_rate(value, "rawvideo frame rate")
}

fn parse_positive_rate(value: &str, description: &str) -> Result<Rational, FfmpegError> {
    let (num, den) = if let Some((num, den)) = value.split_once('/') {
        (
            parse_i32_rate_part(num, value, "numerator")?,
            parse_i32_rate_part(den, value, "denominator")?,
        )
    } else {
        (parse_i32_rate_part(value, value, "value")?, 1)
    };
    if num <= 0 || den <= 0 {
        return Err(FfmpegError::usage(format!(
            "{description} `{value}` must be positive"
        )));
    }
    Rational::new(num, den)
        .map_err(|err| FfmpegError::usage(format!("invalid {description} `{value}`: {err}")))
}

fn parse_i32_rate_part(
    value: &str,
    full_value: &str,
    description: &str,
) -> Result<i32, FfmpegError> {
    value.parse::<i32>().map_err(|_| {
        FfmpegError::usage(format!(
            "invalid rawvideo frame rate {description} `{value}` in `{full_value}`"
        ))
    })
}

fn validate_output_options(muxer: OutputMuxer, output: &PlannedFile) -> Result<(), FfmpegError> {
    for option in output.options() {
        let is_allowed = match muxer {
            OutputMuxer::Image2 => matches!(option.name(), "f" | "start_number"),
            OutputMuxer::FrameHash(hash) => {
                option.name() == "f" || (option.name() == "hash" && hash.allows_hash_option())
            }
            OutputMuxer::StreamHash(_) => matches!(option.name(), "f" | "hash"),
            OutputMuxer::Hash(hash) => {
                option.name() == "f" || (option.name() == "hash" && hash.allows_hash_option())
            }
            _ => option.name() == "f",
        };
        if !is_allowed {
            return Err(FfmpegError::unsupported(format!(
                "output option `-{}` is not implemented",
                option.name()
            )));
        }
    }
    Ok(())
}

fn parse_output_muxer(output: &PlannedFile) -> Result<OutputMuxer, FfmpegError> {
    let format = last_option_value(output.options(), "f").ok_or_else(|| {
        FfmpegError::usage(
            "ffmpeg-rs currently requires explicit output `-f null`, `-f framecrc`, `-f framehash`, `-f framemd5`, `-f streamhash`, `-f hash`, `-f md5`, `-f image2`, `-f s16le`, `-f rawvideo`, `-f wav`, `-f yuv4mpegpipe`, or `-f avi`",
        )
    })?;

    match format.to_ascii_lowercase().as_str() {
        "avi" => Ok(OutputMuxer::Avi),
        "null" => Ok(OutputMuxer::Null),
        "framecrc" => Ok(OutputMuxer::FrameCrc),
        "framehash" => Ok(OutputMuxer::FrameHash(FrameHashOutputMuxer::FrameHash(
            parse_hash_algorithm(output)?,
        ))),
        "framemd5" => Ok(OutputMuxer::FrameHash(FrameHashOutputMuxer::FrameMd5)),
        "streamhash" => Ok(OutputMuxer::StreamHash(parse_hash_algorithm(output)?)),
        "hash" => Ok(OutputMuxer::Hash(HashOutputMuxer::Hash(
            parse_hash_algorithm(output)?,
        ))),
        "md5" => Ok(OutputMuxer::Hash(HashOutputMuxer::Md5)),
        "image2" => Ok(OutputMuxer::Image2),
        "s16le" | "pcm_s16le" => Ok(OutputMuxer::PcmS16le),
        "rawvideo" => Ok(OutputMuxer::RawVideo),
        "wav" | "wave" => Ok(OutputMuxer::Wav),
        "yuv4mpegpipe" => Ok(OutputMuxer::Yuv4MpegPipe),
        _ => Err(FfmpegError::unsupported(format!(
            "output format `{format}` is not implemented"
        ))),
    }
}

fn parse_hash_algorithm(output: &PlannedFile) -> Result<HashAlgorithm, FfmpegError> {
    let Some(value) = last_option_value(output.options(), "hash") else {
        return Ok(HashAlgorithm::Sha256);
    };
    match value
        .chars()
        .filter(|ch| *ch != '-' && *ch != '_')
        .collect::<String>()
        .to_ascii_lowercase()
        .as_str()
    {
        "adler32" => Ok(HashAlgorithm::Adler32),
        "crc32" => Ok(HashAlgorithm::Crc32),
        "md5" => Ok(HashAlgorithm::Md5),
        "sha1" | "sha160" => Ok(HashAlgorithm::Sha160),
        "sha224" => Ok(HashAlgorithm::Sha224),
        "sha256" => Ok(HashAlgorithm::Sha256),
        "sha384" => Ok(HashAlgorithm::Sha384),
        "sha512" => Ok(HashAlgorithm::Sha512),
        _ => Err(FfmpegError::unsupported(format!(
            "hash algorithm `{value}` is not implemented"
        ))),
    }
}

fn last_option_value<'a>(options: &'a [CliOption], name: &str) -> Option<&'a str> {
    options
        .iter()
        .rev()
        .find(|option| option.name() == name)
        .and_then(CliOption::value_ref)
}

fn validate_input_signature(
    format: InputFormat,
    path: &str,
    bytes: &[u8],
) -> Result<(), FfmpegError> {
    match format {
        InputFormat::Image2 { .. } => Ok(()),
        InputFormat::Avi if is_avi_like(path, bytes) => Ok(()),
        InputFormat::Avi => Err(FfmpegError::invalid_data(
            "AVI input signature was not found",
        )),
        InputFormat::Mov => validate_mov_probe(path, bytes),
        InputFormat::PcmS16le { .. } => Ok(()),
        InputFormat::RawVideo { .. } => Ok(()),
        InputFormat::Wav if is_wav_like(path, bytes) => Ok(()),
        InputFormat::Wav => Err(FfmpegError::invalid_data(format!(
            "{} input signature was not found",
            format.name()
        ))),
        InputFormat::Yuv4MpegPipe if is_yuv4mpegpipe_like(path, bytes) => Ok(()),
        InputFormat::Yuv4MpegPipe => Err(FfmpegError::invalid_data(
            "yuv4mpegpipe input signature was not found",
        )),
    }
}

fn is_wav_like(path: &str, bytes: &[u8]) -> bool {
    has_wav_signature(bytes) || path_has_extension(path, "wav")
}

fn has_wav_signature(bytes: &[u8]) -> bool {
    bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WAVE"
}

fn is_avi_like(path: &str, bytes: &[u8]) -> bool {
    has_avi_signature(bytes) || path_has_extension(path, "avi")
}

fn has_avi_signature(bytes: &[u8]) -> bool {
    bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"AVI "
}

fn is_yuv4mpegpipe_like(path: &str, bytes: &[u8]) -> bool {
    has_yuv4mpegpipe_signature(bytes) || path_has_extension(path, "y4m")
}

fn has_yuv4mpegpipe_signature(bytes: &[u8]) -> bool {
    bytes.starts_with(b"YUV4MPEG2")
}

fn path_has_extension(path: &str, extension: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case(extension))
}

fn is_mov_like(path: &str, bytes: &[u8]) -> Result<bool, FfmpegError> {
    let mut registry = ProbeRegistry::new();
    register_mov_probe(&mut registry)
        .map_err(|err| FfmpegError::invalid_data(format!("failed to register MOV probe: {err}")))?;
    Ok(registry
        .probe(ProbeRequest::new(bytes).with_extension(path))
        .is_some_and(|matched| matched.descriptor().name() == MOV_FORMAT_NAME))
}

fn validate_mov_probe(path: &str, bytes: &[u8]) -> Result<(), FfmpegError> {
    let mut registry = ProbeRegistry::new();
    register_mov_probe(&mut registry)
        .map_err(|err| FfmpegError::invalid_data(format!("failed to register MOV probe: {err}")))?;
    let matched = registry
        .probe(ProbeRequest::new(bytes).with_extension(path))
        .ok_or_else(|| FfmpegError::unsupported("unsupported input format"))?;
    if matched.descriptor().name() != MOV_FORMAT_NAME {
        return Err(FfmpegError::unsupported(format!(
            "unsupported input format `{}`",
            matched.descriptor().name()
        )));
    }
    Ok(())
}

fn streamhash_types_from_avi_info(info: &AviInfo) -> StreamHashStreamTypeMap {
    StreamHashStreamTypeMap::from_indexed_streams(info.streams().iter().map(|stream| {
        let stream_type = match stream.media_type() {
            AviMediaType::Video => StreamHashStreamType::Video,
        };
        (stream.index(), stream_type)
    }))
}

fn streamhash_types_from_mov_info(info: &MovInfo) -> StreamHashStreamTypeMap {
    StreamHashStreamTypeMap::from_indexed_streams(
        info.tracks()
            .iter()
            .enumerate()
            .map(|(index, track)| (index, streamhash_type_from_mov_track(track))),
    )
}

fn streamhash_type_from_mov_track(track: &MovTrackInfo) -> StreamHashStreamType {
    if let Some(stream_type) = track
        .handler_type()
        .and_then(streamhash_type_from_mov_handler)
    {
        stream_type
    } else {
        match track
            .codec_parameters()
            .map(mov_streamhash_type_from_sample_entry)
        {
            Some(StreamHashStreamType::Video) => StreamHashStreamType::Video,
            Some(StreamHashStreamType::Subtitle) => StreamHashStreamType::Subtitle,
            Some(StreamHashStreamType::Data) => StreamHashStreamType::Data,
            Some(StreamHashStreamType::Audio) => StreamHashStreamType::Audio,
            Some(StreamHashStreamType::Unknown) | None => {
                if track.width().is_some() && track.height().is_some() {
                    StreamHashStreamType::Video
                } else {
                    StreamHashStreamType::Unknown
                }
            }
        }
    }
}

fn mov_streamhash_type_from_sample_entry(parameters: &MovCodecParameters) -> StreamHashStreamType {
    match parameters.details() {
        MovSampleEntryDetails::Generic => StreamHashStreamType::Unknown,
        MovSampleEntryDetails::Audio(_) => StreamHashStreamType::Audio,
        MovSampleEntryDetails::Video(_) => StreamHashStreamType::Video,
        MovSampleEntryDetails::Subtitle(_) => StreamHashStreamType::Subtitle,
        MovSampleEntryDetails::Data(_) => StreamHashStreamType::Data,
    }
}

fn streamhash_type_from_mov_handler(handler_type: &str) -> Option<StreamHashStreamType> {
    match handler_type {
        "vide" => Some(StreamHashStreamType::Video),
        "soun" => Some(StreamHashStreamType::Audio),
        "subt" | "sbtl" | "text" | "clcp" => Some(StreamHashStreamType::Subtitle),
        "meta" | "hint" => Some(StreamHashStreamType::Data),
        _ => None,
    }
}

fn run_output_muxer<F>(
    output_muxer: OutputMuxer,
    streamhash_stream_types: StreamHashStreamTypeMap,
    read_packet: F,
) -> Result<FfmpegOutput, FfmpegError>
where
    F: FnMut() -> Result<Option<Packet>, FfmpegError>,
{
    match output_muxer {
        OutputMuxer::Null => run_null_muxer(read_packet),
        OutputMuxer::FrameCrc => run_framecrc_muxer(read_packet),
        OutputMuxer::FrameHash(hash) => run_framehash_muxer(hash, read_packet),
        OutputMuxer::StreamHash(algorithm) => {
            run_streamhash_muxer(algorithm, streamhash_stream_types, read_packet)
        }
        OutputMuxer::Hash(hash) => run_hash_muxer(hash, read_packet),
        OutputMuxer::Avi => Err(FfmpegError::unsupported(
            "ffmpeg-rs AVI output is only implemented for rgb24 rawvideo inputs",
        )),
        OutputMuxer::Image2 => Err(FfmpegError::unsupported(
            "ffmpeg-rs image2 output is only implemented for image2 inputs",
        )),
        OutputMuxer::PcmS16le => Err(FfmpegError::unsupported(
            "ffmpeg-rs s16le output is only implemented for raw pcm_s16le inputs",
        )),
        OutputMuxer::RawVideo => Err(FfmpegError::unsupported(
            "ffmpeg-rs rawvideo output is only implemented for rawvideo inputs",
        )),
        OutputMuxer::Wav => Err(FfmpegError::unsupported(
            "ffmpeg-rs WAV output is only implemented for raw pcm_s16le inputs",
        )),
        OutputMuxer::Yuv4MpegPipe => Err(FfmpegError::unsupported(
            "ffmpeg-rs yuv4mpegpipe output is only implemented for raw yuv420p inputs",
        )),
    }
}

fn run_pcm_s16le_file_muxer<F>(
    output: &PlannedFile,
    sample_rate: u32,
    channels: u16,
    mut read_packet: F,
) -> Result<FfmpegOutput, FfmpegError>
where
    F: FnMut() -> Result<Option<Packet>, FfmpegError>,
{
    let output_path = local_output_path(output, "pcm_s16le")?;
    let mut muxer = PcmS16leMuxer::new(sample_rate, channels).map_err(|err| {
        FfmpegError::invalid_data(format!("failed to configure pcm_s16le muxer: {err}"))
    })?;

    while let Some(packet) = read_packet()? {
        muxer.write_packet(&packet).map_err(|err| {
            FfmpegError::invalid_data(format!("failed to mux pcm_s16le packet: {err}"))
        })?;
    }

    let packet_count = muxer.packets();
    let output_bytes = muxer.finish();
    let byte_count = u64::try_from(output_bytes.len())
        .map_err(|_| FfmpegError::invalid_data("pcm_s16le output size does not fit u64"))?;
    write_new_output_file(output_path, &output_bytes)?;

    Ok(FfmpegOutput::media(
        String::new(),
        String::new(),
        OutputMuxer::PcmS16le,
        packet_count,
        byte_count,
    ))
}

fn run_image2_file_muxer<F>(
    output: &PlannedFile,
    frame_rate: Rational,
    mut read_packet: F,
) -> Result<FfmpegOutput, FfmpegError>
where
    F: FnMut() -> Result<Option<Packet>, FfmpegError>,
{
    let output_path = local_output_path(output, "image2")?;
    let start_number =
        parse_optional_i64_option(output, "start_number", "image2 output start number", 0)?;
    let mut muxer = Image2Muxer::new(output_path, start_number, frame_rate).map_err(|err| {
        FfmpegError::invalid_data(format!("failed to configure image2 muxer: {err}"))
    })?;

    while let Some(packet) = read_packet()? {
        muxer.write_packet(&packet).map_err(|err| {
            FfmpegError::invalid_data(format!("failed to mux image2 packet: {err}"))
        })?;
    }

    let entries = muxer.finish();
    for entry in &entries {
        if Path::new(entry.path()).exists() {
            return Err(FfmpegError::io(format!(
                "failed to create output `{}`: file already exists",
                entry.path()
            )));
        }
    }

    let packet_count = u64::try_from(entries.len())
        .map_err(|_| FfmpegError::invalid_data("image2 frame count does not fit u64"))?;
    let byte_count = entries.iter().try_fold(0_u64, |total, entry| {
        let len = u64::try_from(entry.data().len())
            .map_err(|_| FfmpegError::invalid_data("image2 output size does not fit u64"))?;
        total
            .checked_add(len)
            .ok_or_else(|| FfmpegError::invalid_data("image2 output byte count overflow"))
    })?;

    for entry in entries {
        write_new_output_file(entry.path(), entry.data())?;
    }

    Ok(FfmpegOutput::media(
        String::new(),
        String::new(),
        OutputMuxer::Image2,
        packet_count,
        byte_count,
    ))
}

fn run_rawvideo_file_muxer<F>(
    output: &PlannedFile,
    width: usize,
    height: usize,
    pixel_format: RawVideoPixelFormat,
    frame_rate: Rational,
    mut read_packet: F,
) -> Result<FfmpegOutput, FfmpegError>
where
    F: FnMut() -> Result<Option<Packet>, FfmpegError>,
{
    let output_path = local_output_path(output, "rawvideo")?;
    let mut muxer = RawVideoMuxer::new(width, height, pixel_format, frame_rate).map_err(|err| {
        FfmpegError::invalid_data(format!("failed to configure rawvideo muxer: {err}"))
    })?;

    while let Some(packet) = read_packet()? {
        muxer.write_packet(&packet).map_err(|err| {
            FfmpegError::invalid_data(format!("failed to mux rawvideo packet: {err}"))
        })?;
    }

    let packet_count = u64::try_from(muxer.info().frame_count())
        .map_err(|_| FfmpegError::invalid_data("rawvideo frame count does not fit u64"))?;
    let output_bytes = muxer.finish();
    let byte_count = u64::try_from(output_bytes.len())
        .map_err(|_| FfmpegError::invalid_data("rawvideo output size does not fit u64"))?;
    write_new_output_file(output_path, &output_bytes)?;

    Ok(FfmpegOutput::media(
        String::new(),
        String::new(),
        OutputMuxer::RawVideo,
        packet_count,
        byte_count,
    ))
}

fn run_avi_file_muxer<F>(
    output: &PlannedFile,
    width: usize,
    height: usize,
    pixel_format: RawVideoPixelFormat,
    frame_rate: Rational,
    mut read_packet: F,
) -> Result<FfmpegOutput, FfmpegError>
where
    F: FnMut() -> Result<Option<Packet>, FfmpegError>,
{
    if pixel_format != RawVideoPixelFormat::Rgb24 {
        return Err(FfmpegError::unsupported(
            "ffmpeg-rs AVI output currently supports only rgb24 rawvideo input",
        ));
    }

    let output_path = local_output_path(output, "avi")?;
    let width = u32::try_from(width)
        .map_err(|_| FfmpegError::invalid_data("AVI width does not fit u32"))?;
    let height = u32::try_from(height)
        .map_err(|_| FfmpegError::invalid_data("AVI height does not fit u32"))?;
    let mut muxer = AviMuxer::new_rgb24(width, height, frame_rate).map_err(|err| {
        FfmpegError::invalid_data(format!("failed to configure AVI muxer: {err}"))
    })?;

    while let Some(packet) = read_packet()? {
        muxer
            .write_packet(&packet)
            .map_err(|err| FfmpegError::invalid_data(format!("failed to mux AVI packet: {err}")))?;
    }

    let packet_count = u64::try_from(muxer.packet_count())
        .map_err(|_| FfmpegError::invalid_data("AVI frame count does not fit u64"))?;
    let output_bytes = muxer
        .finish()
        .map_err(|err| FfmpegError::invalid_data(format!("failed to finish AVI muxer: {err}")))?;
    let byte_count = u64::try_from(output_bytes.len())
        .map_err(|_| FfmpegError::invalid_data("AVI output size does not fit u64"))?;
    write_new_output_file(output_path, &output_bytes)?;

    Ok(FfmpegOutput::media(
        String::new(),
        String::new(),
        OutputMuxer::Avi,
        packet_count,
        byte_count,
    ))
}

fn run_wav_file_muxer<F>(
    output: &PlannedFile,
    sample_rate: u32,
    channels: u16,
    mut read_packet: F,
) -> Result<FfmpegOutput, FfmpegError>
where
    F: FnMut() -> Result<Option<Packet>, FfmpegError>,
{
    let output_path = local_output_path(output, "wav")?;
    let mut muxer = WavMuxer::new_pcm_s16le(channels, sample_rate).map_err(|err| {
        FfmpegError::invalid_data(format!("failed to configure WAV muxer: {err}"))
    })?;

    while let Some(packet) = read_packet()? {
        muxer
            .write_packet(&packet)
            .map_err(|err| FfmpegError::invalid_data(format!("failed to mux WAV packet: {err}")))?;
    }

    let packet_count = muxer.packets();
    let output_bytes = muxer
        .finish()
        .map_err(|err| FfmpegError::invalid_data(format!("failed to finish WAV muxer: {err}")))?;
    let byte_count = u64::try_from(output_bytes.len())
        .map_err(|_| FfmpegError::invalid_data("WAV output size does not fit u64"))?;
    write_new_output_file(output_path, &output_bytes)?;

    Ok(FfmpegOutput::media(
        String::new(),
        String::new(),
        OutputMuxer::Wav,
        packet_count,
        byte_count,
    ))
}

fn run_yuv4mpegpipe_file_muxer<F>(
    output: &PlannedFile,
    width: usize,
    height: usize,
    pixel_format: RawVideoPixelFormat,
    frame_rate: Rational,
    mut read_packet: F,
) -> Result<FfmpegOutput, FfmpegError>
where
    F: FnMut() -> Result<Option<Packet>, FfmpegError>,
{
    if pixel_format != RawVideoPixelFormat::Yuv420p {
        return Err(FfmpegError::unsupported(
            "ffmpeg-rs yuv4mpegpipe output currently supports only yuv420p rawvideo input",
        ));
    }

    let output_path = local_output_path(output, "yuv4mpegpipe")?;
    let width = u32::try_from(width)
        .map_err(|_| FfmpegError::invalid_data("yuv4mpegpipe width does not fit u32"))?;
    let height = u32::try_from(height)
        .map_err(|_| FfmpegError::invalid_data("yuv4mpegpipe height does not fit u32"))?;
    let mut muxer = Yuv4MpegMuxer::new(width, height, frame_rate, None).map_err(|err| {
        FfmpegError::invalid_data(format!("failed to configure yuv4mpegpipe muxer: {err}"))
    })?;

    while let Some(packet) = read_packet()? {
        muxer.write_packet(&packet).map_err(|err| {
            FfmpegError::invalid_data(format!("failed to mux yuv4mpegpipe packet: {err}"))
        })?;
    }

    let packet_count = u64::try_from(muxer.frame_count())
        .map_err(|_| FfmpegError::invalid_data("yuv4mpegpipe frame count does not fit u64"))?;
    let output_bytes = muxer.finish();
    let byte_count = u64::try_from(output_bytes.len())
        .map_err(|_| FfmpegError::invalid_data("yuv4mpegpipe output size does not fit u64"))?;
    write_new_output_file(output_path, &output_bytes)?;

    Ok(FfmpegOutput::media(
        String::new(),
        String::new(),
        OutputMuxer::Yuv4MpegPipe,
        packet_count,
        byte_count,
    ))
}

fn write_new_output_file(path: &str, bytes: &[u8]) -> Result<(), FfmpegError> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|err| FfmpegError::io(format!("failed to create output `{path}`: {err}")))?;
    file.write_all(bytes)
        .map_err(|err| FfmpegError::io(format!("failed to write output `{path}`: {err}")))
}

fn run_null_muxer<F>(mut read_packet: F) -> Result<FfmpegOutput, FfmpegError>
where
    F: FnMut() -> Result<Option<Packet>, FfmpegError>,
{
    let mut muxer = NullMuxer::new();
    while let Some(packet) = read_packet()? {
        muxer.write_packet(&packet).map_err(|err| {
            FfmpegError::invalid_data(format!("failed to mux null packet: {err}"))
        })?;
    }

    let report = muxer.finish();
    Ok(FfmpegOutput::media(
        String::new(),
        String::new(),
        OutputMuxer::Null,
        report.total_packets(),
        report.total_bytes(),
    ))
}

fn run_framecrc_muxer<F>(mut read_packet: F) -> Result<FfmpegOutput, FfmpegError>
where
    F: FnMut() -> Result<Option<Packet>, FfmpegError>,
{
    let mut muxer = FrameCrcMuxer::new();
    let mut packet_count = 0_u64;
    let mut byte_count = 0_u64;

    while let Some(packet) = read_packet()? {
        packet_count = packet_count
            .checked_add(1)
            .ok_or_else(|| FfmpegError::invalid_data("packet count overflow"))?;
        let packet_bytes = u64::try_from(packet.data().len())
            .map_err(|_| FfmpegError::invalid_data("packet size does not fit u64"))?;
        byte_count = byte_count
            .checked_add(packet_bytes)
            .ok_or_else(|| FfmpegError::invalid_data("byte count overflow"))?;
        muxer.write_packet(&packet).map_err(|err| {
            FfmpegError::invalid_data(format!("failed to mux framecrc packet: {err}"))
        })?;
    }

    Ok(FfmpegOutput::media(
        muxer.finish(),
        String::new(),
        OutputMuxer::FrameCrc,
        packet_count,
        byte_count,
    ))
}

fn run_framehash_muxer<F>(
    hash: FrameHashOutputMuxer,
    mut read_packet: F,
) -> Result<FfmpegOutput, FfmpegError>
where
    F: FnMut() -> Result<Option<Packet>, FfmpegError>,
{
    let mut muxer = FrameHashMuxer::new(hash.algorithm());
    let mut packet_count = 0_u64;
    let mut byte_count = 0_u64;

    while let Some(packet) = read_packet()? {
        packet_count = packet_count
            .checked_add(1)
            .ok_or_else(|| FfmpegError::invalid_data("packet count overflow"))?;
        let packet_bytes = u64::try_from(packet.data().len())
            .map_err(|_| FfmpegError::invalid_data("packet size does not fit u64"))?;
        byte_count = byte_count
            .checked_add(packet_bytes)
            .ok_or_else(|| FfmpegError::invalid_data("byte count overflow"))?;
        muxer.write_packet(&packet).map_err(|err| {
            FfmpegError::invalid_data(format!("failed to mux framehash packet: {err}"))
        })?;
    }

    Ok(FfmpegOutput::media(
        muxer.finish(),
        String::new(),
        OutputMuxer::FrameHash(hash),
        packet_count,
        byte_count,
    ))
}

fn run_streamhash_muxer<F>(
    algorithm: HashAlgorithm,
    stream_types: StreamHashStreamTypeMap,
    mut read_packet: F,
) -> Result<FfmpegOutput, FfmpegError>
where
    F: FnMut() -> Result<Option<Packet>, FfmpegError>,
{
    let mut muxer = StreamHashMuxer::new(algorithm);
    let mut packet_count = 0_u64;
    let mut byte_count = 0_u64;

    while let Some(packet) = read_packet()? {
        packet_count = packet_count
            .checked_add(1)
            .ok_or_else(|| FfmpegError::invalid_data("packet count overflow"))?;
        let packet_bytes = u64::try_from(packet.data().len())
            .map_err(|_| FfmpegError::invalid_data("packet size does not fit u64"))?;
        byte_count = byte_count
            .checked_add(packet_bytes)
            .ok_or_else(|| FfmpegError::invalid_data("byte count overflow"))?;
        let stream_type = stream_types.stream_type_for_packet(&packet)?;
        muxer.write_packet(&packet, stream_type).map_err(|err| {
            FfmpegError::invalid_data(format!("failed to mux streamhash packet: {err}"))
        })?;
    }

    Ok(FfmpegOutput::media(
        muxer.finish(),
        String::new(),
        OutputMuxer::StreamHash(algorithm),
        packet_count,
        byte_count,
    ))
}

fn run_hash_muxer<F>(hash: HashOutputMuxer, mut read_packet: F) -> Result<FfmpegOutput, FfmpegError>
where
    F: FnMut() -> Result<Option<Packet>, FfmpegError>,
{
    let algorithm = hash.algorithm();
    let mut muxer = HashMuxer::new(algorithm);

    while let Some(packet) = read_packet()? {
        muxer.write_packet(&packet).map_err(|err| {
            FfmpegError::invalid_data(format!("failed to mux hash packet: {err}"))
        })?;
    }

    let report = muxer.finish();
    Ok(FfmpegOutput::media(
        report.line(),
        String::new(),
        OutputMuxer::Hash(hash),
        report.packets(),
        report.bytes(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
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
    const HDLR_ID: [u8; 4] = *b"hdlr";
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
    fn ffmpeg_output_prints_version_banner() {
        let output = ffmpeg_output(&strings(&["-version"])).unwrap();

        assert!(output
            .stdout()
            .starts_with("ffmpeg version 8.1.1-rust target FFmpeg 8.1.1"));
        assert!(output.stdout().contains("libavformat"));
        assert!(output.stderr().is_empty());
        assert_eq!(output.output_format(), None);
    }

    #[test]
    fn runs_mov_to_framecrc_stdout() {
        let path = write_temp_mov(
            "framecrc",
            &sampled_mov_file(&[b"abc".as_slice(), b"defg".as_slice()], &[1_000, 2_000]),
        );
        let path_arg = path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-hide_banner",
            "-i",
            path_arg.as_str(),
            "-f",
            "framecrc",
            "-",
        ]))
        .expect("ffmpeg command path should execute");

        let _ = fs::remove_file(&path);

        assert_eq!(output.output_format(), Some("framecrc"));
        assert_eq!(output.packet_count(), 2);
        assert_eq!(output.byte_count(), 7);
        assert!(output.stderr().is_empty());
        assert!(output
            .stdout()
            .starts_with("# framecrc-rs packet checksums\n"));
        assert!(output
            .stdout()
            .contains("stream=0 pts=0 dts=0 duration=1000 size=3 crc32=0x352441c2\n"));
        assert!(output
            .stdout()
            .contains("stream=0 pts=1000 dts=1000 duration=2000 size=4"));
    }

    #[test]
    fn runs_mov_to_framehash_stdout_with_default_sha256() {
        let path = write_temp_mov(
            "framehash",
            &sampled_mov_file(&[b"abc".as_slice(), b"defg".as_slice()], &[1_000, 2_000]),
        );
        let path_arg = path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-hide_banner",
            "-i",
            path_arg.as_str(),
            "-f",
            "framehash",
            "-",
        ]))
        .expect("ffmpeg framehash command path should execute");

        let _ = fs::remove_file(&path);

        assert_eq!(output.output_format(), Some("framehash"));
        assert_eq!(output.packet_count(), 2);
        assert_eq!(output.byte_count(), 7);
        assert!(output.stderr().is_empty());
        assert!(output
            .stdout()
            .starts_with("# framehash-rs packet hashes algorithm=SHA256\n"));
        assert!(output.stdout().contains(&format!(
            "stream=0 pts=0 dts=0 duration=1000 size=3 sha256={}\n",
            avutil::digest_to_hex(&avutil::sha256(b"abc"))
        )));
        assert!(output.stdout().contains(&format!(
            "stream=0 pts=1000 dts=1000 duration=2000 size=4 sha256={}\n",
            avutil::digest_to_hex(&avutil::sha256(b"defg"))
        )));
    }

    #[test]
    fn runs_mov_to_framehash_stdout_with_md5_option() {
        let path = write_temp_mov("framehash-md5", &sampled_mov_file(&[b"abc"], &[1_000]));
        let path_arg = path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-i",
            path_arg.as_str(),
            "-f",
            "framehash",
            "-hash",
            "md5",
            "-",
        ]))
        .expect("ffmpeg framehash hash-option path should execute");

        let _ = fs::remove_file(&path);

        assert_eq!(output.output_format(), Some("framehash"));
        assert_eq!(output.packet_count(), 1);
        assert_eq!(output.byte_count(), 3);
        assert!(output.stdout().contains(&format!(
            "stream=0 pts=0 dts=0 duration=1000 size=3 md5={}\n",
            avutil::digest_to_hex(&avutil::md5(b"abc"))
        )));
    }

    #[test]
    fn runs_mov_to_framemd5_muxer_stdout() {
        let path = write_temp_mov("framemd5", &sampled_mov_file(&[b"abc"], &[1_000]));
        let path_arg = path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-hide_banner",
            "-i",
            path_arg.as_str(),
            "-f",
            "framemd5",
            "-",
        ]))
        .expect("ffmpeg framemd5 command path should execute");

        let _ = fs::remove_file(&path);

        assert_eq!(output.output_format(), Some("framemd5"));
        assert_eq!(output.packet_count(), 1);
        assert_eq!(output.byte_count(), 3);
        assert!(output.stderr().is_empty());
        assert!(output
            .stdout()
            .starts_with("# framehash-rs packet hashes algorithm=MD5\n"));
        assert!(output.stdout().contains(&format!(
            "stream=0 pts=0 dts=0 duration=1000 size=3 md5={}\n",
            avutil::digest_to_hex(&avutil::md5(b"abc"))
        )));
    }

    #[test]
    fn runs_mov_to_null_without_media_stdout() {
        let path = write_temp_mov(
            "null",
            &sampled_mov_file(&[b"abc".as_slice(), b"defg".as_slice()], &[1_000, 2_000]),
        );
        let path_arg = path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&["-i", path_arg.as_str(), "-f", "null", "-"]))
            .expect("ffmpeg command path should execute");

        let _ = fs::remove_file(&path);

        assert_eq!(output.output_format(), Some("null"));
        assert_eq!(output.packet_count(), 2);
        assert_eq!(output.byte_count(), 7);
        assert!(output.stdout().is_empty());
        assert!(output.stderr().is_empty());
    }

    #[test]
    fn runs_mov_to_hash_stdout_with_default_sha256() {
        let path = write_temp_mov(
            "hash",
            &sampled_mov_file(&[b"abc".as_slice(), b"defg".as_slice()], &[1_000, 2_000]),
        );
        let path_arg = path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-hide_banner",
            "-i",
            path_arg.as_str(),
            "-f",
            "hash",
            "-",
        ]))
        .expect("ffmpeg hash command path should execute");

        let _ = fs::remove_file(&path);

        assert_eq!(output.output_format(), Some("hash"));
        assert_eq!(output.packet_count(), 2);
        assert_eq!(output.byte_count(), 7);
        assert!(output.stderr().is_empty());
        assert_eq!(
            output.stdout(),
            format!(
                "SHA256={}\n",
                avutil::digest_to_hex(&avutil::sha256(b"abcdefg"))
            )
        );
    }

    #[test]
    fn runs_mov_to_md5_muxer_stdout() {
        let path = write_temp_mov(
            "md5",
            &sampled_mov_file(&[b"abc".as_slice(), b"defg".as_slice()], &[1_000, 2_000]),
        );
        let path_arg = path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-hide_banner",
            "-i",
            path_arg.as_str(),
            "-f",
            "md5",
            "-",
        ]))
        .expect("ffmpeg md5 command path should execute");

        let _ = fs::remove_file(&path);

        assert_eq!(output.output_format(), Some("md5"));
        assert_eq!(output.packet_count(), 2);
        assert_eq!(output.byte_count(), 7);
        assert!(output.stderr().is_empty());
        assert_eq!(
            output.stdout(),
            format!("MD5={}\n", avutil::digest_to_hex(&avutil::md5(b"abcdefg")))
        );
    }

    #[test]
    fn runs_mov_to_streamhash_stdout_with_default_sha256() {
        let path = write_temp_mov(
            "streamhash",
            &sampled_mov_file(&[b"abc".as_slice(), b"defg".as_slice()], &[1_000, 2_000]),
        );
        let path_arg = path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-hide_banner",
            "-i",
            path_arg.as_str(),
            "-f",
            "streamhash",
            "-",
        ]))
        .expect("ffmpeg streamhash command path should execute");

        let _ = fs::remove_file(&path);

        assert_eq!(output.output_format(), Some("streamhash"));
        assert_eq!(output.packet_count(), 2);
        assert_eq!(output.byte_count(), 7);
        assert!(output.stderr().is_empty());
        assert_eq!(
            output.stdout(),
            format!(
                "0,v,SHA256={}\n",
                avutil::digest_to_hex(&avutil::sha256(b"abcdefg"))
            )
        );
    }

    #[test]
    fn runs_mov_audio_handler_to_streamhash_stdout_with_audio_type() {
        let path = write_temp_mov(
            "streamhash-audio-handler",
            &sampled_mov_file_with_handler(
                &[b"abc".as_slice(), b"defg".as_slice()],
                &[1_000, 2_000],
                *b"soun",
                0,
                0,
            ),
        );
        let path_arg = path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-hide_banner",
            "-i",
            path_arg.as_str(),
            "-f",
            "streamhash",
            "-",
        ]))
        .expect("ffmpeg streamhash command path should execute");

        let _ = fs::remove_file(&path);

        assert_eq!(output.output_format(), Some("streamhash"));
        assert_eq!(output.packet_count(), 2);
        assert_eq!(output.byte_count(), 7);
        assert!(output.stderr().is_empty());
        assert_eq!(
            output.stdout(),
            format!(
                "0,a,SHA256={}\n",
                avutil::digest_to_hex(&avutil::sha256(b"abcdefg"))
            )
        );
    }

    #[test]
    fn streamhash_type_maps_derive_from_container_metadata() {
        let avi_bytes = avi_file_bytes(2, 1, Rational::new(25, 1).unwrap(), &[b"abcdef"]);
        let avi = AviDemuxer::open(&avi_bytes).unwrap();
        let avi_types = streamhash_types_from_avi_info(avi.info());

        let mov_bytes = sampled_mov_file(&[b"abc"], &[1_000]);
        let mov = MovDemuxer::open(&mov_bytes).unwrap();
        let mov_types = streamhash_types_from_mov_info(mov.info());

        let mov_audio_bytes = sampled_mov_file_with_handler(&[b"abc"], &[1_000], *b"soun", 0, 0);
        let mov_audio = MovDemuxer::open(&mov_audio_bytes).unwrap();
        let mov_audio_types = streamhash_types_from_mov_info(mov_audio.info());

        assert_eq!(
            avi_types.stream_type_for_index(0).unwrap(),
            StreamHashStreamType::Video
        );
        assert_eq!(
            mov_types.stream_type_for_index(0).unwrap(),
            StreamHashStreamType::Video
        );
        assert_eq!(
            mov_audio_types.stream_type_for_index(0).unwrap(),
            StreamHashStreamType::Audio
        );
        assert!(avi_types.stream_type_for_index(1).is_err());
        assert!(mov_types.stream_type_for_index(1).is_err());
        assert!(mov_audio_types.stream_type_for_index(1).is_err());
    }

    #[test]
    fn streamhash_rejects_packets_without_stream_metadata() {
        let mut emitted = false;
        let result = run_streamhash_muxer(
            HashAlgorithm::Sha256,
            StreamHashStreamTypeMap::single(StreamHashStreamType::Video),
            || {
                if emitted {
                    Ok(None)
                } else {
                    emitted = true;
                    Ok(Some(Packet::new(vec![1, 2, 3], 1)))
                }
            },
        );

        assert!(matches!(
            result,
            Err(err) if err.to_string().contains("missing streamhash stream metadata for stream 1")
        ));
    }

    #[test]
    fn runs_avi_to_streamhash_stdout_with_demuxer_stream_type() {
        let first = [0, 1, 2, 3, 4, 5];
        let second = [6, 7, 8, 9, 10, 11];
        let path = write_temp_bytes(
            "avi-streamhash",
            "avi",
            &avi_file_bytes(2, 1, Rational::new(25, 1).unwrap(), &[&first, &second]),
        );
        let path_arg = path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-hide_banner",
            "-i",
            path_arg.as_str(),
            "-f",
            "streamhash",
            "-",
        ]))
        .expect("AVI streamhash command path should execute");

        let _ = fs::remove_file(&path);

        let mut expected = Vec::new();
        expected.extend_from_slice(&first);
        expected.extend_from_slice(&second);

        assert_eq!(output.output_format(), Some("streamhash"));
        assert_eq!(output.packet_count(), 2);
        assert_eq!(output.byte_count(), 12);
        assert!(output.stderr().is_empty());
        assert_eq!(
            output.stdout(),
            format!(
                "0,v,SHA256={}\n",
                avutil::digest_to_hex(&avutil::sha256(&expected))
            )
        );
    }

    #[test]
    fn runs_avi_to_framecrc_stdout() {
        let first = [0, 1, 2, 3, 4, 5];
        let second = [6, 7, 8, 9, 10, 11];
        let path = write_temp_bytes(
            "avi-framecrc",
            "avi",
            &avi_file_bytes(2, 1, Rational::new(25, 1).unwrap(), &[&first, &second]),
        );
        let path_arg = path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-hide_banner",
            "-i",
            path_arg.as_str(),
            "-f",
            "framecrc",
            "-",
        ]))
        .expect("AVI command path should execute");

        let _ = fs::remove_file(&path);

        assert_eq!(output.output_format(), Some("framecrc"));
        assert_eq!(output.packet_count(), 2);
        assert_eq!(output.byte_count(), 12);
        assert!(output.stderr().is_empty());
        assert!(output
            .stdout()
            .contains("stream=0 pts=0 dts=0 duration=1 size=6"));
        assert!(output
            .stdout()
            .contains("stream=0 pts=1 dts=1 duration=1 size=6"));
    }

    #[test]
    fn runs_explicit_avi_input_format_to_null() {
        let frame = [0, 1, 2, 3, 4, 5];
        let path = write_temp_bytes(
            "explicit-avi",
            "bin",
            &avi_file_bytes(2, 1, Rational::new(30, 1).unwrap(), &[&frame]),
        );
        let path_arg = path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-f",
            "avi",
            "-i",
            path_arg.as_str(),
            "-f",
            "null",
            "-",
        ]))
        .expect("explicit AVI command path should execute");

        let _ = fs::remove_file(&path);

        assert_eq!(output.output_format(), Some("null"));
        assert_eq!(output.packet_count(), 1);
        assert_eq!(output.byte_count(), 6);
        assert!(output.stdout().is_empty());
        assert!(output.stderr().is_empty());
    }

    #[test]
    fn rejects_avi_bad_header() {
        let path = write_temp_bytes("avi-bad-header", "avi", b"not an avi");
        let path_arg = path.to_string_lossy().into_owned();

        let err = ffmpeg_output(&strings(&["-i", path_arg.as_str(), "-f", "framecrc", "-"]))
            .expect_err("AVI input should reject malformed headers");

        let _ = fs::remove_file(&path);

        assert!(err.message().contains("failed to parse AVI input"));
    }

    #[test]
    fn runs_wav_to_framecrc_stdout() {
        let payload = [0, 0, 1, 0, 2, 0, 3, 0];
        let path = write_temp_bytes("wav-framecrc", "wav", &wav_file_bytes(2, 48_000, &payload));
        let path_arg = path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-hide_banner",
            "-i",
            path_arg.as_str(),
            "-f",
            "framecrc",
            "-",
        ]))
        .expect("ffmpeg WAV command path should execute");

        let _ = fs::remove_file(&path);

        assert_eq!(output.output_format(), Some("framecrc"));
        assert_eq!(output.packet_count(), 1);
        assert_eq!(output.byte_count(), 8);
        assert!(output.stderr().is_empty());
        assert!(output
            .stdout()
            .contains("stream=0 pts=0 dts=0 duration=2 size=8"));
    }

    #[test]
    fn runs_explicit_wav_input_format_to_null() {
        let payload = [1, 0, 2, 0, 3, 0, 4, 0];
        let path = write_temp_bytes("explicit-wav", "bin", &wav_file_bytes(1, 44_100, &payload));
        let path_arg = path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-f",
            "wav",
            "-i",
            path_arg.as_str(),
            "-f",
            "null",
            "-",
        ]))
        .expect("explicit WAV command path should execute");

        let _ = fs::remove_file(&path);

        assert_eq!(output.output_format(), Some("null"));
        assert_eq!(output.packet_count(), 1);
        assert_eq!(output.byte_count(), 8);
        assert!(output.stdout().is_empty());
        assert!(output.stderr().is_empty());
    }

    #[test]
    fn rejects_unsupported_output_muxer() {
        let path = write_temp_mov("unsupported-muxer", &sampled_mov_file(&[b"abc"], &[1_000]));
        let path_arg = path.to_string_lossy().into_owned();

        let err = ffmpeg_output(&strings(&["-i", path_arg.as_str(), "-f", "matroska", "-"]))
            .expect_err("unsupported muxer should fail");

        let _ = fs::remove_file(&path);

        assert!(err.message().contains("output format `matroska`"));
    }

    #[test]
    fn rejects_non_file_input_and_non_stdout_output() {
        let pipe_input =
            ffmpeg_output(&strings(&["-i", "pipe:0", "-f", "framecrc", "-"])).unwrap_err();
        assert!(pipe_input.message().contains("local seekable file inputs"));

        let path = write_temp_mov("file-output", &sampled_mov_file(&[b"abc"], &[1_000]));
        let path_arg = path.to_string_lossy().into_owned();
        let file_output = ffmpeg_output(&strings(&[
            "-i",
            path_arg.as_str(),
            "-f",
            "framecrc",
            "out.framecrc",
        ]))
        .unwrap_err();

        let _ = fs::remove_file(&path);

        assert!(file_output.message().contains("stdout pipe output"));
    }

    #[test]
    fn rejects_unmatched_local_input_format() {
        let path = write_temp_bytes("not-mov", "bin", b"not a movie");
        let path_arg = path.to_string_lossy().into_owned();

        let err = ffmpeg_output(&strings(&["-i", path_arg.as_str(), "-f", "framecrc", "-"]))
            .expect_err("unmatched input should fail");

        let _ = fs::remove_file(&path);

        assert!(err.message().contains("unsupported input format"));
    }

    #[test]
    fn runs_s16le_to_framecrc_stdout() {
        let path = write_temp_bytes("raw-pcm-framecrc", "raw", &[0, 0, 1, 0, 2, 0, 3, 0]);
        let path_arg = path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-f",
            "s16le",
            "-ar",
            "48000",
            "-ac",
            "2",
            "-i",
            path_arg.as_str(),
            "-f",
            "framecrc",
            "-",
        ]))
        .expect("raw PCM command path should execute");

        let _ = fs::remove_file(&path);

        assert_eq!(output.output_format(), Some("framecrc"));
        assert_eq!(output.packet_count(), 1);
        assert_eq!(output.byte_count(), 8);
        assert!(output.stderr().is_empty());
        assert!(output
            .stdout()
            .contains("stream=0 pts=0 dts=0 duration=2 size=8"));
    }

    #[test]
    fn runs_s16le_to_null_stdout() {
        let path = write_temp_bytes("raw-pcm-null", "raw", &[0, 0, 1, 0, 2, 0, 3, 0]);
        let path_arg = path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-f",
            "pcm_s16le",
            "-ar",
            "44100",
            "-ac",
            "1",
            "-i",
            path_arg.as_str(),
            "-f",
            "null",
            "-",
        ]))
        .expect("raw PCM command path should execute");

        let _ = fs::remove_file(&path);

        assert_eq!(output.output_format(), Some("null"));
        assert_eq!(output.packet_count(), 1);
        assert_eq!(output.byte_count(), 8);
        assert!(output.stdout().is_empty());
        assert!(output.stderr().is_empty());
    }

    #[test]
    fn runs_s16le_to_hash_stdout_with_md5_option() {
        let payload = [0, 0, 1, 0, 2, 0, 3, 0];
        let path = write_temp_bytes("raw-pcm-hash", "raw", &payload);
        let path_arg = path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-f",
            "s16le",
            "-ar",
            "48000",
            "-ac",
            "2",
            "-i",
            path_arg.as_str(),
            "-f",
            "hash",
            "-hash",
            "md5",
            "-",
        ]))
        .expect("raw PCM hash command path should execute");

        let _ = fs::remove_file(&path);

        assert_eq!(output.output_format(), Some("hash"));
        assert_eq!(output.packet_count(), 1);
        assert_eq!(output.byte_count(), u64::try_from(payload.len()).unwrap());
        assert!(output.stderr().is_empty());
        assert_eq!(
            output.stdout(),
            format!("MD5={}\n", avutil::digest_to_hex(&avutil::md5(&payload)))
        );
    }

    #[test]
    fn runs_s16le_to_hash_stdout_with_sha160_option() {
        let payload = [0, 0, 1, 0, 2, 0, 3, 0];
        let path = write_temp_bytes("raw-pcm-hash-sha160", "raw", &payload);
        let path_arg = path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-f",
            "s16le",
            "-ar",
            "48000",
            "-ac",
            "2",
            "-i",
            path_arg.as_str(),
            "-f",
            "hash",
            "-hash",
            "sha-160",
            "-",
        ]))
        .expect("raw PCM SHA-160 hash command path should execute");

        let _ = fs::remove_file(&path);

        assert_eq!(output.output_format(), Some("hash"));
        assert_eq!(output.packet_count(), 1);
        assert_eq!(output.byte_count(), u64::try_from(payload.len()).unwrap());
        assert!(output.stderr().is_empty());
        assert_eq!(
            output.stdout(),
            format!(
                "SHA160={}\n",
                avutil::digest_to_hex(&avutil::sha1(&payload))
            )
        );
    }

    #[test]
    fn runs_s16le_to_streamhash_stdout_with_md5_option() {
        let payload = [0, 0, 1, 0, 2, 0, 3, 0];
        let path = write_temp_bytes("raw-pcm-streamhash", "raw", &payload);
        let path_arg = path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-f",
            "s16le",
            "-ar",
            "48000",
            "-ac",
            "2",
            "-i",
            path_arg.as_str(),
            "-f",
            "streamhash",
            "-hash",
            "md5",
            "-",
        ]))
        .expect("raw PCM streamhash command path should execute");

        let _ = fs::remove_file(&path);

        assert_eq!(output.output_format(), Some("streamhash"));
        assert_eq!(output.packet_count(), 1);
        assert_eq!(output.byte_count(), u64::try_from(payload.len()).unwrap());
        assert!(output.stderr().is_empty());
        assert_eq!(
            output.stdout(),
            format!(
                "0,a,MD5={}\n",
                avutil::digest_to_hex(&avutil::md5(&payload))
            )
        );
    }

    #[test]
    fn rejects_unknown_hash_algorithm() {
        let path = write_temp_mov("unknown-hash", &sampled_mov_file(&[b"abc"], &[1_000]));
        let path_arg = path.to_string_lossy().into_owned();

        let err = ffmpeg_output(&strings(&[
            "-i",
            path_arg.as_str(),
            "-f",
            "hash",
            "-hash",
            "sha999",
            "-",
        ]))
        .expect_err("unknown hash algorithm should fail");

        let _ = fs::remove_file(&path);

        assert!(err.message().contains("hash algorithm `sha999`"));
    }

    #[test]
    fn rejects_hash_option_for_md5_muxer() {
        let path = write_temp_mov("md5-hash-option", &sampled_mov_file(&[b"abc"], &[1_000]));
        let path_arg = path.to_string_lossy().into_owned();

        let err = ffmpeg_output(&strings(&[
            "-i",
            path_arg.as_str(),
            "-f",
            "md5",
            "-hash",
            "sha256",
            "-",
        ]))
        .expect_err("md5 muxer should not accept hash option");

        let _ = fs::remove_file(&path);

        assert!(err.message().contains("output option `-hash`"));
    }

    #[test]
    fn rejects_hash_option_for_framemd5_muxer() {
        let path = write_temp_mov(
            "framemd5-hash-option",
            &sampled_mov_file(&[b"abc"], &[1_000]),
        );
        let path_arg = path.to_string_lossy().into_owned();

        let err = ffmpeg_output(&strings(&[
            "-i",
            path_arg.as_str(),
            "-f",
            "framemd5",
            "-hash",
            "sha256",
            "-",
        ]))
        .expect_err("framemd5 muxer should not accept hash option");

        let _ = fs::remove_file(&path);

        assert!(err.message().contains("output option `-hash`"));
    }

    #[test]
    fn runs_s16le_to_s16le_file_output() {
        let payload = [0, 0, 1, 0, 2, 0, 3, 0, 4, 0, 5, 0];
        let input_path = write_temp_bytes("raw-pcm-file-input", "raw", &payload);
        let output_path = unique_temp_path("raw-pcm-file-output", "raw");
        let input_arg = input_path.to_string_lossy().into_owned();
        let output_arg = output_path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-f",
            "s16le",
            "-ar",
            "48000",
            "-ac",
            "2",
            "-i",
            input_arg.as_str(),
            "-f",
            "s16le",
            output_arg.as_str(),
        ]))
        .expect("raw PCM file output path should execute");
        let written = fs::read(&output_path).expect("raw PCM output file should be readable");

        remove_temp_files(&[input_path, output_path]);

        assert_eq!(output.output_format(), Some("s16le"));
        assert_eq!(output.packet_count(), 1);
        assert_eq!(output.byte_count(), u64::try_from(payload.len()).unwrap());
        assert!(output.stdout().is_empty());
        assert!(output.stderr().is_empty());
        assert_eq!(written, payload);
    }

    #[test]
    fn rejects_s16le_file_output_overwrite() {
        let input_path = write_temp_bytes("raw-pcm-overwrite-input", "raw", &[0, 0, 1, 0]);
        let output_path = write_temp_bytes("raw-pcm-overwrite-output", "raw", b"existing");
        let input_arg = input_path.to_string_lossy().into_owned();
        let output_arg = output_path.to_string_lossy().into_owned();

        let err = ffmpeg_output(&strings(&[
            "-f",
            "s16le",
            "-ar",
            "44100",
            "-ac",
            "1",
            "-i",
            input_arg.as_str(),
            "-f",
            "s16le",
            output_arg.as_str(),
        ]))
        .expect_err("raw PCM file output should not overwrite existing files");
        let existing = fs::read(&output_path).expect("existing output file should remain readable");

        remove_temp_files(&[input_path, output_path]);

        assert!(err.message().contains("failed to create output"));
        assert_eq!(existing, b"existing");
    }

    #[test]
    fn runs_s16le_to_wav_file_output() {
        let payload = [0, 0, 1, 0, 2, 0, 3, 0, 4, 0, 5, 0];
        let input_path = write_temp_bytes("raw-pcm-wav-input", "raw", &payload);
        let output_path = unique_temp_path("raw-pcm-wav-output", "wav");
        let input_arg = input_path.to_string_lossy().into_owned();
        let output_arg = output_path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-f",
            "s16le",
            "-ar",
            "48000",
            "-ac",
            "2",
            "-i",
            input_arg.as_str(),
            "-f",
            "wav",
            output_arg.as_str(),
        ]))
        .expect("raw PCM to WAV file output path should execute");
        let written = fs::read(&output_path).expect("WAV output file should be readable");
        let mut demuxer =
            WavDemuxer::open(&written).expect("WAV output should parse with Rust demuxer");
        let packet = demuxer
            .read_packet()
            .expect("WAV packet read should succeed")
            .expect("WAV output should contain one packet");

        remove_temp_files(&[input_path, output_path]);

        assert_eq!(output.output_format(), Some("wav"));
        assert_eq!(output.packet_count(), 1);
        assert_eq!(output.byte_count(), u64::try_from(written.len()).unwrap());
        assert!(output.stdout().is_empty());
        assert!(output.stderr().is_empty());
        assert_eq!(demuxer.info().channels(), 2);
        assert_eq!(demuxer.info().sample_rate(), 48_000);
        assert_eq!(packet.data(), payload);
        assert_eq!(packet.duration(), 3);
        assert!(demuxer.read_packet().unwrap().is_none());
    }

    #[test]
    fn rejects_wav_file_output_overwrite() {
        let input_path = write_temp_bytes("raw-pcm-wav-overwrite-input", "raw", &[0, 0, 1, 0]);
        let output_path = write_temp_bytes("raw-pcm-wav-overwrite-output", "wav", b"existing");
        let input_arg = input_path.to_string_lossy().into_owned();
        let output_arg = output_path.to_string_lossy().into_owned();

        let err = ffmpeg_output(&strings(&[
            "-f",
            "s16le",
            "-ar",
            "44100",
            "-ac",
            "1",
            "-i",
            input_arg.as_str(),
            "-f",
            "wav",
            output_arg.as_str(),
        ]))
        .expect_err("WAV file output should not overwrite existing files");
        let existing = fs::read(&output_path).expect("existing output file should remain readable");

        remove_temp_files(&[input_path, output_path]);

        assert!(err.message().contains("failed to create output"));
        assert_eq!(existing, b"existing");
    }

    #[test]
    fn rejects_s16le_without_required_stream_parameters() {
        let path = write_temp_bytes("raw-pcm-missing-params", "raw", &[0, 0, 1, 0]);
        let path_arg = path.to_string_lossy().into_owned();

        let missing_sample_rate = ffmpeg_output(&strings(&[
            "-f",
            "s16le",
            "-ac",
            "1",
            "-i",
            path_arg.as_str(),
            "-f",
            "framecrc",
            "-",
        ]))
        .expect_err("raw PCM input requires a sample rate");
        let missing_channels = ffmpeg_output(&strings(&[
            "-f",
            "s16le",
            "-ar",
            "48000",
            "-i",
            path_arg.as_str(),
            "-f",
            "framecrc",
            "-",
        ]))
        .expect_err("raw PCM input requires a channel count");

        let _ = fs::remove_file(&path);

        assert!(missing_sample_rate.message().contains("sample rate"));
        assert!(missing_channels.message().contains("channel count"));
    }

    #[test]
    fn rejects_s16le_partial_sample_frame() {
        let path = write_temp_bytes("raw-pcm-partial", "raw", &[0, 0, 1]);
        let path_arg = path.to_string_lossy().into_owned();

        let err = ffmpeg_output(&strings(&[
            "-f",
            "s16le",
            "-ar",
            "48000",
            "-ac",
            "1",
            "-i",
            path_arg.as_str(),
            "-f",
            "framecrc",
            "-",
        ]))
        .expect_err("raw PCM input must contain whole sample frames");

        let _ = fs::remove_file(&path);

        assert!(err.message().contains("partial sample frame"));
    }

    #[test]
    fn runs_rawvideo_to_framecrc_stdout() {
        let path = write_temp_bytes(
            "rawvideo-framecrc",
            "raw",
            &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
        );
        let path_arg = path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgb24",
            "-s",
            "2x1",
            "-r",
            "30",
            "-i",
            path_arg.as_str(),
            "-f",
            "framecrc",
            "-",
        ]))
        .expect("rawvideo command path should execute");

        let _ = fs::remove_file(&path);

        assert_eq!(output.output_format(), Some("framecrc"));
        assert_eq!(output.packet_count(), 2);
        assert_eq!(output.byte_count(), 12);
        assert!(output.stderr().is_empty());
        assert!(output
            .stdout()
            .contains("stream=0 pts=0 dts=0 duration=1 size=6"));
        assert!(output
            .stdout()
            .contains("stream=0 pts=1 dts=1 duration=1 size=6"));
    }

    #[test]
    fn runs_rawvideo_to_null_stdout() {
        let path = write_temp_bytes("rawvideo-null", "raw", &[0, 1, 2, 3]);
        let path_arg = path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-f",
            "rawvideo",
            "-pix_fmt",
            "gray8",
            "-s",
            "2x2",
            "-r",
            "1/1",
            "-i",
            path_arg.as_str(),
            "-f",
            "null",
            "-",
        ]))
        .expect("rawvideo command path should execute");

        let _ = fs::remove_file(&path);

        assert_eq!(output.output_format(), Some("null"));
        assert_eq!(output.packet_count(), 1);
        assert_eq!(output.byte_count(), 4);
        assert!(output.stdout().is_empty());
        assert!(output.stderr().is_empty());
    }

    #[test]
    fn runs_rawvideo_yuv420p10le_to_null_stdout() {
        let payload = (0_u8..24).collect::<Vec<_>>();
        let path = write_temp_bytes("rawvideo-yuv420p10le-null", "raw", &payload);
        let path_arg = path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-f",
            "rawvideo",
            "-pix_fmt",
            "yuv420p10le",
            "-s",
            "4x2",
            "-r",
            "25",
            "-i",
            path_arg.as_str(),
            "-f",
            "null",
            "-",
        ]))
        .expect("rawvideo yuv420p10le command path should execute");

        let _ = fs::remove_file(&path);

        assert_eq!(output.output_format(), Some("null"));
        assert_eq!(output.packet_count(), 1);
        assert_eq!(output.byte_count(), 24);
        assert!(output.stdout().is_empty());
        assert!(output.stderr().is_empty());
    }

    #[test]
    fn runs_rawvideo_bgra_to_null_stdout() {
        let path = write_temp_bytes("rawvideo-bgra-null", "raw", &[0, 1, 2, 3, 4, 5, 6, 7]);
        let path_arg = path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-f",
            "rawvideo",
            "-pix_fmt",
            "BGRA",
            "-s",
            "1x1",
            "-r",
            "25",
            "-i",
            path_arg.as_str(),
            "-f",
            "null",
            "-",
        ]))
        .expect("rawvideo BGRA command path should execute");

        let _ = fs::remove_file(&path);

        assert_eq!(output.output_format(), Some("null"));
        assert_eq!(output.packet_count(), 2);
        assert_eq!(output.byte_count(), 8);
        assert!(output.stdout().is_empty());
        assert!(output.stderr().is_empty());
    }

    #[test]
    fn runs_rawvideo_rgb0_to_null_stdout() {
        let path = write_temp_bytes("rawvideo-rgb0-null", "raw", &[0, 1, 2, 3, 4, 5, 6, 7]);
        let path_arg = path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgb0",
            "-s",
            "1x1",
            "-r",
            "25",
            "-i",
            path_arg.as_str(),
            "-f",
            "null",
            "-",
        ]))
        .expect("rawvideo rgb0 command path should execute");

        let _ = fs::remove_file(&path);

        assert_eq!(output.output_format(), Some("null"));
        assert_eq!(output.packet_count(), 2);
        assert_eq!(output.byte_count(), 8);
        assert!(output.stdout().is_empty());
        assert!(output.stderr().is_empty());
    }

    #[test]
    fn runs_rawvideo_rgb8_to_null_stdout() {
        let path = write_temp_bytes("rawvideo-rgb8-null", "raw", &[0, 1, 2, 3]);
        let path_arg = path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgb8",
            "-s",
            "2x2",
            "-r",
            "25",
            "-i",
            path_arg.as_str(),
            "-f",
            "null",
            "-",
        ]))
        .expect("rawvideo rgb8 command path should execute");

        let _ = fs::remove_file(&path);

        assert_eq!(output.output_format(), Some("null"));
        assert_eq!(output.packet_count(), 1);
        assert_eq!(output.byte_count(), 4);
        assert!(output.stdout().is_empty());
        assert!(output.stderr().is_empty());
    }

    #[test]
    fn runs_rawvideo_pal8_to_null_stdout() {
        let path = write_temp_bytes("rawvideo-pal8-null", "raw", &[0, 1, 2, 3]);
        let path_arg = path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-f",
            "rawvideo",
            "-pix_fmt",
            "pal8",
            "-s",
            "2x2",
            "-r",
            "25",
            "-i",
            path_arg.as_str(),
            "-f",
            "null",
            "-",
        ]))
        .expect("rawvideo pal8 command path should execute");

        let _ = fs::remove_file(&path);

        assert_eq!(output.output_format(), Some("null"));
        assert_eq!(output.packet_count(), 1);
        assert_eq!(output.byte_count(), 4);
        assert!(output.stdout().is_empty());
        assert!(output.stderr().is_empty());
    }

    #[test]
    fn runs_rawvideo_rgb4_to_null_stdout() {
        let path = write_temp_bytes("rawvideo-rgb4-null", "raw", &[0, 1, 2, 3]);
        let path_arg = path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgb4",
            "-s",
            "3x2",
            "-r",
            "25",
            "-i",
            path_arg.as_str(),
            "-f",
            "null",
            "-",
        ]))
        .expect("rawvideo rgb4 command path should execute");

        let _ = fs::remove_file(&path);

        assert_eq!(output.output_format(), Some("null"));
        assert_eq!(output.packet_count(), 1);
        assert_eq!(output.byte_count(), 4);
        assert!(output.stdout().is_empty());
        assert!(output.stderr().is_empty());
    }

    #[test]
    fn runs_rawvideo_monow_to_null_stdout() {
        let path = write_temp_bytes("rawvideo-monow-null", "raw", &[0x80, 0x01, 0xff, 0x00]);
        let path_arg = path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-f",
            "rawvideo",
            "-pix_fmt",
            "monow",
            "-s",
            "9x2",
            "-r",
            "25",
            "-i",
            path_arg.as_str(),
            "-f",
            "null",
            "-",
        ]))
        .expect("rawvideo monow command path should execute");

        let _ = fs::remove_file(&path);

        assert_eq!(output.output_format(), Some("null"));
        assert_eq!(output.packet_count(), 1);
        assert_eq!(output.byte_count(), 4);
        assert!(output.stdout().is_empty());
        assert!(output.stderr().is_empty());
    }

    #[test]
    fn runs_rawvideo_rgb565le_to_null_stdout() {
        let path = write_temp_bytes("rawvideo-rgb565le-null", "raw", &[0, 1, 2, 3]);
        let path_arg = path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgb565le",
            "-s",
            "1x1",
            "-r",
            "25",
            "-i",
            path_arg.as_str(),
            "-f",
            "null",
            "-",
        ]))
        .expect("rawvideo rgb565le command path should execute");

        let _ = fs::remove_file(&path);

        assert_eq!(output.output_format(), Some("null"));
        assert_eq!(output.packet_count(), 2);
        assert_eq!(output.byte_count(), 4);
        assert!(output.stdout().is_empty());
        assert!(output.stderr().is_empty());
    }

    #[test]
    fn runs_rawvideo_yuyv422_to_null_stdout() {
        let path = write_temp_bytes("rawvideo-yuyv422-null", "raw", &[0, 1, 2, 3, 4, 5, 6, 7]);
        let path_arg = path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-f",
            "rawvideo",
            "-pix_fmt",
            "yuyv422",
            "-s",
            "2x1",
            "-r",
            "25",
            "-i",
            path_arg.as_str(),
            "-f",
            "null",
            "-",
        ]))
        .expect("rawvideo yuyv422 command path should execute");

        let _ = fs::remove_file(&path);

        assert_eq!(output.output_format(), Some("null"));
        assert_eq!(output.packet_count(), 2);
        assert_eq!(output.byte_count(), 8);
        assert!(output.stdout().is_empty());
        assert!(output.stderr().is_empty());
    }

    #[test]
    fn runs_rawvideo_nv12_to_null_stdout() {
        let path = write_temp_bytes("rawvideo-nv12-null", "raw", &(0..12).collect::<Vec<_>>());
        let path_arg = path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-f",
            "rawvideo",
            "-pix_fmt",
            "nv12",
            "-s",
            "4x2",
            "-r",
            "25",
            "-i",
            path_arg.as_str(),
            "-f",
            "null",
            "-",
        ]))
        .expect("rawvideo nv12 command path should execute");

        let _ = fs::remove_file(&path);

        assert_eq!(output.output_format(), Some("null"));
        assert_eq!(output.packet_count(), 1);
        assert_eq!(output.byte_count(), 12);
        assert!(output.stdout().is_empty());
        assert!(output.stderr().is_empty());
    }

    #[test]
    fn runs_rawvideo_gray16le_to_null_stdout() {
        let path = write_temp_bytes("rawvideo-gray16le-null", "raw", &[0, 1, 2, 3]);
        let path_arg = path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-f",
            "rawvideo",
            "-pix_fmt",
            "gray16le",
            "-s",
            "1x1",
            "-r",
            "25",
            "-i",
            path_arg.as_str(),
            "-f",
            "null",
            "-",
        ]))
        .expect("rawvideo gray16le command path should execute");

        let _ = fs::remove_file(&path);

        assert_eq!(output.output_format(), Some("null"));
        assert_eq!(output.packet_count(), 2);
        assert_eq!(output.byte_count(), 4);
        assert!(output.stdout().is_empty());
        assert!(output.stderr().is_empty());
    }

    #[test]
    fn runs_rawvideo_gray10le_alias_to_null_stdout() {
        let path = write_temp_bytes("rawvideo-gray10le-null", "raw", &[0, 1, 2, 3]);
        let path_arg = path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-f",
            "rawvideo",
            "-pix_fmt",
            "y10le",
            "-s",
            "1x1",
            "-r",
            "25",
            "-i",
            path_arg.as_str(),
            "-f",
            "null",
            "-",
        ]))
        .expect("rawvideo y10le command path should execute");

        let _ = fs::remove_file(&path);

        assert_eq!(output.output_format(), Some("null"));
        assert_eq!(output.packet_count(), 2);
        assert_eq!(output.byte_count(), 4);
        assert!(output.stdout().is_empty());
        assert!(output.stderr().is_empty());
    }

    #[test]
    fn runs_rawvideo_ya8_to_null_stdout() {
        let path = write_temp_bytes("rawvideo-ya8-null", "raw", &[0x10, 0xff, 0x80, 0x40]);
        let path_arg = path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-f",
            "rawvideo",
            "-pix_fmt",
            "ya8",
            "-s",
            "1x1",
            "-r",
            "25",
            "-i",
            path_arg.as_str(),
            "-f",
            "null",
            "-",
        ]))
        .expect("rawvideo ya8 command path should execute");

        let _ = fs::remove_file(&path);

        assert_eq!(output.output_format(), Some("null"));
        assert_eq!(output.packet_count(), 2);
        assert_eq!(output.byte_count(), 4);
        assert!(output.stdout().is_empty());
        assert!(output.stderr().is_empty());
    }

    #[test]
    fn runs_rawvideo_ya16le_to_null_stdout() {
        let path = write_temp_bytes(
            "rawvideo-ya16le-null",
            "raw",
            &[0x10, 0x00, 0xff, 0xff, 0x80, 0x00, 0x40, 0x00],
        );
        let path_arg = path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-f",
            "rawvideo",
            "-pix_fmt",
            "ya16le",
            "-s",
            "1x1",
            "-r",
            "25",
            "-i",
            path_arg.as_str(),
            "-f",
            "null",
            "-",
        ]))
        .expect("rawvideo ya16le command path should execute");

        let _ = fs::remove_file(&path);

        assert_eq!(output.output_format(), Some("null"));
        assert_eq!(output.packet_count(), 2);
        assert_eq!(output.byte_count(), 8);
        assert!(output.stdout().is_empty());
        assert!(output.stderr().is_empty());
    }

    #[test]
    fn runs_rawvideo_gray32le_to_null_stdout() {
        let path = write_temp_bytes("rawvideo-gray32le-null", "raw", &[0, 1, 2, 3, 4, 5, 6, 7]);
        let path_arg = path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-f",
            "rawvideo",
            "-pix_fmt",
            "gray32le",
            "-s",
            "1x1",
            "-r",
            "25",
            "-i",
            path_arg.as_str(),
            "-f",
            "null",
            "-",
        ]))
        .expect("rawvideo gray32le command path should execute");

        let _ = fs::remove_file(&path);

        assert_eq!(output.output_format(), Some("null"));
        assert_eq!(output.packet_count(), 2);
        assert_eq!(output.byte_count(), 8);
        assert!(output.stdout().is_empty());
        assert!(output.stderr().is_empty());
    }

    #[test]
    fn runs_rawvideo_grayf32le_to_null_stdout() {
        let path = write_temp_bytes("rawvideo-grayf32le-null", "raw", &[0, 1, 2, 3, 4, 5, 6, 7]);
        let path_arg = path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-f",
            "rawvideo",
            "-pix_fmt",
            "grayf32le",
            "-s",
            "1x1",
            "-r",
            "25",
            "-i",
            path_arg.as_str(),
            "-f",
            "null",
            "-",
        ]))
        .expect("rawvideo grayf32le command path should execute");

        let _ = fs::remove_file(&path);

        assert_eq!(output.output_format(), Some("null"));
        assert_eq!(output.packet_count(), 2);
        assert_eq!(output.byte_count(), 8);
        assert!(output.stdout().is_empty());
        assert!(output.stderr().is_empty());
    }

    #[test]
    fn runs_rawvideo_gbrp_to_null_stdout() {
        let path = write_temp_bytes(
            "rawvideo-gbrp-null",
            "raw",
            &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
        );
        let path_arg = path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-f",
            "rawvideo",
            "-pix_fmt",
            "gbrp",
            "-s",
            "2x1",
            "-r",
            "25",
            "-i",
            path_arg.as_str(),
            "-f",
            "null",
            "-",
        ]))
        .expect("rawvideo gbrp command path should execute");

        let _ = fs::remove_file(&path);

        assert_eq!(output.output_format(), Some("null"));
        assert_eq!(output.packet_count(), 2);
        assert_eq!(output.byte_count(), 12);
        assert!(output.stdout().is_empty());
        assert!(output.stderr().is_empty());
    }

    #[test]
    fn runs_rawvideo_gbrp9le_to_null_stdout() {
        let payload = (0_u8..24).collect::<Vec<_>>();
        let path = write_temp_bytes("rawvideo-gbrp9le-null", "raw", &payload);
        let path_arg = path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-f",
            "rawvideo",
            "-pix_fmt",
            "gbrp9le",
            "-s",
            "2x1",
            "-r",
            "25",
            "-i",
            path_arg.as_str(),
            "-f",
            "null",
            "-",
        ]))
        .expect("rawvideo gbrp9le command path should execute");

        let _ = fs::remove_file(&path);

        assert_eq!(output.output_format(), Some("null"));
        assert_eq!(output.packet_count(), 2);
        assert_eq!(output.byte_count(), 24);
        assert!(output.stdout().is_empty());
        assert!(output.stderr().is_empty());
    }

    #[test]
    fn runs_rawvideo_gbrp10le_to_null_stdout() {
        let payload = (0_u8..24).collect::<Vec<_>>();
        let path = write_temp_bytes("rawvideo-gbrp10le-null", "raw", &payload);
        let path_arg = path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-f",
            "rawvideo",
            "-pix_fmt",
            "gbrp10le",
            "-s",
            "2x1",
            "-r",
            "25",
            "-i",
            path_arg.as_str(),
            "-f",
            "null",
            "-",
        ]))
        .expect("rawvideo gbrp10le command path should execute");

        let _ = fs::remove_file(&path);

        assert_eq!(output.output_format(), Some("null"));
        assert_eq!(output.packet_count(), 2);
        assert_eq!(output.byte_count(), 24);
        assert!(output.stdout().is_empty());
        assert!(output.stderr().is_empty());
    }

    #[test]
    fn runs_rawvideo_gbrp12le_to_null_stdout() {
        let payload = (0_u8..24).collect::<Vec<_>>();
        let path = write_temp_bytes("rawvideo-gbrp12le-null", "raw", &payload);
        let path_arg = path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-f",
            "rawvideo",
            "-pix_fmt",
            "gbrp12le",
            "-s",
            "2x1",
            "-r",
            "25",
            "-i",
            path_arg.as_str(),
            "-f",
            "null",
            "-",
        ]))
        .expect("rawvideo gbrp12le command path should execute");

        let _ = fs::remove_file(&path);

        assert_eq!(output.output_format(), Some("null"));
        assert_eq!(output.packet_count(), 2);
        assert_eq!(output.byte_count(), 24);
        assert!(output.stdout().is_empty());
        assert!(output.stderr().is_empty());
    }

    #[test]
    fn runs_rawvideo_gbrp14le_to_null_stdout() {
        let payload = (0_u8..24).collect::<Vec<_>>();
        let path = write_temp_bytes("rawvideo-gbrp14le-null", "raw", &payload);
        let path_arg = path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-f",
            "rawvideo",
            "-pix_fmt",
            "gbrp14le",
            "-s",
            "2x1",
            "-r",
            "25",
            "-i",
            path_arg.as_str(),
            "-f",
            "null",
            "-",
        ]))
        .expect("rawvideo gbrp14le command path should execute");

        let _ = fs::remove_file(&path);

        assert_eq!(output.output_format(), Some("null"));
        assert_eq!(output.packet_count(), 2);
        assert_eq!(output.byte_count(), 24);
        assert!(output.stdout().is_empty());
        assert!(output.stderr().is_empty());
    }

    #[test]
    fn runs_rawvideo_gbrp16le_to_null_stdout() {
        let payload = (0_u8..24).collect::<Vec<_>>();
        let path = write_temp_bytes("rawvideo-gbrp16le-null", "raw", &payload);
        let path_arg = path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-f",
            "rawvideo",
            "-pix_fmt",
            "gbrp16le",
            "-s",
            "2x1",
            "-r",
            "25",
            "-i",
            path_arg.as_str(),
            "-f",
            "null",
            "-",
        ]))
        .expect("rawvideo gbrp16le command path should execute");

        let _ = fs::remove_file(&path);

        assert_eq!(output.output_format(), Some("null"));
        assert_eq!(output.packet_count(), 2);
        assert_eq!(output.byte_count(), 24);
        assert!(output.stdout().is_empty());
        assert!(output.stderr().is_empty());
    }

    #[test]
    fn runs_rawvideo_gbrapf32le_to_null_stdout() {
        let payload = (0_u8..32).collect::<Vec<_>>();
        let path = write_temp_bytes("rawvideo-gbrapf32le-null", "raw", &payload);
        let path_arg = path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-f",
            "rawvideo",
            "-pix_fmt",
            "gbrapf32le",
            "-s",
            "1x1",
            "-r",
            "25",
            "-i",
            path_arg.as_str(),
            "-f",
            "null",
            "-",
        ]))
        .expect("rawvideo gbrapf32le command path should execute");

        let _ = fs::remove_file(&path);

        assert_eq!(output.output_format(), Some("null"));
        assert_eq!(output.packet_count(), 2);
        assert_eq!(output.byte_count(), 32);
        assert!(output.stdout().is_empty());
        assert!(output.stderr().is_empty());
    }

    #[test]
    fn runs_rawvideo_rgb48le_to_null_stdout() {
        let path = write_temp_bytes(
            "rawvideo-rgb48le-null",
            "raw",
            &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
        );
        let path_arg = path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgb48le",
            "-s",
            "1x1",
            "-r",
            "25",
            "-i",
            path_arg.as_str(),
            "-f",
            "null",
            "-",
        ]))
        .expect("rawvideo rgb48le command path should execute");

        let _ = fs::remove_file(&path);

        assert_eq!(output.output_format(), Some("null"));
        assert_eq!(output.packet_count(), 2);
        assert_eq!(output.byte_count(), 12);
        assert!(output.stdout().is_empty());
        assert!(output.stderr().is_empty());
    }

    #[test]
    fn runs_rawvideo_rgba64le_to_null_stdout() {
        let path = write_temp_bytes(
            "rawvideo-rgba64le-null",
            "raw",
            &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
        );
        let path_arg = path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgba64le",
            "-s",
            "1x1",
            "-r",
            "25",
            "-i",
            path_arg.as_str(),
            "-f",
            "null",
            "-",
        ]))
        .expect("rawvideo rgba64le command path should execute");

        let _ = fs::remove_file(&path);

        assert_eq!(output.output_format(), Some("null"));
        assert_eq!(output.packet_count(), 2);
        assert_eq!(output.byte_count(), 16);
        assert!(output.stdout().is_empty());
        assert!(output.stderr().is_empty());
    }

    #[test]
    fn runs_rawvideo_yuv422p_to_null_stdout() {
        let path = write_temp_bytes("rawvideo-yuv422p-null", "raw", &[0, 1, 2, 3, 4, 5, 6, 7]);
        let path_arg = path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-f",
            "rawvideo",
            "-pix_fmt",
            "yuv422p",
            "-s",
            "2x1",
            "-r",
            "25",
            "-i",
            path_arg.as_str(),
            "-f",
            "null",
            "-",
        ]))
        .expect("rawvideo yuv422p command path should execute");

        let _ = fs::remove_file(&path);

        assert_eq!(output.output_format(), Some("null"));
        assert_eq!(output.packet_count(), 2);
        assert_eq!(output.byte_count(), 8);
        assert!(output.stdout().is_empty());
        assert!(output.stderr().is_empty());
    }

    #[test]
    fn runs_rawvideo_yuvj420p_to_null_stdout() {
        let payload = (0_u8..12).collect::<Vec<_>>();
        let path = write_temp_bytes("rawvideo-yuvj420p-null", "raw", &payload);
        let path_arg = path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-f",
            "rawvideo",
            "-pix_fmt",
            "yuvj420p",
            "-s",
            "4x2",
            "-r",
            "25",
            "-i",
            path_arg.as_str(),
            "-f",
            "null",
            "-",
        ]))
        .expect("rawvideo yuvj420p null path should execute");

        remove_temp_files(&[path]);

        assert_eq!(output.output_format(), Some("null"));
        assert_eq!(output.packet_count(), 1);
        assert_eq!(output.byte_count(), 12);
        assert!(output.stdout().is_empty());
        assert!(output.stderr().is_empty());
    }

    #[test]
    fn runs_rawvideo_yuv411p_to_null_stdout() {
        let path = write_temp_bytes("rawvideo-yuv411p-null", "raw", &[0, 1, 2, 3, 4, 5]);
        let path_arg = path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-f",
            "rawvideo",
            "-pix_fmt",
            "yuv411p",
            "-s",
            "4x1",
            "-r",
            "25",
            "-i",
            path_arg.as_str(),
            "-f",
            "null",
            "-",
        ]))
        .expect("rawvideo yuv411p command path should execute");

        let _ = fs::remove_file(&path);

        assert_eq!(output.output_format(), Some("null"));
        assert_eq!(output.packet_count(), 1);
        assert_eq!(output.byte_count(), 6);
        assert!(output.stdout().is_empty());
        assert!(output.stderr().is_empty());
    }

    #[test]
    fn runs_rawvideo_yuv410p_to_null_stdout() {
        let payload = (0_u8..18).collect::<Vec<_>>();
        let path = write_temp_bytes("rawvideo-yuv410p-null", "raw", &payload);
        let path_arg = path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-f",
            "rawvideo",
            "-pix_fmt",
            "yuv410p",
            "-s",
            "4x4",
            "-r",
            "25",
            "-i",
            path_arg.as_str(),
            "-f",
            "null",
            "-",
        ]))
        .expect("rawvideo yuv410p command path should execute");

        let _ = fs::remove_file(&path);

        assert_eq!(output.output_format(), Some("null"));
        assert_eq!(output.packet_count(), 1);
        assert_eq!(output.byte_count(), 18);
        assert!(output.stdout().is_empty());
        assert!(output.stderr().is_empty());
    }

    #[test]
    fn runs_rawvideo_yuv440p_to_null_stdout() {
        let payload = (0_u8..12).collect::<Vec<_>>();
        let path = write_temp_bytes("rawvideo-yuv440p-null", "raw", &payload);
        let path_arg = path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-f",
            "rawvideo",
            "-pix_fmt",
            "yuv440p",
            "-s",
            "3x2",
            "-r",
            "25",
            "-i",
            path_arg.as_str(),
            "-f",
            "null",
            "-",
        ]))
        .expect("rawvideo yuv440p command path should execute");

        let _ = fs::remove_file(&path);

        assert_eq!(output.output_format(), Some("null"));
        assert_eq!(output.packet_count(), 1);
        assert_eq!(output.byte_count(), 12);
        assert!(output.stdout().is_empty());
        assert!(output.stderr().is_empty());
    }

    #[test]
    fn runs_rawvideo_yuv444p_to_null_stdout() {
        let path = write_temp_bytes(
            "rawvideo-yuv444p-null",
            "raw",
            &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
        );
        let path_arg = path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-f",
            "rawvideo",
            "-pix_fmt",
            "yuv444p",
            "-s",
            "2x1",
            "-r",
            "25",
            "-i",
            path_arg.as_str(),
            "-f",
            "null",
            "-",
        ]))
        .expect("rawvideo yuv444p command path should execute");

        let _ = fs::remove_file(&path);

        assert_eq!(output.output_format(), Some("null"));
        assert_eq!(output.packet_count(), 2);
        assert_eq!(output.byte_count(), 12);
        assert!(output.stdout().is_empty());
        assert!(output.stderr().is_empty());
    }

    #[test]
    fn runs_rawvideo_to_rawvideo_file_output() {
        let payload = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
        let input_path = write_temp_bytes("rawvideo-file-input", "raw", &payload);
        let output_path = unique_temp_path("rawvideo-file-output", "raw");
        let input_arg = input_path.to_string_lossy().into_owned();
        let output_arg = output_path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgb24",
            "-s",
            "2x1",
            "-r",
            "30",
            "-i",
            input_arg.as_str(),
            "-f",
            "rawvideo",
            output_arg.as_str(),
        ]))
        .expect("rawvideo file output path should execute");
        let written = fs::read(&output_path).expect("rawvideo output file should be readable");

        remove_temp_files(&[input_path, output_path]);

        assert_eq!(output.output_format(), Some("rawvideo"));
        assert_eq!(output.packet_count(), 2);
        assert_eq!(output.byte_count(), u64::try_from(payload.len()).unwrap());
        assert!(output.stdout().is_empty());
        assert!(output.stderr().is_empty());
        assert_eq!(written, payload);
    }

    #[test]
    fn runs_rawvideo_rgb24_to_avi_file_output() {
        let first = [0, 1, 2, 3, 4, 5];
        let second = [6, 7, 8, 9, 10, 11];
        let payload = [first.as_slice(), second.as_slice()].concat();
        let input_path = write_temp_bytes("rawvideo-avi-file-input", "raw", &payload);
        let output_path = unique_temp_path("rawvideo-avi-file-output", "avi");
        let input_arg = input_path.to_string_lossy().into_owned();
        let output_arg = output_path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgb24",
            "-s",
            "2x1",
            "-r",
            "25",
            "-i",
            input_arg.as_str(),
            "-f",
            "avi",
            output_arg.as_str(),
        ]))
        .expect("rawvideo rgb24 to AVI file output path should execute");
        let written = fs::read(&output_path).expect("AVI output file should be readable");
        let mut demuxer = avformat::AviDemuxer::open(&written).expect("AVI output should parse");
        let first_packet = demuxer
            .read_packet()
            .expect("first AVI packet read should succeed")
            .expect("first frame should exist");
        let second_packet = demuxer
            .read_packet()
            .expect("second AVI packet read should succeed")
            .expect("second frame should exist");

        remove_temp_files(&[input_path, output_path]);

        assert_eq!(output.output_format(), Some("avi"));
        assert_eq!(output.packet_count(), 2);
        assert_eq!(output.byte_count(), u64::try_from(written.len()).unwrap());
        assert!(output.stdout().is_empty());
        assert!(output.stderr().is_empty());
        assert_eq!(demuxer.info().width(), 2);
        assert_eq!(demuxer.info().height(), 1);
        assert_eq!(demuxer.info().total_frames(), 2);
        assert_eq!(demuxer.info().packet_count(), 2);
        let stream = &demuxer.info().streams()[0];
        assert_eq!(stream.width(), 2);
        assert_eq!(stream.height(), 1);
        assert_eq!(stream.frame_rate(), Rational::new(25, 1).unwrap());
        assert_eq!(stream.bit_count(), 24);
        assert_eq!(stream.compression(), "BI_RGB");
        assert_eq!(first_packet.data(), first);
        assert_eq!(first_packet.pts(), Some(0));
        assert_eq!(second_packet.data(), second);
        assert_eq!(second_packet.pts(), Some(1));
        assert!(demuxer.read_packet().unwrap().is_none());
    }

    #[test]
    fn rejects_rawvideo_file_output_overwrite() {
        let input_path = write_temp_bytes("rawvideo-overwrite-input", "raw", &[0, 1, 2, 3]);
        let output_path = write_temp_bytes("rawvideo-overwrite-output", "raw", b"existing");
        let input_arg = input_path.to_string_lossy().into_owned();
        let output_arg = output_path.to_string_lossy().into_owned();

        let err = ffmpeg_output(&strings(&[
            "-f",
            "rawvideo",
            "-pix_fmt",
            "gray8",
            "-s",
            "2x2",
            "-r",
            "1",
            "-i",
            input_arg.as_str(),
            "-f",
            "rawvideo",
            output_arg.as_str(),
        ]))
        .expect_err("rawvideo file output should not overwrite existing files");
        let existing = fs::read(&output_path).expect("existing output file should remain readable");

        remove_temp_files(&[input_path, output_path]);

        assert!(err.message().contains("failed to create output"));
        assert_eq!(existing, b"existing");
    }

    #[test]
    fn rejects_avi_file_output_overwrite() {
        let input_path =
            write_temp_bytes("rawvideo-avi-overwrite-input", "raw", &[0, 1, 2, 3, 4, 5]);
        let output_path = write_temp_bytes("rawvideo-avi-overwrite-output", "avi", b"existing");
        let input_arg = input_path.to_string_lossy().into_owned();
        let output_arg = output_path.to_string_lossy().into_owned();

        let err = ffmpeg_output(&strings(&[
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgb24",
            "-s",
            "2x1",
            "-r",
            "25",
            "-i",
            input_arg.as_str(),
            "-f",
            "avi",
            output_arg.as_str(),
        ]))
        .expect_err("AVI file output should not overwrite existing files");
        let existing = fs::read(&output_path).expect("existing output file should remain readable");

        remove_temp_files(&[input_path, output_path]);

        assert!(err.message().contains("failed to create output"));
        assert_eq!(existing, b"existing");
    }

    #[test]
    fn rejects_avi_file_output_for_non_rgb24_rawvideo() {
        let input_path = write_temp_bytes("rawvideo-avi-yuv-input", "raw", &[0, 1, 2, 3, 4, 5]);
        let output_path = unique_temp_path("rawvideo-avi-yuv-output", "avi");
        let input_arg = input_path.to_string_lossy().into_owned();
        let output_arg = output_path.to_string_lossy().into_owned();

        let err = ffmpeg_output(&strings(&[
            "-f",
            "rawvideo",
            "-pix_fmt",
            "yuv420p",
            "-s",
            "2x2",
            "-r",
            "25",
            "-i",
            input_arg.as_str(),
            "-f",
            "avi",
            output_arg.as_str(),
        ]))
        .expect_err("AVI output should reject non-rgb24 rawvideo input");

        remove_temp_files(&[input_path, output_path]);

        assert!(err.message().contains("supports only rgb24"));
    }

    #[test]
    fn runs_rawvideo_yuv420p_to_yuv4mpegpipe_file_output() {
        let first = [0, 1, 2, 3, 4, 5];
        let second = [6, 7, 8, 9, 10, 11];
        let payload = [first.as_slice(), second.as_slice()].concat();
        let input_path = write_temp_bytes("rawvideo-y4m-file-input", "raw", &payload);
        let output_path = unique_temp_path("rawvideo-y4m-file-output", "y4m");
        let input_arg = input_path.to_string_lossy().into_owned();
        let output_arg = output_path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-f",
            "rawvideo",
            "-pix_fmt",
            "yuv420p",
            "-s",
            "2x2",
            "-r",
            "25",
            "-i",
            input_arg.as_str(),
            "-f",
            "yuv4mpegpipe",
            output_arg.as_str(),
        ]))
        .expect("rawvideo yuv420p to yuv4mpegpipe file output path should execute");
        let written = fs::read(&output_path).expect("YUV4MPEG2 output file should be readable");
        let mut demuxer = Yuv4MpegDemuxer::open(&written).expect("YUV4MPEG2 output should parse");
        let first_packet = demuxer
            .read_packet()
            .expect("first YUV4MPEG2 packet read should succeed")
            .expect("first frame should exist");
        let second_packet = demuxer
            .read_packet()
            .expect("second YUV4MPEG2 packet read should succeed")
            .expect("second frame should exist");

        remove_temp_files(&[input_path, output_path]);

        assert_eq!(output.output_format(), Some("yuv4mpegpipe"));
        assert_eq!(output.packet_count(), 2);
        assert_eq!(output.byte_count(), u64::try_from(written.len()).unwrap());
        assert!(output.stdout().is_empty());
        assert!(output.stderr().is_empty());
        assert_eq!(demuxer.info().width(), 2);
        assert_eq!(demuxer.info().height(), 2);
        assert_eq!(demuxer.info().frame_rate(), Rational::new(25, 1).unwrap());
        assert_eq!(first_packet.data(), first);
        assert_eq!(second_packet.data(), second);
        assert!(demuxer.read_packet().unwrap().is_none());
    }

    #[test]
    fn rejects_yuv4mpegpipe_file_output_overwrite() {
        let input_path =
            write_temp_bytes("rawvideo-y4m-overwrite-input", "raw", &[0, 1, 2, 3, 4, 5]);
        let output_path = write_temp_bytes("rawvideo-y4m-overwrite-output", "y4m", b"existing");
        let input_arg = input_path.to_string_lossy().into_owned();
        let output_arg = output_path.to_string_lossy().into_owned();

        let err = ffmpeg_output(&strings(&[
            "-f",
            "rawvideo",
            "-pix_fmt",
            "yuv420p",
            "-s",
            "2x2",
            "-r",
            "25",
            "-i",
            input_arg.as_str(),
            "-f",
            "yuv4mpegpipe",
            output_arg.as_str(),
        ]))
        .expect_err("YUV4MPEG2 file output should not overwrite existing files");
        let existing = fs::read(&output_path).expect("existing output file should remain readable");

        remove_temp_files(&[input_path, output_path]);

        assert!(err.message().contains("failed to create output"));
        assert_eq!(existing, b"existing");
    }

    #[test]
    fn rejects_yuv4mpegpipe_file_output_for_non_yuv420p_rawvideo() {
        let input_path = write_temp_bytes("rawvideo-y4m-rgb-input", "raw", &[0, 1, 2, 3, 4, 5]);
        let output_path = unique_temp_path("rawvideo-y4m-rgb-output", "y4m");
        let input_arg = input_path.to_string_lossy().into_owned();
        let output_arg = output_path.to_string_lossy().into_owned();

        let err = ffmpeg_output(&strings(&[
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgb24",
            "-s",
            "2x1",
            "-r",
            "25",
            "-i",
            input_arg.as_str(),
            "-f",
            "yuv4mpegpipe",
            output_arg.as_str(),
        ]))
        .expect_err("YUV4MPEG2 output should reject non-yuv420p rawvideo input");

        remove_temp_files(&[input_path, output_path]);

        assert!(err.message().contains("supports only yuv420p"));
    }

    #[test]
    fn rejects_rawvideo_without_required_stream_parameters() {
        let path = write_temp_bytes("rawvideo-missing-params", "raw", &[0, 1, 2, 3]);
        let path_arg = path.to_string_lossy().into_owned();

        let missing_size = ffmpeg_output(&strings(&[
            "-f",
            "rawvideo",
            "-pix_fmt",
            "gray8",
            "-r",
            "1",
            "-i",
            path_arg.as_str(),
            "-f",
            "framecrc",
            "-",
        ]))
        .expect_err("rawvideo input requires dimensions");
        let missing_pixel_format = ffmpeg_output(&strings(&[
            "-f",
            "rawvideo",
            "-s",
            "2x2",
            "-r",
            "1",
            "-i",
            path_arg.as_str(),
            "-f",
            "framecrc",
            "-",
        ]))
        .expect_err("rawvideo input requires a pixel format");
        let missing_frame_rate = ffmpeg_output(&strings(&[
            "-f",
            "rawvideo",
            "-pix_fmt",
            "gray8",
            "-s",
            "2x2",
            "-i",
            path_arg.as_str(),
            "-f",
            "framecrc",
            "-",
        ]))
        .expect_err("rawvideo input requires a frame rate");

        let _ = fs::remove_file(&path);

        assert!(missing_size.message().contains("dimensions"));
        assert!(missing_pixel_format.message().contains("pixel format"));
        assert!(missing_frame_rate.message().contains("frame rate"));
    }

    #[test]
    fn rejects_rawvideo_truncated_frame() {
        let path = write_temp_bytes("rawvideo-partial", "raw", &[0, 1, 2, 3, 4]);
        let path_arg = path.to_string_lossy().into_owned();

        let err = ffmpeg_output(&strings(&[
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgb24",
            "-s",
            "2x1",
            "-r",
            "30/1",
            "-i",
            path_arg.as_str(),
            "-f",
            "framecrc",
            "-",
        ]))
        .expect_err("rawvideo input must contain whole frames");

        let _ = fs::remove_file(&path);

        assert!(err.message().contains("partial frame"));
    }

    #[test]
    fn runs_image2_single_to_framecrc_stdout() {
        let path = write_temp_bytes("image2-framecrc", "png", b"\x89PNG\r\n\x1a\n");
        let path_arg = path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-f",
            "image2",
            "-framerate",
            "25",
            "-i",
            path_arg.as_str(),
            "-f",
            "framecrc",
            "-",
        ]))
        .expect("image2 command path should execute");

        let _ = fs::remove_file(&path);

        assert_eq!(output.output_format(), Some("framecrc"));
        assert_eq!(output.packet_count(), 1);
        assert_eq!(output.byte_count(), 8);
        assert!(output.stderr().is_empty());
        assert!(output
            .stdout()
            .contains("stream=0 pts=0 dts=0 duration=1 size=8"));
    }

    #[test]
    fn runs_image2_single_to_null_stdout() {
        let path = write_temp_bytes("image2-null", "jpg", b"jpeg bytes");
        let path_arg = path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-f",
            "image2",
            "-r",
            "1/1",
            "-i",
            path_arg.as_str(),
            "-f",
            "null",
            "-",
        ]))
        .expect("image2 command path should execute");

        let _ = fs::remove_file(&path);

        assert_eq!(output.output_format(), Some("null"));
        assert_eq!(output.packet_count(), 1);
        assert_eq!(output.byte_count(), 10);
        assert!(output.stdout().is_empty());
        assert!(output.stderr().is_empty());
    }

    #[test]
    fn runs_image2_single_to_image2_file_output() {
        let payload = b"\x89PNG\r\n\x1a\n";
        let input_path = write_temp_bytes("image2-file-input", "png", payload);
        let output_path = unique_temp_path("image2-file-output", "png");
        let input_arg = input_path.to_string_lossy().into_owned();
        let output_arg = output_path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-f",
            "image2",
            "-framerate",
            "25",
            "-i",
            input_arg.as_str(),
            "-f",
            "image2",
            output_arg.as_str(),
        ]))
        .expect("single image2 file output path should execute");
        let written = fs::read(&output_path).expect("image2 output file should be readable");

        remove_temp_files(&[input_path, output_path]);

        assert_eq!(output.output_format(), Some("image2"));
        assert_eq!(output.packet_count(), 1);
        assert_eq!(output.byte_count(), u64::try_from(payload.len()).unwrap());
        assert!(output.stdout().is_empty());
        assert!(output.stderr().is_empty());
        assert_eq!(written, payload);
    }

    #[test]
    fn runs_image2_sequence_to_framecrc_stdout() {
        let (pattern, paths) = write_temp_image2_sequence(
            "image2-sequence-framecrc",
            "png",
            &[
                (0, b"zero".as_slice()),
                (1, b"one".as_slice()),
                (2, b"three".as_slice()),
            ],
        );
        let pattern_arg = pattern.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-f",
            "image2",
            "-framerate",
            "25",
            "-i",
            pattern_arg.as_str(),
            "-f",
            "framecrc",
            "-",
        ]))
        .expect("image2 sequence command path should execute");

        remove_temp_files(&paths);

        assert_eq!(output.output_format(), Some("framecrc"));
        assert_eq!(output.packet_count(), 3);
        assert_eq!(output.byte_count(), 12);
        assert!(output.stderr().is_empty());
        assert!(output
            .stdout()
            .contains("stream=0 pts=0 dts=0 duration=1 size=4"));
        assert!(output
            .stdout()
            .contains("stream=0 pts=1 dts=1 duration=1 size=3"));
        assert!(output
            .stdout()
            .contains("stream=0 pts=2 dts=2 duration=1 size=5"));
    }

    #[test]
    fn runs_image2_sequence_to_null_stdout() {
        let (pattern, paths) = write_temp_image2_sequence(
            "image2-sequence-null",
            "jpg",
            &[(0, b"jpeg0".as_slice()), (1, b"jpeg1".as_slice())],
        );
        let pattern_arg = pattern.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-f",
            "image2",
            "-r",
            "1/1",
            "-i",
            pattern_arg.as_str(),
            "-f",
            "null",
            "-",
        ]))
        .expect("image2 sequence command path should execute");

        remove_temp_files(&paths);

        assert_eq!(output.output_format(), Some("null"));
        assert_eq!(output.packet_count(), 2);
        assert_eq!(output.byte_count(), 10);
        assert!(output.stdout().is_empty());
        assert!(output.stderr().is_empty());
    }

    #[test]
    fn runs_image2_sequence_with_input_start_number_to_framecrc_stdout() {
        let (pattern, paths) = write_temp_image2_sequence(
            "image2-sequence-input-start",
            "png",
            &[(5, b"five".as_slice()), (6, b"six".as_slice())],
        );
        let pattern_arg = pattern.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-f",
            "image2",
            "-framerate",
            "25",
            "-start_number",
            "5",
            "-i",
            pattern_arg.as_str(),
            "-f",
            "framecrc",
            "-",
        ]))
        .expect("image2 sequence with input start_number should execute");

        remove_temp_files(&paths);

        assert_eq!(output.output_format(), Some("framecrc"));
        assert_eq!(output.packet_count(), 2);
        assert_eq!(output.byte_count(), 7);
        assert!(output.stderr().is_empty());
        assert!(output
            .stdout()
            .contains("stream=0 pts=0 dts=0 duration=1 size=4"));
        assert!(output
            .stdout()
            .contains("stream=0 pts=1 dts=1 duration=1 size=3"));
    }

    #[test]
    fn runs_image2_sequence_to_image2_file_output() {
        let (input_pattern, input_paths) = write_temp_image2_sequence(
            "image2-sequence-file-input",
            "png",
            &[(0, b"zero".as_slice()), (1, b"one".as_slice())],
        );
        let output_pattern = unique_temp_path("image2-sequence-file-output-%03d", "png");
        let output_pattern_arg = output_pattern.to_string_lossy().into_owned();
        let output_pattern = Image2Pattern::parse(output_pattern_arg.clone())
            .expect("generated output pattern should parse");
        let output_zero = PathBuf::from(output_pattern.path_for_frame_number(0).unwrap());
        let output_one = PathBuf::from(output_pattern.path_for_frame_number(1).unwrap());
        let input_arg = input_pattern.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-f",
            "image2",
            "-framerate",
            "25",
            "-i",
            input_arg.as_str(),
            "-f",
            "image2",
            output_pattern_arg.as_str(),
        ]))
        .expect("image2 sequence file output path should execute");
        let written_zero =
            fs::read(&output_zero).expect("first image2 output file should be readable");
        let written_one =
            fs::read(&output_one).expect("second image2 output file should be readable");

        remove_temp_files(&input_paths);
        remove_temp_files(&[output_zero, output_one]);

        assert_eq!(output.output_format(), Some("image2"));
        assert_eq!(output.packet_count(), 2);
        assert_eq!(output.byte_count(), 7);
        assert!(output.stdout().is_empty());
        assert!(output.stderr().is_empty());
        assert_eq!(written_zero, b"zero");
        assert_eq!(written_one, b"one");
    }

    #[test]
    fn runs_image2_sequence_with_output_start_number_to_image2_file_output() {
        let (input_pattern, input_paths) = write_temp_image2_sequence(
            "image2-sequence-file-output-start-input",
            "png",
            &[(0, b"zero".as_slice()), (1, b"one".as_slice())],
        );
        let output_pattern = unique_temp_path("image2-sequence-file-output-start-%03d", "png");
        let output_pattern_arg = output_pattern.to_string_lossy().into_owned();
        let output_pattern = Image2Pattern::parse(output_pattern_arg.clone())
            .expect("generated output pattern should parse");
        let output_seven = PathBuf::from(output_pattern.path_for_frame_number(7).unwrap());
        let output_eight = PathBuf::from(output_pattern.path_for_frame_number(8).unwrap());
        let input_arg = input_pattern.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-f",
            "image2",
            "-framerate",
            "25",
            "-i",
            input_arg.as_str(),
            "-f",
            "image2",
            "-start_number",
            "7",
            output_pattern_arg.as_str(),
        ]))
        .expect("image2 sequence output start_number should control generated filenames");
        let written_seven =
            fs::read(&output_seven).expect("first numbered image2 output should be readable");
        let written_eight =
            fs::read(&output_eight).expect("second numbered image2 output should be readable");

        remove_temp_files(&input_paths);
        remove_temp_files(&[output_seven, output_eight]);

        assert_eq!(output.output_format(), Some("image2"));
        assert_eq!(output.packet_count(), 2);
        assert_eq!(output.byte_count(), 7);
        assert!(output.stdout().is_empty());
        assert!(output.stderr().is_empty());
        assert_eq!(written_seven, b"zero");
        assert_eq!(written_eight, b"one");
    }

    #[test]
    fn rejects_image2_file_output_overwrite() {
        let input_path = write_temp_bytes("image2-overwrite-input", "png", b"new-png");
        let output_path = write_temp_bytes("image2-overwrite-output", "png", b"existing");
        let input_arg = input_path.to_string_lossy().into_owned();
        let output_arg = output_path.to_string_lossy().into_owned();

        let err = ffmpeg_output(&strings(&[
            "-f",
            "image2",
            "-framerate",
            "1",
            "-i",
            input_arg.as_str(),
            "-f",
            "image2",
            output_arg.as_str(),
        ]))
        .expect_err("image2 file output should not overwrite existing files");
        let existing = fs::read(&output_path).expect("existing output file should remain readable");

        remove_temp_files(&[input_path, output_path]);

        assert!(err.message().contains("failed to create output"));
        assert_eq!(existing, b"existing");
    }

    #[test]
    fn rejects_image2_sequence_missing_frame() {
        let (pattern, paths) = write_temp_image2_sequence(
            "image2-sequence-gap",
            "png",
            &[(0, b"zero".as_slice()), (2, b"two".as_slice())],
        );
        let pattern_arg = pattern.to_string_lossy().into_owned();

        let err = ffmpeg_output(&strings(&[
            "-f",
            "image2",
            "-framerate",
            "25",
            "-i",
            pattern_arg.as_str(),
            "-f",
            "framecrc",
            "-",
        ]))
        .expect_err("image2 sequence input should reject missing frames");

        remove_temp_files(&paths);

        assert!(err.message().contains("missing frame number 1"));
    }

    #[test]
    fn rejects_image2_invalid_start_number() {
        let (pattern, paths) = write_temp_image2_sequence(
            "image2-sequence-bad-start",
            "png",
            &[(0, b"zero".as_slice())],
        );
        let pattern_arg = pattern.to_string_lossy().into_owned();

        let err = ffmpeg_output(&strings(&[
            "-f",
            "image2",
            "-framerate",
            "25",
            "-start_number",
            "abc",
            "-i",
            pattern_arg.as_str(),
            "-f",
            "framecrc",
            "-",
        ]))
        .expect_err("image2 input should reject invalid start_number");

        remove_temp_files(&paths);

        assert!(err.message().contains("invalid image2 start number"));
    }

    #[test]
    fn rejects_image2_without_frame_rate() {
        let path = write_temp_bytes("image2-missing-rate", "png", b"\x89PNG\r\n\x1a\n");
        let path_arg = path.to_string_lossy().into_owned();

        let err = ffmpeg_output(&strings(&[
            "-f",
            "image2",
            "-i",
            path_arg.as_str(),
            "-f",
            "framecrc",
            "-",
        ]))
        .expect_err("image2 input requires an explicit frame rate");

        let _ = fs::remove_file(&path);

        assert!(err.message().contains("frame rate"));
    }

    #[test]
    fn rejects_image2_empty_payload() {
        let path = write_temp_bytes("image2-empty", "png", b"");
        let path_arg = path.to_string_lossy().into_owned();

        let err = ffmpeg_output(&strings(&[
            "-f",
            "image2",
            "-framerate",
            "1",
            "-i",
            path_arg.as_str(),
            "-f",
            "framecrc",
            "-",
        ]))
        .expect_err("image2 input should reject empty image payloads");

        let _ = fs::remove_file(&path);

        assert!(err.message().contains("payload must not be empty"));
    }

    #[test]
    fn runs_yuv4mpegpipe_to_framecrc_stdout() {
        let first = y4m_frame(6, 0x10);
        let second = y4m_frame(6, 0x80);
        let path = write_temp_bytes(
            "y4m-framecrc",
            "bin",
            &y4m_file_bytes(2, 2, &[first, second]),
        );
        let path_arg = path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&[
            "-f",
            "yuv4mpegpipe",
            "-i",
            path_arg.as_str(),
            "-f",
            "framecrc",
            "-",
        ]))
        .expect("YUV4MPEG2 command path should execute");

        let _ = fs::remove_file(&path);

        assert_eq!(output.output_format(), Some("framecrc"));
        assert_eq!(output.packet_count(), 2);
        assert_eq!(output.byte_count(), 12);
        assert!(output.stderr().is_empty());
        assert!(output
            .stdout()
            .contains("stream=0 pts=0 dts=0 duration=1 size=6"));
        assert!(output
            .stdout()
            .contains("stream=0 pts=1 dts=1 duration=1 size=6"));
    }

    #[test]
    fn runs_detected_yuv4mpegpipe_to_null_stdout() {
        let frame = y4m_frame(6, 0x20);
        let path = write_temp_bytes("y4m-null", "y4m", &y4m_file_bytes(2, 2, &[frame]));
        let path_arg = path.to_string_lossy().into_owned();

        let output = ffmpeg_output(&strings(&["-i", path_arg.as_str(), "-f", "null", "-"]))
            .expect("YUV4MPEG2 command path should execute");

        let _ = fs::remove_file(&path);

        assert_eq!(output.output_format(), Some("null"));
        assert_eq!(output.packet_count(), 1);
        assert_eq!(output.byte_count(), 6);
        assert!(output.stdout().is_empty());
        assert!(output.stderr().is_empty());
    }

    #[test]
    fn rejects_yuv4mpegpipe_bad_header() {
        let path = write_temp_bytes("y4m-bad-header", "y4m", b"YUV4MPEG2 H2 F25:1 Ip C420jpeg\n");
        let path_arg = path.to_string_lossy().into_owned();

        let err = ffmpeg_output(&strings(&[
            "-f",
            "yuv4mpegpipe",
            "-i",
            path_arg.as_str(),
            "-f",
            "framecrc",
            "-",
        ]))
        .expect_err("YUV4MPEG2 input should reject missing width");

        let _ = fs::remove_file(&path);

        assert!(err.message().contains("missing width"));
    }

    #[test]
    fn rejects_yuv4mpegpipe_truncated_frame() {
        let path = write_temp_bytes(
            "y4m-truncated-frame",
            "y4m",
            b"YUV4MPEG2 W2 H2 F25:1 Ip C420jpeg\nFRAME\nabc",
        );
        let path_arg = path.to_string_lossy().into_owned();

        let err = ffmpeg_output(&strings(&["-i", path_arg.as_str(), "-f", "framecrc", "-"]))
            .expect_err("YUV4MPEG2 input should reject truncated frames");

        let _ = fs::remove_file(&path);

        assert!(err.message().contains("truncated"));
    }

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    fn write_temp_mov(label: &str, bytes: &[u8]) -> PathBuf {
        write_temp_bytes(label, "mp4", bytes)
    }

    fn write_temp_bytes(label: &str, extension: &str, bytes: &[u8]) -> PathBuf {
        let path = unique_temp_path(label, extension);
        fs::write(&path, bytes).expect("temp media file should be writable");
        path
    }

    fn unique_temp_path(label: &str, extension: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "ffmpegrust-{}-{label}-{unique}.{extension}",
            std::process::id()
        ));
        path
    }

    fn write_temp_image2_sequence(
        label: &str,
        extension: &str,
        frames: &[(usize, &[u8])],
    ) -> (PathBuf, Vec<PathBuf>) {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let prefix = format!("ffmpegrust-{}-{label}-{unique}", std::process::id());
        let temp_dir = std::env::temp_dir();
        let pattern = temp_dir.join(format!("{prefix}-%03d.{extension}"));
        let mut paths = Vec::new();

        for (number, bytes) in frames {
            let path = temp_dir.join(format!("{prefix}-{number:03}.{extension}"));
            fs::write(&path, bytes).expect("temp image2 sequence file should be writable");
            paths.push(path);
        }

        (pattern, paths)
    }

    fn remove_temp_files(paths: &[PathBuf]) {
        for path in paths {
            let _ = fs::remove_file(path);
        }
    }

    fn wav_file_bytes(channels: u16, sample_rate: u32, payload: &[u8]) -> Vec<u8> {
        let mut muxer = avformat::WavMuxer::new_pcm_s16le(channels, sample_rate).unwrap();
        muxer
            .write_packet(&Packet::new(payload.to_vec(), 0))
            .unwrap();
        muxer.finish().unwrap()
    }

    fn y4m_file_bytes(width: u32, height: u32, frames: &[Vec<u8>]) -> Vec<u8> {
        let mut muxer =
            avformat::Yuv4MpegMuxer::new(width, height, Rational::new(25, 1).unwrap(), None)
                .unwrap();
        for frame in frames {
            muxer.write_packet(&Packet::new(frame.clone(), 0)).unwrap();
        }
        muxer.finish()
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

    fn y4m_frame(len: usize, start: u8) -> Vec<u8> {
        (0..len)
            .map(|offset| start.wrapping_add(offset as u8))
            .collect()
    }

    fn sampled_mov_file(samples: &[&[u8]], durations: &[u32]) -> Vec<u8> {
        sampled_mov_file_with_handler(samples, durations, *b"vide", 1_920, 1_080)
    }

    #[derive(Debug, Clone, Copy)]
    struct MovTrackFixture {
        handler_type: [u8; 4],
        width: u32,
        height: u32,
    }

    fn sampled_mov_file_with_handler(
        samples: &[&[u8]],
        durations: &[u32],
        handler_type: [u8; 4],
        track_width: u32,
        track_height: u32,
    ) -> Vec<u8> {
        let ftyp = ftyp_box();
        let sample_sizes = samples
            .iter()
            .map(|sample| u32::try_from(sample.len()).unwrap())
            .collect::<Vec<_>>();
        let track = MovTrackFixture {
            handler_type,
            width: track_width,
            height: track_height,
        };
        let placeholder_moov = box_(
            MOOV_ID,
            &moov_with_samples(0, &sample_sizes, durations, track),
        );
        let chunk_offset = u32::try_from(ftyp.len() + placeholder_moov.len() + 8).unwrap();
        let moov = box_(
            MOOV_ID,
            &moov_with_samples(chunk_offset, &sample_sizes, durations, track),
        );
        let mut out = Vec::new();
        out.extend_from_slice(&ftyp);
        out.extend_from_slice(&moov);
        out.extend_from_slice(&box_(MDAT_ID, &samples.concat()));
        out
    }

    fn moov_with_samples(
        chunk_offset: u32,
        sample_sizes: &[u32],
        durations: &[u32],
        track: MovTrackFixture,
    ) -> Vec<u8> {
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
                track,
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
        track: MovTrackFixture,
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
            &[
                mdhd_v0(timescale, media_duration),
                hdlr_box(track.handler_type, b"Rust Handler\0"),
                minf,
            ]
            .concat(),
        );
        box_(
            TRAK_ID,
            &[
                tkhd_v0(track_id, media_duration, track.width, track.height),
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
        body.extend_from_slice(&0_u16.to_be_bytes());
        body.extend_from_slice(&0_u16.to_be_bytes());
        box_(MDHD_ID, &full_box(0, &body))
    }

    fn hdlr_box(handler_type: [u8; 4], handler_name: &[u8]) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&0_u32.to_be_bytes());
        body.extend_from_slice(&handler_type);
        body.extend_from_slice(&[0; 12]);
        body.extend_from_slice(handler_name);
        box_(HDLR_ID, &full_box(0, &body))
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
