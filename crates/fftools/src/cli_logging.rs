use crate::option_parser::{apply_log_level_value, parse_log_level_directive};
use crate::CliLogConfig;
use avutil::{LogFlags, LogFormatOptions, LogLevel, LogRecord, LogTimestamp};
use std::fmt;

pub(crate) fn tool_error_stderr(
    tool_name: &str,
    args: &[String],
    error: impl fmt::Display,
) -> String {
    let log_config = log_config_from_args(args);
    let timestamp = timestamp_for_flags(log_config.flags());
    let format_options = LogFormatOptions::new(log_config.flags()).with_ffmpeg_env_color();
    format_tool_error_stderr(tool_name, error, log_config, timestamp, format_options)
}

#[cfg(test)]
fn tool_error_stderr_with_timestamp(
    tool_name: &str,
    args: &[String],
    error: impl fmt::Display,
    timestamp: Option<LogTimestamp>,
) -> String {
    let log_config = log_config_from_args(args);
    let format_options = LogFormatOptions::new(log_config.flags());
    format_tool_error_stderr(tool_name, error, log_config, timestamp, format_options)
}

#[cfg(test)]
fn tool_error_stderr_with_timestamp_and_color_env(
    tool_name: &str,
    args: &[String],
    error: impl fmt::Display,
    timestamp: Option<LogTimestamp>,
    color_env_is_set: impl FnMut(&str) -> bool,
) -> String {
    let log_config = log_config_from_args(args);
    let format_options =
        LogFormatOptions::new(log_config.flags()).with_ffmpeg_env_color_vars(color_env_is_set);
    format_tool_error_stderr(tool_name, error, log_config, timestamp, format_options)
}

fn format_tool_error_stderr(
    tool_name: &str,
    error: impl fmt::Display,
    log_config: CliLogConfig,
    timestamp: Option<LogTimestamp>,
    format_options: LogFormatOptions,
) -> String {
    if log_config.level() == LogLevel::Quiet {
        return String::new();
    }

    let mut record = LogRecord::new(LogLevel::Error, tool_name, error.to_string());
    if let Some(timestamp) = timestamp {
        record = record.with_timestamp(timestamp);
    }
    format!("{}\n", record.format_line_with_options(format_options))
}

fn timestamp_for_flags(flags: LogFlags) -> Option<LogTimestamp> {
    if flags.intersects(LogFlags::PRINT_TIME | LogFlags::PRINT_DATETIME) {
        LogTimestamp::now_utc()
    } else {
        None
    }
}

fn log_config_from_args(args: &[String]) -> CliLogConfig {
    let mut config = CliLogConfig::default();
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "-loglevel" | "-v" => {
                if let Some(value) = args.get(index + 1) {
                    if parse_log_level_directive(value).is_some() {
                        let _ = apply_log_level_value(&mut config, value);
                        index += 2;
                        continue;
                    }
                }
                index += 1;
            }
            _ => index += 1,
        }
    }

    config
}

#[cfg(test)]
mod tests {
    use super::*;
    use avutil::{AV_LOG_FORCE_COLOR_ENV, AV_LOG_FORCE_NOCOLOR_ENV};

    #[test]
    fn formats_tool_error_without_level_by_default() {
        assert_eq!(
            tool_error_stderr("ffmpeg", &strings(&[]), "missing command"),
            "ffmpeg: missing command\n"
        );
    }

    #[test]
    fn quiet_loglevel_suppresses_tool_errors() {
        assert_eq!(
            tool_error_stderr(
                "ffprobe",
                &strings(&["-loglevel", "quiet"]),
                "missing command"
            ),
            ""
        );
        assert_eq!(
            tool_error_stderr("ffmpeg", &strings(&["-v", "-8"]), "missing command"),
            ""
        );
        assert_eq!(
            tool_error_stderr_with_timestamp_and_color_env(
                "ffmpeg",
                &strings(&["-loglevel", "quiet"]),
                "missing command",
                None,
                |name| name == AV_LOG_FORCE_COLOR_ENV,
            ),
            ""
        );
    }

    #[test]
    fn print_level_flag_adds_error_prefix() {
        assert_eq!(
            tool_error_stderr(
                "ffmpeg",
                &strings(&["-loglevel", "level+error"]),
                "missing command"
            ),
            "[error] ffmpeg: missing command\n"
        );
    }

    #[test]
    fn time_flags_add_timestamp_prefix_to_tool_errors() {
        let timestamp = LogTimestamp::from_unix_micros(1_704_112_705_123_456);

        assert_eq!(
            tool_error_stderr_with_timestamp(
                "ffmpeg",
                &strings(&["-loglevel", "time+error"]),
                "missing command",
                Some(timestamp),
            ),
            "[12:38:25.123456] ffmpeg: missing command\n"
        );
        assert_eq!(
            tool_error_stderr_with_timestamp(
                "ffprobe",
                &strings(&["-loglevel", "time+datetime+level+error"]),
                "missing command",
                Some(timestamp),
            ),
            "[2024-01-01 12:38:25.123456] [error] ffprobe: missing command\n"
        );
    }

    #[test]
    fn time_flags_without_timestamp_keep_previous_tool_error_shape() {
        assert_eq!(
            tool_error_stderr_with_timestamp(
                "ffmpeg",
                &strings(&["-loglevel", "time+level+error"]),
                "missing command",
                None,
            ),
            "[error] ffmpeg: missing command\n"
        );
    }

    #[test]
    fn force_color_env_colors_tool_errors() {
        assert_eq!(
            tool_error_stderr_with_timestamp_and_color_env(
                "ffmpeg",
                &strings(&["-loglevel", "level+error"]),
                "missing command",
                None,
                |name| name == AV_LOG_FORCE_COLOR_ENV,
            ),
            "\x1b[31m[error] ffmpeg: missing command\x1b[0m\n"
        );
    }

    #[test]
    fn force_nocolor_env_wins_over_force_color_for_tool_errors() {
        let mut checked = Vec::new();
        let stderr = tool_error_stderr_with_timestamp_and_color_env(
            "ffprobe",
            &strings(&["-loglevel", "level+error"]),
            "missing command",
            None,
            |name| {
                checked.push(name.to_owned());
                name == AV_LOG_FORCE_NOCOLOR_ENV || name == AV_LOG_FORCE_COLOR_ENV
            },
        );

        assert_eq!(stderr, "[error] ffprobe: missing command\n");
        assert_eq!(checked, [AV_LOG_FORCE_NOCOLOR_ENV.to_owned()]);
    }

    #[test]
    fn later_loglevel_values_win() {
        assert_eq!(
            tool_error_stderr(
                "ffprobe",
                &strings(&["-v", "level+warning", "-loglevel", "quiet"]),
                "missing command"
            ),
            ""
        );
        assert_eq!(
            tool_error_stderr(
                "ffprobe",
                &strings(&["-v", "quiet", "-loglevel", "level+error"]),
                "missing command"
            ),
            "[error] ffprobe: missing command\n"
        );
    }

    #[test]
    fn malformed_loglevel_arguments_fall_back_to_default_formatting() {
        assert_eq!(
            tool_error_stderr("ffmpeg", &strings(&["-loglevel"]), "missing command"),
            "ffmpeg: missing command\n"
        );
        assert_eq!(
            tool_error_stderr("ffmpeg", &strings(&["-v", "-bogus"]), "missing command"),
            "ffmpeg: missing command\n"
        );
    }

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }
}
