use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

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

    if !ffmpeg.exists() {
        return Err(format!(
            "ffmpeg oracle does not exist: {}",
            ffmpeg.display()
        ));
    }

    fs::create_dir_all(&out).map_err(|err| format!("failed to create {}: {err}", out.display()))?;

    let mut manifest = String::new();
    manifest.push_str("target_version = \"8.1.1\"\n");
    manifest.push_str("profile = \"ffmpeg-8.1.1-default-native\"\n");
    manifest.push_str(&format!("ffmpeg = {:?}\n\n", ffmpeg));

    for (name, flags) in INVENTORY_COMMANDS {
        let output = Command::new(&ffmpeg)
            .args(*flags)
            .output()
            .map_err(|err| format!("failed to run ffmpeg {}: {err}", flags.join(" ")))?;

        let file_name = format!("{name}.txt");
        let path = out.join(&file_name);
        write_command_output(&path, &output.stdout, &output.stderr)?;
        manifest.push_str(&format!(
            "[[commands]]\nname = \"{name}\"\nfile = \"{file_name}\"\nstatus = {}\n\n",
            output.status.code().unwrap_or(-1)
        ));
    }

    fs::write(out.join("inventory.toml"), manifest)
        .map_err(|err| format!("failed to write inventory manifest: {err}"))?;

    Ok(())
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
