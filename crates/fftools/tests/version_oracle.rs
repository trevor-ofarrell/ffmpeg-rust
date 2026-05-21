use fftools::{
    version_banner, TARGET_FFMPEG_VERSION, TARGET_LIBRARY_VERSIONS, TARGET_RELEASE_NAME,
};
use std::{
    collections::BTreeMap,
    env,
    path::{Path, PathBuf},
    process::Command,
};

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn ffmpeg_version_banner_matches_oracle_target_versions() {
    compare_version_surface("ffmpeg");
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFPROBE_ORACLE, set FFMPEG_ORACLE with sibling ffprobe, or install third_party/ffmpeg-oracle/build/bin/ffprobe"]
fn ffprobe_version_banner_matches_oracle_target_versions() {
    compare_version_surface("ffprobe");
}

#[test]
fn parses_split_ffmpeg_library_version_lines() {
    let versions = parse_library_versions(
        "ffmpeg version 8.1.1 Copyright (c) 2000-2026 the FFmpeg developers\n\
         libavutil      60. 26.101 / 60. 26.101\n\
         libavcodec     62. 28.101 / 62. 28.101\n\
         libswscale      9.  5.101 /  9.  5.101\n",
    );

    assert_eq!(
        versions.get("libavutil").map(String::as_str),
        Some("60.26.101")
    );
    assert_eq!(
        versions.get("libavcodec").map(String::as_str),
        Some("62.28.101")
    );
    assert_eq!(
        versions.get("libswscale").map(String::as_str),
        Some("9.5.101")
    );
}

#[test]
fn rust_banner_library_versions_are_parseable() {
    let versions = parse_library_versions(&version_banner("ffmpeg"));

    for (name, version) in TARGET_LIBRARY_VERSIONS {
        assert_eq!(versions.get(*name).map(String::as_str), Some(*version));
    }
}

fn compare_version_surface(tool_name: &str) {
    let oracle = oracle_tool(tool_name);
    let oracle_stdout = run_oracle_version(&oracle, tool_name);
    let oracle_first_line = oracle_stdout
        .lines()
        .next()
        .expect("oracle version output should include a first line");
    let expected_prefix = format!("{tool_name} version {TARGET_FFMPEG_VERSION}");
    assert!(
        oracle_first_line.starts_with(&expected_prefix),
        "oracle `{}` first line should start with `{expected_prefix}`, got `{oracle_first_line}`",
        oracle.display()
    );

    let rust_stdout = version_banner(tool_name);
    let rust_first_line = rust_stdout
        .lines()
        .next()
        .expect("Rust version banner should include a first line");
    assert!(
        rust_first_line.starts_with(&format!(
            "{tool_name} version {TARGET_FFMPEG_VERSION}-rust target FFmpeg {TARGET_FFMPEG_VERSION} \"{TARGET_RELEASE_NAME}\""
        )),
        "Rust first line did not name the pinned FFmpeg target: `{rust_first_line}`"
    );

    let oracle_versions = parse_library_versions(&oracle_stdout);
    let rust_versions = parse_library_versions(&rust_stdout);
    for (name, expected) in TARGET_LIBRARY_VERSIONS {
        assert_eq!(
            oracle_versions.get(*name).map(String::as_str),
            Some(*expected),
            "oracle `{}` did not report the pinned {name} ABI version",
            oracle.display()
        );
        assert_eq!(
            rust_versions.get(*name).map(String::as_str),
            Some(*expected),
            "Rust {tool_name} banner did not report the pinned {name} ABI version"
        );
    }
}

fn run_oracle_version(path: &Path, tool_name: &str) -> String {
    let output = Command::new(path)
        .arg("-version")
        .output()
        .unwrap_or_else(|err| {
            panic!(
                "failed to run {tool_name} oracle `{}`: {err}",
                path.display()
            )
        });

    assert!(
        output.status.success(),
        "{tool_name} oracle `{}` failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        path.display(),
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout)
        .unwrap_or_else(|err| panic!("{tool_name} oracle output must be UTF-8: {err}"))
}

fn parse_library_versions(stdout: &str) -> BTreeMap<String, String> {
    stdout
        .lines()
        .filter_map(parse_library_version_line)
        .collect()
}

fn parse_library_version_line(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with("lib") {
        return None;
    }

    let mut parts = trimmed.splitn(2, char::is_whitespace);
    let name = parts.next()?;
    let rest = parts.next()?.trim();
    let first_version = rest.split('/').next()?.trim();
    if first_version.is_empty() {
        return None;
    }

    let normalized = first_version.split_whitespace().collect::<String>();
    Some((name.to_owned(), normalized))
}

fn oracle_tool(tool_name: &str) -> PathBuf {
    let env_var = format!("{}_ORACLE", tool_name.to_ascii_uppercase());
    if let Ok(path) = env::var(&env_var) {
        return require_tool_path(PathBuf::from(path), &env_var);
    }

    if tool_name == "ffprobe" {
        if let Ok(ffmpeg_path) = env::var("FFMPEG_ORACLE") {
            let ffmpeg_path = PathBuf::from(ffmpeg_path);
            for candidate in sibling_tool_candidates(&ffmpeg_path, "ffprobe") {
                if candidate.is_file() {
                    return candidate;
                }
            }
        }
    }

    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("fftools crate should be under crates/");
    for candidate in default_tool_candidates(root, tool_name) {
        if candidate.is_file() {
            return candidate;
        }
    }

    panic!(
        "missing pinned {tool_name} oracle; set {env_var} or install `{}`",
        root.join("third_party/ffmpeg-oracle/build/bin")
            .join(tool_name)
            .display()
    );
}

fn require_tool_path(path: PathBuf, env_var: &str) -> PathBuf {
    assert!(
        path.is_file(),
        "{env_var} must point to the pinned FFmpeg 8.1.1 binary, got `{}`",
        path.display()
    );
    path
}

fn sibling_tool_candidates(ffmpeg_path: &Path, tool_name: &str) -> Vec<PathBuf> {
    let Some(parent) = ffmpeg_path.parent() else {
        return Vec::new();
    };

    let mut candidates = Vec::new();
    if ffmpeg_path
        .extension()
        .is_some_and(|extension| extension == "exe")
    {
        candidates.push(parent.join(format!("{tool_name}.exe")));
    }
    candidates.push(parent.join(tool_name));
    candidates.push(parent.join(format!("{tool_name}.exe")));
    candidates
}

fn default_tool_candidates(root: &Path, tool_name: &str) -> [PathBuf; 2] {
    [
        root.join("third_party/ffmpeg-oracle/build/bin")
            .join(tool_name),
        root.join("third_party/ffmpeg-oracle/build/bin")
            .join(format!("{tool_name}.exe")),
    ]
}
