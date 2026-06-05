use fftools::ffprobe_output;
use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

const PACKET_FIELDS: &[&str] = &[
    "codec_type",
    "stream_index",
    "pts",
    "pts_time",
    "dts",
    "dts_time",
    "duration",
    "duration_time",
    "size",
    "pos",
    "flags",
];

const PACKET_HASH_FIELDS: &[&str] = &[
    "codec_type",
    "stream_index",
    "pts",
    "pts_time",
    "dts",
    "dts_time",
    "duration",
    "duration_time",
    "size",
    "pos",
    "flags",
    "data_hash",
];

const PACKET_DATA_FIELDS: &[&str] = &[
    "codec_type",
    "stream_index",
    "pts",
    "pts_time",
    "dts",
    "dts_time",
    "duration",
    "duration_time",
    "size",
    "pos",
    "flags",
    "data",
];

const PACKET_DATA_HASH_FIELDS: &[&str] = &[
    "codec_type",
    "stream_index",
    "pts",
    "pts_time",
    "dts",
    "dts_time",
    "duration",
    "duration_time",
    "size",
    "pos",
    "flags",
    "data",
    "data_hash",
];

const PACKET_SELECTED_FIELDS: &[&str] = &["pts_time", "size", "flags"];
const MIXED_SHOW_ENTRIES: &str =
    "packet=size,pts_time:stream=codec_type,index:format=size,format_name";
const MIXED_PACKET_FIELDS: &[&str] = &["pts_time", "size"];
const MIXED_STREAM_FIELDS: &[&str] = &["index", "codec_type"];
const MIXED_FORMAT_FIELDS: &[&str] = &["format_name", "size"];
const TAG_SHOW_ENTRIES: &str = "packet=size,pts_time:stream_tags=handler_name";
const TAG_PACKET_FIELDS: &[&str] = &["pts_time", "size"];
const MOV_TAG_STREAM_FIELDS: &[&str] = &["TAG:handler_name"];
const EMPTY_TAG_STREAM_FIELDS: &[&str] = &[];
const AAC_SKIP_SAMPLES_SHOW_ENTRIES: &str =
    "packet=pts,dts,pts_time,dts_time,duration,size,flags:packet_side_data";

#[test]
fn parse_default_sections_reads_multiline_packet_data() {
    let output = "\
[PACKET]
codec_type=video
stream_index=0
pts=0
pts_time=0.000000
dts=0
dts_time=0.000000
duration=1
duration_time=0.040000
size=4
pos=36
flags=K__
data=
00000000: 0001 0200                                ....

data_hash=MD5:0f0c725e025036e905dc2ed035406463
[/PACKET]
";

    let sections = parse_default_sections(output);

    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0].field("flags"), Some("K__"));
    assert_eq!(
        sections[0].field("data"),
        Some("\n00000000: 0001 0200                                ....\n")
    );
    assert_eq!(
        sections[0].field("data_hash"),
        Some("MD5:0f0c725e025036e905dc2ed035406463")
    );
}

#[test]
fn parse_json_packet_sections_accepts_compact_and_pretty_packet_objects() {
    let output = r#"{
  "packets": [
    {"codec_type": "video", "stream_index": 0, "pts": 0, "pts_time": "0.000000", "dts": 0, "dts_time": "0.000000", "duration": 1, "duration_time": "0.040000", "size": 6, "pos": 36, "flags": "K__"},
    {
      "codec_type": "video",
      "stream_index": 0,
      "pts": 1,
      "pts_time": "0.040000",
      "dts": 1,
      "dts_time": "0.040000",
      "duration": 1,
      "duration_time": "0.040000",
      "size": 6,
      "pos": 42,
      "flags": "___"
    }
  ]
}
"#;

    let packets = parse_json_packet_sections(output);

    assert_eq!(packets.len(), 2);
    assert_eq!(packets[0].field("codec_type"), Some("\"video\""));
    assert_eq!(packets[0].field("stream_index"), Some("0"));
    assert_eq!(packets[0].field("pos"), Some("36"));
    assert_eq!(packets[0].field("flags"), Some("\"K__\""));
    assert_eq!(packets[1].field("pts"), Some("1"));
    assert_eq!(packets[1].field("pos"), Some("42"));
    assert_eq!(packets[1].field("flags"), Some("\"___\""));
}

#[test]
fn parse_compact_packet_sections_reads_packet_lines() {
    let output = "\
packet|codec_type=video|stream_index=0|pts=0|pts_time=0.000000|dts=0|dts_time=0.000000|duration=1|duration_time=0.040000|size=6|pos=36|flags=K__
packet|codec_type=video|stream_index=0|pts=1|pts_time=0.040000|dts=1|dts_time=0.040000|duration=1|duration_time=0.040000|size=6|pos=42|flags=___
";

    let packets = parse_compact_packet_sections(output);

    assert_eq!(packets.len(), 2);
    assert_eq!(packets[0].field("codec_type"), Some("video"));
    assert_eq!(packets[0].field("stream_index"), Some("0"));
    assert_eq!(packets[0].field("pos"), Some("36"));
    assert_eq!(packets[0].field("flags"), Some("K__"));
    assert_eq!(packets[1].field("pts"), Some("1"));
    assert_eq!(packets[1].field("pos"), Some("42"));
    assert_eq!(packets[1].field("flags"), Some("___"));
}

#[test]
fn parse_csv_packet_sections_reads_packet_lines() {
    let output = "\
packet,video,0,0,0.000000,0,0.000000,1,0.040000,6,36,K__
packet,video,0,1,0.040000,1,0.040000,1,0.040000,6,42,___
";

    let packets = parse_csv_packet_sections(output);

    assert_eq!(packets.len(), 2);
    assert_eq!(packets[0].field("codec_type"), Some("video"));
    assert_eq!(packets[0].field("stream_index"), Some("0"));
    assert_eq!(packets[0].field("pos"), Some("36"));
    assert_eq!(packets[0].field("flags"), Some("K__"));
    assert_eq!(packets[1].field("pts"), Some("1"));
    assert_eq!(packets[1].field("pos"), Some("42"));
    assert_eq!(packets[1].field("flags"), Some("___"));
}

#[test]
fn parse_csv_packet_sections_reads_multiline_packet_data() {
    let output = "packet,video,0,0,0.000000,0,0.000000,1,0.040000,4,36,K__,\"\n00000000: 0001 0200                                ....\n\",MD5:0f0c725e025036e905dc2ed035406463\n";

    let packets = parse_csv_packet_sections_with_fields(output, PACKET_DATA_HASH_FIELDS);

    assert_eq!(packets.len(), 1);
    assert_eq!(packets[0].field("flags"), Some("K__"));
    assert_eq!(
        packets[0].field("data"),
        Some("\n00000000: 0001 0200                                ....\n")
    );
    assert_eq!(
        packets[0].field("data_hash"),
        Some("MD5:0f0c725e025036e905dc2ed035406463")
    );
}

#[test]
fn parse_flat_packet_sections_reads_packet_lines() {
    let output = "\
packets.packet.0.codec_type=\"video\"
packets.packet.0.stream_index=0
packets.packet.0.pts=0
packets.packet.0.pts_time=\"0.000000\"
packets.packet.0.dts=0
packets.packet.0.dts_time=\"0.000000\"
packets.packet.0.duration=1
packets.packet.0.duration_time=\"0.040000\"
packets.packet.0.size=\"6\"
packets.packet.0.pos=\"36\"
packets.packet.0.flags=\"K__\"
packets.packet.1.codec_type=\"video\"
packets.packet.1.stream_index=0
packets.packet.1.pts=1
packets.packet.1.pts_time=\"0.040000\"
packets.packet.1.dts=1
packets.packet.1.dts_time=\"0.040000\"
packets.packet.1.duration=1
packets.packet.1.duration_time=\"0.040000\"
packets.packet.1.size=\"6\"
packets.packet.1.pos=\"42\"
packets.packet.1.flags=\"___\"
";

    let packets = parse_flat_packet_sections(output);

    assert_eq!(packets.len(), 2);
    assert_eq!(packets[0].field("codec_type"), Some("\"video\""));
    assert_eq!(packets[0].field("stream_index"), Some("0"));
    assert_eq!(packets[0].field("pos"), Some("\"36\""));
    assert_eq!(packets[0].field("flags"), Some("\"K__\""));
    assert_eq!(packets[1].field("pts"), Some("1"));
    assert_eq!(packets[1].field("pos"), Some("\"42\""));
    assert_eq!(packets[1].field("flags"), Some("\"___\""));
}

#[test]
fn parse_ini_packet_sections_reads_packet_sections() {
    let output = "\
# ffprobe output

[packets.packet.0]
codec_type=video
stream_index=0
pts=0
pts_time=0.000000
dts=0
dts_time=0.000000
duration=1
duration_time=0.040000
size=6
pos=36
flags=K__

[packets.packet.1]
codec_type=video
stream_index=0
pts=1
pts_time=0.040000
dts=1
dts_time=0.040000
duration=1
duration_time=0.040000
size=6
pos=42
flags=___
";

    let packets = parse_ini_packet_sections(output);

    assert_eq!(packets.len(), 2);
    assert_eq!(packets[0].field("codec_type"), Some("video"));
    assert_eq!(packets[0].field("stream_index"), Some("0"));
    assert_eq!(packets[0].field("pos"), Some("36"));
    assert_eq!(packets[0].field("flags"), Some("K__"));
    assert_eq!(packets[1].field("pts"), Some("1"));
    assert_eq!(packets[1].field("pos"), Some("42"));
    assert_eq!(packets[1].field("flags"), Some("___"));
}

#[test]
fn parse_xml_packet_sections_reads_packet_elements() {
    let output = r#"<?xml version="1.0" encoding="UTF-8"?>
<ffprobe>
    <packets>
        <packet codec_type="video" stream_index="0" pts="0" pts_time="0.000000" dts="0" dts_time="0.000000" duration="1" duration_time="0.040000" size="6" pos="36" flags="K__"/>
        <packet codec_type="video" stream_index="0" pts="1" pts_time="0.040000" dts="1" dts_time="0.040000" duration="1" duration_time="0.040000" size="6" pos="42" flags="___"/>
    </packets>
</ffprobe>
"#;

    let packets = parse_xml_packet_sections(output);

    assert_eq!(packets.len(), 2);
    assert_eq!(packets[0].field("codec_type"), Some("video"));
    assert_eq!(packets[0].field("stream_index"), Some("0"));
    assert_eq!(packets[0].field("pos"), Some("36"));
    assert_eq!(packets[0].field("flags"), Some("K__"));
    assert_eq!(packets[1].field("pts"), Some("1"));
    assert_eq!(packets[1].field("pos"), Some("42"));
    assert_eq!(packets[1].field("flags"), Some("___"));
}

#[test]
fn parse_xml_packet_sections_reads_multiline_packet_data() {
    let output = r#"<?xml version="1.0" encoding="UTF-8"?>
<ffprobe>
    <packets>
        <packet codec_type="video" stream_index="0" pts="0" pts_time="0.000000" dts="0" dts_time="0.000000" duration="1" duration_time="0.040000" size="4" pos="36" flags="K__" data="
00000000: 0001 0200                                ....
" data_hash="MD5:0f0c725e025036e905dc2ed035406463"/>
    </packets>
</ffprobe>
"#;

    let packets = parse_xml_packet_sections(output);

    assert_eq!(packets.len(), 1);
    assert_eq!(packets[0].field("flags"), Some("K__"));
    assert_eq!(
        packets[0].field("data"),
        Some("\n00000000: 0001 0200                                ....\n")
    );
    assert_eq!(
        packets[0].field("data_hash"),
        Some("MD5:0f0c725e025036e905dc2ed035406463")
    );
}

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
            PACKET_FIELDS,
            &format!("PACKET[{index}]"),
        );
    }

    assert_json_packet_fields_match(&ffprobe, mov_arg.as_str(), "MOV");
    assert_compact_packet_fields_match(&ffprobe, mov_arg.as_str(), "MOV");
    assert_csv_packet_fields_match(&ffprobe, mov_arg.as_str(), "MOV");
    assert_flat_packet_fields_match(&ffprobe, mov_arg.as_str(), "MOV");
    assert_ini_packet_fields_match(&ffprobe, mov_arg.as_str(), "MOV");
    assert_xml_packet_fields_match(&ffprobe, mov_arg.as_str(), "MOV");
    assert_packet_data_hash_fields_match(&ffprobe, mov_arg.as_str(), "MOV");
    assert_packet_data_fields_match(&ffprobe, mov_arg.as_str(), "MOV");
    assert_packet_data_and_hash_fields_match(&ffprobe, mov_arg.as_str(), "MOV");
    assert_packet_selected_fields_match(&ffprobe, mov_arg.as_str(), "MOV");
    assert_mixed_show_entries_fields_match(&ffprobe, mov_arg.as_str(), "MOV");
    assert_packet_and_stream_tag_show_entries_fields_match(
        &ffprobe,
        mov_arg.as_str(),
        "MOV",
        MOV_TAG_STREAM_FIELDS,
    );
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
            PACKET_FIELDS,
            &format!("PACKET[{index}]"),
        );
    }

    assert_json_packet_fields_match(&ffprobe, avi_arg.as_str(), "AVI");
    assert_compact_packet_fields_match(&ffprobe, avi_arg.as_str(), "AVI");
    assert_csv_packet_fields_match(&ffprobe, avi_arg.as_str(), "AVI");
    assert_flat_packet_fields_match(&ffprobe, avi_arg.as_str(), "AVI");
    assert_ini_packet_fields_match(&ffprobe, avi_arg.as_str(), "AVI");
    assert_xml_packet_fields_match(&ffprobe, avi_arg.as_str(), "AVI");
    assert_packet_data_hash_fields_match(&ffprobe, avi_arg.as_str(), "AVI");
    assert_packet_data_fields_match(&ffprobe, avi_arg.as_str(), "AVI");
    assert_packet_data_and_hash_fields_match(&ffprobe, avi_arg.as_str(), "AVI");
    assert_packet_selected_fields_match(&ffprobe, avi_arg.as_str(), "AVI");
    assert_mixed_show_entries_fields_match(&ffprobe, avi_arg.as_str(), "AVI");
    assert_packet_and_stream_tag_show_entries_fields_match(
        &ffprobe,
        avi_arg.as_str(),
        "AVI",
        EMPTY_TAG_STREAM_FIELDS,
    );
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE/FFPROBE_ORACLE or install third_party/ffmpeg-oracle/build/bin"]
fn m4a_aac_skip_samples_packet_side_data_matches_ffmpeg_oracle() {
    let ffmpeg = oracle_tool("ffmpeg");
    let ffprobe = oracle_tool("ffprobe");
    let temp = TempDir::new("ffmpegrust-ffprobe-m4a-skip-samples");
    let m4a_path = temp.path().join("input.m4a");
    let m4a_arg = m4a_path.to_string_lossy().into_owned();

    let generate = Command::new(&ffmpeg)
        .args([
            "-nostdin",
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=1000:duration=0.02",
            "-c:a",
            "aac",
            m4a_arg.as_str(),
        ])
        .output()
        .unwrap_or_else(|err| panic!("failed to run oracle `{}`: {err}", ffmpeg.display()));

    assert!(
        generate.status.success(),
        "oracle AAC/M4A generation failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        generate.status.code(),
        String::from_utf8_lossy(&generate.stdout),
        String::from_utf8_lossy(&generate.stderr)
    );

    let args = [
        "-v",
        "error",
        "-show_packets",
        "-show_entries",
        AAC_SKIP_SAMPLES_SHOW_ENTRIES,
        "-of",
        "default",
        m4a_arg.as_str(),
    ];
    let rust = ffprobe_output(&strings(&args))
        .unwrap_or_else(|err| panic!("Rust ffprobe AAC/M4A side-data path should execute: {err}"));

    let oracle = Command::new(&ffprobe)
        .args(args)
        .output()
        .unwrap_or_else(|err| panic!("failed to run oracle `{}`: {err}", ffprobe.display()));
    assert!(
        oracle.status.success(),
        "oracle ffprobe AAC/M4A side-data failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        oracle.status.code(),
        String::from_utf8_lossy(&oracle.stdout),
        String::from_utf8_lossy(&oracle.stderr)
    );
    let oracle_stdout =
        String::from_utf8(oracle.stdout).expect("oracle ffprobe output should be UTF-8");

    assert_eq!(
        rust, oracle_stdout,
        "Rust and oracle AAC/M4A packet side-data output should match byte-for-byte"
    );
    assert!(
        rust.contains("[SIDE_DATA]\nside_data_type=Skip Samples\nskip_samples=1024\n"),
        "output should prove public Skip Samples packet side data"
    );
    assert!(
        rust.contains("flags=KD_\n"),
        "output should prove the priming packet discard flag"
    );
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
    let mut pending_data: Option<String> = None;

    for raw_line in output.lines() {
        let line = raw_line.trim_end_matches('\r');
        if pending_data.is_some() {
            if line.trim().is_empty() {
                let data = pending_data.take().expect("pending data should be present");
                current
                    .as_mut()
                    .expect("data field should be inside a section")
                    .fields
                    .insert("data".to_owned(), data);
                continue;
            }
            if !line.starts_with('[') {
                let data = pending_data
                    .as_mut()
                    .expect("pending data should be present");
                data.push_str(line);
                data.push('\n');
                continue;
            }
            let data = pending_data.take().expect("pending data should be present");
            current
                .as_mut()
                .expect("data field should be inside a section")
                .fields
                .insert("data".to_owned(), data);
        }
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
        if key == "data" && value.is_empty() {
            pending_data = Some("\n".to_owned());
            continue;
        }
        section.fields.insert(key.to_owned(), value.to_owned());
    }

    if let Some(data) = pending_data.take() {
        current
            .as_mut()
            .expect("data field should be inside a section")
            .fields
            .insert("data".to_owned(), data);
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

fn assert_single_section_exact_fields_match(
    rust_sections: &[Section],
    oracle_sections: &[Section],
    name: &str,
    fields: &[&str],
) {
    let rust = single_section(rust_sections, name);
    let oracle = single_section(oracle_sections, name);
    assert_exact_fields(rust, fields, &format!("Rust {name}"));
    assert_exact_fields(oracle, fields, &format!("oracle {name}"));
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

fn assert_json_packet_fields_match(ffprobe: &Path, input: &str, label: &str) {
    let rust_json = ffprobe_output(&strings(&[
        "-hide_banner",
        "-show_packets",
        "-of",
        "json",
        input,
    ]))
    .unwrap_or_else(|err| panic!("Rust ffprobe {label} JSON packet path should execute: {err}"));

    let oracle = Command::new(ffprobe)
        .args(["-v", "error", "-show_packets", "-of", "json", input])
        .output()
        .unwrap_or_else(|err| panic!("failed to run oracle `{}`: {err}", ffprobe.display()));

    assert!(
        oracle.status.success(),
        "oracle ffprobe JSON failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        oracle.status.code(),
        String::from_utf8_lossy(&oracle.stdout),
        String::from_utf8_lossy(&oracle.stderr)
    );

    let oracle_json =
        String::from_utf8(oracle.stdout).expect("oracle ffprobe JSON output should be UTF-8");
    let rust_packets = parse_json_packet_sections(&rust_json);
    let oracle_packets = parse_json_packet_sections(&oracle_json);
    assert_eq!(
        rust_packets.len(),
        oracle_packets.len(),
        "{label} JSON packet counts should match"
    );

    for (index, (rust_packet, oracle_packet)) in
        rust_packets.iter().zip(oracle_packets.iter()).enumerate()
    {
        assert_fields_match(
            rust_packet,
            oracle_packet,
            PACKET_FIELDS,
            &format!("{label} JSON PACKET[{index}]"),
        );
    }
}

fn assert_compact_packet_fields_match(ffprobe: &Path, input: &str, label: &str) {
    let rust_compact = ffprobe_output(&strings(&[
        "-hide_banner",
        "-show_packets",
        "-of",
        "compact",
        input,
    ]))
    .unwrap_or_else(|err| panic!("Rust ffprobe {label} compact packet path should execute: {err}"));

    let oracle = Command::new(ffprobe)
        .args(["-v", "error", "-show_packets", "-of", "compact", input])
        .output()
        .unwrap_or_else(|err| panic!("failed to run oracle `{}`: {err}", ffprobe.display()));

    assert!(
        oracle.status.success(),
        "oracle ffprobe compact failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        oracle.status.code(),
        String::from_utf8_lossy(&oracle.stdout),
        String::from_utf8_lossy(&oracle.stderr)
    );

    let oracle_compact =
        String::from_utf8(oracle.stdout).expect("oracle ffprobe compact output should be UTF-8");
    let rust_packets = parse_compact_packet_sections(&rust_compact);
    let oracle_packets = parse_compact_packet_sections(&oracle_compact);
    assert_eq!(
        rust_packets.len(),
        oracle_packets.len(),
        "{label} compact packet counts should match"
    );

    for (index, (rust_packet, oracle_packet)) in
        rust_packets.iter().zip(oracle_packets.iter()).enumerate()
    {
        assert_fields_match(
            rust_packet,
            oracle_packet,
            PACKET_FIELDS,
            &format!("{label} compact PACKET[{index}]"),
        );
    }
}

fn assert_csv_packet_fields_match(ffprobe: &Path, input: &str, label: &str) {
    let rust_csv = ffprobe_output(&strings(&[
        "-hide_banner",
        "-show_packets",
        "-of",
        "csv",
        input,
    ]))
    .unwrap_or_else(|err| panic!("Rust ffprobe {label} CSV packet path should execute: {err}"));

    let oracle = Command::new(ffprobe)
        .args(["-v", "error", "-show_packets", "-of", "csv", input])
        .output()
        .unwrap_or_else(|err| panic!("failed to run oracle `{}`: {err}", ffprobe.display()));

    assert!(
        oracle.status.success(),
        "oracle ffprobe CSV failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        oracle.status.code(),
        String::from_utf8_lossy(&oracle.stdout),
        String::from_utf8_lossy(&oracle.stderr)
    );

    let oracle_csv =
        String::from_utf8(oracle.stdout).expect("oracle ffprobe CSV output should be UTF-8");
    let rust_packets = parse_csv_packet_sections(&rust_csv);
    let oracle_packets = parse_csv_packet_sections(&oracle_csv);
    assert_eq!(
        rust_packets.len(),
        oracle_packets.len(),
        "{label} CSV packet counts should match"
    );

    for (index, (rust_packet, oracle_packet)) in
        rust_packets.iter().zip(oracle_packets.iter()).enumerate()
    {
        assert_fields_match(
            rust_packet,
            oracle_packet,
            PACKET_FIELDS,
            &format!("{label} CSV PACKET[{index}]"),
        );
    }
}

fn assert_flat_packet_fields_match(ffprobe: &Path, input: &str, label: &str) {
    let rust_flat = ffprobe_output(&strings(&[
        "-hide_banner",
        "-show_packets",
        "-of",
        "flat",
        input,
    ]))
    .unwrap_or_else(|err| panic!("Rust ffprobe {label} flat packet path should execute: {err}"));

    let oracle = Command::new(ffprobe)
        .args(["-v", "error", "-show_packets", "-of", "flat", input])
        .output()
        .unwrap_or_else(|err| panic!("failed to run oracle `{}`: {err}", ffprobe.display()));

    assert!(
        oracle.status.success(),
        "oracle ffprobe flat failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        oracle.status.code(),
        String::from_utf8_lossy(&oracle.stdout),
        String::from_utf8_lossy(&oracle.stderr)
    );

    let oracle_flat =
        String::from_utf8(oracle.stdout).expect("oracle ffprobe flat output should be UTF-8");
    let rust_packets = parse_flat_packet_sections(&rust_flat);
    let oracle_packets = parse_flat_packet_sections(&oracle_flat);
    assert_eq!(
        rust_packets.len(),
        oracle_packets.len(),
        "{label} flat packet counts should match"
    );

    for (index, (rust_packet, oracle_packet)) in
        rust_packets.iter().zip(oracle_packets.iter()).enumerate()
    {
        assert_fields_match(
            rust_packet,
            oracle_packet,
            PACKET_FIELDS,
            &format!("{label} flat PACKET[{index}]"),
        );
    }
}

fn assert_ini_packet_fields_match(ffprobe: &Path, input: &str, label: &str) {
    let rust_ini = ffprobe_output(&strings(&[
        "-hide_banner",
        "-show_packets",
        "-of",
        "ini",
        input,
    ]))
    .unwrap_or_else(|err| panic!("Rust ffprobe {label} INI packet path should execute: {err}"));

    let oracle = Command::new(ffprobe)
        .args(["-v", "error", "-show_packets", "-of", "ini", input])
        .output()
        .unwrap_or_else(|err| panic!("failed to run oracle `{}`: {err}", ffprobe.display()));

    assert!(
        oracle.status.success(),
        "oracle ffprobe INI failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        oracle.status.code(),
        String::from_utf8_lossy(&oracle.stdout),
        String::from_utf8_lossy(&oracle.stderr)
    );

    let oracle_ini =
        String::from_utf8(oracle.stdout).expect("oracle ffprobe INI output should be UTF-8");
    let rust_packets = parse_ini_packet_sections(&rust_ini);
    let oracle_packets = parse_ini_packet_sections(&oracle_ini);
    assert_eq!(
        rust_packets.len(),
        oracle_packets.len(),
        "{label} INI packet counts should match"
    );

    for (index, (rust_packet, oracle_packet)) in
        rust_packets.iter().zip(oracle_packets.iter()).enumerate()
    {
        assert_fields_match(
            rust_packet,
            oracle_packet,
            PACKET_FIELDS,
            &format!("{label} INI PACKET[{index}]"),
        );
    }
}

fn assert_xml_packet_fields_match(ffprobe: &Path, input: &str, label: &str) {
    let rust_xml = ffprobe_output(&strings(&[
        "-hide_banner",
        "-show_packets",
        "-of",
        "xml",
        input,
    ]))
    .unwrap_or_else(|err| panic!("Rust ffprobe {label} XML packet path should execute: {err}"));

    let oracle = Command::new(ffprobe)
        .args(["-v", "error", "-show_packets", "-of", "xml", input])
        .output()
        .unwrap_or_else(|err| panic!("failed to run oracle `{}`: {err}", ffprobe.display()));

    assert!(
        oracle.status.success(),
        "oracle ffprobe XML failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        oracle.status.code(),
        String::from_utf8_lossy(&oracle.stdout),
        String::from_utf8_lossy(&oracle.stderr)
    );

    let oracle_xml =
        String::from_utf8(oracle.stdout).expect("oracle ffprobe XML output should be UTF-8");
    let rust_packets = parse_xml_packet_sections(&rust_xml);
    let oracle_packets = parse_xml_packet_sections(&oracle_xml);
    assert_eq!(
        rust_packets.len(),
        oracle_packets.len(),
        "{label} XML packet counts should match"
    );

    for (index, (rust_packet, oracle_packet)) in
        rust_packets.iter().zip(oracle_packets.iter()).enumerate()
    {
        assert_fields_match(
            rust_packet,
            oracle_packet,
            PACKET_FIELDS,
            &format!("{label} XML PACKET[{index}]"),
        );
    }
}

fn assert_packet_data_hash_fields_match(ffprobe: &Path, input: &str, label: &str) {
    let rust_default = ffprobe_output(&strings(&[
        "-hide_banner",
        "-show_packets",
        "-show_data_hash",
        "md5",
        input,
    ]))
    .unwrap_or_else(|err| panic!("Rust ffprobe {label} default hash path should execute: {err}"));

    let oracle = Command::new(ffprobe)
        .args([
            "-v",
            "error",
            "-show_packets",
            "-show_data_hash",
            "md5",
            input,
        ])
        .output()
        .unwrap_or_else(|err| panic!("failed to run oracle `{}`: {err}", ffprobe.display()));
    assert!(
        oracle.status.success(),
        "oracle ffprobe default hash failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        oracle.status.code(),
        String::from_utf8_lossy(&oracle.stdout),
        String::from_utf8_lossy(&oracle.stderr)
    );
    let oracle_default =
        String::from_utf8(oracle.stdout).expect("oracle ffprobe default output should be UTF-8");
    assert_packet_sections_match(
        parse_default_sections(&rust_default)
            .into_iter()
            .filter(|section| section.name == "PACKET")
            .collect::<Vec<_>>(),
        parse_default_sections(&oracle_default)
            .into_iter()
            .filter(|section| section.name == "PACKET")
            .collect::<Vec<_>>(),
        PACKET_HASH_FIELDS,
        &format!("{label} default hash"),
    );

    assert_packet_hash_writer_fields_match(ffprobe, input, label, "json");
    assert_packet_hash_writer_fields_match(ffprobe, input, label, "compact");
    assert_packet_hash_writer_fields_match(ffprobe, input, label, "csv");
    assert_packet_hash_writer_fields_match(ffprobe, input, label, "flat");
    assert_packet_hash_writer_fields_match(ffprobe, input, label, "ini");
    assert_packet_hash_writer_fields_match(ffprobe, input, label, "xml");
}

fn assert_packet_data_fields_match(ffprobe: &Path, input: &str, label: &str) {
    assert_packet_extra_fields_match(
        ffprobe,
        input,
        label,
        &["-show_data"],
        PACKET_DATA_FIELDS,
        "data",
    );
}

fn assert_packet_data_and_hash_fields_match(ffprobe: &Path, input: &str, label: &str) {
    assert_packet_extra_fields_match(
        ffprobe,
        input,
        label,
        &["-show_data", "-show_data_hash", "md5"],
        PACKET_DATA_HASH_FIELDS,
        "data hash",
    );
}

fn assert_packet_selected_fields_match(ffprobe: &Path, input: &str, label: &str) {
    let rust_default = ffprobe_output(&strings(&[
        "-hide_banner",
        "-show_entries",
        "packet=flags,size,pts_time",
        input,
    ]))
    .unwrap_or_else(|err| {
        panic!("Rust ffprobe {label} implicit selected packet path should execute: {err}")
    });

    let oracle = Command::new(ffprobe)
        .args([
            "-v",
            "error",
            "-show_entries",
            "packet=flags,size,pts_time",
            input,
        ])
        .output()
        .unwrap_or_else(|err| panic!("failed to run oracle `{}`: {err}", ffprobe.display()));
    assert!(
        oracle.status.success(),
        "oracle ffprobe implicit selected packet failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        oracle.status.code(),
        String::from_utf8_lossy(&oracle.stdout),
        String::from_utf8_lossy(&oracle.stderr)
    );
    let oracle_default =
        String::from_utf8(oracle.stdout).expect("oracle ffprobe default output should be UTF-8");
    assert_packet_sections_match(
        parse_default_sections(&rust_default)
            .into_iter()
            .filter(|section| section.name == "PACKET")
            .collect::<Vec<_>>(),
        parse_default_sections(&oracle_default)
            .into_iter()
            .filter(|section| section.name == "PACKET")
            .collect::<Vec<_>>(),
        PACKET_SELECTED_FIELDS,
        &format!("{label} implicit selected packet"),
    );

    assert_packet_extra_fields_match(
        ffprobe,
        input,
        label,
        &["-show_entries", "packet=flags,size,pts_time"],
        PACKET_SELECTED_FIELDS,
        "selected packet",
    );
}

fn assert_mixed_show_entries_fields_match(ffprobe: &Path, input: &str, label: &str) {
    let rust = ffprobe_output(&strings(&[
        "-hide_banner",
        "-show_entries",
        MIXED_SHOW_ENTRIES,
        input,
    ]))
    .unwrap_or_else(|err| {
        panic!("Rust ffprobe {label} mixed show_entries path should execute: {err}")
    });

    let oracle = Command::new(ffprobe)
        .args(["-v", "error", "-show_entries", MIXED_SHOW_ENTRIES, input])
        .output()
        .unwrap_or_else(|err| panic!("failed to run oracle `{}`: {err}", ffprobe.display()));
    assert!(
        oracle.status.success(),
        "oracle ffprobe mixed show_entries failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        oracle.status.code(),
        String::from_utf8_lossy(&oracle.stdout),
        String::from_utf8_lossy(&oracle.stderr)
    );
    let oracle_default =
        String::from_utf8(oracle.stdout).expect("oracle ffprobe default output should be UTF-8");

    let rust_sections = parse_default_sections(&rust);
    let oracle_sections = parse_default_sections(&oracle_default);

    assert_packet_sections_match(
        sections_named(&rust_sections, "PACKET")
            .into_iter()
            .cloned()
            .collect(),
        sections_named(&oracle_sections, "PACKET")
            .into_iter()
            .cloned()
            .collect(),
        MIXED_PACKET_FIELDS,
        &format!("{label} mixed show_entries packet"),
    );
    assert_single_section_exact_fields_match(
        &rust_sections,
        &oracle_sections,
        "STREAM",
        MIXED_STREAM_FIELDS,
    );
    assert_single_section_exact_fields_match(
        &rust_sections,
        &oracle_sections,
        "FORMAT",
        MIXED_FORMAT_FIELDS,
    );
}

fn assert_packet_and_stream_tag_show_entries_fields_match(
    ffprobe: &Path,
    input: &str,
    label: &str,
    stream_fields: &[&str],
) {
    let rust = ffprobe_output(&strings(&[
        "-hide_banner",
        "-show_entries",
        TAG_SHOW_ENTRIES,
        input,
    ]))
    .unwrap_or_else(|err| {
        panic!("Rust ffprobe {label} tag show_entries path should execute: {err}")
    });

    let oracle = Command::new(ffprobe)
        .args(["-v", "error", "-show_entries", TAG_SHOW_ENTRIES, input])
        .output()
        .unwrap_or_else(|err| panic!("failed to run oracle `{}`: {err}", ffprobe.display()));
    assert!(
        oracle.status.success(),
        "oracle ffprobe tag show_entries failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        oracle.status.code(),
        String::from_utf8_lossy(&oracle.stdout),
        String::from_utf8_lossy(&oracle.stderr)
    );
    let oracle_default =
        String::from_utf8(oracle.stdout).expect("oracle ffprobe default output should be UTF-8");

    let rust_sections = parse_default_sections(&rust);
    let oracle_sections = parse_default_sections(&oracle_default);

    assert_packet_sections_match(
        sections_named(&rust_sections, "PACKET")
            .into_iter()
            .cloned()
            .collect(),
        sections_named(&oracle_sections, "PACKET")
            .into_iter()
            .cloned()
            .collect(),
        TAG_PACKET_FIELDS,
        &format!("{label} tag show_entries packet"),
    );
    assert_single_section_exact_fields_match(
        &rust_sections,
        &oracle_sections,
        "STREAM",
        stream_fields,
    );
}

fn assert_packet_extra_fields_match(
    ffprobe: &Path,
    input: &str,
    label: &str,
    extra_args: &[&str],
    fields: &[&str],
    evidence_name: &str,
) {
    let mut rust_args = vec!["-hide_banner", "-show_packets"];
    rust_args.extend_from_slice(extra_args);
    rust_args.push(input);
    let rust_default = ffprobe_output(&strings(&rust_args)).unwrap_or_else(|err| {
        panic!("Rust ffprobe {label} default {evidence_name} path should execute: {err}")
    });

    let mut oracle_args = vec!["-v", "error", "-show_packets"];
    oracle_args.extend_from_slice(extra_args);
    oracle_args.push(input);
    let oracle = Command::new(ffprobe)
        .args(&oracle_args)
        .output()
        .unwrap_or_else(|err| panic!("failed to run oracle `{}`: {err}", ffprobe.display()));
    assert!(
        oracle.status.success(),
        "oracle ffprobe default {evidence_name} failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        oracle.status.code(),
        String::from_utf8_lossy(&oracle.stdout),
        String::from_utf8_lossy(&oracle.stderr)
    );
    let oracle_default =
        String::from_utf8(oracle.stdout).expect("oracle ffprobe default output should be UTF-8");
    assert_packet_sections_match(
        parse_default_sections(&rust_default)
            .into_iter()
            .filter(|section| section.name == "PACKET")
            .collect::<Vec<_>>(),
        parse_default_sections(&oracle_default)
            .into_iter()
            .filter(|section| section.name == "PACKET")
            .collect::<Vec<_>>(),
        fields,
        &format!("{label} default {evidence_name}"),
    );

    assert_packet_extra_writer_fields_match(
        ffprobe,
        input,
        label,
        extra_args,
        fields,
        evidence_name,
        "json",
    );
    assert_packet_extra_writer_fields_match(
        ffprobe,
        input,
        label,
        extra_args,
        fields,
        evidence_name,
        "compact",
    );
    assert_packet_extra_writer_fields_match(
        ffprobe,
        input,
        label,
        extra_args,
        fields,
        evidence_name,
        "csv",
    );
    assert_packet_extra_writer_fields_match(
        ffprobe,
        input,
        label,
        extra_args,
        fields,
        evidence_name,
        "flat",
    );
    assert_packet_extra_writer_fields_match(
        ffprobe,
        input,
        label,
        extra_args,
        fields,
        evidence_name,
        "ini",
    );
    assert_packet_extra_writer_fields_match(
        ffprobe,
        input,
        label,
        extra_args,
        fields,
        evidence_name,
        "xml",
    );
}

fn assert_packet_extra_writer_fields_match(
    ffprobe: &Path,
    input: &str,
    label: &str,
    extra_args: &[&str],
    fields: &[&str],
    evidence_name: &str,
    writer: &str,
) {
    let mut rust_args = vec!["-hide_banner", "-show_packets"];
    rust_args.extend_from_slice(extra_args);
    rust_args.extend_from_slice(&["-of", writer, input]);
    let rust = ffprobe_output(&strings(&rust_args)).unwrap_or_else(|err| {
        panic!("Rust ffprobe {label} {writer} {evidence_name} path should execute: {err}")
    });

    let mut oracle_args = vec!["-v", "error", "-show_packets"];
    oracle_args.extend_from_slice(extra_args);
    oracle_args.extend_from_slice(&["-of", writer, input]);
    let oracle = Command::new(ffprobe)
        .args(&oracle_args)
        .output()
        .unwrap_or_else(|err| panic!("failed to run oracle `{}`: {err}", ffprobe.display()));

    assert!(
        oracle.status.success(),
        "oracle ffprobe {writer} {evidence_name} failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        oracle.status.code(),
        String::from_utf8_lossy(&oracle.stdout),
        String::from_utf8_lossy(&oracle.stderr)
    );

    let oracle_output =
        String::from_utf8(oracle.stdout).expect("oracle ffprobe output should be UTF-8");
    let rust_packets = parse_packet_sections_for_writer(writer, &rust, fields);
    let oracle_packets = parse_packet_sections_for_writer(writer, &oracle_output, fields);
    assert_packet_sections_match(
        rust_packets,
        oracle_packets,
        fields,
        &format!("{label} {writer} {evidence_name}"),
    );
}

fn assert_packet_hash_writer_fields_match(ffprobe: &Path, input: &str, label: &str, writer: &str) {
    let rust = ffprobe_output(&strings(&[
        "-hide_banner",
        "-show_packets",
        "-show_data_hash",
        "md5",
        "-of",
        writer,
        input,
    ]))
    .unwrap_or_else(|err| panic!("Rust ffprobe {label} {writer} hash path should execute: {err}"));

    let oracle = Command::new(ffprobe)
        .args([
            "-v",
            "error",
            "-show_packets",
            "-show_data_hash",
            "md5",
            "-of",
            writer,
            input,
        ])
        .output()
        .unwrap_or_else(|err| panic!("failed to run oracle `{}`: {err}", ffprobe.display()));

    assert!(
        oracle.status.success(),
        "oracle ffprobe {writer} hash failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        oracle.status.code(),
        String::from_utf8_lossy(&oracle.stdout),
        String::from_utf8_lossy(&oracle.stderr)
    );

    let oracle_output =
        String::from_utf8(oracle.stdout).expect("oracle ffprobe output should be UTF-8");
    let rust_packets = parse_packet_sections_for_writer(writer, &rust, PACKET_HASH_FIELDS);
    let oracle_packets =
        parse_packet_sections_for_writer(writer, &oracle_output, PACKET_HASH_FIELDS);
    assert_packet_sections_match(
        rust_packets,
        oracle_packets,
        PACKET_HASH_FIELDS,
        &format!("{label} {writer} hash"),
    );
}

fn parse_packet_sections_for_writer(writer: &str, output: &str, fields: &[&str]) -> Vec<Section> {
    match writer {
        "json" => parse_json_packet_sections(output),
        "compact" => parse_compact_packet_sections(output),
        "csv" => parse_csv_packet_sections_with_fields(output, fields),
        "flat" => parse_flat_packet_sections(output),
        "ini" => parse_ini_packet_sections(output),
        "xml" => parse_xml_packet_sections(output),
        _ => panic!("unsupported packet writer `{writer}`"),
    }
}

fn assert_packet_sections_match(
    rust_packets: Vec<Section>,
    oracle_packets: Vec<Section>,
    fields: &[&str],
    label: &str,
) {
    assert_eq!(
        rust_packets.len(),
        oracle_packets.len(),
        "{label} packet counts should match"
    );

    for (index, (rust_packet, oracle_packet)) in
        rust_packets.iter().zip(oracle_packets.iter()).enumerate()
    {
        assert_exact_packet_fields(
            rust_packet,
            fields,
            &format!("{label} Rust PACKET[{index}]"),
        );
        assert_exact_packet_fields(
            oracle_packet,
            fields,
            &format!("{label} oracle PACKET[{index}]"),
        );
        assert_fields_match(
            rust_packet,
            oracle_packet,
            fields,
            &format!("{label} PACKET[{index}]"),
        );
    }
}

fn assert_exact_packet_fields(packet: &Section, fields: &[&str], label: &str) {
    assert_exact_fields(packet, fields, label);
}

fn assert_exact_fields(section: &Section, fields: &[&str], label: &str) {
    assert_eq!(
        section.fields.len(),
        fields.len(),
        "{label} should have exact field count"
    );
    for key in section.fields.keys() {
        assert!(
            fields.contains(&key.as_str()),
            "{label} should not contain extra field `{key}`"
        );
    }
}

fn parse_compact_packet_sections(output: &str) -> Vec<Section> {
    output
        .lines()
        .filter_map(|line| {
            let mut parts = line.trim_end_matches('\r').split('|');
            match parts.next() {
                Some("packet") => {
                    let mut section = Section {
                        name: "PACKET".to_owned(),
                        fields: BTreeMap::new(),
                    };
                    for field in parts {
                        let Some((key, value)) = field.split_once('=') else {
                            continue;
                        };
                        section
                            .fields
                            .insert(key.to_owned(), unescape_compact_value(value));
                    }
                    Some(section)
                }
                _ => None,
            }
        })
        .collect()
}

fn parse_csv_packet_sections(output: &str) -> Vec<Section> {
    parse_csv_packet_sections_with_fields(output, PACKET_FIELDS)
}

fn parse_csv_packet_sections_with_fields(output: &str, fields: &[&str]) -> Vec<Section> {
    split_csv_records(output)
        .into_iter()
        .filter_map(|line| {
            let values = split_csv_line(&line);
            if values.first().map(String::as_str) != Some("packet") {
                return None;
            }
            assert!(
                values.len() > fields.len(),
                "CSV packet row has too few fields: `{line}`"
            );
            let mut section = Section {
                name: "PACKET".to_owned(),
                fields: BTreeMap::new(),
            };
            for (field, value) in fields.iter().zip(values.iter().skip(1)) {
                section.fields.insert((*field).to_owned(), value.clone());
            }
            Some(section)
        })
        .collect()
}

fn split_csv_records(output: &str) -> Vec<String> {
    let mut records = Vec::new();
    let mut current = String::new();
    let mut chars = output.chars().peekable();
    let mut in_quotes = false;

    while let Some(ch) = chars.next() {
        match ch {
            '"' if in_quotes && chars.peek() == Some(&'"') => {
                current.push('"');
                current.push('"');
                let _ = chars.next();
            }
            '"' => {
                in_quotes = !in_quotes;
                current.push('"');
            }
            '\n' if !in_quotes => {
                let record = current.trim_end_matches('\r').to_owned();
                if !record.is_empty() {
                    records.push(record);
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    assert!(!in_quotes, "unterminated CSV quoted field in output");
    let record = current.trim_end_matches('\r').to_owned();
    if !record.is_empty() {
        records.push(record);
    }
    records
}

fn split_csv_line(line: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut current = String::new();
    let mut chars = line.chars().peekable();
    let mut in_quotes = false;

    while let Some(ch) = chars.next() {
        match ch {
            '"' if in_quotes && chars.peek() == Some(&'"') => {
                current.push('"');
                let _ = chars.next();
            }
            '"' => in_quotes = !in_quotes,
            ',' if !in_quotes => {
                values.push(current);
                current = String::new();
            }
            _ => current.push(ch),
        }
    }

    assert!(!in_quotes, "unterminated CSV quoted field: `{line}`");
    values.push(current);
    values
}

fn parse_flat_packet_sections(output: &str) -> Vec<Section> {
    let mut sections: BTreeMap<usize, Section> = BTreeMap::new();

    for raw_line in output.lines() {
        let line = raw_line.trim_end_matches('\r');
        let Some(rest) = line.strip_prefix("packets.packet.") else {
            continue;
        };
        let Some((index, rest)) = rest.split_once('.') else {
            panic!("flat packet line should include packet index and field: `{line}`");
        };
        let index = index
            .parse::<usize>()
            .unwrap_or_else(|err| panic!("flat packet index should be numeric in `{line}`: {err}"));
        let Some((field, value)) = rest.split_once('=') else {
            panic!("flat packet line should contain `=`: `{line}`");
        };
        sections
            .entry(index)
            .or_insert_with(|| Section {
                name: "PACKET".to_owned(),
                fields: BTreeMap::new(),
            })
            .fields
            .insert(field.to_owned(), value.to_owned());
    }

    sections.into_values().collect()
}

fn parse_ini_packet_sections(output: &str) -> Vec<Section> {
    let mut sections: BTreeMap<usize, Section> = BTreeMap::new();
    let mut current: Option<(usize, Section)> = None;

    for raw_line in output.lines() {
        let line = raw_line.trim_end_matches('\r');
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(section_name) = line
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
        {
            if let Some((index, section)) = current.take() {
                sections.insert(index, section);
            }
            let Some(index_text) = section_name.strip_prefix("packets.packet.") else {
                current = None;
                continue;
            };
            let index = index_text.parse::<usize>().unwrap_or_else(|err| {
                panic!("INI packet index should be numeric in `{line}`: {err}")
            });
            current = Some((
                index,
                Section {
                    name: "PACKET".to_owned(),
                    fields: BTreeMap::new(),
                },
            ));
            continue;
        }
        let Some((_, section)) = &mut current else {
            continue;
        };
        let Some((field, value)) = line.split_once('=') else {
            panic!("INI packet line should contain `=`: `{line}`");
        };
        section.fields.insert(field.to_owned(), value.to_owned());
    }

    if let Some((index, section)) = current {
        sections.insert(index, section);
    }

    sections.into_values().collect()
}

fn parse_xml_packet_sections(output: &str) -> Vec<Section> {
    let mut sections = Vec::new();
    let mut current_packet: Option<String> = None;

    for raw_line in output.lines() {
        let line = raw_line.trim();
        if let Some(packet) = &mut current_packet {
            packet.push('\n');
            packet.push_str(line);
            if line.ends_with("/>") || line.ends_with('>') {
                let packet = current_packet
                    .take()
                    .expect("packet element should be present");
                sections.push(parse_xml_packet_section(&packet));
            }
            continue;
        }

        if line.starts_with("<packet ") {
            if line.ends_with("/>") || line.ends_with('>') {
                sections.push(parse_xml_packet_section(line));
            } else {
                current_packet = Some(line.to_owned());
            }
        }
    }

    assert!(
        current_packet.is_none(),
        "unterminated XML packet element in output"
    );
    sections
}

fn parse_xml_packet_section(packet: &str) -> Section {
    let line = packet.trim();
    let rest = line
        .strip_prefix("<packet ")
        .unwrap_or_else(|| panic!("XML packet element should start with `<packet `: `{line}`"));
    let attributes = rest
        .strip_suffix("/>")
        .or_else(|| rest.strip_suffix('>'))
        .unwrap_or(rest)
        .trim();
    Section {
        name: "PACKET".to_owned(),
        fields: parse_xml_attributes(attributes),
    }
}

fn parse_xml_attributes(attributes: &str) -> BTreeMap<String, String> {
    let mut fields = BTreeMap::new();
    let mut rest = attributes.trim();
    while !rest.is_empty() {
        let (key, after_key) = rest
            .split_once('=')
            .unwrap_or_else(|| panic!("XML packet attribute should be key=value in `{rest}`"));
        let after_key = after_key.trim_start();
        let value_rest = after_key
            .strip_prefix('"')
            .unwrap_or_else(|| panic!("XML packet attribute `{key}` should start with quote"));
        let end = value_rest
            .find('"')
            .unwrap_or_else(|| panic!("XML packet attribute `{key}` should end with quote"));
        fields.insert(key.trim().to_owned(), value_rest[..end].to_owned());
        rest = value_rest[end + 1..].trim_start();
    }
    fields
}

fn unescape_compact_value(value: &str) -> String {
    let mut out = String::new();
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some(next) => out.push(next),
            None => out.push('\\'),
        }
    }
    out
}

fn parse_json_packet_sections(output: &str) -> Vec<Section> {
    let mut sections = Vec::new();
    let Some(packets_key) = output.find("\"packets\"") else {
        return sections;
    };
    let Some(array_offset) = output[packets_key..].find('[') else {
        return sections;
    };
    let array_start = packets_key + array_offset;
    let mut object_start: Option<usize> = None;
    let mut object_depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for (offset, ch) in output[array_start + 1..].char_indices() {
        let index = array_start + 1 + offset;
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '{' => {
                if object_depth == 0 {
                    object_start = Some(index);
                }
                object_depth += 1;
            }
            '}' => {
                assert!(object_depth > 0, "unexpected JSON object close");
                object_depth -= 1;
                if object_depth == 0 {
                    let start = object_start
                        .take()
                        .expect("JSON packet object should have a start offset");
                    sections.push(parse_json_packet_object(&output[start..=index]));
                }
            }
            ']' if object_depth == 0 => break,
            _ => {}
        }
    }

    assert_eq!(object_depth, 0, "unclosed JSON packet object");
    sections
}

fn parse_json_packet_object(object: &str) -> Section {
    let mut section = Section {
        name: "PACKET".to_owned(),
        fields: BTreeMap::new(),
    };
    let mut index = object
        .find('{')
        .map(|index| index + 1)
        .expect("JSON packet object should start with `{`");

    loop {
        skip_json_ws_and_commas(object, &mut index);
        if object[index..].starts_with('}') {
            break;
        }

        let key = parse_json_string_token(object, &mut index);
        skip_json_ws(object, &mut index);
        assert!(
            object[index..].starts_with(':'),
            "JSON packet field should contain `:`"
        );
        index += 1;
        skip_json_ws(object, &mut index);

        let value_start = index;
        if object[index..].starts_with('"') {
            let _ = parse_json_string_token(object, &mut index);
        } else {
            while index < object.len() && !matches!(object.as_bytes()[index], b',' | b'}') {
                index += 1;
            }
        }
        let value = object[value_start..index].trim().to_owned();
        section.fields.insert(key, value);
    }

    section
}

fn skip_json_ws_and_commas(input: &str, index: &mut usize) {
    while *index < input.len()
        && matches!(
            input.as_bytes()[*index],
            b' ' | b'\n' | b'\r' | b'\t' | b','
        )
    {
        *index += 1;
    }
}

fn skip_json_ws(input: &str, index: &mut usize) {
    while *index < input.len() && matches!(input.as_bytes()[*index], b' ' | b'\n' | b'\r' | b'\t') {
        *index += 1;
    }
}

fn parse_json_string_token(input: &str, index: &mut usize) -> String {
    assert!(
        input[*index..].starts_with('"'),
        "JSON string token should start with quote"
    );
    *index += 1;
    let start = *index;
    let mut escaped = false;
    while *index < input.len() {
        let byte = input.as_bytes()[*index];
        if escaped {
            escaped = false;
            *index += 1;
            continue;
        }
        match byte {
            b'\\' => {
                escaped = true;
                *index += 1;
            }
            b'"' => {
                let token = input[start..*index].to_owned();
                *index += 1;
                return token;
            }
            _ => *index += 1,
        }
    }
    panic!("unterminated JSON string token");
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
