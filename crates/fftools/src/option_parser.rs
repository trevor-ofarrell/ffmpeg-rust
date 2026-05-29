use avutil::{LogFlags, LogLevel};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliOption {
    name: String,
    value: Option<String>,
}

impl CliOption {
    fn flag(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: None,
        }
    }

    fn value(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: Some(value.into()),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn value_ref(&self) -> Option<&str> {
        self.value.as_deref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliFile {
    url: String,
    options: Vec<CliOption>,
}

impl CliFile {
    fn new(url: impl Into<String>, options: Vec<CliOption>) -> Self {
        Self {
            url: url.into(),
            options,
        }
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn options(&self) -> &[CliOption] {
        &self.options
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ParsedCommand {
    global_options: Vec<CliOption>,
    inputs: Vec<CliFile>,
    outputs: Vec<CliFile>,
}

impl ParsedCommand {
    pub fn global_options(&self) -> &[CliOption] {
        &self.global_options
    }

    pub fn inputs(&self) -> &[CliFile] {
        &self.inputs
    }

    pub fn outputs(&self) -> &[CliFile] {
        &self.outputs
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliParseError {
    message: String,
}

impl CliParseError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for CliParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for CliParseError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OptionScope {
    Global,
    File,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OptionArity {
    Flag,
    Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OptionValueKind {
    Generic,
    LogLevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OptionSpec {
    scope: OptionScope,
    arity: OptionArity,
    value_kind: OptionValueKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CliLogConfig {
    raw_level: i32,
    flags: LogFlags,
}

impl Default for CliLogConfig {
    fn default() -> Self {
        Self {
            raw_level: LogLevel::Info.as_ffmpeg_value(),
            flags: LogFlags::SKIP_REPEATED,
        }
    }
}

impl CliLogConfig {
    pub fn level(&self) -> LogLevel {
        LogLevel::from_ffmpeg_value(self.raw_level).unwrap_or(LogLevel::Trace)
    }

    pub fn known_level(&self) -> Option<LogLevel> {
        LogLevel::from_ffmpeg_value(self.raw_level)
    }

    pub fn raw_level(&self) -> i32 {
        self.raw_level
    }

    pub fn flags(&self) -> LogFlags {
        self.flags
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LogLevelDirective {
    raw_level: Option<i32>,
    enable_flags: LogFlags,
    disable_flags: LogFlags,
    reset_flags: bool,
}

impl LogLevelDirective {
    pub fn level(&self) -> Option<LogLevel> {
        self.raw_level.and_then(LogLevel::from_ffmpeg_value)
    }

    pub fn raw_level(&self) -> Option<i32> {
        self.raw_level
    }

    pub fn enable_flags(&self) -> LogFlags {
        self.enable_flags
    }

    pub fn disable_flags(&self) -> LogFlags {
        self.disable_flags
    }

    pub fn reset_flags(&self) -> bool {
        self.reset_flags
    }
}

pub fn parse_ffmpeg_args(args: &[String]) -> Result<ParsedCommand, CliParseError> {
    let mut parsed = ParsedCommand::default();
    let mut pending_file_options = Vec::new();
    let mut index = 0;

    while index < args.len() {
        let arg = &args[index];

        if arg == "-i" {
            let url = take_value(args, index, "-i", OptionValueKind::Generic)?;
            parsed.inputs.push(CliFile::new(
                url.clone(),
                take_pending(&mut pending_file_options),
            ));
            index += 2;
            continue;
        }

        if is_option_token(arg) {
            let option_name = option_name(arg)?;
            let spec = option_spec(option_name)
                .ok_or_else(|| CliParseError::new(format!("unknown option `{arg}`")))?;
            validate_stream_specifier_name(option_name, arg)?;

            let option = match spec.arity {
                OptionArity::Flag => {
                    index += 1;
                    CliOption::flag(option_name)
                }
                OptionArity::Value => {
                    let value = take_value(args, index, arg, spec.value_kind)?;
                    validate_option_value(option_name, &value, spec.value_kind)?;
                    index += 2;
                    CliOption::value(option_name, value.clone())
                }
            };

            match spec.scope {
                OptionScope::Global => parsed.global_options.push(option),
                OptionScope::File => pending_file_options.push(option),
            }
            continue;
        }

        parsed.outputs.push(CliFile::new(
            arg.clone(),
            take_pending(&mut pending_file_options),
        ));
        index += 1;
    }

    if let Some(option) = pending_file_options.first() {
        return Err(CliParseError::new(format!(
            "option `-{}` was not followed by an input or output file",
            option.name()
        )));
    }

    Ok(parsed)
}

fn option_name(arg: &str) -> Result<&str, CliParseError> {
    let name = arg
        .strip_prefix('-')
        .expect("option_name is only called for option tokens");
    if name.is_empty() {
        return Err(CliParseError::new("empty option name"));
    }
    Ok(name)
}

fn option_spec(name: &str) -> Option<OptionSpec> {
    let base_name = name.split_once(':').map_or(name, |(base, _)| base);
    let (scope, arity, value_kind) = match base_name {
        "hide_banner" | "y" | "n" | "nostdin" | "version" | "buildconf" => (
            OptionScope::Global,
            OptionArity::Flag,
            OptionValueKind::Generic,
        ),
        "loglevel" | "v" => (
            OptionScope::Global,
            OptionArity::Value,
            OptionValueKind::LogLevel,
        ),
        "an" | "vn" | "sn" | "dn" | "shortest" | "bitexact" => (
            OptionScope::File,
            OptionArity::Flag,
            OptionValueKind::Generic,
        ),
        "f" | "c" | "codec" | "map" | "ar" | "ac" | "s" | "r" | "framerate" | "pix_fmt"
        | "start_number" | "hash" | "vf" | "af" | "filter" | "metadata" => (
            OptionScope::File,
            OptionArity::Value,
            OptionValueKind::Generic,
        ),
        _ => return None,
    };

    Some(OptionSpec {
        scope,
        arity,
        value_kind,
    })
}

fn validate_stream_specifier_name(name: &str, arg: &str) -> Result<(), CliParseError> {
    let Some((_, specifier)) = name.split_once(':') else {
        return Ok(());
    };

    if specifier.starts_with(':') || specifier.contains("::") {
        return Err(CliParseError::new(format!(
            "invalid stream specifier in option `{arg}`"
        )));
    }

    Ok(())
}

fn take_value(
    args: &[String],
    option_index: usize,
    option: &str,
    value_kind: OptionValueKind,
) -> Result<String, CliParseError> {
    let value_index = option_index + 1;
    let value = args
        .get(value_index)
        .ok_or_else(|| CliParseError::new(format!("missing value for option `{option}`")))?;

    if is_option_token(value) && !allows_option_like_value(value_kind, value) {
        return Err(CliParseError::new(format!(
            "missing value for option `{option}` before `{value}`"
        )));
    }

    Ok(value.clone())
}

fn validate_option_value(
    option: &str,
    value: &str,
    value_kind: OptionValueKind,
) -> Result<(), CliParseError> {
    match value_kind {
        OptionValueKind::Generic => Ok(()),
        OptionValueKind::LogLevel => {
            parse_log_level_directive(value).map(|_| ()).ok_or_else(|| {
                CliParseError::new(format!("invalid loglevel `{value}` for `-{option}`"))
            })
        }
    }
}

fn allows_option_like_value(value_kind: OptionValueKind, value: &str) -> bool {
    matches!(value_kind, OptionValueKind::LogLevel) && parse_log_level_directive(value).is_some()
}

fn is_option_token(value: &str) -> bool {
    value.starts_with('-') && value != "-"
}

fn take_pending(options: &mut Vec<CliOption>) -> Vec<CliOption> {
    std::mem::take(options)
}

pub fn parse_log_level_value(value: &str) -> Option<LogLevel> {
    parse_log_level_directive(value)?.level()
}

pub fn parse_log_level_raw_value(value: &str) -> Option<i32> {
    parse_log_level_directive(value)?.raw_level()
}

pub fn parse_log_level_directive(value: &str) -> Option<LogLevelDirective> {
    if value.is_empty() {
        return None;
    }

    let mut rest = value;
    let mut consumed_flags = false;
    let mut reset_flags = false;
    let mut enable_flags = LogFlags::empty();
    let mut disable_flags = LogFlags::empty();

    while !rest.is_empty() {
        let token_start = rest;
        let (cmd, token) = match token_start.as_bytes()[0] {
            b'+' => (Some(b'+'), &token_start[1..]),
            b'-' => (Some(b'-'), &token_start[1..]),
            _ => (None, token_start),
        };

        if !consumed_flags && cmd.is_none() {
            reset_flags = true;
        }

        let Some((flag, suffix, repeat_flag)) = consume_log_flag(token) else {
            rest = token_start;
            break;
        };

        if repeat_flag {
            if cmd == Some(b'-') {
                enable_flags.insert(flag);
            } else {
                disable_flags.insert(flag);
            }
        } else if cmd == Some(b'-') {
            disable_flags.insert(flag);
        } else {
            enable_flags.insert(flag);
        }

        consumed_flags = true;
        rest = suffix;
    }

    let raw_level = if rest.is_empty() {
        None
    } else {
        let level_text = rest.strip_prefix('+').unwrap_or(rest);
        Some(parse_plain_log_level_raw(level_text)?)
    };

    if !consumed_flags {
        reset_flags = false;
    }

    Some(LogLevelDirective {
        raw_level,
        enable_flags,
        disable_flags,
        reset_flags,
    })
}

pub fn apply_log_level_value(config: &mut CliLogConfig, value: &str) -> Option<()> {
    let directive = parse_log_level_directive(value)?;

    if directive.reset_flags() {
        config.flags = LogFlags::empty();
    }
    if let Some(raw_level) = directive.raw_level() {
        config.raw_level = raw_level;
    }
    config.flags.insert(directive.enable_flags());
    config.flags.remove(directive.disable_flags());

    Some(())
}

fn parse_plain_log_level_raw(value: &str) -> Option<i32> {
    parse_named_log_level(value)
        .map(LogLevel::as_ffmpeg_value)
        .or_else(|| value.parse::<i32>().ok())
}

fn parse_named_log_level(value: &str) -> Option<LogLevel> {
    match value {
        "quiet" => Some(LogLevel::Quiet),
        "panic" => Some(LogLevel::Panic),
        "fatal" => Some(LogLevel::Fatal),
        "error" => Some(LogLevel::Error),
        "warning" => Some(LogLevel::Warning),
        "info" => Some(LogLevel::Info),
        "verbose" => Some(LogLevel::Verbose),
        "debug" => Some(LogLevel::Debug),
        "trace" => Some(LogLevel::Trace),
        _ => None,
    }
}

fn consume_log_flag(token: &str) -> Option<(LogFlags, &str, bool)> {
    if let Some(suffix) = token.strip_prefix("repeat") {
        Some((LogFlags::SKIP_REPEATED, suffix, true))
    } else if let Some(suffix) = token.strip_prefix("level") {
        Some((LogFlags::PRINT_LEVEL, suffix, false))
    } else if let Some(suffix) = token.strip_prefix("time") {
        Some((LogFlags::PRINT_TIME, suffix, false))
    } else if let Some(suffix) = token.strip_prefix("datetime") {
        Some((LogFlags::PRINT_DATETIME, suffix, false))
    } else {
        None
    }
}

pub fn validate_loglevel_options(args: &[String]) -> Result<(), CliParseError> {
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "-loglevel" | "-v" => {
                let option = args[index].as_str();
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| CliParseError::new(format!("missing value for `{option}`")))?;
                if parse_log_level_directive(value).is_none() {
                    return Err(CliParseError::new(format!(
                        "invalid loglevel `{value}` for `{option}`"
                    )));
                }
                index += 2;
            }
            _ => index += 1,
        }
    }
    Ok(())
}

pub fn log_config_from_options(options: &[CliOption]) -> Result<CliLogConfig, CliParseError> {
    let mut config = CliLogConfig::default();
    for option in options {
        if matches!(option.name(), "loglevel" | "v") {
            let value = option.value_ref().ok_or_else(|| {
                CliParseError::new(format!("missing value for option `-{}`", option.name()))
            })?;
            apply_log_level_value(&mut config, value).ok_or_else(|| {
                CliParseError::new(format!(
                    "invalid loglevel `{value}` for `-{}`",
                    option.name()
                ))
            })?;
        }
    }
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_input_and_output_options_by_order() {
        let args = strings(&[
            "-hide_banner",
            "-f",
            "lavfi",
            "-pix_fmt",
            "rgb24",
            "-framerate",
            "25",
            "-i",
            "testsrc=size=16x16",
            "-map",
            "0:v",
            "-c:v",
            "rawvideo",
            "out.yuv",
        ]);

        let parsed = parse_ffmpeg_args(&args).unwrap();

        assert_eq!(parsed.global_options()[0].name(), "hide_banner");
        assert_eq!(parsed.inputs()[0].url(), "testsrc=size=16x16");
        assert_eq!(parsed.inputs()[0].options()[0].name(), "f");
        assert_eq!(parsed.inputs()[0].options()[0].value_ref(), Some("lavfi"));
        assert_eq!(parsed.inputs()[0].options()[1].name(), "pix_fmt");
        assert_eq!(parsed.inputs()[0].options()[1].value_ref(), Some("rgb24"));
        assert_eq!(parsed.inputs()[0].options()[2].name(), "framerate");
        assert_eq!(parsed.inputs()[0].options()[2].value_ref(), Some("25"));
        assert_eq!(parsed.outputs()[0].url(), "out.yuv");
        assert_eq!(parsed.outputs()[0].options()[0].name(), "map");
        assert_eq!(parsed.outputs()[0].options()[1].name(), "c:v");
        assert_eq!(
            parsed.outputs()[0].options()[1].value_ref(),
            Some("rawvideo")
        );
    }

    #[test]
    fn handles_multiple_inputs_and_outputs() {
        let args = strings(&[
            "-f",
            "s16le",
            "-ar",
            "48000",
            "-i",
            "audio.raw",
            "-f",
            "rawvideo",
            "-i",
            "video.raw",
            "-c:a",
            "pcm_s16le",
            "out.wav",
            "-f",
            "null",
            "-",
        ]);

        let parsed = parse_ffmpeg_args(&args).unwrap();

        assert_eq!(parsed.inputs().len(), 2);
        assert_eq!(parsed.inputs()[0].url(), "audio.raw");
        assert_eq!(parsed.inputs()[0].options().len(), 2);
        assert_eq!(parsed.inputs()[1].url(), "video.raw");
        assert_eq!(parsed.outputs().len(), 2);
        assert_eq!(parsed.outputs()[0].url(), "out.wav");
        assert_eq!(parsed.outputs()[1].url(), "-");
        assert_eq!(parsed.outputs()[1].options()[0].value_ref(), Some("null"));
    }

    #[test]
    fn global_options_do_not_consume_pending_file_options() {
        let args = strings(&[
            "-f",
            "lavfi",
            "-loglevel",
            "error",
            "-i",
            "testsrc",
            "out.mkv",
        ]);

        let parsed = parse_ffmpeg_args(&args).unwrap();

        assert_eq!(parsed.global_options()[0].name(), "loglevel");
        assert_eq!(parsed.global_options()[0].value_ref(), Some("error"));
        assert_eq!(parsed.inputs()[0].options()[0].name(), "f");
        assert_eq!(parsed.outputs()[0].url(), "out.mkv");
    }

    #[test]
    fn validates_loglevel_values_with_shared_avutil_levels() {
        let args = strings(&[
            "-loglevel",
            "warning",
            "-v",
            "-8",
            "-i",
            "in.wav",
            "out.wav",
        ]);

        let parsed = parse_ffmpeg_args(&args).unwrap();
        let config = log_config_from_options(parsed.global_options()).unwrap();

        assert_eq!(parsed.global_options()[0].value_ref(), Some("warning"));
        assert_eq!(parsed.global_options()[1].value_ref(), Some("-8"));
        assert_eq!(config.level(), LogLevel::Quiet);
        assert_eq!(config.known_level(), Some(LogLevel::Quiet));
        assert_eq!(config.raw_level(), LogLevel::Quiet.as_ffmpeg_value());
        assert_eq!(config.flags(), LogFlags::SKIP_REPEATED);
        assert_eq!(parse_log_level_value("48"), Some(LogLevel::Debug));
        assert_eq!(parse_log_level_raw_value("48"), Some(48));
        assert_eq!(parse_log_level_value("+error"), Some(LogLevel::Error));
        assert_eq!(parse_log_level_value("ERROR"), None);
    }

    #[test]
    fn parses_loglevel_flag_directives_and_preserves_last_level() {
        let args = strings(&[
            "-loglevel",
            "repeat+level+debug",
            "-v",
            "+time",
            "-loglevel",
            "-level",
            "-i",
            "in.wav",
            "out.wav",
        ]);

        let parsed = parse_ffmpeg_args(&args).unwrap();
        let config = log_config_from_options(parsed.global_options()).unwrap();

        assert_eq!(config.level(), LogLevel::Debug);
        assert_eq!(config.known_level(), Some(LogLevel::Debug));
        assert_eq!(config.raw_level(), LogLevel::Debug.as_ffmpeg_value());
        assert!(!config.flags().contains(LogFlags::SKIP_REPEATED));
        assert!(config.flags().contains(LogFlags::PRINT_TIME));
        assert!(!config.flags().contains(LogFlags::PRINT_LEVEL));
        assert_eq!(
            parse_log_level_value("repeat+datetime+trace"),
            Some(LogLevel::Trace)
        );
        assert_eq!(parse_log_level_value("REPEAT+datetime+trace"), None);
        assert_eq!(parse_log_level_value("+repeat"), None);
        assert!(parse_log_level_directive("+repeat").is_some());
        assert!(parse_log_level_directive("-repeat").is_some());
        assert!(parse_log_level_directive("repeat").is_some());
        assert!(parse_log_level_directive("level").is_some());
    }

    #[test]
    fn preserves_raw_numeric_loglevel_thresholds() {
        let args = strings(&[
            "-loglevel",
            "repeat+level+23",
            "-v",
            "-1",
            "-i",
            "in.wav",
            "out.wav",
        ]);

        let parsed = parse_ffmpeg_args(&args).unwrap();
        let config = log_config_from_options(parsed.global_options()).unwrap();

        assert_eq!(config.raw_level(), -1);
        assert_eq!(config.known_level(), None);
        assert_eq!(config.level(), LogLevel::Trace);
        assert!(parse_log_level_value("23").is_none());
        assert_eq!(parse_log_level_raw_value("23"), Some(23));
        assert_eq!(parse_log_level_raw_value("repeat+23"), Some(23));
        assert_eq!(parse_log_level_raw_value("level+23"), Some(23));
        assert_eq!(parse_log_level_raw_value("+23"), Some(23));
        assert_eq!(parse_log_level_raw_value("999"), Some(999));
    }

    #[test]
    fn rejects_invalid_loglevel_values() {
        let err = parse_ffmpeg_args(&strings(&["-loglevel", "warn", "-i", "in.wav", "out.wav"]))
            .unwrap_err();

        assert!(err.message().contains("invalid loglevel"));
        assert!(
            parse_ffmpeg_args(&strings(&["-v", "-not-a-level", "-i", "in.wav", "out.wav"]))
                .unwrap_err()
                .message()
                .contains("missing value")
        );
    }

    #[test]
    fn treats_single_dash_as_output_url() {
        let args = strings(&["-f", "null", "-"]);

        let parsed = parse_ffmpeg_args(&args).unwrap();

        assert_eq!(parsed.outputs()[0].url(), "-");
        assert_eq!(parsed.outputs()[0].options()[0].name(), "f");
    }

    #[test]
    fn treats_start_number_as_file_scoped_value_option() {
        let args = strings(&[
            "-f",
            "image2",
            "-start_number",
            "5",
            "-i",
            "in-%03d.png",
            "-f",
            "image2",
            "-start_number",
            "9",
            "out-%03d.png",
        ]);

        let parsed = parse_ffmpeg_args(&args).unwrap();

        assert_eq!(parsed.inputs()[0].options()[1].name(), "start_number");
        assert_eq!(parsed.inputs()[0].options()[1].value_ref(), Some("5"));
        assert_eq!(parsed.outputs()[0].options()[1].name(), "start_number");
        assert_eq!(parsed.outputs()[0].options()[1].value_ref(), Some("9"));
    }

    #[test]
    fn treats_hash_as_file_scoped_value_option() {
        let args = strings(&["-i", "in.wav", "-f", "hash", "-hash", "md5", "-"]);

        let parsed = parse_ffmpeg_args(&args).unwrap();

        assert_eq!(parsed.outputs()[0].url(), "-");
        assert_eq!(parsed.outputs()[0].options()[0].name(), "f");
        assert_eq!(parsed.outputs()[0].options()[0].value_ref(), Some("hash"));
        assert_eq!(parsed.outputs()[0].options()[1].name(), "hash");
        assert_eq!(parsed.outputs()[0].options()[1].value_ref(), Some("md5"));
    }

    #[test]
    fn rejects_missing_values_and_dangling_file_options() {
        assert!(parse_ffmpeg_args(&strings(&["-i"])).is_err());
        assert!(parse_ffmpeg_args(&strings(&["-f", "-i", "in"])).is_err());
        assert!(parse_ffmpeg_args(&strings(&["-f", "lavfi"])).is_err());
    }

    #[test]
    fn rejects_unknown_options() {
        let err = parse_ffmpeg_args(&strings(&["-definitely_not_ffmpeg", "out"])).unwrap_err();

        assert!(err.message().contains("unknown option"));
    }

    #[test]
    fn double_dash_options_are_not_normalized_to_single_dash_options() {
        let err = parse_ffmpeg_args(&strings(&["--version"])).unwrap_err();

        assert!(err.message().contains("unknown option `--version`"));
    }

    #[test]
    fn rejects_malformed_stream_specifier_option_names() {
        for invalid in ["-c::", "-c::0", "-c:a::0"] {
            let err =
                parse_ffmpeg_args(&strings(&["-i", "in.wav", invalid, "pcm_s16le", "out.wav"]))
                    .unwrap_err();

            assert!(err.message().contains("invalid stream specifier"));
        }

        let parsed = parse_ffmpeg_args(&strings(&[
            "-i",
            "in.wav",
            "-c:",
            "copy",
            "-c:0",
            "pcm_s16le",
            "-c:a:0",
            "pcm_s16le",
            "-c:a:",
            "pcm_s16le",
            "-c:v:",
            "rawvideo",
            "out.wav",
        ]))
        .unwrap();

        assert_eq!(parsed.outputs()[0].options()[0].name(), "c:");
        assert_eq!(parsed.outputs()[0].options()[1].name(), "c:0");
        assert_eq!(parsed.outputs()[0].options()[2].name(), "c:a:0");
        assert_eq!(parsed.outputs()[0].options()[3].name(), "c:a:");
        assert_eq!(parsed.outputs()[0].options()[4].name(), "c:v:");
    }

    #[test]
    fn treats_buildconf_as_global_flag() {
        let parsed = parse_ffmpeg_args(&strings(&["-hide_banner", "-buildconf"])).unwrap();

        assert_eq!(parsed.global_options()[0].name(), "hide_banner");
        assert_eq!(parsed.global_options()[1].name(), "buildconf");
        assert!(parsed.inputs().is_empty());
        assert!(parsed.outputs().is_empty());
    }

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }
}
