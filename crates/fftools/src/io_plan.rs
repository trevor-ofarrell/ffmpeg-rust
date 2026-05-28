use crate::{log_config_from_options, CliFile, CliLogConfig, CliOption, ParsedCommand};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileRole {
    Input,
    Output,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Endpoint {
    File(String),
    Pipe { fd: Option<u32> },
    Protocol { name: String, url: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedFile {
    role: FileRole,
    url: String,
    endpoint: Endpoint,
    options: Vec<CliOption>,
}

impl PlannedFile {
    fn new(role: FileRole, file: &CliFile) -> Result<Self, IoPlanError> {
        let url = file.url();
        if url.is_empty() {
            return Err(IoPlanError::new("file URL must not be empty"));
        }

        Ok(Self {
            role,
            url: url.to_owned(),
            endpoint: classify_endpoint(role, url)?,
            options: file.options().to_vec(),
        })
    }

    pub fn role(&self) -> FileRole {
        self.role
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }

    pub fn options(&self) -> &[CliOption] {
        &self.options
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IoPlan {
    inputs: Vec<PlannedFile>,
    outputs: Vec<PlannedFile>,
    log_config: CliLogConfig,
}

impl IoPlan {
    pub fn inputs(&self) -> &[PlannedFile] {
        &self.inputs
    }

    pub fn outputs(&self) -> &[PlannedFile] {
        &self.outputs
    }

    pub fn log_config(&self) -> CliLogConfig {
        self.log_config
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IoPlanError {
    message: String,
}

impl IoPlanError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for IoPlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for IoPlanError {}

pub fn build_io_plan(command: &ParsedCommand) -> Result<IoPlan, IoPlanError> {
    let log_config = log_config_from_options(command.global_options())
        .map_err(|err| IoPlanError::new(format!("invalid log options: {err}")))?;

    if command.inputs().is_empty() {
        return Err(IoPlanError::new("at least one input is required"));
    }

    if command.outputs().is_empty() {
        return Err(IoPlanError::new("at least one output is required"));
    }

    let inputs = command
        .inputs()
        .iter()
        .map(|file| PlannedFile::new(FileRole::Input, file))
        .collect::<Result<Vec<_>, _>>()?;
    let outputs = command
        .outputs()
        .iter()
        .map(|file| PlannedFile::new(FileRole::Output, file))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(IoPlan {
        inputs,
        outputs,
        log_config,
    })
}

fn classify_endpoint(role: FileRole, url: &str) -> Result<Endpoint, IoPlanError> {
    if url == "-" {
        return Ok(match role {
            FileRole::Input => Endpoint::Pipe { fd: Some(0) },
            FileRole::Output => Endpoint::Pipe { fd: Some(1) },
        });
    }

    if let Some(rest) = url.strip_prefix("pipe:") {
        if rest.is_empty() {
            return Ok(Endpoint::Pipe { fd: None });
        }

        let fd = rest
            .parse::<u32>()
            .map_err(|_| IoPlanError::new(format!("invalid pipe file descriptor `{rest}`")))?;
        return Ok(Endpoint::Pipe { fd: Some(fd) });
    }

    if let Some((scheme, _)) = split_protocol(url) {
        return Ok(Endpoint::Protocol {
            name: scheme.to_ascii_lowercase(),
            url: url.to_owned(),
        });
    }

    Ok(Endpoint::File(url.to_owned()))
}

fn split_protocol(url: &str) -> Option<(&str, &str)> {
    let (scheme, rest) = url.split_once(':')?;

    if scheme.len() == 1 && rest.starts_with(['\\', '/']) {
        return None;
    }

    if scheme.is_empty() {
        return None;
    }

    let valid = scheme
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'.' | b'-'));
    valid.then_some((scheme, rest))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_ffmpeg_args;
    use avutil::{LogFlags, LogLevel};

    #[test]
    fn builds_file_input_and_output_plan_with_options() {
        let command = parse(&[
            "-f",
            "s16le",
            "-ar",
            "48000",
            "-i",
            "audio.raw",
            "-c:a",
            "pcm_s16le",
            "out.wav",
        ]);

        let plan = build_io_plan(&command).unwrap();

        assert_eq!(plan.inputs()[0].role(), FileRole::Input);
        assert_eq!(
            plan.inputs()[0].endpoint(),
            &Endpoint::File("audio.raw".to_string())
        );
        assert_eq!(plan.inputs()[0].options()[0].name(), "f");
        assert_eq!(plan.outputs()[0].role(), FileRole::Output);
        assert_eq!(
            plan.outputs()[0].endpoint(),
            &Endpoint::File("out.wav".to_string())
        );
        assert_eq!(plan.outputs()[0].options()[0].name(), "c:a");
    }

    #[test]
    fn maps_single_dash_by_file_role() {
        let input = parse(&["-f", "s16le", "-i", "-", "out.wav"]);
        let output = parse(&["-f", "s16le", "-i", "audio.raw", "-f", "null", "-"]);

        let input_plan = build_io_plan(&input).unwrap();
        let output_plan = build_io_plan(&output).unwrap();

        assert_eq!(
            input_plan.inputs()[0].endpoint(),
            &Endpoint::Pipe { fd: Some(0) }
        );
        assert_eq!(
            output_plan.outputs()[0].endpoint(),
            &Endpoint::Pipe { fd: Some(1) }
        );
    }

    #[test]
    fn classifies_explicit_pipe_and_protocol_urls() {
        let command = parse(&["-i", "pipe:3", "-f", "mpegts", "udp://239.0.0.1:1234"]);

        let plan = build_io_plan(&command).unwrap();

        assert_eq!(plan.inputs()[0].endpoint(), &Endpoint::Pipe { fd: Some(3) });
        assert_eq!(
            plan.outputs()[0].endpoint(),
            &Endpoint::Protocol {
                name: "udp".to_string(),
                url: "udp://239.0.0.1:1234".to_string()
            }
        );
    }

    #[test]
    fn keeps_windows_drive_paths_as_files() {
        let command = parse(&["-i", r"C:\media\in.wav", r"D:\media\out.wav"]);

        let plan = build_io_plan(&command).unwrap();

        assert_eq!(
            plan.inputs()[0].endpoint(),
            &Endpoint::File(r"C:\media\in.wav".to_string())
        );
        assert_eq!(
            plan.outputs()[0].endpoint(),
            &Endpoint::File(r"D:\media\out.wav".to_string())
        );
    }

    #[test]
    fn rejects_commands_without_required_file_roles() {
        let no_input = parse_ffmpeg_args(&strings(&["out.wav"])).unwrap();
        let no_output = parse_ffmpeg_args(&strings(&["-i", "in.wav"])).unwrap();

        assert!(build_io_plan(&no_input)
            .unwrap_err()
            .message()
            .contains("input"));
        assert!(build_io_plan(&no_output)
            .unwrap_err()
            .message()
            .contains("output"));
    }

    #[test]
    fn resolves_global_log_level_for_execution_plan() {
        let command = parse(&[
            "-loglevel",
            "repeat+level+warning",
            "-v",
            "+time",
            "-loglevel",
            "-level",
            "-i",
            "in.wav",
            "out.wav",
        ]);

        let plan = build_io_plan(&command).unwrap();

        assert_eq!(plan.log_config().level(), LogLevel::Warning);
        assert!(!plan.log_config().flags().contains(LogFlags::SKIP_REPEATED));
        assert!(plan.log_config().flags().contains(LogFlags::PRINT_TIME));
        assert!(!plan.log_config().flags().contains(LogFlags::PRINT_LEVEL));
    }

    #[test]
    fn rejects_invalid_pipe_descriptors() {
        let command = parse(&["-i", "pipe:not-a-number", "out.wav"]);

        let err = build_io_plan(&command).unwrap_err();

        assert!(err.message().contains("invalid pipe"));
    }

    fn parse(values: &[&str]) -> ParsedCommand {
        parse_ffmpeg_args(&strings(values)).unwrap()
    }

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }
}
