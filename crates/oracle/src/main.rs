use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const TARGET_VERSION: &str = "8.1.1";
const TARGET_PROFILE: &str = "ffmpeg-8.1.1-default-native";

const INVENTORY_COMMANDS: &[(&str, &[&str])] = &[
    ("version", &["-version"]),
    ("buildconf", &["-buildconf"]),
    ("formats", &["-formats"]),
    ("codecs", &["-codecs"]),
    ("decoders", &["-decoders"]),
    ("encoders", &["-encoders"]),
    ("muxers", &["-muxers"]),
    ("demuxers", &["-demuxers"]),
    ("protocols", &["-protocols"]),
    ("filters", &["-filters"]),
    ("bsfs", &["-bsfs"]),
    ("pix_fmts", &["-pix_fmts"]),
    ("sample_fmts", &["-sample_fmts"]),
    ("layouts", &["-layouts"]),
    ("colors", &["-colors"]),
];

#[derive(Debug, PartialEq, Eq)]
struct InventoryArgs {
    ffmpeg: PathBuf,
    out: PathBuf,
}

fn main() {
    match real_main() {
        Ok(()) => {}
        Err(err) => {
            eprintln!("oracle: {err}");
            std::process::exit(1);
        }
    }
}

fn real_main() -> Result<(), String> {
    let mut args = env::args_os().skip(1);
    match args
        .next()
        .and_then(|arg| arg.into_string().ok())
        .as_deref()
    {
        Some("inventory") => inventory(args.collect()),
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

fn inventory(args: Vec<OsString>) -> Result<(), String> {
    let InventoryArgs { ffmpeg, out } = parse_inventory_args(args)?;

    if !ffmpeg.exists() {
        return Err(format!(
            "ffmpeg oracle does not exist: {}",
            ffmpeg.display()
        ));
    }

    fs::create_dir_all(&out).map_err(|err| format!("failed to create {}: {err}", out.display()))?;

    let mut manifest = inventory_manifest_header(&ffmpeg);
    for (name, flags) in INVENTORY_COMMANDS {
        let output = Command::new(&ffmpeg)
            .args(*flags)
            .output()
            .map_err(|err| format!("failed to run ffmpeg {}: {err}", flags.join(" ")))?;

        let file_name = format!("{name}.txt");
        let path = out.join(&file_name);
        write_command_output(&path, &output.stdout, &output.stderr)?;
        append_manifest_command(
            &mut manifest,
            name,
            &file_name,
            output.status.code().unwrap_or(-1),
        );
    }

    fs::write(out.join("inventory.toml"), manifest)
        .map_err(|err| format!("failed to write inventory manifest: {err}"))?;

    Ok(())
}

fn parse_inventory_args(args: Vec<OsString>) -> Result<InventoryArgs, String> {
    let mut ffmpeg: Option<PathBuf> = None;
    let mut out: Option<PathBuf> = None;

    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.to_string_lossy().as_ref() {
            "--ffmpeg" => {
                ffmpeg = iter.next().map(PathBuf::from);
            }
            "--out" => {
                out = iter.next().map(PathBuf::from);
            }
            other => return Err(format!("unsupported inventory argument `{other}`")),
        }
    }

    let ffmpeg = ffmpeg.ok_or_else(|| "missing --ffmpeg <path>".to_string())?;
    let out = out.ok_or_else(|| "missing --out <dir>".to_string())?;

    Ok(InventoryArgs { ffmpeg, out })
}

fn inventory_manifest_header(ffmpeg: &Path) -> String {
    let mut manifest = String::new();
    manifest.push_str(&format!("target_version = \"{TARGET_VERSION}\"\n"));
    manifest.push_str(&format!("profile = \"{TARGET_PROFILE}\"\n"));
    manifest.push_str(&format!("ffmpeg = {:?}\n\n", ffmpeg));
    manifest
}

fn append_manifest_command(manifest: &mut String, name: &str, file_name: &str, status: i32) {
    manifest.push_str(&format!(
        "[[commands]]\nname = \"{name}\"\nfile = \"{file_name}\"\nstatus = {status}\n\n"
    ));
}

fn write_command_output(path: &Path, stdout: &[u8], stderr: &[u8]) -> Result<(), String> {
    let mut data = Vec::new();
    data.extend_from_slice(b"--- stdout ---\n");
    data.extend_from_slice(stdout);
    data.extend_from_slice(b"\n--- stderr ---\n");
    data.extend_from_slice(stderr);
    fs::write(path, data).map_err(|err| format!("failed to write {}: {err}", path.display()))
}

fn print_help() {
    eprintln!("usage: oracle inventory --ffmpeg <path> --out <dir>");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn inventory_command_list_matches_required_ffmpeg_surface() {
        let commands: Vec<(&str, Vec<&str>)> = INVENTORY_COMMANDS
            .iter()
            .map(|(name, flags)| (*name, flags.to_vec()))
            .collect();

        assert_eq!(
            commands,
            vec![
                ("version", vec!["-version"]),
                ("buildconf", vec!["-buildconf"]),
                ("formats", vec!["-formats"]),
                ("codecs", vec!["-codecs"]),
                ("decoders", vec!["-decoders"]),
                ("encoders", vec!["-encoders"]),
                ("muxers", vec!["-muxers"]),
                ("demuxers", vec!["-demuxers"]),
                ("protocols", vec!["-protocols"]),
                ("filters", vec!["-filters"]),
                ("bsfs", vec!["-bsfs"]),
                ("pix_fmts", vec!["-pix_fmts"]),
                ("sample_fmts", vec!["-sample_fmts"]),
                ("layouts", vec!["-layouts"]),
                ("colors", vec!["-colors"]),
            ]
        );
    }

    #[test]
    fn parse_inventory_args_accepts_required_paths() {
        assert_eq!(
            parse_inventory_args(os_strings(&[
                "--ffmpeg",
                "third_party/ffmpeg-oracle/build/bin/ffmpeg",
                "--out",
                "compat/ffmpeg-8.1.1",
            ]))
            .unwrap(),
            InventoryArgs {
                ffmpeg: PathBuf::from("third_party/ffmpeg-oracle/build/bin/ffmpeg"),
                out: PathBuf::from("compat/ffmpeg-8.1.1"),
            }
        );
    }

    #[test]
    fn parse_inventory_args_rejects_missing_or_unknown_arguments() {
        assert_eq!(
            parse_inventory_args(os_strings(&["--out", "compat/ffmpeg-8.1.1"])).unwrap_err(),
            "missing --ffmpeg <path>"
        );
        assert_eq!(
            parse_inventory_args(os_strings(&["--ffmpeg", "ffmpeg"])).unwrap_err(),
            "missing --out <dir>"
        );
        assert_eq!(
            parse_inventory_args(os_strings(&[
                "--ffmpeg", "ffmpeg", "--out", "compat", "--extra",
            ]))
            .unwrap_err(),
            "unsupported inventory argument `--extra`"
        );
    }

    #[test]
    fn manifest_helpers_record_target_profile_and_command_status() {
        let mut manifest = inventory_manifest_header(Path::new("oracle-ffmpeg"));
        append_manifest_command(&mut manifest, "formats", "formats.txt", 0);
        append_manifest_command(&mut manifest, "filters", "filters.txt", 1);

        assert!(manifest.contains("target_version = \"8.1.1\""));
        assert!(manifest.contains("profile = \"ffmpeg-8.1.1-default-native\""));
        assert!(manifest.contains("ffmpeg = \"oracle-ffmpeg\""));
        assert!(manifest.contains("name = \"formats\"\nfile = \"formats.txt\"\nstatus = 0"));
        assert!(manifest.contains("name = \"filters\"\nfile = \"filters.txt\"\nstatus = 1"));
    }

    #[test]
    fn write_command_output_keeps_stdout_and_stderr_sections() {
        let path = unique_temp_path("oracle-command-output", "txt");
        write_command_output(&path, b"stdout bytes", b"stderr bytes").unwrap();

        let written = fs::read_to_string(&path).unwrap();
        let _ = fs::remove_file(&path);

        assert_eq!(
            written,
            "--- stdout ---\nstdout bytes\n--- stderr ---\nstderr bytes"
        );
    }

    fn os_strings(values: &[&str]) -> Vec<OsString> {
        values.iter().map(OsString::from).collect()
    }

    fn unique_temp_path(label: &str, extension: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        env::temp_dir().join(format!(
            "ffmpegrust-{label}-{}-{unique}.{extension}",
            std::process::id()
        ))
    }
}
