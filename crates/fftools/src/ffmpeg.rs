use crate::{
    build_io_plan, parse_ffmpeg_args, version_banner, CliOption, Endpoint, IoPlan, PlannedFile,
};
use avformat::{
    register_mov_probe, FrameCrcMuxer, MovDemuxer, NullMuxer, ProbeRegistry, ProbeRequest,
};
use std::{fmt, fs};

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
}

impl OutputMuxer {
    fn name(self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::FrameCrc => "framecrc",
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
    validate_input_options(input)?;
    let output_muxer = parse_output_muxer(output)?;
    validate_output_options(output)?;
    validate_stdout_output(output)?;

    let input_path = local_input_path(input)?;
    let bytes = fs::read(input_path)
        .map_err(|err| FfmpegError::io(format!("failed to read `{input_path}`: {err}")))?;
    validate_mov_probe(input_path, &bytes)?;
    let mut demuxer = MovDemuxer::open(&bytes).map_err(|err| {
        FfmpegError::invalid_data(format!("failed to parse MOV/MP4 input: {err}"))
    })?;

    match output_muxer {
        OutputMuxer::Null => run_null_muxer(&mut demuxer),
        OutputMuxer::FrameCrc => run_framecrc_muxer(&mut demuxer),
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

fn validate_input_options(input: &PlannedFile) -> Result<(), FfmpegError> {
    for option in input.options() {
        if option.name() != "f" {
            return Err(FfmpegError::unsupported(format!(
                "input option `-{}` is not implemented",
                option.name()
            )));
        }
    }

    if let Some(format) = last_option_value(input.options(), "f") {
        match format.to_ascii_lowercase().as_str() {
            "mov" | "mp4" | "m4a" | "3gp" | "3g2" | "mj2" => Ok(()),
            _ => Err(FfmpegError::unsupported(format!(
                "input format `{format}` is not implemented"
            ))),
        }
    } else {
        Ok(())
    }
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
            "ffmpeg-rs currently requires explicit output `-f null` or `-f framecrc`",
        )
    })?;

    match format.to_ascii_lowercase().as_str() {
        "null" => Ok(OutputMuxer::Null),
        "framecrc" => Ok(OutputMuxer::FrameCrc),
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

fn run_null_muxer(demuxer: &mut MovDemuxer<'_>) -> Result<FfmpegOutput, FfmpegError> {
    let mut muxer = NullMuxer::new();
    while let Some(packet) = demuxer
        .read_packet()
        .map_err(|err| FfmpegError::invalid_data(format!("failed to read MOV/MP4 packet: {err}")))?
    {
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

fn run_framecrc_muxer(demuxer: &mut MovDemuxer<'_>) -> Result<FfmpegOutput, FfmpegError> {
    let mut muxer = FrameCrcMuxer::new();
    let mut packet_count = 0_u64;
    let mut byte_count = 0_u64;

    while let Some(packet) = demuxer
        .read_packet()
        .map_err(|err| FfmpegError::invalid_data(format!("failed to read MOV/MP4 packet: {err}")))?
    {
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

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    fn write_temp_mov(label: &str, bytes: &[u8]) -> PathBuf {
        write_temp_bytes(label, "mp4", bytes)
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
