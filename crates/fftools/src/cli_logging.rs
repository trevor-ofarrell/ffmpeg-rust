use crate::option_parser::{apply_log_level_value, parse_log_level_directive};
use crate::CliLogConfig;
use avutil::{LogFlags, LogFormatOptions, LogRecord, LogTimestamp, Logger};
use std::fmt;

pub(crate) fn tool_error_stderr(
    tool_name: &str,
    args: &[String],
    error: impl fmt::Display,
) -> String {
    let log_config = log_config_from_args(args);
    let timestamp = timestamp_for_flags(log_config.flags());
    let format_options = LogFormatOptions::new(log_config.flags()).with_ffmpeg_env_color();
    format_tool_diagnostics_stderr(tool_name, [error], log_config, timestamp, format_options)
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
    format_tool_diagnostics_stderr(tool_name, [error], log_config, timestamp, format_options)
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
    format_tool_diagnostics_stderr(tool_name, [error], log_config, timestamp, format_options)
}

#[cfg(test)]
fn tool_error_stderr_with_timestamp_color_env_and_terminal(
    tool_name: &str,
    args: &[String],
    error: impl fmt::Display,
    timestamp: Option<LogTimestamp>,
    color_env_is_set: impl FnMut(&str) -> bool,
    stderr_is_terminal: bool,
) -> String {
    let log_config = log_config_from_args(args);
    let format_options = LogFormatOptions::new(log_config.flags())
        .with_ffmpeg_env_color_vars_and_stderr(color_env_is_set, stderr_is_terminal);
    format_tool_diagnostics_stderr(tool_name, [error], log_config, timestamp, format_options)
}

#[cfg(test)]
fn tool_diagnostics_stderr_with_timestamp(
    tool_name: &str,
    args: &[String],
    diagnostics: &[&str],
    timestamp: Option<LogTimestamp>,
) -> String {
    let log_config = log_config_from_args(args);
    let format_options = LogFormatOptions::new(log_config.flags());
    format_tool_diagnostics_stderr(
        tool_name,
        diagnostics.iter().copied(),
        log_config,
        timestamp,
        format_options,
    )
}

fn format_tool_diagnostics_stderr<I, E>(
    tool_name: &str,
    diagnostics: I,
    log_config: CliLogConfig,
    timestamp: Option<LogTimestamp>,
    format_options: LogFormatOptions,
) -> String
where
    I: IntoIterator<Item = E>,
    E: fmt::Display,
{
    let mut logger = Logger::new_with_raw_level(log_config.raw_level(), log_config.flags());
    for diagnostic in diagnostics {
        let mut record = LogRecord::new(avutil::LogLevel::Error, tool_name, diagnostic.to_string());
        if let Some(timestamp) = timestamp {
            record = record.with_timestamp(timestamp);
        }
        logger.log(record);
    }

    let lines: Vec<_> = logger
        .formatted_records_with_options(format_options)
        .into_iter()
        .map(format_tool_stderr_line)
        .collect();
    if lines.is_empty() {
        String::new()
    } else {
        format!("{}\n", lines.join("\n"))
    }
}

fn format_tool_stderr_line(line: String) -> String {
    if line.starts_with("Last message repeated ") {
        format!("    {line}")
    } else {
        line
    }
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
    fn raw_numeric_loglevel_thresholds_filter_tool_errors() {
        assert_eq!(
            tool_error_stderr("ffmpeg", &strings(&["-loglevel", "15"]), "missing command"),
            ""
        );
        assert_eq!(
            tool_error_stderr("ffmpeg", &strings(&["-loglevel", "23"]), "missing command"),
            "ffmpeg: missing command\n"
        );
        assert_eq!(
            tool_error_stderr(
                "ffprobe",
                &strings(&["-loglevel", "level+23"]),
                "missing command"
            ),
            "[error] ffprobe: missing command\n"
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
    fn repeated_tool_errors_are_compressed_by_default() {
        assert_eq!(
            tool_diagnostics_stderr_with_timestamp(
                "ffmpeg",
                &strings(&[]),
                &["bad packet", "bad packet", "bad packet", "bad output"],
                None,
            ),
            "ffmpeg: bad packet\n    Last message repeated 2 times\nffmpeg: bad output\n"
        );
    }

    #[test]
    fn minus_repeat_flag_compresses_repeated_tool_errors() {
        assert_eq!(
            tool_diagnostics_stderr_with_timestamp(
                "ffmpeg",
                &strings(&["-loglevel", "-repeat+level+error"]),
                &["bad packet", "bad packet", "bad packet", "bad output"],
                None,
            ),
            "[error] ffmpeg: bad packet\n    Last message repeated 2 times\n[error] ffmpeg: bad output\n"
        );
    }

    #[test]
    fn repeat_flag_preserves_repeated_tool_errors() {
        assert_eq!(
            tool_diagnostics_stderr_with_timestamp(
                "ffmpeg",
                &strings(&["-loglevel", "repeat+level+error"]),
                &["bad packet", "bad packet"],
                None,
            ),
            "[error] ffmpeg: bad packet\n[error] ffmpeg: bad packet\n"
        );
    }

    #[test]
    fn absolute_level_flag_preserves_repeated_tool_errors() {
        assert_eq!(
            tool_diagnostics_stderr_with_timestamp(
                "ffprobe",
                &strings(&["-loglevel", "level+error"]),
                &["bad packet", "bad packet"],
                None,
            ),
            "[error] ffprobe: bad packet\n[error] ffprobe: bad packet\n"
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
    fn terminal_stderr_colors_tool_errors_without_force_env() {
        assert_eq!(
            tool_error_stderr_with_timestamp_color_env_and_terminal(
                "ffmpeg",
                &strings(&["-loglevel", "level+error"]),
                "missing command",
                None,
                |_| false,
                true,
            ),
            "\x1b[31m[error] ffmpeg: missing command\x1b[0m\n"
        );
        assert_eq!(
            tool_error_stderr_with_timestamp_color_env_and_terminal(
                "ffprobe",
                &strings(&["-loglevel", "level+error"]),
                "missing command",
                None,
                |_| false,
                false,
            ),
            "[error] ffprobe: missing command\n"
        );
    }

    #[test]
    fn force_nocolor_env_wins_over_terminal_for_tool_errors() {
        let mut checked = Vec::new();
        let stderr = tool_error_stderr_with_timestamp_color_env_and_terminal(
            "ffmpeg",
            &strings(&["-loglevel", "level+error"]),
            "missing command",
            None,
            |name| {
                checked.push(name.to_owned());
                name == AV_LOG_FORCE_NOCOLOR_ENV
            },
            true,
        );

        assert_eq!(stderr, "[error] ffmpeg: missing command\n");
        assert_eq!(checked, [AV_LOG_FORCE_NOCOLOR_ENV.to_owned()]);
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
