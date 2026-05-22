use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const FORBIDDEN_FFMPEG_DEPENDENCIES: &[&str] = &[
    "ffmpeg",
    "ffmpeg-next",
    "ffmpeg-sys",
    "ffmpeg-sys-next",
    "libav-sys",
    "libavcodec-sys",
    "libavdevice-sys",
    "libavfilter-sys",
    "libavformat-sys",
    "libavutil-sys",
    "libswresample-sys",
    "libswscale-sys",
    "rsmpeg",
];

const FORBIDDEN_RUST_TOKENS: &[&str] = &[
    "ffmpeg_sys",
    "ffmpeg_next",
    "libav_sys",
    "libavcodec_sys",
    "libavdevice_sys",
    "libavfilter_sys",
    "libavformat_sys",
    "libavutil_sys",
    "libswresample_sys",
    "libswscale_sys",
    "rsmpeg",
];

const FORBIDDEN_RUNTIME_SHELL_PATTERNS: &[&str] = &[
    "Command::new(\"ffmpeg\")",
    "Command::new(\"ffprobe\")",
    "Command::new(\"ffplay\")",
];

const RUNTIME_CRATES: &[&str] = &[
    "crates/avutil",
    "crates/avcodec",
    "crates/avformat",
    "crates/avfilter",
    "crates/avdevice",
    "crates/swscale",
    "crates/swresample",
    "crates/fftools",
];

const TARGET_FFMPEG_VERSION: &str = "8.1.1";
const EXPECTED_LIBRARY_ABIS: &[(&str, &str)] = &[
    ("libavutil", "60.26.101"),
    ("libavcodec", "62.28.101"),
    ("libavformat", "62.12.101"),
    ("libavdevice", "62.3.101"),
    ("libavfilter", "11.14.101"),
    ("libswscale", "9.5.101"),
    ("libswresample", "6.3.101"),
];

#[derive(Debug, Default, PartialEq, Eq)]
struct OracleDoctorArgs {
    ffmpeg: Option<PathBuf>,
    ffprobe: Option<PathBuf>,
}

fn main() {
    match real_main() {
        Ok(()) => {}
        Err(err) => {
            eprintln!("xtask: {err}");
            std::process::exit(1);
        }
    }
}

fn real_main() -> Result<(), String> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("quick") => quick(),
        Some("changed") => changed(),
        Some("full") => full(),
        Some("inventory") => inventory(args.collect()),
        Some("oracle-doctor") => oracle_doctor(args.collect()),
        Some("guard-runtime") => guard_runtime(),
        Some("help") | Some("--help") | Some("-h") => {
            print_help();
            Ok(())
        }
        Some(other) => Err(format!("unsupported command `{other}`")),
        None => {
            print_help();
            Err("missing command".to_string())
        }
    }
}

fn quick() -> Result<(), String> {
    guard_runtime()?;
    run("cargo", &["test", "-p", "avutil", "-p", "fftools"])
}

fn changed() -> Result<(), String> {
    quick()
}

fn full() -> Result<(), String> {
    guard_runtime()?;
    run("cargo", &["fmt", "--all", "--", "--check"])?;
    run(
        "cargo",
        &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--all-features",
            "--",
            "-D",
            "warnings",
        ],
    )?;
    run("cargo", &["test", "--workspace", "--all-features"])
}

fn inventory(args: Vec<String>) -> Result<(), String> {
    let mut command_args = vec!["run", "-p", "oracle", "--", "inventory"];
    let owned: Vec<String> = args;
    for arg in &owned {
        command_args.push(arg);
    }
    run("cargo", &command_args)
}

fn oracle_doctor(args: Vec<String>) -> Result<(), String> {
    let args = parse_oracle_doctor_args(args)?;
    let root = env::current_dir().map_err(|err| format!("failed to read current dir: {err}"))?;
    let ffmpeg = match args.ffmpeg {
        Some(path) => path,
        None => find_default_oracle_tool(&root, "ffmpeg")?,
    };
    let ffprobe = match args.ffprobe {
        Some(path) => path,
        None => find_default_oracle_tool(&root, "ffprobe")?,
    };

    let ffmpeg_output = run_version_command(&ffmpeg)?;
    validate_version_output("ffmpeg", &ffmpeg_output)?;
    let ffprobe_output = run_version_command(&ffprobe)?;
    validate_version_output("ffprobe", &ffprobe_output)?;

    println!(
        "oracle doctor passed: {} and {} report FFmpeg {} with pinned library ABIs",
        ffmpeg.display(),
        ffprobe.display(),
        TARGET_FFMPEG_VERSION
    );
    Ok(())
}

fn parse_oracle_doctor_args(args: Vec<String>) -> Result<OracleDoctorArgs, String> {
    let mut parsed = OracleDoctorArgs::default();
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--ffmpeg" => {
                let value = iter
                    .next()
                    .ok_or_else(|| "--ffmpeg requires a path".to_string())?;
                parsed.ffmpeg = Some(PathBuf::from(value));
            }
            "--ffprobe" => {
                let value = iter
                    .next()
                    .ok_or_else(|| "--ffprobe requires a path".to_string())?;
                parsed.ffprobe = Some(PathBuf::from(value));
            }
            other => return Err(format!("unsupported oracle-doctor argument `{other}`")),
        }
    }
    Ok(parsed)
}

fn find_default_oracle_tool(root: &Path, tool: &str) -> Result<PathBuf, String> {
    let candidates = default_oracle_tool_candidates(root, tool);
    candidates
        .iter()
        .find(|path| path.is_file())
        .cloned()
        .ok_or_else(|| {
            let searched = candidates
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "could not find local pinned {tool} oracle; searched {searched}. \
Run scripts/bootstrap_ffmpeg_oracle_wsl.sh from WSL on this Windows workspace."
            )
        })
}

fn default_oracle_tool_candidates(root: &Path, tool: &str) -> Vec<PathBuf> {
    let bin = root.join("third_party/ffmpeg-oracle/build/bin");
    if cfg!(windows) {
        vec![
            bin.join(format!("{tool}.exe")),
            bin.join(format!("{tool}.cmd")),
            bin.join(tool),
        ]
    } else {
        vec![
            bin.join(tool),
            bin.join(format!("{tool}.exe")),
            bin.join(format!("{tool}.cmd")),
        ]
    }
}

fn run_version_command(path: &Path) -> Result<String, String> {
    let output = Command::new(path)
        .arg("-version")
        .output()
        .map_err(|err| format!("failed to run {} -version: {err}", path.display()))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = if stderr.is_empty() {
        stdout.into_owned()
    } else {
        format!("{stdout}{stderr}")
    };

    if output.status.success() {
        Ok(combined)
    } else {
        Err(format!(
            "{} -version exited with status {}; output:\n{}",
            path.display(),
            output.status,
            combined
        ))
    }
}

fn validate_version_output(tool: &str, output: &str) -> Result<(), String> {
    let expected_prefix = format!("{tool} version {TARGET_FFMPEG_VERSION}");
    let first_line = output.lines().next().unwrap_or_default();
    if !first_line.starts_with(&expected_prefix) {
        return Err(format!(
            "{tool} oracle version mismatch: expected first line to start with `{expected_prefix}`, got `{first_line}`"
        ));
    }

    for (library, abi) in EXPECTED_LIBRARY_ABIS {
        if !output
            .lines()
            .any(|line| library_version_line_matches(line, library, abi))
        {
            return Err(format!(
                "{tool} oracle ABI mismatch: missing `{library} {abi} / {abi}`"
            ));
        }
    }

    Ok(())
}

fn library_version_line_matches(line: &str, library: &str, abi: &str) -> bool {
    let normalized: String = line.chars().filter(|ch| !ch.is_whitespace()).collect();
    let expected = format!("{library}{abi}/{abi}");
    normalized.starts_with(&expected)
}

fn guard_runtime() -> Result<(), String> {
    let root = env::current_dir().map_err(|err| format!("failed to read current dir: {err}"))?;
    let mut violations = Vec::new();

    for manifest in manifest_paths(&root)? {
        let contents = fs::read_to_string(&manifest)
            .map_err(|err| format!("failed to read {}: {err}", manifest.display()))?;
        violations.extend(manifest_dependency_violations(&manifest, &contents));
    }

    let lockfile = root.join("Cargo.lock");
    if lockfile.is_file() {
        let contents = fs::read_to_string(&lockfile)
            .map_err(|err| format!("failed to read {}: {err}", lockfile.display()))?;
        violations.extend(lockfile_dependency_violations(&lockfile, &contents));
    }

    for source in runtime_source_paths(&root)? {
        let contents = fs::read_to_string(&source)
            .map_err(|err| format!("failed to read {}: {err}", source.display()))?;
        violations.extend(runtime_source_violations(&source, &contents));
    }

    if violations.is_empty() {
        println!(
            "runtime guard passed: no FFmpeg wrapper dependencies, lockfile packages, or runtime shell-outs found"
        );
        Ok(())
    } else {
        Err(format!(
            "runtime guard found forbidden FFmpeg runtime linkage:\n{}",
            violations.join("\n")
        ))
    }
}

fn manifest_paths(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut paths = vec![root.join("Cargo.toml")];
    let crates_dir = root.join("crates");
    if crates_dir.is_dir() {
        for entry in fs::read_dir(&crates_dir)
            .map_err(|err| format!("failed to read {}: {err}", crates_dir.display()))?
        {
            let entry = entry
                .map_err(|err| format!("failed to read {} entry: {err}", crates_dir.display()))?;
            let path = entry.path().join("Cargo.toml");
            if path.is_file() {
                paths.push(path);
            }
        }
    }
    for relative in ["fuzz/Cargo.toml", "xtask/Cargo.toml"] {
        let path = root.join(relative);
        if path.is_file() {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

fn runtime_source_paths(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut paths = Vec::new();
    for relative in RUNTIME_CRATES {
        let path = root.join(relative);
        if path.is_dir() {
            collect_rs_files(&path, &mut paths)?;
        }
    }
    paths.sort();
    Ok(paths)
}

fn collect_rs_files(dir: &Path, paths: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in
        fs::read_dir(dir).map_err(|err| format!("failed to read {}: {err}", dir.display()))?
    {
        let entry =
            entry.map_err(|err| format!("failed to read {} entry: {err}", dir.display()))?;
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, paths)?;
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            paths.push(path);
        }
    }
    Ok(())
}

fn manifest_dependency_violations(path: &Path, contents: &str) -> Vec<String> {
    let mut violations = Vec::new();
    let mut in_dependency_section = false;

    for (index, line) in contents.lines().enumerate() {
        let line_number = index + 1;
        let trimmed = line.split('#').next().unwrap_or("").trim();
        if trimmed.is_empty() {
            continue;
        }

        if trimmed.starts_with('[') {
            let section = trimmed.trim_matches(|ch| ch == '[' || ch == ']').trim();
            in_dependency_section = is_dependency_section(section);
            if let Some(name) = dependency_name_from_section(section) {
                push_forbidden_dependency_violation(path, line_number, &name, &mut violations);
            }
            continue;
        }

        if !in_dependency_section {
            continue;
        }

        if let Some((name, value)) = trimmed.split_once('=') {
            let name = name.trim().trim_matches('"');
            if !matches!(
                name,
                "version" | "path" | "package" | "features" | "default-features"
            ) {
                push_forbidden_dependency_violation(path, line_number, name, &mut violations);
            }
            if let Some(package) = package_rename(value) {
                push_forbidden_dependency_violation(path, line_number, &package, &mut violations);
            }
        }
    }

    violations
}

fn lockfile_dependency_violations(path: &Path, contents: &str) -> Vec<String> {
    let mut violations = Vec::new();
    let mut in_package = false;

    for (index, line) in contents.lines().enumerate() {
        let line_number = index + 1;
        let trimmed = line.split('#').next().unwrap_or("").trim();
        if trimmed.is_empty() {
            continue;
        }

        if trimmed == "[[package]]" {
            in_package = true;
            continue;
        }

        if trimmed.starts_with('[') {
            in_package = false;
            continue;
        }

        if !in_package {
            continue;
        }

        if let Some((field, value)) = trimmed.split_once('=') {
            if field.trim() == "name" {
                if let Some(name) = first_quoted_value(value) {
                    push_forbidden_dependency_violation(path, line_number, &name, &mut violations);
                }
            }
        }
    }

    violations
}

fn is_dependency_section(section: &str) -> bool {
    section == "dependencies"
        || section == "dev-dependencies"
        || section == "build-dependencies"
        || section.ends_with(".dependencies")
        || section.ends_with(".dev-dependencies")
        || section.ends_with(".build-dependencies")
        || section.contains(".dependencies.")
        || section.contains(".dev-dependencies.")
        || section.contains(".build-dependencies.")
}

fn dependency_name_from_section(section: &str) -> Option<String> {
    for marker in ["dependencies.", "dev-dependencies.", "build-dependencies."] {
        if let Some((_, name)) = section.rsplit_once(marker) {
            return Some(name.trim().trim_matches('"').to_string());
        }
    }
    None
}

fn package_rename(value: &str) -> Option<String> {
    let package_index = value.find("package")?;
    let after_package = &value[package_index..];
    first_quoted_value(after_package)
}

fn first_quoted_value(value: &str) -> Option<String> {
    let first_quote = value.find('"')?;
    let after_first_quote = &value[first_quote + 1..];
    let second_quote = after_first_quote.find('"')?;
    Some(after_first_quote[..second_quote].to_string())
}

fn push_forbidden_dependency_violation(
    path: &Path,
    line_number: usize,
    name: &str,
    violations: &mut Vec<String>,
) {
    let normalized = normalize_dependency_name(name);
    if FORBIDDEN_FFMPEG_DEPENDENCIES.contains(&normalized.as_str()) {
        violations.push(format!(
            "{}:{line_number}: forbidden FFmpeg wrapper dependency `{name}`",
            path.display()
        ));
    }
}

fn normalize_dependency_name(name: &str) -> String {
    name.trim_matches('"')
        .trim()
        .to_ascii_lowercase()
        .replace('_', "-")
}

fn runtime_source_violations(path: &Path, contents: &str) -> Vec<String> {
    let mut violations = Vec::new();
    for (index, line) in contents.lines().enumerate() {
        let line_number = index + 1;
        for token in FORBIDDEN_RUST_TOKENS {
            if line.contains(token) {
                violations.push(format!(
                    "{}:{line_number}: forbidden FFmpeg wrapper token `{token}`",
                    path.display()
                ));
            }
        }
        for pattern in FORBIDDEN_RUNTIME_SHELL_PATTERNS {
            if line.contains(pattern) {
                violations.push(format!(
                    "{}:{line_number}: forbidden runtime FFmpeg process spawn `{pattern}`",
                    path.display()
                ));
            }
        }
    }
    violations
}

fn run(program: &str, args: &[&str]) -> Result<(), String> {
    let status = Command::new(program)
        .args(args)
        .status()
        .map_err(|err| format!("failed to run {program}: {err}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "`{program} {}` exited with status {status}",
            args.join(" ")
        ))
    }
}

fn print_help() {
    eprintln!(
        "usage: cargo run -p xtask -- <quick|changed|full|inventory|oracle-doctor|guard-runtime>"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dependency_guard_allows_local_project_crates() {
        let manifest = r#"
[dependencies]
avcodec = { path = "../avcodec" }
avformat = { path = "../avformat" }
libfuzzer-sys = "0.4"
"#;

        assert!(manifest_dependency_violations(Path::new("Cargo.toml"), manifest).is_empty());
    }

    #[test]
    fn dependency_guard_rejects_forbidden_dependency_names_and_renames() {
        let manifest = r#"
[dependencies]
ffmpeg-next = "7"
codec = { package = "libavcodec-sys", version = "1" }

[target.'cfg(windows)'.dependencies.ffmpeg_sys]
version = "1"
"#;

        let violations = manifest_dependency_violations(Path::new("Cargo.toml"), manifest);
        assert_eq!(violations.len(), 3);
        assert!(violations
            .iter()
            .any(|violation| violation.contains("ffmpeg-next")));
        assert!(violations
            .iter()
            .any(|violation| violation.contains("libavcodec-sys")));
        assert!(violations
            .iter()
            .any(|violation| violation.contains("ffmpeg_sys")));
    }

    #[test]
    fn lockfile_guard_rejects_forbidden_transitive_packages() {
        let lockfile = r#"
[[package]]
name = "serde"
version = "1.0.0"

[[package]]
name = "libavformat_sys"
version = "0.1.0"
"#;

        let violations = lockfile_dependency_violations(Path::new("Cargo.lock"), lockfile);
        assert_eq!(violations.len(), 1);
        assert!(violations
            .first()
            .is_some_and(|violation| violation.contains("libavformat_sys")));
    }

    #[test]
    fn runtime_source_guard_rejects_wrapper_imports_and_process_spawns() {
        let source = r#"
use ffmpeg_next as ffmpeg;

fn bad() {
    let _ = std::process::Command::new("ffmpeg");
}
"#;

        let violations = runtime_source_violations(Path::new("src/lib.rs"), source);
        assert_eq!(violations.len(), 2);
        assert!(violations
            .iter()
            .any(|violation| violation.contains("ffmpeg_next")));
        assert!(violations
            .iter()
            .any(|violation| violation.contains("Command::new(\"ffmpeg\")")));
    }

    #[test]
    fn runtime_source_guard_allows_ffmpeg_named_rust_binaries() {
        let source = r#"
const VERSION: &str = "ffmpeg-rs version 8.1.1-compatible";
fn binary_name() -> &'static str { "ffprobe-rs" }
"#;

        assert!(runtime_source_violations(Path::new("src/lib.rs"), source).is_empty());
    }

    #[test]
    fn oracle_doctor_args_accept_defaults_and_overrides() {
        assert_eq!(
            parse_oracle_doctor_args(Vec::new()).unwrap(),
            OracleDoctorArgs::default()
        );

        let parsed = parse_oracle_doctor_args(vec![
            "--ffmpeg".to_string(),
            "tools/ffmpeg".to_string(),
            "--ffprobe".to_string(),
            "tools/ffprobe".to_string(),
        ])
        .unwrap();

        assert_eq!(parsed.ffmpeg, Some(PathBuf::from("tools/ffmpeg")));
        assert_eq!(parsed.ffprobe, Some(PathBuf::from("tools/ffprobe")));
    }

    #[test]
    fn oracle_doctor_args_reject_missing_values_and_unknown_flags() {
        assert!(parse_oracle_doctor_args(vec!["--ffmpeg".to_string()]).is_err());
        assert!(parse_oracle_doctor_args(vec!["--unknown".to_string()]).is_err());
    }

    #[test]
    fn oracle_version_validation_accepts_pinned_output_shape() {
        let output = "\
ffmpeg version 8.1.1 Copyright (c) 2000-2026 the FFmpeg developers
libavutil      60. 26.101 / 60. 26.101
libavcodec     62. 28.101 / 62. 28.101
libavformat    62. 12.101 / 62. 12.101
libavdevice    62.  3.101 / 62.  3.101
libavfilter    11. 14.101 / 11. 14.101
libswscale      9.  5.101 /  9.  5.101
libswresample   6.  3.101 /  6.  3.101
";

        validate_version_output("ffmpeg", output).unwrap();
    }

    #[test]
    fn oracle_version_validation_rejects_wrong_tool_version_or_abi() {
        let output = "\
ffmpeg version 8.1.0
libavutil      60. 26.101 / 60. 26.101
libavcodec     62. 28.101 / 62. 28.101
libavformat    62. 12.101 / 62. 12.101
libavdevice    62.  3.101 / 62.  3.101
libavfilter    11. 14.101 / 11. 14.101
libswscale      9.  5.101 /  9.  5.101
libswresample   6.  3.101 /  6.  3.101
";
        assert!(validate_version_output("ffmpeg", output).is_err());

        let output = "\
ffprobe version 8.1.1
libavutil      60. 26.101 / 60. 26.101
libavcodec     62. 99.101 / 62. 99.101
libavformat    62. 12.101 / 62. 12.101
libavdevice    62.  3.101 / 62.  3.101
libavfilter    11. 14.101 / 11. 14.101
libswscale      9.  5.101 /  9.  5.101
libswresample   6.  3.101 /  6.  3.101
";
        assert!(validate_version_output("ffprobe", output).is_err());
    }
}
