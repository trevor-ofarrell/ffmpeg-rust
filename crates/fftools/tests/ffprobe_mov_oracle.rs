use fftools::ffprobe_output;
use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE/FFPROBE_ORACLE or install third_party/ffmpeg-oracle/build/bin"]
fn mov_rgb24_ffprobe_core_fields_match_ffmpeg_oracle() {
    let ffmpeg = oracle_tool("ffmpeg");
    let ffprobe = oracle_tool("ffprobe");
    let temp = TempDir::new("ffmpegrust-ffprobe-mov");
    let raw_path = temp.path().join("input.rgb");
    let mov_path = temp.path().join("input.mov");
    let payload = (0_u8..12).collect::<Vec<_>>();

    fs::write(&raw_path, &payload).expect("raw RGB input should be writable");

    let raw_arg = raw_path.to_string_lossy().into_owned();
    let mov_arg = mov_path.to_string_lossy().into_owned();

    let generate = Command::new(&ffmpeg)
        .args([
            "-nostdin",
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgb24",
            "-s",
            "2x1",
            "-r",
            "25",
            "-i",
            raw_arg.as_str(),
            "-c:v",
            "rawvideo",
            "-f",
            "mov",
            "-use_editlist",
            "0",
            mov_arg.as_str(),
        ])
        .output()
        .unwrap_or_else(|err| panic!("failed to run oracle `{}`: {err}", ffmpeg.display()));

    assert!(
        generate.status.success(),
        "oracle MOV generation failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        generate.status.code(),
        String::from_utf8_lossy(&generate.stdout),
        String::from_utf8_lossy(&generate.stderr)
    );

    let args = [
        "-hide_banner",
        "-count_frames",
        "-count_packets",
        "-show_format",
        "-show_streams",
        "-show_packets",
        mov_arg.as_str(),
    ];
    let rust = ffprobe_output(&strings(&args))
        .unwrap_or_else(|err| panic!("Rust ffprobe MOV path should execute: {err}"));

    let oracle = Command::new(&ffprobe)
        .args([
            "-v",
            "error",
            "-count_frames",
            "-count_packets",
            "-show_format",
            "-show_streams",
            "-show_packets",
            mov_arg.as_str(),
        ])
        .output()
        .unwrap_or_else(|err| panic!("failed to run oracle `{}`: {err}", ffprobe.display()));

    assert!(
        oracle.status.success(),
        "oracle ffprobe failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        oracle.status.code(),
        String::from_utf8_lossy(&oracle.stdout),
        String::from_utf8_lossy(&oracle.stderr)
    );

    let oracle_stdout =
        String::from_utf8(oracle.stdout).expect("oracle ffprobe output should be UTF-8");
    let rust_sections = parse_default_sections(&rust);
    let oracle_sections = parse_default_sections(&oracle_stdout);

    assert_single_section_fields_match(
        &rust_sections,
        &oracle_sections,
        "FORMAT",
        &[
            "nb_streams",
            "nb_programs",
            "nb_stream_groups",
            "format_name",
            "format_long_name",
            "duration",
            "size",
            "probe_score",
        ],
    );

    assert_single_section_fields_match(
        &rust_sections,
        &oracle_sections,
        "STREAM",
        &[
            "index",
            "codec_name",
            "codec_long_name",
            "codec_type",
            "codec_tag_string",
            "codec_tag",
            "width",
            "height",
            "coded_width",
            "coded_height",
            "r_frame_rate",
            "avg_frame_rate",
            "time_base",
            "start_pts",
            "start_time",
            "duration_ts",
            "duration",
            "nb_frames",
            "nb_read_frames",
            "nb_read_packets",
        ],
    );

    let rust_packets = sections_named(&rust_sections, "PACKET");
    let oracle_packets = sections_named(&oracle_sections, "PACKET");
    assert_eq!(rust_packets.len(), 2, "Rust should report two packets");
    assert_eq!(
        rust_packets.len(),
        oracle_packets.len(),
        "Rust and oracle packet counts should match"
    );
    for (index, (rust_packet, oracle_packet)) in
        rust_packets.iter().zip(oracle_packets.iter()).enumerate()
    {
        assert_fields_match(
            rust_packet,
            oracle_packet,
            &[
                "stream_index",
                "pts",
                "pts_time",
                "dts",
                "dts_time",
                "duration",
                "duration_time",
                "size",
                "flags",
            ],
            &format!("PACKET[{index}]"),
        );
    }
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE/FFPROBE_ORACLE or install third_party/ffmpeg-oracle/build/bin"]
fn avi_bgr24_ffprobe_packet_fields_match_ffmpeg_oracle() {
    let ffmpeg = oracle_tool("ffmpeg");
    let ffprobe = oracle_tool("ffprobe");
    let temp = TempDir::new("ffmpegrust-ffprobe-avi");
    let raw_path = temp.path().join("input.bgr");
    let avi_path = temp.path().join("input.avi");
    let payload = (0_u8..12).collect::<Vec<_>>();

    fs::write(&raw_path, &payload).expect("raw BGR input should be writable");

    let raw_arg = raw_path.to_string_lossy().into_owned();
    let avi_arg = avi_path.to_string_lossy().into_owned();

    let generate = Command::new(&ffmpeg)
        .args([
            "-nostdin",
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "bgr24",
            "-s",
            "2x1",
            "-r",
            "25",
            "-i",
            raw_arg.as_str(),
            "-c:v",
            "rawvideo",
            "-f",
            "avi",
            avi_arg.as_str(),
        ])
        .output()
        .unwrap_or_else(|err| panic!("failed to run oracle `{}`: {err}", ffmpeg.display()));

    assert!(
        generate.status.success(),
        "oracle AVI generation failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        generate.status.code(),
        String::from_utf8_lossy(&generate.stdout),
        String::from_utf8_lossy(&generate.stderr)
    );

    let args = [
        "-hide_banner",
        "-count_packets",
        "-show_format",
        "-show_streams",
        "-show_packets",
        avi_arg.as_str(),
    ];
    let rust = ffprobe_output(&strings(&args))
        .unwrap_or_else(|err| panic!("Rust ffprobe AVI path should execute: {err}"));

    let oracle = Command::new(&ffprobe)
        .args([
            "-v",
            "error",
            "-count_packets",
            "-show_format",
            "-show_streams",
            "-show_packets",
            avi_arg.as_str(),
        ])
        .output()
        .unwrap_or_else(|err| panic!("failed to run oracle `{}`: {err}", ffprobe.display()));

    assert!(
        oracle.status.success(),
        "oracle ffprobe failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        oracle.status.code(),
        String::from_utf8_lossy(&oracle.stdout),
        String::from_utf8_lossy(&oracle.stderr)
    );

    let oracle_stdout =
        String::from_utf8(oracle.stdout).expect("oracle ffprobe output should be UTF-8");
    let rust_sections = parse_default_sections(&rust);
    let oracle_sections = parse_default_sections(&oracle_stdout);

    assert_single_section_fields_match(
        &rust_sections,
        &oracle_sections,
        "FORMAT",
        &[
            "nb_streams",
            "nb_programs",
            "nb_stream_groups",
            "format_name",
            "format_long_name",
            "duration",
            "size",
            "probe_score",
        ],
    );

    assert_single_section_fields_match(
        &rust_sections,
        &oracle_sections,
        "STREAM",
        &[
            "index",
            "codec_name",
            "codec_long_name",
            "codec_type",
            "codec_tag_string",
            "codec_tag",
            "width",
            "height",
            "coded_width",
            "coded_height",
            "r_frame_rate",
            "avg_frame_rate",
            "time_base",
            "start_pts",
            "start_time",
            "duration_ts",
            "duration",
            "nb_frames",
            "nb_read_packets",
        ],
    );

    let rust_packets = sections_named(&rust_sections, "PACKET");
    let oracle_packets = sections_named(&oracle_sections, "PACKET");
    assert_eq!(rust_packets.len(), 2, "Rust should report two AVI packets");
    assert_eq!(
        rust_packets.len(),
        oracle_packets.len(),
        "Rust and oracle AVI packet counts should match"
    );
    for (index, (rust_packet, oracle_packet)) in
        rust_packets.iter().zip(oracle_packets.iter()).enumerate()
    {
        assert_fields_match(
            rust_packet,
            oracle_packet,
            &[
                "stream_index",
                "pts",
                "pts_time",
                "dts",
                "dts_time",
                "duration",
                "duration_time",
                "size",
                "flags",
            ],
            &format!("PACKET[{index}]"),
        );
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Section {
    name: String,
    fields: BTreeMap<String, String>,
}

impl Section {
    fn field(&self, key: &str) -> Option<&str> {
        self.fields.get(key).map(String::as_str)
    }
}

fn parse_default_sections(output: &str) -> Vec<Section> {
    let mut sections = Vec::new();
    let mut current: Option<Section> = None;

    for raw_line in output.lines() {
        let line = raw_line.trim_end_matches('\r');
        if let Some(name) = line
            .strip_prefix("[/")
            .and_then(|rest| rest.strip_suffix(']'))
        {
            let section = current
                .take()
                .unwrap_or_else(|| panic!("closing section `{name}` without an open section"));
            assert_eq!(section.name, name, "mismatched closing section");
            sections.push(section);
            continue;
        }
        if let Some(name) = line
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
        {
            assert!(
                current.is_none(),
                "opening section `{name}` before closing previous section"
            );
            current = Some(Section {
                name: name.to_owned(),
                fields: BTreeMap::new(),
            });
            continue;
        }
        if line.trim().is_empty() {
            continue;
        }
        let Some(section) = &mut current else {
            panic!("field outside section: `{line}`");
        };
        let (key, value) = line
            .split_once('=')
            .unwrap_or_else(|| panic!("section field should be key=value: `{line}`"));
        section.fields.insert(key.to_owned(), value.to_owned());
    }

    assert!(current.is_none(), "unclosed ffprobe section");
    sections
}

fn assert_single_section_fields_match(
    rust_sections: &[Section],
    oracle_sections: &[Section],
    name: &str,
    fields: &[&str],
) {
    let rust = single_section(rust_sections, name);
    let oracle = single_section(oracle_sections, name);
    assert_fields_match(rust, oracle, fields, name);
}

fn assert_fields_match(rust: &Section, oracle: &Section, fields: &[&str], label: &str) {
    for field in fields {
        assert_eq!(
            rust.field(field),
            oracle.field(field),
            "{label}.{field} should match oracle"
        );
    }
}

fn single_section<'a>(sections: &'a [Section], name: &str) -> &'a Section {
    let matches = sections_named(sections, name);
    assert_eq!(matches.len(), 1, "expected exactly one {name} section");
    matches[0]
}

fn sections_named<'a>(sections: &'a [Section], name: &str) -> Vec<&'a Section> {
    sections
        .iter()
        .filter(|section| section.name == name)
        .collect()
}

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(prefix: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after Unix epoch")
            .as_nanos();
        let path = env::temp_dir().join(format!("{prefix}-{nonce}-{}", std::process::id()));
        fs::create_dir_all(&path).expect("temporary test directory should be creatable");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn strings(args: &[&str]) -> Vec<String> {
    args.iter().map(|arg| (*arg).to_string()).collect()
}

fn oracle_tool(tool_name: &str) -> PathBuf {
    let env_var = format!("{}_ORACLE", tool_name.to_ascii_uppercase());
    if let Ok(path) = env::var(&env_var) {
        return require_tool_path(PathBuf::from(path), &env_var);
    }

    if tool_name == "ffprobe" {
        if let Ok(ffmpeg_path) = env::var("FFMPEG_ORACLE") {
            let ffmpeg_path = resolve_tool_path(PathBuf::from(ffmpeg_path));
            for candidate in sibling_tool_candidates(&ffmpeg_path, "ffprobe") {
                if candidate.is_file() {
                    return candidate;
                }
            }
        }
    }

    let root = repository_root();
    for candidate in default_tool_candidates(&root, tool_name) {
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
    let resolved = resolve_tool_path(path);
    assert!(
        resolved.is_file(),
        "{env_var} must point to the pinned FFmpeg 8.1.1 binary, got `{}`",
        resolved.display()
    );
    resolved
}

fn resolve_tool_path(path: PathBuf) -> PathBuf {
    if path.is_file() || path.is_absolute() {
        return path;
    }
    let candidate = repository_root().join(&path);
    if candidate.is_file() {
        return candidate;
    }
    path
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("fftools crate should be under crates/")
        .to_path_buf()
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
    if ffmpeg_path
        .extension()
        .is_some_and(|extension| extension == "cmd")
    {
        candidates.push(parent.join(format!("{tool_name}.cmd")));
    }
    candidates.push(parent.join(tool_name));
    candidates.push(parent.join(format!("{tool_name}.exe")));
    candidates.push(parent.join(format!("{tool_name}.cmd")));
    candidates
}

fn default_tool_candidates(root: &Path, tool_name: &str) -> Vec<PathBuf> {
    let bin = root.join("third_party/ffmpeg-oracle/build/bin");
    if cfg!(windows) {
        vec![
            bin.join(format!("{tool_name}.exe")),
            bin.join(format!("{tool_name}.cmd")),
            bin.join(tool_name),
        ]
    } else {
        vec![
            bin.join(tool_name),
            bin.join(format!("{tool_name}.exe")),
            bin.join(format!("{tool_name}.cmd")),
        ]
    }
}
