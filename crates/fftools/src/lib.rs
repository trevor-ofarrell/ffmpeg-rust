//! Command-line compatibility helpers for ffmpeg-like tools.

#![forbid(unsafe_code)]

pub mod ffprobe;
pub mod io_plan;
pub mod option_parser;

pub use ffprobe::{
    ffprobe_output, probe_local_file, run_ffprobe_tool, FfprobeError, FfprobeReport,
};
pub use io_plan::{build_io_plan, Endpoint, FileRole, IoPlan, IoPlanError, PlannedFile};
pub use option_parser::{parse_ffmpeg_args, CliFile, CliOption, CliParseError, ParsedCommand};

pub const TARGET_FFMPEG_VERSION: &str = "8.1.1";
pub const TARGET_RELEASE_NAME: &str = "Hoare";

const LIBRARY_VERSIONS: &[(&str, &str)] = &[
    ("libavutil", "60.26.101"),
    ("libavcodec", "62.28.101"),
    ("libavformat", "62.12.101"),
    ("libavdevice", "62.3.101"),
    ("libavfilter", "11.14.101"),
    ("libswscale", "9.5.101"),
    ("libswresample", "6.3.101"),
];

pub fn version_banner(tool_name: &str) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{tool_name} version {TARGET_FFMPEG_VERSION}-rust target FFmpeg {TARGET_FFMPEG_VERSION} \"{TARGET_RELEASE_NAME}\"\n"
    ));
    out.push_str("built with rustc\n");
    out.push_str("configuration: --disable-gpl --disable-nonfree --disable-doc\n");
    for (name, version) in LIBRARY_VERSIONS {
        out.push_str(&format!("{name:>13} {version}\n"));
    }
    out
}

pub fn run_version_tool(tool_name: &str, args: &[String]) -> i32 {
    let hide_banner = args.iter().any(|arg| arg == "-hide_banner");
    let asks_version = args
        .iter()
        .any(|arg| arg == "-version" || arg == "--version");

    if asks_version {
        print!("{}", version_banner(tool_name));
        return 0;
    }

    if hide_banner && args.len() == 1 {
        eprintln!("{tool_name}: missing command");
        return 1;
    }

    if args.is_empty() {
        eprintln!("{tool_name}: missing command");
        return 1;
    }

    eprintln!(
        "{tool_name}: unsupported arguments in current compatibility slice: {}",
        args.join(" ")
    );
    2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_banner_names_target_and_libraries() {
        let banner = version_banner("ffprobe");

        assert!(banner.starts_with("ffprobe version 8.1.1-rust target FFmpeg 8.1.1"));
        assert!(banner.contains("libavutil"));
        assert!(banner.contains("60.26.101"));
        assert!(banner.contains("--disable-gpl --disable-nonfree --disable-doc"));
    }

    #[test]
    fn version_request_exits_successfully() {
        let args = vec!["-version".to_string()];

        assert_eq!(run_version_tool("ffmpeg", &args), 0);
    }
}
