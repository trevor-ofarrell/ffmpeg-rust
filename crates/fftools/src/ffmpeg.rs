use crate::{
    build_io_plan, parse_ffmpeg_args, version_banner, CliOption, Endpoint, IoPlan, PlannedFile,
};
use avformat::{
    register_mov_probe, FrameCrcMuxer, Image2Demuxer, Image2Entry, Image2Pattern, MovDemuxer,
    NullMuxer, PcmS16leDemuxer, PcmS16leMuxer, ProbeRegistry, ProbeRequest, RawVideoDemuxer,
    RawVideoMuxer, RawVideoPixelFormat, WavDemuxer, WavMuxer, Yuv4MpegDemuxer,
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
    Null,
    FrameCrc,
    PcmS16le,
    RawVideo,
    Wav,
}

impl OutputMuxer {
    fn name(self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::FrameCrc => "framecrc",
            Self::PcmS16le => "s16le",
            Self::RawVideo => "rawvideo",
            Self::Wav => "wav",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputFormat {
    Image2 {
        frame_rate: Rational,
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
            eprintln!("ffmpeg: {err}");
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
    validate_output_options(output)?;
    validate_output_endpoint(output_muxer, output)?;

    let input_path = local_input_path(input)?;
    let explicit_input = explicit_input_format(input)?;
    if let Some(InputFormat::Image2 { frame_rate }) = explicit_input {
        return run_image2_input(input_path, frame_rate, output_muxer);
    }

    let bytes = fs::read(input_path)
        .map_err(|err| FfmpegError::io(format!("failed to read `{input_path}`: {err}")))?;
    let input_format = detect_input_format(explicit_input, input_path, &bytes)?;

    match input_format {
        InputFormat::Image2 { frame_rate } => {
            run_image2_input(input_path, frame_rate, output_muxer)
        }
        InputFormat::Mov => {
            let mut demuxer = MovDemuxer::open(&bytes).map_err(|err| {
                FfmpegError::invalid_data(format!("failed to parse MOV/MP4 input: {err}"))
            })?;
            run_output_muxer(output_muxer, || {
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
                OutputMuxer::Null | OutputMuxer::FrameCrc => {
                    run_output_muxer(output_muxer, read_packet)
                }
                OutputMuxer::RawVideo => run_output_muxer(output_muxer, read_packet),
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
                OutputMuxer::Null | OutputMuxer::FrameCrc => {
                    run_output_muxer(output_muxer, read_packet)
                }
                OutputMuxer::PcmS16le => run_output_muxer(output_muxer, read_packet),
                OutputMuxer::Wav => run_output_muxer(output_muxer, read_packet),
            }
        }
        InputFormat::Wav => {
            let mut demuxer = WavDemuxer::open(&bytes).map_err(|err| {
                FfmpegError::invalid_data(format!("failed to parse WAV input: {err}"))
            })?;
            run_output_muxer(output_muxer, || {
                demuxer.read_packet().map_err(|err| {
                    FfmpegError::invalid_data(format!("failed to read WAV packet: {err}"))
                })
            })
        }
        InputFormat::Yuv4MpegPipe => {
            let mut demuxer = Yuv4MpegDemuxer::open(&bytes).map_err(|err| {
                FfmpegError::invalid_data(format!("failed to parse YUV4MPEG2 input: {err}"))
            })?;
            run_output_muxer(output_muxer, || {
                demuxer.read_packet().map_err(|err| {
                    FfmpegError::invalid_data(format!("failed to read YUV4MPEG2 packet: {err}"))
                })
            })
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
        OutputMuxer::Null | OutputMuxer::FrameCrc => validate_stdout_output(output),
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
    output_muxer: OutputMuxer,
) -> Result<FfmpegOutput, FfmpegError> {
    let entries = image2_entries_for_path(path)?;
    let mut demuxer = Image2Demuxer::open(path, entries, 0, frame_rate)
        .map_err(|err| FfmpegError::invalid_data(format!("failed to parse image2 input: {err}")))?;
    run_output_muxer(output_muxer, || {
        demuxer.read_packet().map_err(|err| {
            FfmpegError::invalid_data(format!("failed to read image2 packet: {err}"))
        })
    })
}

fn image2_entries_for_path(path: &str) -> Result<Vec<Image2Entry>, FfmpegError> {
    let pattern = Image2Pattern::parse(path).map_err(|err| {
        FfmpegError::invalid_data(format!("failed to parse image2 pattern: {err}"))
    })?;

    if pattern.is_sequence() {
        return discover_image2_sequence_entries(&pattern);
    }

    let actual_path = pattern.path_for_frame_number(0).map_err(|err| {
        FfmpegError::invalid_data(format!("failed to resolve image2 input path: {err}"))
    })?;
    let entry = read_image2_entry(actual_path.clone(), Path::new(&actual_path))?;
    Ok(vec![entry])
}

fn discover_image2_sequence_entries(
    pattern: &Image2Pattern,
) -> Result<Vec<Image2Entry>, FfmpegError> {
    let first_path = pattern.path_for_frame_number(0).map_err(|err| {
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
            "f" | "ar" | "ac" | "s" | "r" | "framerate" | "pix_fmt"
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
    reject_options_except(input, "image2", &["f", "r", "framerate"])?;
    let frame_rate = parse_image2_frame_rate_option(input)?;
    Ok(InputFormat::Image2 { frame_rate })
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
    match value.to_ascii_lowercase().as_str() {
        "gray" | "gray8" => Ok(RawVideoPixelFormat::Gray8),
        "rgb24" => Ok(RawVideoPixelFormat::Rgb24),
        "rgba" => Ok(RawVideoPixelFormat::Rgba),
        "yuv420p" => Ok(RawVideoPixelFormat::Yuv420p),
        _ => Err(FfmpegError::unsupported(format!(
            "rawvideo pixel format `{value}` is not implemented"
        ))),
    }
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

fn validate_output_options(output: &PlannedFile) -> Result<(), FfmpegError> {
    for option in output.options() {
        if option.name() != "f" {
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
            "ffmpeg-rs currently requires explicit output `-f null`, `-f framecrc`, `-f s16le`, `-f rawvideo`, or `-f wav`",
        )
    })?;

    match format.to_ascii_lowercase().as_str() {
        "null" => Ok(OutputMuxer::Null),
        "framecrc" => Ok(OutputMuxer::FrameCrc),
        "s16le" | "pcm_s16le" => Ok(OutputMuxer::PcmS16le),
        "rawvideo" => Ok(OutputMuxer::RawVideo),
        "wav" | "wave" => Ok(OutputMuxer::Wav),
        _ => Err(FfmpegError::unsupported(format!(
            "output format `{format}` is not implemented"
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

fn run_output_muxer<F>(
    output_muxer: OutputMuxer,
    read_packet: F,
) -> Result<FfmpegOutput, FfmpegError>
where
    F: FnMut() -> Result<Option<Packet>, FfmpegError>,
{
    match output_muxer {
        OutputMuxer::Null => run_null_muxer(read_packet),
        OutputMuxer::FrameCrc => run_framecrc_muxer(read_packet),
        OutputMuxer::PcmS16le => Err(FfmpegError::unsupported(
            "ffmpeg-rs s16le output is only implemented for raw pcm_s16le inputs",
        )),
        OutputMuxer::RawVideo => Err(FfmpegError::unsupported(
            "ffmpeg-rs rawvideo output is only implemented for rawvideo inputs",
        )),
        OutputMuxer::Wav => Err(FfmpegError::unsupported(
            "ffmpeg-rs WAV output is only implemented for raw pcm_s16le inputs",
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

    fn y4m_frame(len: usize, start: u8) -> Vec<u8> {
        (0..len)
            .map(|offset| start.wrapping_add(offset as u8))
            .collect()
    }

    fn sampled_mov_file(samples: &[&[u8]], durations: &[u32]) -> Vec<u8> {
        let ftyp = ftyp_box();
        let sample_sizes = samples
            .iter()
            .map(|sample| u32::try_from(sample.len()).unwrap())
            .collect::<Vec<_>>();
        let placeholder_moov = box_(MOOV_ID, &moov_with_samples(0, &sample_sizes, durations));
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
