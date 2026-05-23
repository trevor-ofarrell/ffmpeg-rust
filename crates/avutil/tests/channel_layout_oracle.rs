use avutil::{AvErrorCode, Channel, ChannelId, ChannelLayout, ChannelLayoutSpec};
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

const PARSER_CASES: &[ParserCase] = &[
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
        id: "described-native",
        input: "2 channels (FL+FR)",
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
        id: "repeated-at-name",
        input: "FL@Left@Again",
    },
    ParserCase {
        id: "duplicate-native-custom",
        input: "FL+FL",
    },
    ParserCase {
        id: "trailing-separator",
        input: "FL+",
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
        id: "zero-order-extra",
        input: "ambisonic +stereo",
    },
    ParserCase {
        id: "raw-extra-ambisonic",
        input: "ambisonic 1+0x200000000000",
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
        id: "invalid-uppercase-layout",
        input: "STEREO",
    },
];

const LOOKUP_NAMES: &[&str] = &[
    "FL", "FR", "FC", "AMBI0", "AMBI3", "USR45", "@Left", "FL@Left",
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
    PARSER_CASES
        .iter()
        .map(|case| {
            (
                format!("parse:{}", case.id),
                expected_parser_fields(case.input),
            )
        })
        .collect()
}

fn expected_parser_fields(input: &str) -> Vec<String> {
    match ChannelLayoutSpec::parse(input) {
        Ok(layout) => {
            let mut fields = vec![
                "ok".to_string(),
                layout_order_name(&layout).to_string(),
                layout.channel_count().to_string(),
                format!("{:016x}", layout_native_mask(&layout)),
                layout.describe(),
                channel_sequence(&layout),
                format!(
                    "{:016x}",
                    layout.subset_mask(ChannelLayout::stereo().channel_mask())
                ),
            ];
            fields.extend(
                LOOKUP_NAMES
                    .iter()
                    .map(|name| layout_index_from_string_field(&layout, name)),
            );
            fields
        }
        Err(err) => vec![
            "err".to_string(),
            err.code().unwrap_or(AvErrorCode::EINVAL).raw().to_string(),
        ],
    }
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

    format!(
        r#"#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>
#include <libavutil/channel_layout.h>

struct parser_case {{
    const char *id;
    const char *input;
}};

static const struct parser_case parser_cases[] = {{
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

int main(void) {{
    for (size_t index = 0; index < sizeof(parser_cases) / sizeof(parser_cases[0]); index++)
        print_parse_case(&parser_cases[index]);
    return 0;
}}
"#
    )
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
