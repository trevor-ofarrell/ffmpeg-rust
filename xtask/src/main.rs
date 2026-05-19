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

fn guard_runtime() -> Result<(), String> {
    let root = env::current_dir().map_err(|err| format!("failed to read current dir: {err}"))?;
    let mut violations = Vec::new();

    for manifest in manifest_paths(&root)? {
        let contents = fs::read_to_string(&manifest)
            .map_err(|err| format!("failed to read {}: {err}", manifest.display()))?;
        violations.extend(manifest_dependency_violations(&manifest, &contents));
    }

    for source in runtime_source_paths(&root)? {
        let contents = fs::read_to_string(&source)
            .map_err(|err| format!("failed to read {}: {err}", source.display()))?;
        violations.extend(runtime_source_violations(&source, &contents));
    }

    if violations.is_empty() {
        println!(
            "runtime guard passed: no FFmpeg wrapper dependencies or runtime shell-outs found"
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
    let first_quote = after_package.find('"')?;
    let after_first_quote = &after_package[first_quote + 1..];
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
    eprintln!("usage: cargo run -p xtask -- <quick|changed|full|inventory|guard-runtime>");
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
}
