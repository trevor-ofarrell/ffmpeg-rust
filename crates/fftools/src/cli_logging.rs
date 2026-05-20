use crate::option_parser::{apply_log_level_value, parse_log_level_directive};
use crate::CliLogConfig;
use avutil::{LogFormatOptions, LogLevel, LogRecord};
use std::fmt;

pub(crate) fn tool_error_stderr(
    tool_name: &str,
    args: &[String],
    error: impl fmt::Display,
) -> String {
    let log_config = log_config_from_args(args);
    if log_config.level() == LogLevel::Quiet {
        return String::new();
    }

    let record = LogRecord::new(LogLevel::Error, tool_name, error.to_string());
    format!(
        "{}\n",
        record.format_line_with_options(LogFormatOptions::new(log_config.flags()))
    )
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
