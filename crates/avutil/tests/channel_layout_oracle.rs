use avutil::{
    channel_layout::ChannelLayoutRetypeResult, AvErrorCode, Channel, ChannelId, ChannelLayout,
    ChannelLayoutSpec,
};
use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Debug, Clone, Copy)]
struct ParserCase {
    id: &'static str,
    input: &'static str,
}

#[derive(Debug, Clone, Copy)]
struct ByteParserCase {
    id: &'static str,
    input: &'static [u8],
}

#[derive(Debug, Clone, Copy)]
struct RetypeCase {
    id: &'static str,
    input: &'static str,
    target: RetypeTarget,
    allow_lossy: bool,
    canonical: bool,
}

#[derive(Debug, Clone, Copy)]
struct CompareCase {
    id: &'static str,
    left: &'static str,
    right: &'static str,
}

#[derive(Debug, Clone, Copy)]
struct DefaultCase {
    id: &'static str,
    channels: i32,
}

#[derive(Debug, Clone, Copy)]
enum RetypeTarget {
    Native,
    Custom,
    Unspecified,
    Ambisonic,
}

const PARSER_CASES: &[ParserCase] = &[
    ParserCase {
        id: "empty",
        input: "",
    },
    ParserCase {
        id: "all-space",
        input: " ",
    },
    ParserCase {
        id: "native-name",
        input: "stereo",
    },
    ParserCase {
        id: "native-list",
        input: "FL+FR",
    },
    ParserCase {
        id: "sparse-list",
        input: "FL+FC",
    },
    ParserCase {
        id: "sparse-mask",
        input: "0x5",
    },
    ParserCase {
        id: "plus-hex-mask",
        input: " +0x3",
    },
    ParserCase {
        id: "octal-mask",
        input: "03",
    },
    ParserCase {
        id: "invalid-zero-mask",
        input: "0x0",
    },
    ParserCase {
        id: "high-bit-mask",
        input: "0x8000000000000000",
    },
    ParserCase {
        id: "leading-space-mask",
        input: " 0x3",
    },
    ParserCase {
        id: "trailing-space-mask",
        input: "0x3 ",
    },
    ParserCase {
        id: "default-count",
        input: "10c",
    },
    ParserCase {
        id: "unspecified-count",
        input: "2C",
    },
    ParserCase {
        id: "plus-unspecified-count",
        input: "+2C",
    },
    ParserCase {
        id: "described-native",
        input: "2 channels (FL+FR)",
    },
    ParserCase {
        id: "leading-plus-described-native",
        input: " +2 channels (FL+FR)",
    },
    ParserCase {
        id: "described-sparse",
        input: "2 channels (FL+FC)",
    },
    ParserCase {
        id: "invalid-described-count-mismatch",
        input: "1 channels (FL+FR)",
    },
    ParserCase {
        id: "described-custom",
        input: "2 channels (FL@Left+FR@Right)",
    },
    ParserCase {
        id: "named-custom",
        input: "FL@Left+FR@Right",
    },
    ParserCase {
        id: "escaped-custom-name",
        input: "FL@Left\\+Right+FR",
    },
    ParserCase {
        id: "escaped-at-custom-name",
        input: "FL@Left\\@Name+FR",
    },
    ParserCase {
        id: "repeated-at-name",
        input: "FL@Left@Again",
    },
    ParserCase {
        id: "spaced-custom-key",
        input: "FL @ Left + FR",
    },
    ParserCase {
        id: "quoted-channel-id",
        input: "'FL'+FR",
    },
    ParserCase {
        id: "quoted-custom-name",
        input: "FL@'Left Right'+FR",
    },
    ParserCase {
        id: "duplicate-native-custom",
        input: "FL+FL",
    },
    ParserCase {
        id: "unknown-unused-custom",
        input: "UNK+UNSD",
    },
    ParserCase {
        id: "trailing-separator",
        input: "FL+",
    },
    ParserCase {
        id: "invalid-leading-separator",
        input: "+FL",
    },
    ParserCase {
        id: "invalid-empty-token",
        input: "FL++FR",
    },
    ParserCase {
        id: "zeroth-order-ambisonic",
        input: "ambisonic 0",
    },
    ParserCase {
        id: "signed-zero-ambisonic",
        input: "ambisonic -0",
    },
    ParserCase {
        id: "ambisonic-list",
        input: "AMBI0+AMBI1+AMBI2+AMBI3",
    },
    ParserCase {
        id: "explicit-ambisonic",
        input: "ambisonic 1+stereo",
    },
    ParserCase {
        id: "hex-order-ambisonic",
        input: "ambisonic 0x1+stereo",
    },
    ParserCase {
        id: "plus-hex-order-ambisonic",
        input: "ambisonic +0x1",
    },
    ParserCase {
        id: "sparse-extra-ambisonic",
        input: "ambisonic 1+FL+FC",
    },
    ParserCase {
        id: "zero-order-mask-extra-ambisonic",
        input: "ambisonic 0+0x5",
    },
    ParserCase {
        id: "signed-zero-mask-extra-ambisonic",
        input: "ambisonic -0+0x5",
    },
    ParserCase {
        id: "signed-zero-list-extra-ambisonic",
        input: "ambisonic -0+FL+FC",
    },
    ParserCase {
        id: "named-extra-ambisonic",
        input: "ambisonic +1+FL@Left+FR@Right",
    },
    ParserCase {
        id: "zero-order-extra",
        input: "ambisonic +stereo",
    },
    ParserCase {
        id: "raw-extra-ambisonic",
        input: "ambisonic 1+0x200000000000",
    },
    ParserCase {
        id: "raw-users-base0",
        input: "USR0x2d+USR056",
    },
    ParserCase {
        id: "raw-users-base0-uppercase-x",
        input: "USR0X2D+USR056",
    },
    ParserCase {
        id: "signed-zero-user",
        input: "USR-0",
    },
    ParserCase {
        id: "ambisonic-no-conversion-extra",
        input: "AMBIx+USR-0",
    },
    ParserCase {
        id: "signed-zero-explicit-ambisonic",
        input: "ambisonic -0+stereo",
    },
    ParserCase {
        id: "invalid-ambisonic-trailing",
        input: "ambisonic 1 trailing",
    },
    ParserCase {
        id: "invalid-ambisonic-extra-ambisonic",
        input: "ambisonic 1+AMBI0",
    },
    ParserCase {
        id: "invalid-ambisonic-octal-junk",
        input: "ambisonic 09",
    },
    ParserCase {
        id: "invalid-lowercase-list",
        input: "fl+fr",
    },
    ParserCase {
        id: "invalid-default-count",
        input: "9c",
    },
    ParserCase {
        id: "invalid-trailing-count",
        input: "2C ",
    },
    ParserCase {
        id: "invalid-trailing-channels",
        input: "2 channels ",
    },
    ParserCase {
        id: "invalid-uppercase-layout",
        input: "STEREO",
    },
];

const LOOKUP_NAMES: &[&str] = &[
    "FL", "FR", "FC", "BR", "AMBI0", "AMBI3", "AMBI4", "UNK", "UNSD", "USR45", "USR0x2d",
    "USR0X2D", "USR055", "USR-0", "@Left", "FL@Left", "@Right", "FR@Right", "NOPE",
];

const BYTE_PARSER_CASES: &[ByteParserCase] = &[
    ByteParserCase {
        id: "raw-byte-name",
        input: b"FL@\xff+FR",
    },
    ByteParserCase {
        id: "escaped-byte-name",
        input: b"FL@Left\\\xff+FR",
    },
    ByteParserCase {
        id: "quoted-byte-name",
        input: b"FL@'A\xffB'+FR",
    },
    ByteParserCase {
        id: "overlong-name-truncates",
        input: b"FL@1234567890123456+FR",
    },
    ByteParserCase {
        id: "invalid-byte-id",
        input: b"\xffL@A",
    },
    ByteParserCase {
        id: "invalid-ambisonic-bytes",
        input: b"ambisonic \xc3\x28",
    },
];

const BYTE_LOOKUP_NAMES: &[&[u8]] = &[
    b"@\xff",
    b"FL@\xff",
    b"@Left\xff",
    b"FL@Left\xff",
    b"@A\xffB",
    b"FL@A\xffB",
    b"@123456789012345",
    b"@1234567890123456",
    b"FR",
];

const DEFAULT_CASES: &[DefaultCase] = &[
    DefaultCase {
        id: "invalid-negative",
        channels: -1,
    },
    DefaultCase {
        id: "invalid-zero",
        channels: 0,
    },
    DefaultCase {
        id: "mono",
        channels: 1,
    },
    DefaultCase {
        id: "stereo",
        channels: 2,
    },
    DefaultCase {
        id: "two-one",
        channels: 3,
    },
    DefaultCase {
        id: "four-zero",
        channels: 4,
    },
    DefaultCase {
        id: "five-zero",
        channels: 5,
    },
    DefaultCase {
        id: "five-one",
        channels: 6,
    },
    DefaultCase {
        id: "six-one",
        channels: 7,
    },
    DefaultCase {
        id: "seven-one",
        channels: 8,
    },
    DefaultCase {
        id: "unspecified-nine",
        channels: 9,
    },
    DefaultCase {
        id: "five-one-four",
        channels: 10,
    },
    DefaultCase {
        id: "unspecified-eleven",
        channels: 11,
    },
    DefaultCase {
        id: "seven-one-four",
        channels: 12,
    },
    DefaultCase {
        id: "unspecified-thirteen",
        channels: 13,
    },
    DefaultCase {
        id: "nine-one-four",
        channels: 14,
    },
    DefaultCase {
        id: "unspecified-fifteen",
        channels: 15,
    },
    DefaultCase {
        id: "nine-one-six",
        channels: 16,
    },
    DefaultCase {
        id: "unspecified-seventeen",
        channels: 17,
    },
    DefaultCase {
        id: "unspecified-twentythree",
        channels: 23,
    },
    DefaultCase {
        id: "twentytwo-two",
        channels: 24,
    },
    DefaultCase {
        id: "unspecified-twentyfive",
        channels: 25,
    },
    DefaultCase {
        id: "unspecified-sixtyfour",
        channels: 64,
    },
];

const RETYPE_CASES: &[RetypeCase] = &[
    RetypeCase {
        id: "native-to-custom",
        input: "stereo",
        target: RetypeTarget::Custom,
        allow_lossy: true,
        canonical: false,
    },
    RetypeCase {
        id: "native-to-unspec-lossy",
        input: "stereo",
        target: RetypeTarget::Unspecified,
        allow_lossy: true,
        canonical: false,
    },
    RetypeCase {
        id: "native-to-unspec-lossless-reject",
        input: "stereo",
        target: RetypeTarget::Unspecified,
        allow_lossy: false,
        canonical: false,
    },
    RetypeCase {
        id: "named-custom-to-native-lossy",
        input: "FL@Left+FR@Right",
        target: RetypeTarget::Native,
        allow_lossy: true,
        canonical: false,
    },
    RetypeCase {
        id: "named-custom-to-native-lossless-reject",
        input: "FL@Left+FR@Right",
        target: RetypeTarget::Native,
        allow_lossy: false,
        canonical: false,
    },
    RetypeCase {
        id: "duplicate-custom-to-native-reject",
        input: "FL+FL",
        target: RetypeTarget::Native,
        allow_lossy: true,
        canonical: false,
    },
    RetypeCase {
        id: "raw-users-to-native-lossless",
        input: "USR45+USR46",
        target: RetypeTarget::Native,
        allow_lossy: false,
        canonical: false,
    },
    RetypeCase {
        id: "named-raw-users-to-native-lossy",
        input: "USR45@Wide+USR46",
        target: RetypeTarget::Native,
        allow_lossy: true,
        canonical: false,
    },
    RetypeCase {
        id: "named-raw-users-to-native-lossless-reject",
        input: "USR45@Wide+USR46",
        target: RetypeTarget::Native,
        allow_lossy: false,
        canonical: false,
    },
    RetypeCase {
        id: "uppercase-hex-raw-users-to-unspec-lossy",
        input: "USR0X2D+USR056",
        target: RetypeTarget::Unspecified,
        allow_lossy: true,
        canonical: false,
    },
    RetypeCase {
        id: "unspec-to-custom",
        input: "2C",
        target: RetypeTarget::Custom,
        allow_lossy: true,
        canonical: false,
    },
    RetypeCase {
        id: "unspec-to-native-reject",
        input: "2C",
        target: RetypeTarget::Native,
        allow_lossy: true,
        canonical: false,
    },
    RetypeCase {
        id: "unknown-unused-to-unspec-lossy",
        input: "UNK+UNSD",
        target: RetypeTarget::Unspecified,
        allow_lossy: true,
        canonical: false,
    },
    RetypeCase {
        id: "unknown-unused-to-unspec-lossless-reject",
        input: "UNK+UNSD",
        target: RetypeTarget::Unspecified,
        allow_lossy: false,
        canonical: false,
    },
    RetypeCase {
        id: "ambisonic-to-custom",
        input: "ambisonic 1+stereo",
        target: RetypeTarget::Custom,
        allow_lossy: true,
        canonical: false,
    },
    RetypeCase {
        id: "ambisonic-to-unspec-lossy",
        input: "ambisonic 1+stereo",
        target: RetypeTarget::Unspecified,
        allow_lossy: true,
        canonical: false,
    },
    RetypeCase {
        id: "ambisonic-to-unspec-lossless-reject",
        input: "ambisonic 1+stereo",
        target: RetypeTarget::Unspecified,
        allow_lossy: false,
        canonical: false,
    },
    RetypeCase {
        id: "named-custom-to-ambisonic-lossy",
        input: "AMBI0@W+AMBI1@Y+AMBI2@Z+AMBI3@X+FL@Left+FR",
        target: RetypeTarget::Ambisonic,
        allow_lossy: true,
        canonical: false,
    },
    RetypeCase {
        id: "named-custom-to-ambisonic-lossless-reject",
        input: "AMBI0@W+AMBI1@Y+AMBI2@Z+AMBI3@X+FL@Left+FR",
        target: RetypeTarget::Ambisonic,
        allow_lossy: false,
        canonical: false,
    },
    RetypeCase {
        id: "native-to-ambisonic-reject",
        input: "stereo",
        target: RetypeTarget::Ambisonic,
        allow_lossy: true,
        canonical: false,
    },
    RetypeCase {
        id: "incomplete-ambisonic-custom-reject",
        input: "AMBI0+AMBI1",
        target: RetypeTarget::Ambisonic,
        allow_lossy: true,
        canonical: false,
    },
    RetypeCase {
        id: "raw-ambisonic-extra-to-ambisonic-lossless",
        input: "AMBI0+AMBI1+AMBI2+AMBI3+USR45",
        target: RetypeTarget::Ambisonic,
        allow_lossy: false,
        canonical: false,
    },
    RetypeCase {
        id: "named-raw-ambisonic-extra-to-ambisonic-lossy",
        input: "AMBI0@W+AMBI1+AMBI2+AMBI3+USR45@Wide",
        target: RetypeTarget::Ambisonic,
        allow_lossy: true,
        canonical: false,
    },
    RetypeCase {
        id: "raw-ambisonic-extra-out-of-order-reject",
        input: "AMBI0+AMBI1+AMBI2+AMBI3+USR46+USR45",
        target: RetypeTarget::Ambisonic,
        allow_lossy: true,
        canonical: false,
    },
    RetypeCase {
        id: "named-custom-canonical-noop",
        input: "FL@Left+FR@Right",
        target: RetypeTarget::Native,
        allow_lossy: true,
        canonical: true,
    },
    RetypeCase {
        id: "duplicate-custom-canonical-noop",
        input: "FL+FL",
        target: RetypeTarget::Native,
        allow_lossy: true,
        canonical: true,
    },
    RetypeCase {
        id: "named-ambisonic-canonical-noop",
        input: "AMBI0@W+AMBI1+AMBI2+AMBI3",
        target: RetypeTarget::Native,
        allow_lossy: true,
        canonical: true,
    },
    RetypeCase {
        id: "raw-ambisonic-extra-canonical",
        input: "AMBI0+AMBI1+AMBI2+AMBI3+USR45",
        target: RetypeTarget::Native,
        allow_lossy: true,
        canonical: true,
    },
    RetypeCase {
        id: "unspec-canonical-noop",
        input: "2C",
        target: RetypeTarget::Native,
        allow_lossy: true,
        canonical: true,
    },
];

const COMPARE_CASES: &[CompareCase] = &[
    CompareCase {
        id: "native-same",
        left: "stereo",
        right: "FL+FR",
    },
    CompareCase {
        id: "native-different",
        left: "stereo",
        right: "mono",
    },
    CompareCase {
        id: "sparse-native-same",
        left: "0x5",
        right: "FL+FC",
    },
    CompareCase {
        id: "sparse-native-order-different",
        left: "FL+FC",
        right: "FC+FL",
    },
    CompareCase {
        id: "named-custom-name-insensitive",
        left: "FL@Left+FR@Right",
        right: "FL+FR",
    },
    CompareCase {
        id: "named-custom-different-channel",
        left: "FL@Left+FR@Right",
        right: "FL+FC",
    },
    CompareCase {
        id: "duplicate-custom-same",
        left: "FL+FL",
        right: "FL+FL",
    },
    CompareCase {
        id: "duplicate-custom-vs-native",
        left: "FL+FL",
        right: "stereo",
    },
    CompareCase {
        id: "unknown-unused-name-insensitive",
        left: "UNK@A+UNSD@B",
        right: "UNK+UNSD",
    },
    CompareCase {
        id: "unknown-unused-vs-unspec",
        left: "UNK+UNSD",
        right: "2C",
    },
    CompareCase {
        id: "unspecified-same",
        left: "2C",
        right: "2 channels",
    },
    CompareCase {
        id: "unspecified-different",
        left: "2C",
        right: "3C",
    },
    CompareCase {
        id: "ambisonic-same",
        left: "ambisonic 1+stereo",
        right: "AMBI0+AMBI1+AMBI2+AMBI3+FL+FR",
    },
    CompareCase {
        id: "ambisonic-extra-different",
        left: "ambisonic 1+stereo",
        right: "ambisonic 1+FL+FC",
    },
    CompareCase {
        id: "plus-hex-order-ambisonic-same",
        left: "ambisonic +0x1",
        right: "ambisonic 1",
    },
    CompareCase {
        id: "zero-order-mask-custom-same",
        left: "ambisonic -0+0x5",
        right: "AMBI0+FL+FC",
    },
    CompareCase {
        id: "zero-order-mask-list-same",
        left: "ambisonic 0+0x5",
        right: "ambisonic -0+FL+FC",
    },
    CompareCase {
        id: "zero-order-extra-different",
        left: "ambisonic -0+0x5",
        right: "ambisonic +stereo",
    },
    CompareCase {
        id: "signed-zero-extra-same",
        left: "ambisonic -0+stereo",
        right: "ambisonic +stereo",
    },
    CompareCase {
        id: "ambisonic-vs-native",
        left: "ambisonic 0+stereo",
        right: "stereo",
    },
    CompareCase {
        id: "raw-mask-same",
        left: "USR45+USR46",
        right: "0x600000000000",
    },
    CompareCase {
        id: "raw-mask-order-different",
        left: "USR46+USR45",
        right: "0x600000000000",
    },
    CompareCase {
        id: "raw-mask-uppercase-token-equivalent",
        left: "USR0X2D+USR056",
        right: "USR0x2d+USR056",
    },
    CompareCase {
        id: "named-raw-ambisonic-extra-same",
        left: "AMBI0@W+AMBI1+AMBI2+AMBI3+USR45@Wide",
        right: "ambisonic 1+0x200000000000",
    },
];

#[derive(Debug, Default)]
struct LayoutInventory {
    channels: BTreeMap<String, String>,
    layouts: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LayoutSection {
    None,
    Channels,
    Layouts,
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn ffmpeg_layout_inventory_matches_current_channel_layout_model() {
    let oracle = oracle_ffmpeg();
    let output = Command::new(&oracle)
        .args(["-hide_banner", "-layouts"])
        .output()
        .unwrap_or_else(|err| panic!("failed to run oracle `{}`: {err}", oracle.display()));

    assert!(
        output.status.success(),
        "oracle failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    if !output.stderr.is_empty() {
        text.push('\n');
        text.push_str(&String::from_utf8_lossy(&output.stderr));
    }

    let inventory = parse_layout_inventory(&text);

    assert_eq!(
        inventory.channels,
        expected_channels(),
        "ffmpeg -layouts individual channel inventory diverged"
    );
    assert_eq!(
        inventory.layouts,
        expected_layouts(),
        "ffmpeg -layouts standard layout inventory diverged"
    );
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 libavutil oracle under third_party/ffmpeg-oracle/wsl"]
fn libavutil_channel_layout_parser_vectors_match_current_model() {
    let repo_root = repo_root();
    let oracle_root = oracle_root(&repo_root);
    let include_dir = oracle_root.join("wsl/include");
    let libavutil = oracle_root.join("wsl/lib/libavutil.a");

    assert!(
        include_dir.join("libavutil/channel_layout.h").is_file(),
        "missing pinned FFmpeg libavutil channel layout headers under `{}`",
        include_dir.display()
    );
    assert!(
        libavutil.is_file(),
        "missing pinned FFmpeg libavutil static library `{}`",
        libavutil.display()
    );

    let work_dir = repo_root.join("target/oracle/avutil-channel-layout");
    fs::create_dir_all(&work_dir).expect("create avutil-channel-layout oracle work dir");
    let source = work_dir.join("channel_layout_parser_oracle.c");
    let executable = work_dir.join("channel_layout_parser_oracle");
    fs::write(&source, parser_oracle_c_source()).expect("write channel layout oracle C source");

    let stdout = compile_and_run_oracle(&include_dir, &libavutil, &source, &executable);
    let oracle = parse_parser_oracle_output(&stdout);

    assert_eq!(
        oracle.keys().collect::<Vec<_>>(),
        expected_parser_rows().keys().collect::<Vec<_>>(),
        "channel layout parser oracle row set diverged"
    );

    for (name, expected_fields) in expected_parser_rows() {
        assert_eq!(
            oracle
                .get(&name)
                .unwrap_or_else(|| panic!("missing parser oracle row `{name}`")),
            &expected_fields,
            "{name} diverged"
        );
    }
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 libavutil oracle under third_party/ffmpeg-oracle/wsl"]
fn libavutil_channel_layout_retype_vectors_match_current_model() {
    let repo_root = repo_root();
    let oracle_root = oracle_root(&repo_root);
    let include_dir = oracle_root.join("wsl/include");
    let libavutil = oracle_root.join("wsl/lib/libavutil.a");

    assert!(
        include_dir.join("libavutil/channel_layout.h").is_file(),
        "missing pinned FFmpeg libavutil channel layout headers under `{}`",
        include_dir.display()
    );
    assert!(
        libavutil.is_file(),
        "missing pinned FFmpeg libavutil static library `{}`",
        libavutil.display()
    );

    let work_dir = repo_root.join("target/oracle/avutil-channel-layout");
    fs::create_dir_all(&work_dir).expect("create avutil-channel-layout oracle work dir");
    let source = work_dir.join("channel_layout_retype_oracle.c");
    let executable = work_dir.join("channel_layout_retype_oracle");
    fs::write(&source, retype_oracle_c_source())
        .expect("write channel layout retype oracle C source");

    let stdout = compile_and_run_oracle(&include_dir, &libavutil, &source, &executable);
    let oracle = parse_parser_oracle_output(&stdout);

    assert_eq!(
        oracle.keys().collect::<Vec<_>>(),
        expected_retype_rows().keys().collect::<Vec<_>>(),
        "channel layout retype oracle row set diverged"
    );

    for (name, expected_fields) in expected_retype_rows() {
        assert_eq!(
            oracle
                .get(&name)
                .unwrap_or_else(|| panic!("missing retype oracle row `{name}`")),
            &expected_fields,
            "{name} diverged"
        );
    }
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 libavutil oracle under third_party/ffmpeg-oracle/wsl"]
fn libavutil_channel_layout_compare_vectors_match_current_model() {
    let repo_root = repo_root();
    let oracle_root = oracle_root(&repo_root);
    let include_dir = oracle_root.join("wsl/include");
    let libavutil = oracle_root.join("wsl/lib/libavutil.a");

    assert!(
        include_dir.join("libavutil/channel_layout.h").is_file(),
        "missing pinned FFmpeg libavutil channel layout headers under `{}`",
        include_dir.display()
    );
    assert!(
        libavutil.is_file(),
        "missing pinned FFmpeg libavutil static library `{}`",
        libavutil.display()
    );

    let work_dir = repo_root.join("target/oracle/avutil-channel-layout");
    fs::create_dir_all(&work_dir).expect("create avutil-channel-layout oracle work dir");
    let source = work_dir.join("channel_layout_compare_oracle.c");
    let executable = work_dir.join("channel_layout_compare_oracle");
    fs::write(&source, compare_oracle_c_source())
        .expect("write channel layout compare oracle C source");

    let stdout = compile_and_run_oracle(&include_dir, &libavutil, &source, &executable);
    let oracle = parse_parser_oracle_output(&stdout);

    assert_eq!(
        oracle.keys().collect::<Vec<_>>(),
        expected_compare_rows().keys().collect::<Vec<_>>(),
        "channel layout compare oracle row set diverged"
    );

    for (name, expected_fields) in expected_compare_rows() {
        assert_eq!(
            oracle
                .get(&name)
                .unwrap_or_else(|| panic!("missing compare oracle row `{name}`")),
            &expected_fields,
            "{name} diverged"
        );
    }
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 libavutil oracle under third_party/ffmpeg-oracle/wsl"]
fn libavutil_channel_layout_default_vectors_match_current_model() {
    let repo_root = repo_root();
    let oracle_root = oracle_root(&repo_root);
    let include_dir = oracle_root.join("wsl/include");
    let libavutil = oracle_root.join("wsl/lib/libavutil.a");

    assert!(
        include_dir.join("libavutil/channel_layout.h").is_file(),
        "missing pinned FFmpeg libavutil channel layout headers under `{}`",
        include_dir.display()
    );
    assert!(
        libavutil.is_file(),
        "missing pinned FFmpeg libavutil static library `{}`",
        libavutil.display()
    );

    let work_dir = repo_root.join("target/oracle/avutil-channel-layout");
    fs::create_dir_all(&work_dir).expect("create avutil-channel-layout oracle work dir");
    let source = work_dir.join("channel_layout_default_oracle.c");
    let executable = work_dir.join("channel_layout_default_oracle");
    fs::write(&source, default_oracle_c_source())
        .expect("write channel layout default oracle C source");

    let stdout = compile_and_run_oracle(&include_dir, &libavutil, &source, &executable);
    let oracle = parse_parser_oracle_output(&stdout);

    assert_eq!(
        oracle.keys().collect::<Vec<_>>(),
        expected_default_rows().keys().collect::<Vec<_>>(),
        "channel layout default oracle row set diverged"
    );

    for (name, expected_fields) in expected_default_rows() {
        assert_eq!(
            oracle
                .get(&name)
                .unwrap_or_else(|| panic!("missing default oracle row `{name}`")),
            &expected_fields,
            "{name} diverged"
        );
    }
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 source/build cache; set FFMPEG_FATE_BUILD_DIR or run scripts/bootstrap_ffmpeg_oracle_wsl.sh"]
fn upstream_fate_channel_layout_passes() {
    let output = if cfg!(windows) {
        let script = match env::var("FFMPEG_FATE_BUILD_DIR") {
            Ok(build_dir) => {
                let build_dir = if build_dir.starts_with('/') || build_dir.starts_with('~') {
                    build_dir
                } else {
                    to_wsl_path(Path::new(&build_dir))
                };
                format!(
                    "test -d {0} || {{ echo 'missing FFmpeg FATE build dir: {0}' >&2; exit 66; }}; make -C {0} fate-channel_layout",
                    shell_quote(&build_dir)
                )
            }
            Err(_) => concat!(
                "build_dir=\"${FFMPEGRUST_ORACLE_WORK:-$HOME/.cache/ffmpegrust/ffmpeg-oracle-n8.1.1}/build\"; ",
                "test -d \"$build_dir\" || { echo \"missing FFmpeg FATE build dir: $build_dir\" >&2; exit 66; }; ",
                "make -C \"$build_dir\" fate-channel_layout"
            )
            .to_string(),
        };
        Command::new("wsl")
            .args(["-d", "Ubuntu", "--exec", "bash", "-lc", &script])
            .output()
            .expect("run upstream FFmpeg fate-channel_layout through WSL")
    } else {
        let build_dir = env::var_os("FFMPEG_FATE_BUILD_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(env::var("HOME").expect("HOME must be set"))
                    .join(".cache/ffmpegrust/ffmpeg-oracle-n8.1.1/build")
            });
        Command::new("make")
            .arg("-C")
            .arg(&build_dir)
            .arg("fate-channel_layout")
            .output()
            .unwrap_or_else(|err| {
                panic!(
                    "run upstream FFmpeg fate-channel_layout in `{}`: {err}",
                    build_dir.display()
                )
            })
    };

    assert!(
        output.status.success(),
        "upstream FFmpeg fate-channel_layout failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn expected_channels() -> BTreeMap<String, String> {
    Channel::ALL
        .iter()
        .map(|channel| {
            (
                channel.name().to_string(),
                channel.description().to_string(),
            )
        })
        .collect()
}

fn expected_layouts() -> BTreeMap<String, String> {
    ChannelLayout::known_layouts()
        .into_iter()
        .map(|layout| (layout.name().to_string(), layout.channel_string()))
        .collect()
}

fn parse_layout_inventory(text: &str) -> LayoutInventory {
    let mut inventory = LayoutInventory::default();
    let mut section = LayoutSection::None;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if trimmed.starts_with("Individual channels:") {
            section = LayoutSection::Channels;
            continue;
        }
        if trimmed.starts_with("Standard channel layouts:") {
            section = LayoutSection::Layouts;
            continue;
        }
        if section == LayoutSection::None || trimmed.starts_with("NAME") {
            continue;
        }

        let Some((name, value)) = parse_inventory_entry(trimmed) else {
            continue;
        };

        let previous = match section {
            LayoutSection::Channels => inventory.channels.insert(name, value),
            LayoutSection::Layouts => inventory.layouts.insert(name, value),
            LayoutSection::None => unreachable!(),
        };
        assert!(previous.is_none(), "duplicate ffmpeg -layouts entry");
    }

    inventory
}

fn parse_inventory_entry(line: &str) -> Option<(String, String)> {
    let split_at = line.find(char::is_whitespace)?;
    let name = line[..split_at].to_string();
    let value = line[split_at..].trim().to_string();
    if value.is_empty() {
        None
    } else {
        Some((name, value))
    }
}

fn oracle_ffmpeg() -> PathBuf {
    let root = repo_root();

    if let Ok(path) = env::var("FFMPEG_ORACLE") {
        let path = PathBuf::from(path);
        let path = if path.is_absolute() {
            path
        } else {
            root.join(path)
        };
        assert!(
            path.is_file(),
            "FFMPEG_ORACLE must point to the pinned FFmpeg 8.1.1 binary, got `{}`",
            path.display()
        );
        return path;
    }

    for candidate in default_ffmpeg_candidates(&root) {
        if candidate.is_file() {
            return candidate;
        }
    }

    panic!(
        "missing pinned FFmpeg oracle; set FFMPEG_ORACLE or install `{}`",
        root.join("third_party/ffmpeg-oracle/build/bin/ffmpeg")
            .display()
    );
}

fn default_ffmpeg_candidates(root: &Path) -> Vec<PathBuf> {
    let bin = root.join("third_party/ffmpeg-oracle/build/bin");
    if cfg!(windows) {
        vec![
            bin.join("ffmpeg.exe"),
            bin.join("ffmpeg.cmd"),
            bin.join("ffmpeg"),
        ]
    } else {
        vec![
            bin.join("ffmpeg"),
            bin.join("ffmpeg.exe"),
            bin.join("ffmpeg.cmd"),
        ]
    }
}

fn expected_parser_rows() -> BTreeMap<String, Vec<String>> {
    let mut rows = PARSER_CASES
        .iter()
        .map(|case| {
            (
                format!("parse:{}", case.id),
                expected_parser_fields(case.input),
            )
        })
        .collect::<BTreeMap<_, _>>();
    rows.extend(BYTE_PARSER_CASES.iter().map(|case| {
        (
            format!("parse-bytes:{}", case.id),
            expected_byte_parser_fields(case.input),
        )
    }));
    rows
}

fn expected_parser_fields(input: &str) -> Vec<String> {
    match ChannelLayoutSpec::parse(input) {
        Ok(layout) => {
            let mut fields = vec!["ok".to_string()];
            fields.extend(layout_fields(&layout));
            fields
        }
        Err(err) => vec![
            "err".to_string(),
            err.code().unwrap_or(AvErrorCode::EINVAL).raw().to_string(),
        ],
    }
}

fn expected_byte_parser_fields(input: &[u8]) -> Vec<String> {
    match ChannelLayoutSpec::parse_bytes(input) {
        Ok(layout) => {
            let mut fields = vec!["ok".to_string()];
            fields.extend(layout_byte_fields(&layout));
            fields
        }
        Err(err) => vec![
            "err".to_string(),
            err.code().unwrap_or(AvErrorCode::EINVAL).raw().to_string(),
        ],
    }
}

fn expected_retype_rows() -> BTreeMap<String, Vec<String>> {
    RETYPE_CASES
        .iter()
        .map(|case| (format!("retype:{}", case.id), expected_retype_fields(case)))
        .collect()
}

fn expected_compare_rows() -> BTreeMap<String, Vec<String>> {
    COMPARE_CASES
        .iter()
        .map(|case| {
            (
                format!("compare:{}", case.id),
                expected_compare_fields(case),
            )
        })
        .collect()
}

fn expected_default_rows() -> BTreeMap<String, Vec<String>> {
    DEFAULT_CASES
        .iter()
        .map(|case| {
            (
                format!("default:{}", case.id),
                expected_default_fields(case),
            )
        })
        .collect()
}

fn expected_retype_fields(case: &RetypeCase) -> Vec<String> {
    let original = ChannelLayoutSpec::parse(case.input)
        .unwrap_or_else(|err| panic!("Rust parser rejected retype case `{}`: {err}", case.id));
    match retype_layout(&original, case) {
        Ok(result) => {
            let mut fields = vec![if result.is_lossy() {
                "lossy".to_string()
            } else {
                "lossless".to_string()
            }];
            fields.extend(layout_fields(result.layout()));
            fields
        }
        Err(err) => {
            let mut fields = vec![
                "err".to_string(),
                err.code().unwrap_or(AvErrorCode::EINVAL).raw().to_string(),
            ];
            fields.extend(layout_fields(&original));
            fields
        }
    }
}

fn expected_compare_fields(case: &CompareCase) -> Vec<String> {
    let left = ChannelLayoutSpec::parse(case.left)
        .unwrap_or_else(|err| panic!("Rust parser rejected compare left `{}`: {err}", case.id));
    let right = ChannelLayoutSpec::parse(case.right)
        .unwrap_or_else(|err| panic!("Rust parser rejected compare right `{}`: {err}", case.id));
    vec![if left.is_equivalent_to(right) {
        "equal".to_string()
    } else {
        "different".to_string()
    }]
}

fn expected_default_fields(case: &DefaultCase) -> Vec<String> {
    if case.channels <= 0 || case.channels > i32::from(u16::MAX) {
        return vec!["err".to_string(), AvErrorCode::EINVAL.raw().to_string()];
    }

    match ChannelLayoutSpec::default_for_count(case.channels as u16) {
        Ok(layout) => {
            let mut fields = vec!["ok".to_string()];
            fields.extend(layout_fields(&layout));
            fields
        }
        Err(err) => vec![
            "err".to_string(),
            err.code().unwrap_or(AvErrorCode::EINVAL).raw().to_string(),
        ],
    }
}

fn retype_layout(
    layout: &ChannelLayoutSpec,
    case: &RetypeCase,
) -> avutil::AvResult<ChannelLayoutRetypeResult> {
    if case.canonical {
        return layout.retype_to_canonical_order(case.allow_lossy);
    }

    match case.target {
        RetypeTarget::Native => layout.retype_to_native_order(case.allow_lossy),
        RetypeTarget::Custom => Ok(ChannelLayoutRetypeResult::new(
            ChannelLayoutSpec::Custom(layout.to_custom_layout()?),
            false,
        )),
        RetypeTarget::Unspecified => layout.retype_to_unspecified_order(case.allow_lossy),
        RetypeTarget::Ambisonic => layout.retype_to_ambisonic_order(case.allow_lossy),
    }
}

fn layout_fields(layout: &ChannelLayoutSpec) -> Vec<String> {
    let mut fields = vec![
        layout_order_name(layout).to_string(),
        layout.channel_count().to_string(),
        format!("{:016x}", layout_native_mask(layout)),
        layout.describe(),
        channel_sequence(layout),
        format!(
            "{:016x}",
            layout.subset_mask(ChannelLayout::stereo().channel_mask())
        ),
    ];
    fields.extend(
        LOOKUP_NAMES
            .iter()
            .map(|name| layout_index_from_string_field(layout, name)),
    );
    fields.extend(
        LOOKUP_NAMES
            .iter()
            .map(|name| layout_channel_from_string_field(layout, name)),
    );
    fields
}

fn layout_byte_fields(layout: &ChannelLayoutSpec) -> Vec<String> {
    let mut fields = vec![
        layout_order_name(layout).to_string(),
        layout.channel_count().to_string(),
        format!("{:016x}", layout_native_mask(layout)),
        bytes_to_hex(&layout.describe_bytes()),
        channel_sequence(layout),
        format!(
            "{:016x}",
            layout.subset_mask(ChannelLayout::stereo().channel_mask())
        ),
        custom_name_sequence_hex(layout),
    ];
    fields.extend(
        BYTE_LOOKUP_NAMES
            .iter()
            .map(|name| layout_index_from_string_bytes_field(layout, name)),
    );
    fields.extend(
        BYTE_LOOKUP_NAMES
            .iter()
            .map(|name| layout_channel_from_string_bytes_field(layout, name)),
    );
    fields
}

fn layout_order_name(layout: &ChannelLayoutSpec) -> &'static str {
    match layout {
        ChannelLayoutSpec::Native(_) | ChannelLayoutSpec::NativeMask(_) => "NATIVE",
        ChannelLayoutSpec::Ambisonic(_) => "AMBISONIC",
        ChannelLayoutSpec::Custom(_) => "CUSTOM",
        ChannelLayoutSpec::Unspecified(_) => "UNSPEC",
    }
}

fn layout_native_mask(layout: &ChannelLayoutSpec) -> u64 {
    match layout {
        ChannelLayoutSpec::Native(layout) => layout.channel_mask(),
        ChannelLayoutSpec::NativeMask(layout) => layout.channel_mask(),
        ChannelLayoutSpec::Ambisonic(layout) => layout.extra_native_mask(),
        ChannelLayoutSpec::Custom(_) | ChannelLayoutSpec::Unspecified(_) => 0,
    }
}

fn custom_name_sequence_hex(layout: &ChannelLayoutSpec) -> String {
    let Some(custom) = layout.as_custom() else {
        return "-".to_string();
    };
    let names = custom
        .channels()
        .iter()
        .map(|channel| {
            if channel.has_name() {
                bytes_to_hex(channel.name_bytes())
            } else {
                "-".to_string()
            }
        })
        .collect::<Vec<_>>();
    if names.is_empty() {
        "-".to_string()
    } else {
        names.join("+")
    }
}

fn channel_sequence(layout: &ChannelLayoutSpec) -> String {
    let channels = (0..usize::from(layout.channel_count()))
        .filter_map(|index| layout.channel_from_index(index))
        .map(channel_name)
        .collect::<Vec<_>>();
    if channels.is_empty() {
        "-".to_string()
    } else {
        channels.join("+")
    }
}

fn channel_name(channel: ChannelId) -> String {
    channel.name()
}

fn layout_index_from_string_field(layout: &ChannelLayoutSpec, name: &str) -> String {
    layout
        .index_from_string(name)
        .map(|index| index.to_string())
        .unwrap_or_else(|err| err.code().unwrap_or(AvErrorCode::EINVAL).raw().to_string())
}

fn layout_index_from_string_bytes_field(layout: &ChannelLayoutSpec, name: &[u8]) -> String {
    layout
        .index_from_string_bytes(name)
        .map(|index| index.to_string())
        .unwrap_or_else(|err| err.code().unwrap_or(AvErrorCode::EINVAL).raw().to_string())
}

fn layout_channel_from_string_field(layout: &ChannelLayoutSpec, name: &str) -> String {
    layout
        .channel_from_string(name)
        .map(ChannelId::raw_id)
        .unwrap_or(ChannelId::NONE_RAW)
        .to_string()
}

fn layout_channel_from_string_bytes_field(layout: &ChannelLayoutSpec, name: &[u8]) -> String {
    layout
        .channel_from_string_bytes(name)
        .map(ChannelId::raw_id)
        .unwrap_or(ChannelId::NONE_RAW)
        .to_string()
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

fn parse_parser_oracle_output(stdout: &str) -> BTreeMap<String, Vec<String>> {
    let mut rows = BTreeMap::new();
    for line in stdout.lines() {
        let columns = line.split('|').collect::<Vec<_>>();
        assert!(columns.len() >= 2, "malformed oracle row `{line}`");
        let previous = rows.insert(
            columns[0].to_string(),
            columns[1..].iter().map(|field| field.to_string()).collect(),
        );
        assert!(previous.is_none(), "duplicate oracle row `{}`", columns[0]);
    }
    assert!(!rows.is_empty(), "missing parser oracle rows");
    rows
}

fn compile_and_run_oracle(
    include_dir: &Path,
    libavutil: &Path,
    source: &Path,
    executable: &Path,
) -> String {
    let output = if cfg!(windows) {
        let script = format!(
            "gcc -I {} {} {} -lm -pthread -ldl -o {} && {}",
            shell_quote(&to_wsl_path(include_dir)),
            shell_quote(&to_wsl_path(source)),
            shell_quote(&to_wsl_path(libavutil)),
            shell_quote(&to_wsl_path(executable)),
            shell_quote(&to_wsl_path(executable))
        );
        Command::new("wsl")
            .args(["-d", "Ubuntu", "--exec", "bash", "-lc", &script])
            .output()
            .expect("run WSL libavutil channel layout parser oracle")
    } else {
        Command::new("sh")
            .arg("-c")
            .arg(format!(
                "gcc -I {} {} {} -lm -pthread -ldl -o {} && {}",
                shell_quote(&include_dir.display().to_string()),
                shell_quote(&source.display().to_string()),
                shell_quote(&libavutil.display().to_string()),
                shell_quote(&executable.display().to_string()),
                shell_quote(&executable.display().to_string())
            ))
            .output()
            .expect("run libavutil channel layout parser oracle")
    };

    assert!(
        output.status.success(),
        "libavutil channel layout parser oracle failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout).expect("oracle stdout should be UTF-8")
}

fn parser_oracle_c_source() -> String {
    let cases = PARSER_CASES
        .iter()
        .map(|case| {
            format!(
                "    {{ {}, {} }},",
                c_string_literal(case.id),
                c_string_literal(case.input)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let lookups = LOOKUP_NAMES
        .iter()
        .map(|name| format!("    {},", c_string_literal(name)))
        .collect::<Vec<_>>()
        .join("\n");
    let byte_arrays =
        BYTE_PARSER_CASES
            .iter()
            .enumerate()
            .map(|(index, case)| c_byte_array_definition(&format!("byte_case_{index}"), case.input))
            .chain(BYTE_LOOKUP_NAMES.iter().enumerate().map(|(index, name)| {
                c_byte_array_definition(&format!("byte_lookup_{index}"), name)
            }))
            .collect::<Vec<_>>()
            .join("\n");
    let byte_cases = BYTE_PARSER_CASES
        .iter()
        .enumerate()
        .map(|(index, case)| {
            format!(
                "    {{ {}, byte_case_{index} }},",
                c_string_literal(case.id)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let byte_lookups = BYTE_LOOKUP_NAMES
        .iter()
        .enumerate()
        .map(|(index, _)| format!("    byte_lookup_{index},"))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <libavutil/channel_layout.h>

struct parser_case {{
    const char *id;
    const char *input;
}};

struct byte_parser_case {{
    const char *id;
    const unsigned char *input;
}};

static const struct parser_case parser_cases[] = {{
{cases}
}};

static const char *lookup_names[] = {{
{lookups}
}};

{byte_arrays}

static const struct byte_parser_case byte_parser_cases[] = {{
{byte_cases}
}};

static const unsigned char *byte_lookup_names[] = {{
{byte_lookups}
}};

static const char *order_name(enum AVChannelOrder order) {{
    switch (order) {{
    case AV_CHANNEL_ORDER_UNSPEC:
        return "UNSPEC";
    case AV_CHANNEL_ORDER_NATIVE:
        return "NATIVE";
    case AV_CHANNEL_ORDER_CUSTOM:
        return "CUSTOM";
    case AV_CHANNEL_ORDER_AMBISONIC:
        return "AMBISONIC";
    default:
        return "UNKNOWN";
    }}
}}

static uint64_t comparable_mask(const AVChannelLayout *layout) {{
    if (layout->order == AV_CHANNEL_ORDER_NATIVE || layout->order == AV_CHANNEL_ORDER_AMBISONIC)
        return layout->u.mask;
    return 0;
}}

static void print_hex_bytes(const unsigned char *bytes, size_t len) {{
    for (size_t index = 0; index < len; index++)
        printf("%02x", bytes[index]);
}}

static void print_cstr_hex(const char *text) {{
    print_hex_bytes((const unsigned char *)text, strlen(text));
}}

static void print_custom_names_hex(const AVChannelLayout *layout) {{
    if (layout->order != AV_CHANNEL_ORDER_CUSTOM) {{
        putchar('-');
        return;
    }}

    for (int index = 0; index < layout->nb_channels; index++) {{
        if (index)
            putchar('+');
        size_t len = 0;
        while (len < 16 && layout->u.map[index].name[len] != 0)
            len++;
        if (len)
            print_hex_bytes((const unsigned char *)layout->u.map[index].name, len);
        else
            putchar('-');
    }}
}}

static void print_channel_sequence(const AVChannelLayout *layout) {{
    int printed = 0;
    for (int index = 0; index < layout->nb_channels; index++) {{
        enum AVChannel channel = av_channel_layout_channel_from_index(layout, index);
        if (channel == AV_CHAN_NONE)
            continue;

        char name[64];
        av_channel_name(name, sizeof(name), channel);
        if (printed)
            putchar('+');
        fputs(name, stdout);
        printed++;
    }}
    if (!printed)
        putchar('-');
}}

static void print_lookup_results(const AVChannelLayout *layout) {{
    for (size_t index = 0; index < sizeof(lookup_names) / sizeof(lookup_names[0]); index++) {{
        printf("|%d", av_channel_layout_index_from_string(layout, lookup_names[index]));
    }}
    for (size_t index = 0; index < sizeof(lookup_names) / sizeof(lookup_names[0]); index++) {{
        printf("|%d", av_channel_layout_channel_from_string(layout, lookup_names[index]));
    }}
}}

static void print_byte_lookup_results(const AVChannelLayout *layout) {{
    for (size_t index = 0; index < sizeof(byte_lookup_names) / sizeof(byte_lookup_names[0]); index++) {{
        printf("|%d", av_channel_layout_index_from_string(layout, (const char *)byte_lookup_names[index]));
    }}
    for (size_t index = 0; index < sizeof(byte_lookup_names) / sizeof(byte_lookup_names[0]); index++) {{
        printf("|%d", av_channel_layout_channel_from_string(layout, (const char *)byte_lookup_names[index]));
    }}
}}

static void print_parse_case(const struct parser_case *test_case) {{
    AVChannelLayout layout = {{0}};
    int ret = av_channel_layout_from_string(&layout, test_case->input);
    printf("parse:%s|", test_case->id);
    if (ret < 0) {{
        printf("err|%d\n", ret);
        return;
    }}

    char description[512];
    ret = av_channel_layout_describe(&layout, description, sizeof(description));
    if (ret < 0)
        snprintf(description, sizeof(description), "<describe-error:%d>", ret);

    printf(
        "ok|%s|%d|%016" PRIx64 "|%s|",
        order_name(layout.order),
        layout.nb_channels,
        comparable_mask(&layout),
        description
    );
    print_channel_sequence(&layout);
    printf("|%016" PRIx64, av_channel_layout_subset(&layout, AV_CH_LAYOUT_STEREO));
    print_lookup_results(&layout);
    putchar('\n');

    av_channel_layout_uninit(&layout);
}}

static void print_byte_parse_case(const struct byte_parser_case *test_case) {{
    AVChannelLayout layout = {{0}};
    int ret = av_channel_layout_from_string(&layout, (const char *)test_case->input);
    printf("parse-bytes:%s|", test_case->id);
    if (ret < 0) {{
        printf("err|%d\n", ret);
        return;
    }}

    char description[512];
    ret = av_channel_layout_describe(&layout, description, sizeof(description));
    if (ret < 0)
        snprintf(description, sizeof(description), "<describe-error:%d>", ret);

    printf(
        "ok|%s|%d|%016" PRIx64 "|",
        order_name(layout.order),
        layout.nb_channels,
        comparable_mask(&layout)
    );
    print_cstr_hex(description);
    putchar('|');
    print_channel_sequence(&layout);
    printf("|%016" PRIx64 "|", av_channel_layout_subset(&layout, AV_CH_LAYOUT_STEREO));
    print_custom_names_hex(&layout);
    print_byte_lookup_results(&layout);
    putchar('\n');

    av_channel_layout_uninit(&layout);
}}

int main(void) {{
    for (size_t index = 0; index < sizeof(parser_cases) / sizeof(parser_cases[0]); index++)
        print_parse_case(&parser_cases[index]);
    for (size_t index = 0; index < sizeof(byte_parser_cases) / sizeof(byte_parser_cases[0]); index++)
        print_byte_parse_case(&byte_parser_cases[index]);
    return 0;
}}
"#
    )
}

fn compare_oracle_c_source() -> String {
    let cases = COMPARE_CASES
        .iter()
        .map(|case| {
            format!(
                "    {{ {}, {}, {} }},",
                c_string_literal(case.id),
                c_string_literal(case.left),
                c_string_literal(case.right)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"#include <stdio.h>
#include <libavutil/channel_layout.h>

struct compare_case {{
    const char *id;
    const char *left;
    const char *right;
}};

static const struct compare_case compare_cases[] = {{
{cases}
}};

static void print_compare_case(const struct compare_case *test_case) {{
    AVChannelLayout left = {{0}};
    AVChannelLayout right = {{0}};
    int left_ret = av_channel_layout_from_string(&left, test_case->left);
    int right_ret = av_channel_layout_from_string(&right, test_case->right);

    printf("compare:%s|", test_case->id);
    if (left_ret < 0 || right_ret < 0) {{
        printf("parseerr|%d|%d\n", left_ret, right_ret);
        av_channel_layout_uninit(&left);
        av_channel_layout_uninit(&right);
        return;
    }}

    int ret = av_channel_layout_compare(&left, &right);
    printf("%s\n", ret == 0 ? "equal" : "different");

    av_channel_layout_uninit(&left);
    av_channel_layout_uninit(&right);
}}

int main(void) {{
    for (size_t index = 0; index < sizeof(compare_cases) / sizeof(compare_cases[0]); index++)
        print_compare_case(&compare_cases[index]);
    return 0;
}}
"#
    )
}

fn default_oracle_c_source() -> String {
    let cases = DEFAULT_CASES
        .iter()
        .map(|case| {
            format!(
                "    {{ {}, {} }},",
                c_string_literal(case.id),
                case.channels
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let lookups = LOOKUP_NAMES
        .iter()
        .map(|name| format!("    {},", c_string_literal(name)))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"#include <errno.h>
#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>
#include <libavutil/channel_layout.h>
#include <libavutil/error.h>

struct default_case {{
    const char *id;
    int channels;
}};

static const struct default_case default_cases[] = {{
{cases}
}};

static const char *lookup_names[] = {{
{lookups}
}};

static const char *order_name(enum AVChannelOrder order) {{
    switch (order) {{
    case AV_CHANNEL_ORDER_UNSPEC:
        return "UNSPEC";
    case AV_CHANNEL_ORDER_NATIVE:
        return "NATIVE";
    case AV_CHANNEL_ORDER_CUSTOM:
        return "CUSTOM";
    case AV_CHANNEL_ORDER_AMBISONIC:
        return "AMBISONIC";
    default:
        return "UNKNOWN";
    }}
}}

static uint64_t comparable_mask(const AVChannelLayout *layout) {{
    if (layout->order == AV_CHANNEL_ORDER_NATIVE || layout->order == AV_CHANNEL_ORDER_AMBISONIC)
        return layout->u.mask;
    return 0;
}}

static void print_channel_sequence(const AVChannelLayout *layout) {{
    int printed = 0;
    for (int index = 0; index < layout->nb_channels; index++) {{
        enum AVChannel channel = av_channel_layout_channel_from_index(layout, index);
        if (channel == AV_CHAN_NONE)
            continue;

        char name[64];
        av_channel_name(name, sizeof(name), channel);
        if (printed)
            putchar('+');
        fputs(name, stdout);
        printed++;
    }}
    if (!printed)
        putchar('-');
}}

static void print_lookup_results(const AVChannelLayout *layout) {{
    for (size_t index = 0; index < sizeof(lookup_names) / sizeof(lookup_names[0]); index++) {{
        printf("|%d", av_channel_layout_index_from_string(layout, lookup_names[index]));
    }}
    for (size_t index = 0; index < sizeof(lookup_names) / sizeof(lookup_names[0]); index++) {{
        printf("|%d", av_channel_layout_channel_from_string(layout, lookup_names[index]));
    }}
}}

static void print_layout_fields(const AVChannelLayout *layout) {{
    char description[512];
    int ret = av_channel_layout_describe(layout, description, sizeof(description));
    if (ret < 0)
        snprintf(description, sizeof(description), "<describe-error:%d>", ret);

    printf(
        "%s|%d|%016" PRIx64 "|%s|",
        order_name(layout->order),
        layout->nb_channels,
        comparable_mask(layout),
        description
    );
    print_channel_sequence(layout);
    printf("|%016" PRIx64, av_channel_layout_subset(layout, AV_CH_LAYOUT_STEREO));
    print_lookup_results(layout);
}}

static void print_default_case(const struct default_case *test_case) {{
    AVChannelLayout layout = {{0}};
    av_channel_layout_default(&layout, test_case->channels);

    printf("default:%s|", test_case->id);
    if (!av_channel_layout_check(&layout)) {{
        printf("err|%d\n", AVERROR(EINVAL));
        av_channel_layout_uninit(&layout);
        return;
    }}

    printf("ok|");
    print_layout_fields(&layout);
    putchar('\n');

    av_channel_layout_uninit(&layout);
}}

int main(void) {{
    for (size_t index = 0; index < sizeof(default_cases) / sizeof(default_cases[0]); index++)
        print_default_case(&default_cases[index]);
    return 0;
}}
"#
    )
}

fn retype_oracle_c_source() -> String {
    let cases = RETYPE_CASES
        .iter()
        .map(|case| {
            format!(
                "    {{ {}, {}, {}, {} }},",
                c_string_literal(case.id),
                c_string_literal(case.input),
                c_retype_target(case.target),
                c_retype_flags(case)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let lookups = LOOKUP_NAMES
        .iter()
        .map(|name| format!("    {},", c_string_literal(name)))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>
#include <libavutil/channel_layout.h>

struct retype_case {{
    const char *id;
    const char *input;
    enum AVChannelOrder target_order;
    int flags;
}};

static const struct retype_case retype_cases[] = {{
{cases}
}};

static const char *lookup_names[] = {{
{lookups}
}};

static const char *order_name(enum AVChannelOrder order) {{
    switch (order) {{
    case AV_CHANNEL_ORDER_UNSPEC:
        return "UNSPEC";
    case AV_CHANNEL_ORDER_NATIVE:
        return "NATIVE";
    case AV_CHANNEL_ORDER_CUSTOM:
        return "CUSTOM";
    case AV_CHANNEL_ORDER_AMBISONIC:
        return "AMBISONIC";
    default:
        return "UNKNOWN";
    }}
}}

static uint64_t comparable_mask(const AVChannelLayout *layout) {{
    if (layout->order == AV_CHANNEL_ORDER_NATIVE || layout->order == AV_CHANNEL_ORDER_AMBISONIC)
        return layout->u.mask;
    return 0;
}}

static void print_channel_sequence(const AVChannelLayout *layout) {{
    int printed = 0;
    for (int index = 0; index < layout->nb_channels; index++) {{
        enum AVChannel channel = av_channel_layout_channel_from_index(layout, index);
        if (channel == AV_CHAN_NONE)
            continue;

        char name[64];
        av_channel_name(name, sizeof(name), channel);
        if (printed)
            putchar('+');
        fputs(name, stdout);
        printed++;
    }}
    if (!printed)
        putchar('-');
}}

static void print_lookup_results(const AVChannelLayout *layout) {{
    for (size_t index = 0; index < sizeof(lookup_names) / sizeof(lookup_names[0]); index++) {{
        printf("|%d", av_channel_layout_index_from_string(layout, lookup_names[index]));
    }}
    for (size_t index = 0; index < sizeof(lookup_names) / sizeof(lookup_names[0]); index++) {{
        printf("|%d", av_channel_layout_channel_from_string(layout, lookup_names[index]));
    }}
}}

static void print_layout_fields(const AVChannelLayout *layout) {{
    char description[512];
    int ret = av_channel_layout_describe(layout, description, sizeof(description));
    if (ret < 0)
        snprintf(description, sizeof(description), "<describe-error:%d>", ret);

    printf(
        "%s|%d|%016" PRIx64 "|%s|",
        order_name(layout->order),
        layout->nb_channels,
        comparable_mask(layout),
        description
    );
    print_channel_sequence(layout);
    printf("|%016" PRIx64, av_channel_layout_subset(layout, AV_CH_LAYOUT_STEREO));
    print_lookup_results(layout);
}}

static void print_retype_case(const struct retype_case *test_case) {{
    AVChannelLayout layout = {{0}};
    int ret = av_channel_layout_from_string(&layout, test_case->input);
    printf("retype:%s|", test_case->id);
    if (ret < 0) {{
        printf("parseerr|%d\n", ret);
        return;
    }}

    ret = av_channel_layout_retype(&layout, test_case->target_order, test_case->flags);
    if (ret < 0) {{
        printf("err|%d|", ret);
        print_layout_fields(&layout);
        putchar('\n');
        av_channel_layout_uninit(&layout);
        return;
    }}

    printf("%s|", ret > 0 ? "lossy" : "lossless");
    print_layout_fields(&layout);
    putchar('\n');

    av_channel_layout_uninit(&layout);
}}

int main(void) {{
    for (size_t index = 0; index < sizeof(retype_cases) / sizeof(retype_cases[0]); index++)
        print_retype_case(&retype_cases[index]);
    return 0;
}}
"#
    )
}

fn c_retype_target(target: RetypeTarget) -> &'static str {
    match target {
        RetypeTarget::Native => "AV_CHANNEL_ORDER_NATIVE",
        RetypeTarget::Custom => "AV_CHANNEL_ORDER_CUSTOM",
        RetypeTarget::Unspecified => "AV_CHANNEL_ORDER_UNSPEC",
        RetypeTarget::Ambisonic => "AV_CHANNEL_ORDER_AMBISONIC",
    }
}

fn c_retype_flags(case: &RetypeCase) -> String {
    let mut flags = Vec::new();
    if !case.allow_lossy {
        flags.push("AV_CHANNEL_LAYOUT_RETYPE_FLAG_LOSSLESS");
    }
    if case.canonical {
        flags.push("AV_CHANNEL_LAYOUT_RETYPE_FLAG_CANONICAL");
    }
    if flags.is_empty() {
        "0".to_string()
    } else {
        flags.join(" | ")
    }
}

fn c_string_literal(value: &str) -> String {
    let mut output = String::from("\"");
    for byte in value.bytes() {
        match byte {
            b'\\' => output.push_str("\\\\"),
            b'"' => output.push_str("\\\""),
            b'\n' => output.push_str("\\n"),
            b'\r' => output.push_str("\\r"),
            b'\t' => output.push_str("\\t"),
            0x20..=0x7e => output.push(byte as char),
            _ => output.push_str(&format!("\\x{byte:02x}")),
        }
    }
    output.push('"');
    output
}

fn c_byte_array_definition(name: &str, value: &[u8]) -> String {
    let bytes = value
        .iter()
        .map(|byte| format!("0x{byte:02x}"))
        .chain(std::iter::once("0x00".to_string()))
        .collect::<Vec<_>>()
        .join(", ");
    format!("static const unsigned char {name}[] = {{ {bytes} }};")
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/avutil should have a repo root grandparent")
        .to_path_buf()
}

fn oracle_root(repo_root: &Path) -> PathBuf {
    let default_root = repo_root.join("third_party/ffmpeg-oracle");
    if let Ok(ffmpeg) = env::var("FFMPEG_ORACLE") {
        let ffmpeg = PathBuf::from(ffmpeg);
        let ffmpeg = if ffmpeg.is_absolute() {
            ffmpeg
        } else {
            repo_root.join(ffmpeg)
        };
        if let Some(root) = ffmpeg.ancestors().find(|ancestor| {
            ancestor
                .file_name()
                .is_some_and(|name| name == "ffmpeg-oracle")
        }) {
            return root.to_path_buf();
        }
    }
    default_root
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(windows)]
fn to_wsl_path(path: &Path) -> String {
    let absolute = absolute_path(path);
    let mut text = absolute.to_string_lossy().replace('\\', "/");
    if let Some(stripped) = text.strip_prefix("//?/") {
        text = stripped.to_string();
    }
    let bytes = text.as_bytes();
    if bytes.len() >= 3 && bytes[1] == b':' && bytes[2] == b'/' {
        let drive = (bytes[0] as char).to_ascii_lowercase();
        text.replace_range(0..3, &format!("/mnt/{drive}/"));
    }
    text
}

#[cfg(windows)]
fn absolute_path(path: &Path) -> PathBuf {
    if path.exists() {
        return path.canonicalize().unwrap_or_else(|err| {
            panic!(
                "failed to canonicalize existing path `{}`: {err}",
                path.display()
            )
        });
    }
    let parent = path
        .parent()
        .unwrap_or_else(|| panic!("path `{}` has no parent", path.display()))
        .canonicalize()
        .unwrap_or_else(|err| {
            panic!(
                "failed to canonicalize parent of `{}`: {err}",
                path.display()
            )
        });
    parent.join(
        path.file_name()
            .unwrap_or_else(|| panic!("path `{}` has no file name", path.display())),
    )
}

#[cfg(not(windows))]
fn to_wsl_path(path: &Path) -> String {
    path.display().to_string()
}
