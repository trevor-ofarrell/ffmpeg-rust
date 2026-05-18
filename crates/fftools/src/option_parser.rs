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
    level: LogLevel,
    flags: LogFlags,
}

impl Default for CliLogConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            flags: LogFlags::empty(),
        }
    }
}

impl CliLogConfig {
    pub fn level(&self) -> LogLevel {
        self.level
    }

    pub fn flags(&self) -> LogFlags {
        self.flags
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LogLevelDirective {
    level: Option<LogLevel>,
    enable_flags: LogFlags,
    disable_flags: LogFlags,
}

impl LogLevelDirective {
    pub fn level(&self) -> Option<LogLevel> {
        self.level
    }

    pub fn enable_flags(&self) -> LogFlags {
        self.enable_flags
    }

    pub fn disable_flags(&self) -> LogFlags {
        self.disable_flags
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
    let name = arg.trim_start_matches('-');
    if name.is_empty() {
        return Err(CliParseError::new("empty option name"));
    }
    Ok(name)
}

fn option_spec(name: &str) -> Option<OptionSpec> {
    let base_name = name.split_once(':').map_or(name, |(base, _)| base);
    let (scope, arity, value_kind) = match base_name {
        "hide_banner" | "y" | "n" | "nostdin" | "version" => (
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

pub fn parse_log_level_directive(value: &str) -> Option<LogLevelDirective> {
    if value.is_empty() {
        return None;
    }

    if let Some(level) = parse_plain_log_level(value) {
        return Some(LogLevelDirective {
            level: Some(level),
            enable_flags: LogFlags::empty(),
            disable_flags: LogFlags::empty(),
        });
    }

    if let Some(rest) = value.strip_prefix('+') {
        return parse_prefixed_log_flags(rest, true);
    }

    if let Some(rest) = value.strip_prefix('-') {
        return parse_prefixed_log_flags(rest, false);
    }

    parse_log_flags_and_level(value)
}

pub fn apply_log_level_value(config: &mut CliLogConfig, value: &str) -> Option<()> {
    let directive = parse_log_level_directive(value)?;

    if let Some(level) = directive.level() {
        config.level = level;
    }
    config.flags.insert(directive.enable_flags());
    config.flags.remove(directive.disable_flags());

    Some(())
}

fn parse_plain_log_level(value: &str) -> Option<LogLevel> {
    LogLevel::from_name(value).or_else(|| {
        value
            .parse::<i32>()
            .ok()
            .and_then(LogLevel::from_ffmpeg_value)
    })
}

fn parse_prefixed_log_flags(rest: &str, enable: bool) -> Option<LogLevelDirective> {
    let flags = parse_log_flag_list(rest)?;
    Some(if enable {
        LogLevelDirective {
            level: None,
            enable_flags: flags,
            disable_flags: LogFlags::empty(),
        }
    } else {
        LogLevelDirective {
            level: None,
            enable_flags: LogFlags::empty(),
            disable_flags: flags,
        }
    })
}

fn parse_log_flags_and_level(value: &str) -> Option<LogLevelDirective> {
    let (flags, level) = value.rsplit_once('+')?;
    let level = parse_plain_log_level(level)?;
    Some(LogLevelDirective {
        level: Some(level),
        enable_flags: parse_log_flag_list(flags)?,
        disable_flags: LogFlags::empty(),
    })
}

fn parse_log_flag_list(value: &str) -> Option<LogFlags> {
    let mut flags = LogFlags::empty();
    for flag_name in value.split('+') {
        let flag = parse_log_flag_name(flag_name)?;
        flags.insert(flag);
    }
    Some(flags)
}

fn parse_log_flag_name(value: &str) -> Option<LogFlags> {
    if value.eq_ignore_ascii_case("repeat") {
        Some(LogFlags::SKIP_REPEATED)
    } else if value.eq_ignore_ascii_case("level") {
        Some(LogFlags::PRINT_LEVEL)
    } else if value.eq_ignore_ascii_case("time") {
        Some(LogFlags::PRINT_TIME)
    } else if value.eq_ignore_ascii_case("datetime") {
        Some(LogFlags::PRINT_DATETIME)
    } else {
        None
    }
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
        assert_eq!(config.flags(), LogFlags::empty());
        assert_eq!(parse_log_level_value("48"), Some(LogLevel::Debug));
        assert_eq!(parse_log_level_value("ERROR"), Some(LogLevel::Error));
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
        assert!(config.flags().contains(LogFlags::SKIP_REPEATED));
        assert!(config.flags().contains(LogFlags::PRINT_TIME));
        assert!(!config.flags().contains(LogFlags::PRINT_LEVEL));
        assert_eq!(
            parse_log_level_value("REPEAT+datetime+trace"),
            Some(LogLevel::Trace)
        );
        assert_eq!(parse_log_level_value("+repeat"), None);
        assert!(parse_log_level_directive("+repeat").is_some());
        assert!(parse_log_level_directive("repeat").is_none());
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

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }
}
