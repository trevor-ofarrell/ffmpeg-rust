use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

const DEFAULT_FATE_MAPPINGS_PATH: &str = "tests/fate/mappings.txt";
#[cfg(test)]
const UPSTREAM_FATE_MAPPINGS_PATH: &str = "tests/fate/upstream-mappings.txt";
const SAMPLES_ENV_VARS: &[&str] = &["FATE_SAMPLES", "SAMPLES"];
const DEFAULT_SAMPLES_ROOT_CANDIDATES: &[&str] = &[
    "third_party/fate-samples",
    "third_party/fate-suite",
    "fate-suite",
];
const ORACLE_FFMPEG_ENV_VARS: &[&str] = &["FFMPEG_ORACLE"];
const DEFAULT_ORACLE_FFMPEG_CANDIDATES: &[&str] = &[
    "third_party/ffmpeg-oracle/build/bin/ffmpeg.exe",
    "third_party/ffmpeg-oracle/build/bin/ffmpeg.cmd",
    "third_party/ffmpeg-oracle/build/bin/ffmpeg",
];
const SUPPORTED_PLACEHOLDERS: &[&str] = &["samples", "oracle_ffmpeg"];

struct PathRule {
    path: &'static str,
    exact_ids: &'static [&'static str],
    id_prefixes: &'static [&'static str],
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FateMapping {
    component_id: String,
    target: String,
    workdir: String,
    program: String,
    env: Vec<(String, String)>,
    args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RunOptions {
    mode: RunMode,
    mappings_path: String,
    target_filters: Vec<String>,
    context: FateContext,
    execution_mode: ExecutionMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MappingOptions {
    mappings_path: String,
    target_filters: Vec<String>,
    context: FateContext,
    check_prerequisites: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RunMode {
    Components(Vec<String>),
    Changed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExecutionMode {
    Execute,
    DryRun,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct FateContext {
    samples_root: Option<String>,
    oracle_ffmpeg: Option<String>,
}

const PATH_RULES: &[PathRule] = &[
    PathRule {
        path: "Cargo.toml",
        exact_ids: &["repo-runtime-guard"],
        id_prefixes: &[],
    },
    PathRule {
        path: "Cargo.lock",
        exact_ids: &["repo-runtime-guard"],
        id_prefixes: &[],
    },
    PathRule {
        path: "crates/avcodec/Cargo.toml",
        exact_ids: &["repo-runtime-guard"],
        id_prefixes: &["avcodec-"],
    },
    PathRule {
        path: "crates/avdevice/Cargo.toml",
        exact_ids: &["repo-runtime-guard"],
        id_prefixes: &["avdevice-"],
    },
    PathRule {
        path: "crates/avfilter/Cargo.toml",
        exact_ids: &["repo-runtime-guard"],
        id_prefixes: &["avfilter-"],
    },
    PathRule {
        path: "crates/avformat/Cargo.toml",
        exact_ids: &["repo-runtime-guard"],
        id_prefixes: &["avformat-"],
    },
    PathRule {
        path: "crates/avutil/Cargo.toml",
        exact_ids: &["repo-runtime-guard"],
        id_prefixes: &["avutil-"],
    },
    PathRule {
        path: "crates/fftools/Cargo.toml",
        exact_ids: &[
            "fftools-version",
            "fftools-hide-banner",
            "fftools-basic-io",
            "repo-runtime-guard",
        ],
        id_prefixes: &["fftools-ffmpeg-", "fftools-ffprobe-"],
    },
    PathRule {
        path: "crates/swresample/Cargo.toml",
        exact_ids: &["repo-runtime-guard"],
        id_prefixes: &["swresample-"],
    },
    PathRule {
        path: "crates/swscale/Cargo.toml",
        exact_ids: &["repo-runtime-guard"],
        id_prefixes: &["swscale-"],
    },
    PathRule {
        path: "fuzz/Cargo.toml",
        exact_ids: &["repo-runtime-guard"],
        id_prefixes: &[],
    },
    PathRule {
        path: "xtask/Cargo.toml",
        exact_ids: &["repo-runtime-guard"],
        id_prefixes: &[],
    },
    PathRule {
        path: "crates/fate-runner/Cargo.toml",
        exact_ids: &["fate-runner", "repo-runtime-guard"],
        id_prefixes: &[],
    },
    PathRule {
        path: "crates/oracle/Cargo.toml",
        exact_ids: &["oracle-inventory", "repo-runtime-guard"],
        id_prefixes: &[],
    },
    PathRule {
        path: "crates/fate-runner/",
        exact_ids: &["fate-runner"],
        id_prefixes: &[],
    },
    PathRule {
        path: "tests/fate/",
        exact_ids: &["fate-runner"],
        id_prefixes: &[],
    },
    PathRule {
        path: "tests/differential/",
        exact_ids: &["fate-runner"],
        id_prefixes: &[],
    },
    PathRule {
        path: "xtask/",
        exact_ids: &["repo-runtime-guard"],
        id_prefixes: &[],
    },
    PathRule {
        path: "crates/oracle/",
        exact_ids: &["oracle-inventory"],
        id_prefixes: &[],
    },
    PathRule {
        path: "crates/avutil/src/lib.rs",
        exact_ids: &[],
        id_prefixes: &["avutil-"],
    },
    PathRule {
        path: "crates/avutil/src/error.rs",
        exact_ids: &["avutil-error"],
        id_prefixes: &[],
    },
    PathRule {
        path: "crates/avutil/tests/error_oracle.rs",
        exact_ids: &["avutil-error"],
        id_prefixes: &[],
    },
    PathRule {
        path: "crates/avutil/src/rational.rs",
        exact_ids: &["avutil-rational"],
        id_prefixes: &[],
    },
    PathRule {
        path: "crates/avutil/tests/rational_oracle.rs",
        exact_ids: &["avutil-rational"],
        id_prefixes: &[],
    },
    PathRule {
        path: "crates/avutil/src/timebase.rs",
        exact_ids: &["avutil-timebase"],
        id_prefixes: &[],
    },
    PathRule {
        path: "crates/avutil/tests/timebase_oracle.rs",
        exact_ids: &["avutil-timebase"],
        id_prefixes: &[],
    },
    PathRule {
        path: "crates/avutil/src/packet.rs",
        exact_ids: &["avutil-packet"],
        id_prefixes: &[],
    },
    PathRule {
        path: "crates/avutil/src/buffer.rs",
        exact_ids: &["avutil-buffer"],
        id_prefixes: &[],
    },
    PathRule {
        path: "crates/avutil/src/frame.rs",
        exact_ids: &["avutil-frame"],
        id_prefixes: &[],
    },
    PathRule {
        path: "crates/avutil/src/logging.rs",
        exact_ids: &["avutil-logging"],
        id_prefixes: &[],
    },
    PathRule {
        path: "crates/avutil/src/byteio.rs",
        exact_ids: &["avutil-byteio"],
        id_prefixes: &[],
    },
    PathRule {
        path: "crates/avutil/src/bitreader.rs",
        exact_ids: &["avutil-bitreader"],
        id_prefixes: &[],
    },
    PathRule {
        path: "crates/avutil/src/bitwriter.rs",
        exact_ids: &["avutil-bitwriter"],
        id_prefixes: &[],
    },
    PathRule {
        path: "crates/avutil/src/dict.rs",
        exact_ids: &["avutil-dict"],
        id_prefixes: &[],
    },
    PathRule {
        path: "crates/avutil/src/options.rs",
        exact_ids: &["avutil-options"],
        id_prefixes: &[],
    },
    PathRule {
        path: "crates/avutil/src/pixel.rs",
        exact_ids: &["avutil-pixel-format"],
        id_prefixes: &[],
    },
    PathRule {
        path: "crates/avutil/tests/pixel_format_oracle.rs",
        exact_ids: &["avutil-pixel-format"],
        id_prefixes: &[],
    },
    PathRule {
        path: "crates/avutil/src/samplefmt.rs",
        exact_ids: &["avutil-sample-format"],
        id_prefixes: &[],
    },
    PathRule {
        path: "crates/avutil/tests/sample_format_oracle.rs",
        exact_ids: &["avutil-sample-format"],
        id_prefixes: &[],
    },
    PathRule {
        path: "crates/avutil/src/channel_layout.rs",
        exact_ids: &["avutil-channel-layout"],
        id_prefixes: &[],
    },
    PathRule {
        path: "crates/avutil/tests/channel_layout_oracle.rs",
        exact_ids: &["avutil-channel-layout"],
        id_prefixes: &[],
    },
    PathRule {
        path: "crates/avutil/src/color.rs",
        exact_ids: &["avutil-color"],
        id_prefixes: &[],
    },
    PathRule {
        path: "crates/avutil/tests/color_oracle.rs",
        exact_ids: &["avutil-color"],
        id_prefixes: &[],
    },
    PathRule {
        path: "crates/avutil/src/hash.rs",
        exact_ids: &["avutil-hash"],
        id_prefixes: &[],
    },
    PathRule {
        path: "crates/avutil/tests/hash_oracle.rs",
        exact_ids: &["avutil-hash"],
        id_prefixes: &[],
    },
    PathRule {
        path: "crates/avcodec/src/lib.rs",
        exact_ids: &[],
        id_prefixes: &["avcodec-"],
    },
    PathRule {
        path: "crates/avcodec/src/rawvideo.rs",
        exact_ids: &["avcodec-rawvideo"],
        id_prefixes: &[],
    },
    PathRule {
        path: "crates/avcodec/src/pcm.rs",
        exact_ids: &["avcodec-pcm-s16le"],
        id_prefixes: &[],
    },
    PathRule {
        path: "crates/avformat/src/lib.rs",
        exact_ids: &[],
        id_prefixes: &["avformat-"],
    },
    PathRule {
        path: "crates/avformat/src/audio.rs",
        exact_ids: &["avformat-audio-parameters"],
        id_prefixes: &["avformat-pcm-s16le-", "avformat-wav-"],
    },
    PathRule {
        path: "crates/avformat/src/video.rs",
        exact_ids: &["avformat-video-parameters"],
        id_prefixes: &[
            "avformat-avi-muxer",
            "avformat-rawvideo-",
            "avformat-yuv4mpegpipe-",
        ],
    },
    PathRule {
        path: "crates/avformat/src/avio.rs",
        exact_ids: &["avformat-avio"],
        id_prefixes: &[],
    },
    PathRule {
        path: "crates/avformat/src/probe.rs",
        exact_ids: &["avformat-probe"],
        id_prefixes: &[],
    },
    PathRule {
        path: "crates/avformat/src/null_muxer.rs",
        exact_ids: &["avformat-null-muxer"],
        id_prefixes: &[],
    },
    PathRule {
        path: "crates/avformat/src/hash_muxer.rs",
        exact_ids: &["avformat-hash-muxer"],
        id_prefixes: &[],
    },
    PathRule {
        path: "crates/avformat/src/framecrc_muxer.rs",
        exact_ids: &["avformat-framecrc-muxer"],
        id_prefixes: &[],
    },
    PathRule {
        path: "crates/avformat/src/wav.rs",
        exact_ids: &[],
        id_prefixes: &["avformat-wav-"],
    },
    PathRule {
        path: "crates/avformat/src/pcm.rs",
        exact_ids: &[],
        id_prefixes: &["avformat-pcm-s16le-"],
    },
    PathRule {
        path: "crates/avformat/src/rawvideo.rs",
        exact_ids: &[],
        id_prefixes: &["avformat-rawvideo-"],
    },
    PathRule {
        path: "crates/avformat/src/yuv4mpegpipe.rs",
        exact_ids: &[],
        id_prefixes: &["avformat-yuv4mpegpipe-"],
    },
    PathRule {
        path: "crates/avformat/src/image2.rs",
        exact_ids: &[],
        id_prefixes: &["avformat-image2-"],
    },
    PathRule {
        path: "crates/avformat/src/avi.rs",
        exact_ids: &[],
        id_prefixes: &["avformat-avi-"],
    },
    PathRule {
        path: "crates/avformat/src/mov.rs",
        exact_ids: &[],
        id_prefixes: &["avformat-mov-"],
    },
    PathRule {
        path: "crates/fftools/src/lib.rs",
        exact_ids: &["fftools-version", "fftools-hide-banner", "fftools-basic-io"],
        id_prefixes: &["fftools-ffmpeg-", "fftools-ffprobe-"],
    },
    PathRule {
        path: "crates/fftools/src/bin/",
        exact_ids: &["fftools-version", "fftools-hide-banner"],
        id_prefixes: &["fftools-ffmpeg-", "fftools-ffprobe-"],
    },
    PathRule {
        path: "crates/fftools/tests/version_oracle.rs",
        exact_ids: &["fftools-version"],
        id_prefixes: &[],
    },
    PathRule {
        path: "crates/fftools/src/option_parser.rs",
        exact_ids: &["fftools-option-parser"],
        id_prefixes: &[],
    },
    PathRule {
        path: "crates/fftools/src/cli_logging.rs",
        exact_ids: &["fftools-option-parser"],
        id_prefixes: &[],
    },
    PathRule {
        path: "crates/fftools/src/io_plan.rs",
        exact_ids: &["fftools-basic-io"],
        id_prefixes: &[],
    },
    PathRule {
        path: "crates/fftools/src/ffmpeg.rs",
        exact_ids: &["fftools-basic-io"],
        id_prefixes: &["fftools-ffmpeg-"],
    },
    PathRule {
        path: "crates/fftools/tests/rawvideo_oracle.rs",
        exact_ids: &[
            "fftools-ffmpeg-rawvideo-file-output",
            "avformat-rawvideo-demuxer",
            "avformat-rawvideo-muxer",
        ],
        id_prefixes: &[],
    },
    PathRule {
        path: "crates/fftools/tests/wav_oracle.rs",
        exact_ids: &[
            "avformat-wav-demuxer",
            "fftools-ffmpeg-md5-output",
            "fftools-ffmpeg-wav-framecrc-null",
        ],
        id_prefixes: &[],
    },
    PathRule {
        path: "crates/fftools/src/ffprobe.rs",
        exact_ids: &["fftools-basic-io"],
        id_prefixes: &["fftools-ffprobe-"],
    },
    PathRule {
        path: "fuzz/fuzz_targets/avcodec_basic_decoders.rs",
        exact_ids: &["avcodec-rawvideo", "avcodec-pcm-s16le"],
        id_prefixes: &[],
    },
    PathRule {
        path: "fuzz/fuzz_targets/avutil_byteio.rs",
        exact_ids: &["avutil-byteio"],
        id_prefixes: &[],
    },
    PathRule {
        path: "fuzz/fuzz_targets/avutil_bitreader.rs",
        exact_ids: &["avutil-bitreader", "avutil-bitwriter"],
        id_prefixes: &[],
    },
    PathRule {
        path: "fuzz/fuzz_targets/avutil_metadata_options.rs",
        exact_ids: &["avutil-dict", "avutil-options"],
        id_prefixes: &[],
    },
    PathRule {
        path: "fuzz/fuzz_targets/avutil_core_models.rs",
        exact_ids: &[
            "avutil-error",
            "avutil-rational",
            "avutil-timebase",
            "avutil-packet",
            "avutil-buffer",
            "avutil-frame",
            "avutil-pixel-format",
            "avutil-sample-format",
            "avutil-channel-layout",
            "avutil-color",
            "avutil-hash",
        ],
        id_prefixes: &[],
    },
    PathRule {
        path: "fuzz/fuzz_targets/avformat_probe.rs",
        exact_ids: &["avformat-probe"],
        id_prefixes: &[],
    },
    PathRule {
        path: "fuzz/fuzz_targets/avformat_wav.rs",
        exact_ids: &["avformat-wav-demuxer"],
        id_prefixes: &[],
    },
    PathRule {
        path: "fuzz/fuzz_targets/avformat_yuv4mpegpipe.rs",
        exact_ids: &["avformat-yuv4mpegpipe-demuxer"],
        id_prefixes: &[],
    },
    PathRule {
        path: "fuzz/fuzz_targets/avformat_pcm_s16le.rs",
        exact_ids: &["avformat-pcm-s16le-demuxer"],
        id_prefixes: &[],
    },
    PathRule {
        path: "fuzz/fuzz_targets/avformat_rawvideo.rs",
        exact_ids: &["avformat-rawvideo-demuxer"],
        id_prefixes: &[],
    },
    PathRule {
        path: "fuzz/fuzz_targets/avformat_avi.rs",
        exact_ids: &["avformat-avi-demuxer"],
        id_prefixes: &[],
    },
    PathRule {
        path: "fuzz/fuzz_targets/avformat_avi_muxer.rs",
        exact_ids: &["avformat-avi-muxer"],
        id_prefixes: &[],
    },
    PathRule {
        path: "fuzz/fuzz_targets/avformat_mov.rs",
        exact_ids: &["avformat-mov-demuxer"],
        id_prefixes: &[],
    },
    PathRule {
        path: "fuzz/fuzz_targets/avformat_image2.rs",
        exact_ids: &["avformat-image2-demuxer", "avformat-image2-muxer"],
        id_prefixes: &[],
    },
    PathRule {
        path: "fuzz/fuzz_targets/avformat_basic_muxers.rs",
        exact_ids: &[
            "avformat-wav-muxer",
            "avformat-pcm-s16le-muxer",
            "avformat-rawvideo-muxer",
            "avformat-yuv4mpegpipe-muxer",
        ],
        id_prefixes: &[],
    },
    PathRule {
        path: "fuzz/fuzz_targets/avformat_packet_muxers.rs",
        exact_ids: &[
            "avformat-null-muxer",
            "avformat-hash-muxer",
            "avformat-framecrc-muxer",
            "avformat-framehash-muxer",
            "avformat-streamhash-muxer",
        ],
        id_prefixes: &[],
    },
    PathRule {
        path: "fuzz/fuzz_targets/fftools_option_parser.rs",
        exact_ids: &["fftools-option-parser"],
        id_prefixes: &[],
    },
];

fn main() {
    match real_main() {
        Ok(()) => {}
        Err(err) => {
            eprintln!("fate-runner: {err}");
            std::process::exit(1);
        }
    }
}

fn real_main() -> Result<(), String> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("list") => list_components(),
        Some("mappings") => list_mappings(args.collect()),
        Some("run") => run_component(args.collect()),
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

fn list_components() -> Result<(), String> {
    let ids = load_component_ids()?;

    if ids.is_empty() {
        println!("no ledger components found");
    } else {
        for id in ids {
            println!("{id}");
        }
    }
    Ok(())
}

fn list_mappings(args: Vec<String>) -> Result<(), String> {
    let options = parse_mapping_options(&args)?;
    let component_ids = load_component_ids()?;
    let mappings = filter_mappings_by_targets(
        load_fate_mappings(&options.mappings_path, &component_ids)?,
        &options.target_filters,
    );
    let lines =
        fate_mapping_report_lines(&mappings, &options.context, options.check_prerequisites)?;

    if lines.is_empty() {
        println!("no FATE mappings found");
    } else {
        for line in lines {
            println!("{line}");
        }
    }

    Ok(())
}

fn parse_mapping_options(args: &[String]) -> Result<MappingOptions, String> {
    let mut mappings_path = DEFAULT_FATE_MAPPINGS_PATH.to_string();
    let mut target_filters = Vec::new();
    let mut context = FateContext::default();
    let mut check_prerequisites = false;
    let mut iter = args.iter();

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--check-prereqs" => check_prerequisites = true,
            "--mappings" => {
                mappings_path = iter
                    .next()
                    .ok_or_else(|| "missing value for --mappings".to_string())?
                    .clone();
            }
            "--target" => {
                let target = iter
                    .next()
                    .ok_or_else(|| "missing value for --target".to_string())?;
                add_target_filter(&mut target_filters, target.clone())?;
            }
            "--samples" => {
                context.samples_root = Some(
                    iter.next()
                        .ok_or_else(|| "missing value for --samples".to_string())?
                        .clone(),
                );
            }
            "--oracle-ffmpeg" => {
                context.oracle_ffmpeg = Some(
                    iter.next()
                        .ok_or_else(|| "missing value for --oracle-ffmpeg".to_string())?
                        .clone(),
                );
            }
            other => return Err(format!("unsupported mappings argument `{other}`")),
        }
    }

    Ok(MappingOptions {
        mappings_path,
        target_filters,
        context,
        check_prerequisites,
    })
}

fn run_component(args: Vec<String>) -> Result<(), String> {
    let options = parse_run_options(&args)?;
    let RunOptions {
        mode,
        mappings_path,
        target_filters,
        context,
        execution_mode,
    } = options;

    match mode {
        RunMode::Components(components) => {
            let ids = load_component_ids()?;
            for component in &components {
                if !ids.iter().any(|id| id == component) {
                    return Err(format!("unknown ledger component `{component}`"));
                }
            }
            run_mapped_components(
                &ids,
                &components,
                &mappings_path,
                &target_filters,
                &context,
                execution_mode,
            )
        }
        RunMode::Changed => {
            run_changed_components(&mappings_path, &target_filters, &context, execution_mode)
        }
    }
}

fn parse_run_options(args: &[String]) -> Result<RunOptions, String> {
    let mut mode = None;
    let mut mappings_path = DEFAULT_FATE_MAPPINGS_PATH.to_string();
    let mut target_filters = Vec::new();
    let mut context = FateContext::default();
    let mut execution_mode = ExecutionMode::Execute;
    let mut iter = args.iter();

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--dry-run" => execution_mode = ExecutionMode::DryRun,
            "--changed" => set_run_mode(&mut mode, RunMode::Changed)?,
            "--component" => {
                let component = iter
                    .next()
                    .ok_or_else(|| "missing value for --component".to_string())?;
                add_run_component(&mut mode, component.clone())?;
            }
            "--mappings" => {
                mappings_path = iter
                    .next()
                    .ok_or_else(|| "missing value for --mappings".to_string())?
                    .clone();
            }
            "--target" => {
                let target = iter
                    .next()
                    .ok_or_else(|| "missing value for --target".to_string())?;
                add_target_filter(&mut target_filters, target.clone())?;
            }
            "--samples" => {
                context.samples_root = Some(
                    iter.next()
                        .ok_or_else(|| "missing value for --samples".to_string())?
                        .clone(),
                );
            }
            "--oracle-ffmpeg" => {
                context.oracle_ffmpeg = Some(
                    iter.next()
                        .ok_or_else(|| "missing value for --oracle-ffmpeg".to_string())?
                        .clone(),
                );
            }
            other => return Err(format!("unsupported run argument `{other}`")),
        }
    }

    Ok(RunOptions {
        mode: mode.ok_or_else(|| "missing --component <id> or --changed".to_string())?,
        mappings_path,
        target_filters,
        context,
        execution_mode,
    })
}

fn add_target_filter(target_filters: &mut Vec<String>, target: String) -> Result<(), String> {
    if target.trim().is_empty() {
        return Err("target filter must not be empty".to_string());
    }
    if !target_filters.iter().any(|existing| existing == &target) {
        target_filters.push(target);
    }
    Ok(())
}

fn set_run_mode(mode: &mut Option<RunMode>, new_mode: RunMode) -> Result<(), String> {
    if mode.is_some() {
        return Err("choose either --component <id> or --changed, not both".to_string());
    }
    *mode = Some(new_mode);
    Ok(())
}

fn add_run_component(mode: &mut Option<RunMode>, component: String) -> Result<(), String> {
    match mode {
        Some(RunMode::Components(components)) => {
            if !components.iter().any(|existing| existing == &component) {
                components.push(component);
            }
            Ok(())
        }
        Some(RunMode::Changed) => {
            Err("choose either --component <id> or --changed, not both".to_string())
        }
        None => {
            *mode = Some(RunMode::Components(vec![component]));
            Ok(())
        }
    }
}

fn run_changed_components(
    mappings_path: &str,
    target_filters: &[String],
    context: &FateContext,
    execution_mode: ExecutionMode,
) -> Result<(), String> {
    let component_ids = load_component_ids()?;
    let changed_paths = git_changed_paths()?;
    let unmapped = unmapped_relevant_paths(&changed_paths);

    if !unmapped.is_empty() {
        return Err(format!(
            "changed implementation paths have no component selection rule: {}",
            unmapped.join(", ")
        ));
    }

    let components = changed_components(&component_ids, &changed_paths);
    if components.is_empty() {
        println!("no changed ledger components found");
        return Ok(());
    }

    println!("changed ledger components:");
    for component in &components {
        println!("{component}");
    }

    run_mapped_components(
        &component_ids,
        &components,
        mappings_path,
        target_filters,
        context,
        execution_mode,
    )
}

fn run_mapped_components(
    component_ids: &[String],
    selected_components: &[String],
    mappings_path: &str,
    target_filters: &[String],
    context: &FateContext,
    execution_mode: ExecutionMode,
) -> Result<(), String> {
    let mappings = filter_mappings_by_targets(
        load_fate_mappings(mappings_path, component_ids)?,
        target_filters,
    );
    let missing_components = components_without_mappings(selected_components, &mappings);

    if !missing_components.is_empty() {
        return Err(format!(
            "no runnable FATE mappings exist for components: {}",
            missing_components.join(", ")
        ));
    }

    for component in selected_components {
        for mapping in mappings
            .iter()
            .filter(|mapping| &mapping.component_id == component)
        {
            run_fate_mapping(mapping, context, execution_mode)?;
        }
    }

    Ok(())
}

fn filter_mappings_by_targets(
    mappings: Vec<FateMapping>,
    target_filters: &[String],
) -> Vec<FateMapping> {
    if target_filters.is_empty() {
        return mappings;
    }
    mappings
        .into_iter()
        .filter(|mapping| {
            target_filters
                .iter()
                .any(|target| target == &mapping.target)
        })
        .collect()
}

fn components_without_mappings(
    selected_components: &[String],
    mappings: &[FateMapping],
) -> Vec<String> {
    selected_components
        .iter()
        .filter(|component| {
            !mappings
                .iter()
                .any(|mapping| &mapping.component_id == *component)
        })
        .cloned()
        .collect()
}

fn fate_mapping_report_lines(
    mappings: &[FateMapping],
    context: &FateContext,
    check_prerequisites: bool,
) -> Result<Vec<String>, String> {
    fate_mapping_report_lines_with(
        mappings,
        context,
        check_prerequisites,
        &process_env_var,
        &path_is_dir,
        &path_is_file,
    )
}

fn fate_mapping_report_lines_with<E, D, F>(
    mappings: &[FateMapping],
    context: &FateContext,
    check_prerequisites: bool,
    env_var: &E,
    is_dir: &D,
    is_file: &F,
) -> Result<Vec<String>, String>
where
    E: Fn(&str) -> Option<String>,
    D: Fn(&str) -> bool,
    F: Fn(&str) -> bool,
{
    mappings
        .iter()
        .map(|mapping| {
            let mapping = if check_prerequisites {
                resolve_fate_mapping_with(mapping, context, env_var, is_dir, is_file)?
            } else {
                mapping.clone()
            };
            Ok(format!(
                "{}:{} -> {}",
                mapping.component_id,
                mapping.target,
                format_mapping_command(&mapping)
            ))
        })
        .collect()
}

fn load_fate_mappings(path: &str, component_ids: &[String]) -> Result<Vec<FateMapping>, String> {
    let contents = fs::read_to_string(path).map_err(|err| {
        if err.kind() == std::io::ErrorKind::NotFound {
            format!("FATE mapping file `{path}` does not exist")
        } else {
            format!("failed to read FATE mapping file `{path}`: {err}")
        }
    })?;
    parse_fate_mappings(&contents, component_ids)
}

fn parse_fate_mappings(
    contents: &str,
    component_ids: &[String],
) -> Result<Vec<FateMapping>, String> {
    let mut mappings = Vec::new();
    let mut seen_targets: Vec<(String, String)> = Vec::new();

    for (line_index, line) in contents.lines().enumerate() {
        let line_number = line_index + 1;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let fields: Vec<&str> = trimmed.split('|').map(str::trim).collect();
        if fields.len() < 4 {
            return Err(format!(
                "invalid FATE mapping at line {line_number}: expected component|target|workdir|program|args..."
            ));
        }

        let component_id = fields[0];
        let target = fields[1];
        let workdir = fields[2];
        let program = fields[3];
        if component_id.is_empty() || target.is_empty() || workdir.is_empty() || program.is_empty()
        {
            return Err(format!(
                "invalid FATE mapping at line {line_number}: component, target, workdir, and program are required"
            ));
        }

        if !component_ids.iter().any(|id| id == component_id) {
            return Err(format!(
                "invalid FATE mapping at line {line_number}: unknown ledger component `{component_id}`"
            ));
        }
        validate_mapping_field_placeholders(line_number, "workdir", workdir)?;
        validate_mapping_field_placeholders(line_number, "program", program)?;

        let key = (component_id.to_string(), target.to_string());
        if seen_targets.iter().any(|seen| seen == &key) {
            return Err(format!(
                "invalid FATE mapping at line {line_number}: duplicate target `{target}` for component `{component_id}`"
            ));
        }
        seen_targets.push(key);

        let mut env = Vec::new();
        let mut args = Vec::new();
        for field in &fields[4..] {
            if let Some(assignment) = field.strip_prefix("env:") {
                let (name, value) = parse_env_assignment(assignment)
                    .map_err(|err| format!("invalid FATE mapping at line {line_number}: {err}"))?;
                if env.iter().any(|(existing, _)| existing == name) {
                    return Err(format!(
                        "invalid FATE mapping at line {line_number}: duplicate environment assignment for `{name}`"
                    ));
                }
                validate_mapping_field_placeholders(line_number, "environment value", value)?;
                env.push((name.to_string(), value.to_string()));
            } else {
                validate_mapping_field_placeholders(line_number, "argument", field)?;
                args.push((*field).to_string());
            }
        }

        mappings.push(FateMapping {
            component_id: component_id.to_string(),
            target: target.to_string(),
            workdir: normalize_path(workdir),
            program: program.to_string(),
            env,
            args,
        });
    }

    Ok(mappings)
}

fn parse_env_assignment(assignment: &str) -> Result<(&str, &str), String> {
    let (name, value) = assignment
        .split_once('=')
        .ok_or_else(|| "environment assignments must use env:NAME=value".to_string())?;
    if name.is_empty() || value.is_empty() {
        return Err("environment assignments must use non-empty env:NAME=value".to_string());
    }
    if name.contains(char::is_whitespace) {
        return Err(format!(
            "environment variable name `{name}` must not contain whitespace"
        ));
    }
    Ok((name, value))
}

fn validate_mapping_field_placeholders(
    line_number: usize,
    field_kind: &str,
    value: &str,
) -> Result<(), String> {
    validate_placeholders(value).map_err(|err| {
        format!("invalid FATE mapping at line {line_number}: {err} in {field_kind} `{value}`")
    })
}

fn validate_placeholders(value: &str) -> Result<(), String> {
    let mut search_start = 0;
    while search_start < value.len() {
        let next_open = value[search_start..]
            .find('{')
            .map(|index| search_start + index);
        let next_close = value[search_start..]
            .find('}')
            .map(|index| search_start + index);

        match (next_open, next_close) {
            (None, None) => return Ok(()),
            (None, Some(_)) => {
                return Err("unmatched `}` placeholder delimiter".to_string());
            }
            (Some(open), Some(close)) if close < open => {
                return Err("unmatched `}` placeholder delimiter".to_string());
            }
            (Some(open), _) => {
                let Some(close_offset) = value[open + 1..].find('}') else {
                    return Err("unclosed `{` placeholder delimiter".to_string());
                };
                let close = open + 1 + close_offset;
                let placeholder = &value[open + 1..close];
                if !SUPPORTED_PLACEHOLDERS
                    .iter()
                    .any(|supported| supported == &placeholder)
                {
                    return Err(format!(
                        "unknown placeholder `{{{placeholder}}}`; supported placeholders are: {}",
                        SUPPORTED_PLACEHOLDERS
                            .iter()
                            .map(|placeholder| format!("{{{placeholder}}}"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }
                search_start = close + 1;
            }
        }
    }

    Ok(())
}

fn run_fate_mapping(
    mapping: &FateMapping,
    context: &FateContext,
    execution_mode: ExecutionMode,
) -> Result<(), String> {
    run_fate_mapping_with(
        mapping,
        context,
        execution_mode,
        &process_env_var,
        &path_is_dir,
        &path_is_file,
    )
}

fn run_fate_mapping_with<E, D, F>(
    mapping: &FateMapping,
    context: &FateContext,
    execution_mode: ExecutionMode,
    env_var: &E,
    is_dir: &D,
    is_file: &F,
) -> Result<(), String>
where
    E: Fn(&str) -> Option<String>,
    D: Fn(&str) -> bool,
    F: Fn(&str) -> bool,
{
    let mapping = resolve_fate_mapping_with(mapping, context, env_var, is_dir, is_file)?;

    match execution_mode {
        ExecutionMode::DryRun => {
            println!(
                "dry-run {}:{} -> {}",
                mapping.component_id,
                mapping.target,
                format_mapping_command(&mapping)
            );
            return Ok(());
        }
        ExecutionMode::Execute => {
            println!(
                "running {}:{} -> {}",
                mapping.component_id,
                mapping.target,
                format_mapping_command(&mapping)
            );
        }
    }

    let status = Command::new(&mapping.program)
        .envs(mapping.env.iter().map(|(name, value)| (name, value)))
        .args(&mapping.args)
        .current_dir(&mapping.workdir)
        .status()
        .map_err(|err| {
            format!(
                "failed to run FATE mapping {}:{}: {err}",
                mapping.component_id, mapping.target
            )
        })?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "FATE mapping {}:{} exited with status {status}",
            mapping.component_id, mapping.target
        ))
    }
}

#[cfg(test)]
fn resolve_fate_mapping(
    mapping: &FateMapping,
    context: &FateContext,
) -> Result<FateMapping, String> {
    resolve_fate_mapping_with(
        mapping,
        context,
        &process_env_var,
        &path_is_dir,
        &path_is_file,
    )
}

fn resolve_fate_mapping_with<E, D, F>(
    mapping: &FateMapping,
    context: &FateContext,
    env_var: &E,
    is_dir: &D,
    is_file: &F,
) -> Result<FateMapping, String>
where
    E: Fn(&str) -> Option<String>,
    D: Fn(&str) -> bool,
    F: Fn(&str) -> bool,
{
    Ok(FateMapping {
        component_id: mapping.component_id.clone(),
        target: mapping.target.clone(),
        workdir: replace_mapping_placeholders_with(
            mapping,
            &mapping.workdir,
            context,
            env_var,
            is_dir,
            is_file,
        )?,
        program: replace_mapping_placeholders_with(
            mapping,
            &mapping.program,
            context,
            env_var,
            is_dir,
            is_file,
        )?,
        env: mapping
            .env
            .iter()
            .map(|(name, value)| {
                Ok((
                    name.clone(),
                    replace_mapping_placeholders_with(
                        mapping, value, context, env_var, is_dir, is_file,
                    )?,
                ))
            })
            .collect::<Result<Vec<_>, String>>()?,
        args: mapping
            .args
            .iter()
            .map(|arg| {
                replace_mapping_placeholders_with(mapping, arg, context, env_var, is_dir, is_file)
            })
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn replace_mapping_placeholders_with<E, D, F>(
    mapping: &FateMapping,
    value: &str,
    context: &FateContext,
    env_var: &E,
    is_dir: &D,
    is_file: &F,
) -> Result<String, String>
where
    E: Fn(&str) -> Option<String>,
    D: Fn(&str) -> bool,
    F: Fn(&str) -> bool,
{
    let mut value = value.to_string();

    if value.contains("{samples}") {
        let samples_root = required_samples_root_with(mapping, context, env_var, is_dir)?;
        value = value.replace("{samples}", &samples_root);
    }

    if value.contains("{oracle_ffmpeg}") {
        let oracle_ffmpeg = required_oracle_ffmpeg_with(mapping, context, env_var, is_file)?;
        value = value.replace("{oracle_ffmpeg}", &oracle_ffmpeg);
    }

    Ok(value)
}

fn required_samples_root_with<E, D>(
    mapping: &FateMapping,
    context: &FateContext,
    env_var: &E,
    is_dir: &D,
) -> Result<String, String>
where
    E: Fn(&str) -> Option<String>,
    D: Fn(&str) -> bool,
{
    if let Some(samples_root) = context.samples_root.as_deref() {
        if !is_dir(samples_root) {
            return Err(format!(
                "FATE mapping {}:{} requires --samples `{samples_root}` to be an existing directory",
                mapping.component_id, mapping.target
            ));
        }
        return Ok(samples_root.to_string());
    }

    for env_name in SAMPLES_ENV_VARS {
        if let Some(samples_root) = env_var(env_name) {
            if !is_dir(&samples_root) {
                return Err(format!(
                    "FATE mapping {}:{} requires {env_name} `{samples_root}` to be an existing directory",
                    mapping.component_id, mapping.target
                ));
            }
            return Ok(samples_root);
        }
    }

    if let Some(samples_root) = DEFAULT_SAMPLES_ROOT_CANDIDATES
        .iter()
        .copied()
        .find(|path| is_dir(path))
    {
        return Ok(default_prerequisite_path(samples_root));
    }

    Err(format!(
        "FATE mapping {}:{} references {{samples}} but no samples path was provided; pass --samples <path>, set FATE_SAMPLES or SAMPLES, or create one of: {}",
        mapping.component_id,
        mapping.target,
        DEFAULT_SAMPLES_ROOT_CANDIDATES.join(", ")
    ))
}

fn required_oracle_ffmpeg_with<E, F>(
    mapping: &FateMapping,
    context: &FateContext,
    env_var: &E,
    is_file: &F,
) -> Result<String, String>
where
    E: Fn(&str) -> Option<String>,
    F: Fn(&str) -> bool,
{
    if let Some(oracle_ffmpeg) = context.oracle_ffmpeg.as_deref() {
        if !is_file(oracle_ffmpeg) {
            return Err(format!(
                "FATE mapping {}:{} requires --oracle-ffmpeg `{oracle_ffmpeg}` to be an existing file",
                mapping.component_id, mapping.target
            ));
        }
        return Ok(oracle_ffmpeg.to_string());
    }

    for env_name in ORACLE_FFMPEG_ENV_VARS {
        if let Some(oracle_ffmpeg) = env_var(env_name) {
            if !is_file(&oracle_ffmpeg) {
                return Err(format!(
                    "FATE mapping {}:{} requires {env_name} `{oracle_ffmpeg}` to be an existing file",
                    mapping.component_id, mapping.target
                ));
            }
            return Ok(oracle_ffmpeg);
        }
    }

    if let Some(oracle_ffmpeg) = DEFAULT_ORACLE_FFMPEG_CANDIDATES
        .iter()
        .copied()
        .find(|path| is_file(path))
    {
        return Ok(default_prerequisite_path(oracle_ffmpeg));
    }

    Err(format!(
        "FATE mapping {}:{} references {{oracle_ffmpeg}} but no oracle path was provided; pass --oracle-ffmpeg <path>, set FFMPEG_ORACLE, or create one of: {}",
        mapping.component_id,
        mapping.target,
        DEFAULT_ORACLE_FFMPEG_CANDIDATES.join(", ")
    ))
}

fn process_env_var(name: &str) -> Option<String> {
    env::var(name).ok().filter(|value| !value.trim().is_empty())
}

fn path_is_dir(path: &str) -> bool {
    Path::new(path).is_dir()
}

fn path_is_file(path: &str) -> bool {
    Path::new(path).is_file()
}

fn default_prerequisite_path(path: &str) -> String {
    let path = Path::new(path);
    if path.is_absolute() {
        path.display().to_string()
    } else {
        env::current_dir()
            .map(|cwd| cwd.join(path).display().to_string())
            .unwrap_or_else(|_| path.display().to_string())
    }
}

fn format_mapping_command(mapping: &FateMapping) -> String {
    let mut parts = Vec::with_capacity(mapping.env.len() + mapping.args.len() + 1);
    parts.extend(
        mapping
            .env
            .iter()
            .map(|(name, value)| format!("{name}={value}")),
    );
    parts.push(mapping.program.clone());
    parts.extend(mapping.args.iter().cloned());
    format!("(cd {} && {})", mapping.workdir, parts.join(" "))
}

fn load_component_ids() -> Result<Vec<String>, String> {
    let ledger = fs::read_to_string("PORTING_LEDGER.toml")
        .map_err(|err| format!("failed to read PORTING_LEDGER.toml: {err}"))?;
    Ok(component_ids_from_ledger(&ledger))
}

fn component_ids_from_ledger(ledger: &str) -> Vec<String> {
    ledger
        .lines()
        .filter_map(|line| line.trim().strip_prefix("id = "))
        .filter_map(|value| {
            let value = value.trim();
            value
                .strip_prefix('"')
                .and_then(|value| value.strip_suffix('"'))
                .map(str::to_string)
        })
        .collect()
}

fn git_changed_paths() -> Result<Vec<String>, String> {
    let mut paths = git_path_list(&["diff", "--name-only", "--relative", "HEAD", "--"])?;
    paths.extend(git_path_list(&[
        "ls-files",
        "--others",
        "--exclude-standard",
    ])?);
    paths.sort();
    paths.dedup();
    Ok(paths)
}

fn git_path_list(args: &[&str]) -> Result<Vec<String>, String> {
    let output = Command::new("git")
        .args(args)
        .output()
        .map_err(|err| format!("failed to run git {}: {err}", args.join(" ")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "`git {}` exited with status {}: {}",
            args.join(" "),
            output.status,
            stderr.trim()
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(normalize_path)
        .filter(|path| !path.is_empty())
        .collect())
}

fn changed_components(component_ids: &[String], changed_paths: &[String]) -> Vec<String> {
    let mut selected = Vec::new();
    for path in changed_paths {
        let path = normalize_path(path);
        for rule in PATH_RULES {
            if !path_matches_rule(&path, rule) {
                continue;
            }

            for exact_id in rule.exact_ids {
                if component_ids.iter().any(|id| id == exact_id)
                    && !selected.iter().any(|id| id == exact_id)
                {
                    selected.push((*exact_id).to_string());
                }
            }

            for prefix in rule.id_prefixes {
                for component_id in component_ids {
                    if component_id.starts_with(prefix)
                        && !selected.iter().any(|id| id == component_id)
                    {
                        selected.push(component_id.clone());
                    }
                }
            }
        }
    }

    component_ids
        .iter()
        .filter(|component_id| selected.iter().any(|id| id == *component_id))
        .cloned()
        .collect()
}

fn unmapped_relevant_paths(changed_paths: &[String]) -> Vec<String> {
    changed_paths
        .iter()
        .map(|path| normalize_path(path))
        .filter(|path| is_relevant_implementation_path(path))
        .filter(|path| !PATH_RULES.iter().any(|rule| path_matches_rule(path, rule)))
        .collect()
}

fn is_relevant_implementation_path(path: &str) -> bool {
    path == "Cargo.toml"
        || path == "Cargo.lock"
        || path == "fuzz/Cargo.toml"
        || path == "xtask/Cargo.toml"
        || path.starts_with("crates/")
        || path.starts_with("tests/fate/")
        || path.starts_with("tests/differential/")
        || path.starts_with("fuzz/fuzz_targets/")
}

fn path_matches_rule(path: &str, rule: &PathRule) -> bool {
    if rule.path.ends_with('/') {
        path.starts_with(rule.path)
    } else {
        path == rule.path
    }
}

fn normalize_path(path: &str) -> String {
    path.replace('\\', "/")
        .trim_start_matches("./")
        .trim()
        .to_string()
}

fn print_help() {
    eprintln!(
        "usage: fate-runner list | mappings [--check-prereqs] [--mappings <path>] [--target <name> ...] [--samples <path>] [--oracle-ffmpeg <path>] | run [--dry-run] [--mappings <path>] [--target <name> ...] [--samples <path>] [--oracle-ffmpeg <path>] --component <id> [--component <id> ...] | run [--dry-run] [--mappings <path>] [--target <name> ...] [--samples <path>] [--oracle-ffmpeg <path>] --changed"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ledger(ids: &[&str]) -> String {
        ids.iter()
            .map(|id| format!("[[component]]\nid = \"{id}\"\n"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn parses_component_ids_from_ledger() {
        let ledger = ledger(&["avutil-error", "fate-runner"]);

        assert_eq!(
            component_ids_from_ledger(&ledger),
            vec!["avutil-error".to_string(), "fate-runner".to_string()]
        );
    }

    #[test]
    fn changed_selection_maps_paths_and_preserves_ledger_order() {
        let component_ids = component_ids_from_ledger(&ledger(&[
            "avutil-error",
            "avformat-rawvideo-demuxer",
            "avformat-rawvideo-muxer",
            "fate-runner",
            "repo-runtime-guard",
        ]));
        let paths = vec![
            "crates\\fate-runner\\src\\main.rs".to_string(),
            "crates/avformat/src/rawvideo.rs".to_string(),
            "crates/avutil/src/error.rs".to_string(),
            "crates/avformat/src/rawvideo.rs".to_string(),
            "xtask/src/main.rs".to_string(),
        ];

        assert_eq!(
            changed_components(&component_ids, &paths),
            vec![
                "avutil-error".to_string(),
                "avformat-rawvideo-demuxer".to_string(),
                "avformat-rawvideo-muxer".to_string(),
                "fate-runner".to_string(),
                "repo-runtime-guard".to_string(),
            ]
        );
    }

    #[test]
    fn changed_selection_maps_dependency_manifests_to_runtime_guard() {
        let component_ids = component_ids_from_ledger(&ledger(&[
            "avutil-error",
            "avcodec-rawvideo",
            "fftools-version",
            "fate-runner",
            "oracle-inventory",
            "repo-runtime-guard",
        ]));
        let paths = vec![
            "Cargo.toml".to_string(),
            "Cargo.lock".to_string(),
            "crates/avutil/Cargo.toml".to_string(),
            "crates/avcodec/Cargo.toml".to_string(),
            "crates/fftools/Cargo.toml".to_string(),
            "crates/fate-runner/Cargo.toml".to_string(),
            "crates/oracle/Cargo.toml".to_string(),
            "fuzz/Cargo.toml".to_string(),
            "xtask/Cargo.toml".to_string(),
        ];

        assert_eq!(
            changed_components(&component_ids, &paths),
            vec![
                "avutil-error".to_string(),
                "avcodec-rawvideo".to_string(),
                "fftools-version".to_string(),
                "fate-runner".to_string(),
                "oracle-inventory".to_string(),
                "repo-runtime-guard".to_string(),
            ]
        );
    }

    #[test]
    fn changed_selection_expands_prefix_rules_against_existing_ledger_ids() {
        let component_ids = component_ids_from_ledger(&ledger(&[
            "fftools-basic-io",
            "fftools-ffprobe-mov-show-format",
            "fftools-ffmpeg-rawvideo-framecrc-null",
        ]));
        let paths = vec!["crates/fftools/src/ffprobe.rs".to_string()];

        assert_eq!(
            changed_components(&component_ids, &paths),
            vec![
                "fftools-basic-io".to_string(),
                "fftools-ffprobe-mov-show-format".to_string(),
            ]
        );
    }

    #[test]
    fn changed_selection_maps_rawvideo_oracle_test_to_covered_components() {
        let component_ids = component_ids_from_ledger(&ledger(&[
            "fftools-ffmpeg-rawvideo-file-output",
            "avformat-rawvideo-demuxer",
            "avformat-rawvideo-muxer",
        ]));
        let paths = vec!["crates/fftools/tests/rawvideo_oracle.rs".to_string()];

        assert_eq!(
            changed_components(&component_ids, &paths),
            vec![
                "fftools-ffmpeg-rawvideo-file-output".to_string(),
                "avformat-rawvideo-demuxer".to_string(),
                "avformat-rawvideo-muxer".to_string(),
            ]
        );
    }

    #[test]
    fn changed_selection_maps_wav_oracle_test_to_covered_components() {
        let component_ids = component_ids_from_ledger(&ledger(&[
            "avformat-wav-demuxer",
            "fftools-ffmpeg-md5-output",
            "fftools-ffmpeg-wav-framecrc-null",
        ]));
        let paths = vec!["crates/fftools/tests/wav_oracle.rs".to_string()];

        assert_eq!(
            changed_components(&component_ids, &paths),
            vec![
                "avformat-wav-demuxer".to_string(),
                "fftools-ffmpeg-md5-output".to_string(),
                "fftools-ffmpeg-wav-framecrc-null".to_string(),
            ]
        );
    }

    #[test]
    fn changed_selection_maps_version_oracle_test_to_component() {
        let component_ids = component_ids_from_ledger(&ledger(&["fftools-version"]));
        let paths = vec!["crates/fftools/tests/version_oracle.rs".to_string()];

        assert_eq!(
            changed_components(&component_ids, &paths),
            vec!["fftools-version".to_string()]
        );
    }

    #[test]
    fn changed_selection_maps_error_oracle_test_to_component() {
        let component_ids = component_ids_from_ledger(&ledger(&["avutil-error"]));
        let paths = vec!["crates/avutil/tests/error_oracle.rs".to_string()];

        assert_eq!(
            changed_components(&component_ids, &paths),
            vec!["avutil-error".to_string()]
        );
    }

    #[test]
    fn changed_selection_maps_rational_oracle_test_to_component() {
        let component_ids = component_ids_from_ledger(&ledger(&["avutil-rational"]));
        let paths = vec!["crates/avutil/tests/rational_oracle.rs".to_string()];

        assert_eq!(
            changed_components(&component_ids, &paths),
            vec!["avutil-rational".to_string()]
        );
    }

    #[test]
    fn changed_selection_maps_timebase_oracle_test_to_component() {
        let component_ids = component_ids_from_ledger(&ledger(&["avutil-timebase"]));
        let paths = vec!["crates/avutil/tests/timebase_oracle.rs".to_string()];

        assert_eq!(
            changed_components(&component_ids, &paths),
            vec!["avutil-timebase".to_string()]
        );
    }

    #[test]
    fn changed_selection_maps_channel_layout_oracle_test_to_component() {
        let component_ids = component_ids_from_ledger(&ledger(&["avutil-channel-layout"]));
        let paths = vec!["crates/avutil/tests/channel_layout_oracle.rs".to_string()];

        assert_eq!(
            changed_components(&component_ids, &paths),
            vec!["avutil-channel-layout".to_string()]
        );
    }

    #[test]
    fn changed_selection_maps_sample_format_oracle_test_to_component() {
        let component_ids = component_ids_from_ledger(&ledger(&["avutil-sample-format"]));
        let paths = vec!["crates/avutil/tests/sample_format_oracle.rs".to_string()];

        assert_eq!(
            changed_components(&component_ids, &paths),
            vec!["avutil-sample-format".to_string()]
        );
    }

    #[test]
    fn changed_selection_maps_pixel_format_oracle_test_to_component() {
        let component_ids = component_ids_from_ledger(&ledger(&["avutil-pixel-format"]));
        let paths = vec!["crates/avutil/tests/pixel_format_oracle.rs".to_string()];

        assert_eq!(
            changed_components(&component_ids, &paths),
            vec!["avutil-pixel-format".to_string()]
        );
    }

    #[test]
    fn changed_selection_maps_color_oracle_test_to_component() {
        let component_ids = component_ids_from_ledger(&ledger(&["avutil-color"]));
        let paths = vec!["crates/avutil/tests/color_oracle.rs".to_string()];

        assert_eq!(
            changed_components(&component_ids, &paths),
            vec!["avutil-color".to_string()]
        );
    }

    #[test]
    fn changed_selection_maps_hash_oracle_test_to_component() {
        let component_ids = component_ids_from_ledger(&ledger(&["avutil-hash"]));
        let paths = vec!["crates/avutil/tests/hash_oracle.rs".to_string()];

        assert_eq!(
            changed_components(&component_ids, &paths),
            vec!["avutil-hash".to_string()]
        );
    }

    #[test]
    fn changed_selection_maps_differential_files_to_runner() {
        let component_ids = component_ids_from_ledger(&ledger(&["fate-runner"]));
        let paths = vec![
            "tests/differential/mappings.txt".to_string(),
            "tests/differential/README.md".to_string(),
        ];

        assert_eq!(
            changed_components(&component_ids, &paths),
            vec!["fate-runner".to_string()]
        );
    }

    #[test]
    fn differential_mappings_parse_against_current_ledger() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let ledger_contents = fs::read_to_string(repo_root.join("PORTING_LEDGER.toml")).unwrap();
        let component_ids = component_ids_from_ledger(&ledger_contents);
        let mapping_contents =
            fs::read_to_string(repo_root.join("tests/differential/mappings.txt")).unwrap();

        let mappings = parse_fate_mappings(&mapping_contents, &component_ids).unwrap();
        let pairs: Vec<_> = mappings
            .iter()
            .map(|mapping| (mapping.component_id.as_str(), mapping.target.as_str()))
            .collect();

        assert!(pairs.contains(&(
            "fftools-ffmpeg-rawvideo-file-output",
            "oracle-rawvideo-file-output"
        )));
        assert!(pairs.contains(&("avformat-rawvideo-demuxer", "oracle-rawvideo-file-output")));
        assert!(pairs.contains(&("avformat-rawvideo-muxer", "oracle-rawvideo-file-output")));
        assert!(pairs.contains(&("avutil-channel-layout", "oracle-ffmpeg-layouts")));
        assert!(pairs.contains(&("avformat-wav-demuxer", "oracle-wav-generated-md5")));
        assert!(pairs.contains(&("avutil-sample-format", "oracle-ffmpeg-sample-fmts")));
        assert!(pairs.contains(&("avutil-pixel-format", "oracle-ffmpeg-pix-fmts-subset")));
        assert!(pairs.contains(&("avutil-color", "oracle-ffmpeg-colors")));
        assert!(pairs.contains(&("avutil-hash", "oracle-libavutil-hash")));
    }

    #[test]
    fn upstream_fate_mappings_parse_against_current_ledger() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let ledger_contents = fs::read_to_string(repo_root.join("PORTING_LEDGER.toml")).unwrap();
        let component_ids = component_ids_from_ledger(&ledger_contents);
        let mapping_contents =
            fs::read_to_string(repo_root.join(UPSTREAM_FATE_MAPPINGS_PATH)).unwrap();

        let mappings = parse_fate_mappings(&mapping_contents, &component_ids).unwrap();
        let mapping = mappings
            .iter()
            .find(|mapping| {
                mapping.component_id == "avformat-wav-demuxer"
                    && mapping.target == "fate-wav-pcm-s16le-md5"
            })
            .expect("sample-backed WAV FATE mapping should exist");

        assert_eq!(mapping.workdir, ".");
        assert_eq!(mapping.program, "cargo");
        assert_eq!(
            mapping.env,
            vec![
                ("FFMPEG_ORACLE".to_string(), "{oracle_ffmpeg}".to_string()),
                (
                    "FATE_WAV_SAMPLE".to_string(),
                    "{samples}/audio-reference/luckynight_2ch_44kHz_s16.wav".to_string(),
                ),
            ]
        );
        assert_eq!(
            mapping.args,
            vec![
                "test".to_string(),
                "-p".to_string(),
                "fftools".to_string(),
                "--test".to_string(),
                "wav_oracle".to_string(),
                "wav_pcm_s16le_md5_matches_ffmpeg_oracle_sample".to_string(),
                "--".to_string(),
                "--ignored".to_string(),
            ]
        );
    }

    #[test]
    fn default_mappings_cover_current_fftools_smoke_selections() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let ledger_contents = fs::read_to_string(repo_root.join("PORTING_LEDGER.toml")).unwrap();
        let component_ids = component_ids_from_ledger(&ledger_contents);
        let mapping_contents =
            fs::read_to_string(repo_root.join(DEFAULT_FATE_MAPPINGS_PATH)).unwrap();
        let mappings = parse_fate_mappings(&mapping_contents, &component_ids).unwrap();
        let selected_components = changed_components(
            &component_ids,
            &[
                "crates/fftools/src/lib.rs".to_string(),
                "crates/fftools/src/bin/ffmpeg-rs.rs".to_string(),
                "crates/fftools/src/option_parser.rs".to_string(),
                "crates/fftools/src/cli_logging.rs".to_string(),
                "crates/fftools/src/io_plan.rs".to_string(),
                "crates/fftools/src/ffmpeg.rs".to_string(),
                "crates/fftools/tests/version_oracle.rs".to_string(),
                "crates/fftools/tests/rawvideo_oracle.rs".to_string(),
                "crates/fftools/tests/wav_oracle.rs".to_string(),
                "crates/fftools/src/ffprobe.rs".to_string(),
                "fuzz/fuzz_targets/fftools_option_parser.rs".to_string(),
            ],
        );

        let missing_components = components_without_mappings(&selected_components, &mappings);

        assert!(
            missing_components.is_empty(),
            "missing local FATE smoke mappings for {:?}",
            missing_components
        );
    }

    #[test]
    fn default_mappings_cover_current_avutil_smoke_selections() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let ledger_contents = fs::read_to_string(repo_root.join("PORTING_LEDGER.toml")).unwrap();
        let component_ids = component_ids_from_ledger(&ledger_contents);
        let mapping_contents =
            fs::read_to_string(repo_root.join(DEFAULT_FATE_MAPPINGS_PATH)).unwrap();
        let mappings = parse_fate_mappings(&mapping_contents, &component_ids).unwrap();
        let selected_components = changed_components(
            &component_ids,
            &[
                "crates/avutil/src/lib.rs".to_string(),
                "crates/avutil/src/buffer.rs".to_string(),
                "crates/avutil/tests/pixel_format_oracle.rs".to_string(),
                "crates/avutil/tests/sample_format_oracle.rs".to_string(),
                "crates/avutil/tests/channel_layout_oracle.rs".to_string(),
                "crates/avutil/tests/color_oracle.rs".to_string(),
                "fuzz/fuzz_targets/avutil_core_models.rs".to_string(),
            ],
        );

        let missing_components = components_without_mappings(&selected_components, &mappings);

        assert!(
            missing_components.is_empty(),
            "missing local FATE smoke mappings for {:?}",
            missing_components
        );
    }

    #[test]
    fn default_mappings_cover_runtime_guard_selection() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let ledger_contents = fs::read_to_string(repo_root.join("PORTING_LEDGER.toml")).unwrap();
        let component_ids = component_ids_from_ledger(&ledger_contents);
        let mapping_contents =
            fs::read_to_string(repo_root.join(DEFAULT_FATE_MAPPINGS_PATH)).unwrap();
        let mappings = parse_fate_mappings(&mapping_contents, &component_ids).unwrap();
        let selected_components = changed_components(
            &component_ids,
            &[
                "xtask/src/main.rs".to_string(),
                "Cargo.toml".to_string(),
                "Cargo.lock".to_string(),
                "fuzz/Cargo.toml".to_string(),
                "xtask/Cargo.toml".to_string(),
            ],
        );

        let missing_components = components_without_mappings(&selected_components, &mappings);

        assert!(
            missing_components.is_empty(),
            "missing local FATE smoke mappings for {:?}",
            missing_components
        );
    }

    #[test]
    fn changed_selection_maps_fuzz_targets_to_covered_components() {
        let component_ids = component_ids_from_ledger(&ledger(&[
            "avformat-null-muxer",
            "avformat-framecrc-muxer",
            "avformat-image2-demuxer",
            "avformat-image2-muxer",
            "avformat-mov-demuxer",
            "fftools-option-parser",
        ]));
        let paths = vec![
            "fuzz\\fuzz_targets\\avformat_mov.rs".to_string(),
            "fuzz/fuzz_targets/avformat_image2.rs".to_string(),
            "fuzz/fuzz_targets/avformat_packet_muxers.rs".to_string(),
            "fuzz/fuzz_targets/fftools_option_parser.rs".to_string(),
        ];

        assert_eq!(
            changed_components(&component_ids, &paths),
            vec![
                "avformat-null-muxer".to_string(),
                "avformat-framecrc-muxer".to_string(),
                "avformat-image2-demuxer".to_string(),
                "avformat-image2-muxer".to_string(),
                "avformat-mov-demuxer".to_string(),
                "fftools-option-parser".to_string(),
            ]
        );
    }

    #[test]
    fn unmapped_relevant_paths_report_crate_files_but_ignore_docs() {
        let paths = vec![
            "docs/architecture.md".to_string(),
            "Cargo.toml".to_string(),
            "Cargo.lock".to_string(),
            "crates/swscale/src/lib.rs".to_string(),
            "tests/fate/README.md".to_string(),
            "fuzz/Cargo.toml".to_string(),
            "xtask/Cargo.toml".to_string(),
            "fuzz/fuzz_targets/new_target.rs".to_string(),
        ];

        assert_eq!(
            unmapped_relevant_paths(&paths),
            vec![
                "crates/swscale/src/lib.rs".to_string(),
                "fuzz/fuzz_targets/new_target.rs".to_string(),
            ]
        );
    }

    #[test]
    fn parses_run_options_for_component_changed_and_mapping_path() {
        assert_eq!(
            parse_run_options(&[
                "--mappings".to_string(),
                "custom.map".to_string(),
                "--component".to_string(),
                "fate-runner".to_string(),
            ])
            .unwrap(),
            RunOptions {
                mode: RunMode::Components(vec!["fate-runner".to_string()]),
                mappings_path: "custom.map".to_string(),
                target_filters: vec![],
                context: FateContext::default(),
                execution_mode: ExecutionMode::Execute,
            }
        );

        assert_eq!(
            parse_run_options(&["--changed".to_string()]).unwrap(),
            RunOptions {
                mode: RunMode::Changed,
                mappings_path: DEFAULT_FATE_MAPPINGS_PATH.to_string(),
                target_filters: vec![],
                context: FateContext::default(),
                execution_mode: ExecutionMode::Execute,
            }
        );

        assert_eq!(
            parse_run_options(&[
                "--samples".to_string(),
                "tests/fate".to_string(),
                "--oracle-ffmpeg".to_string(),
                "Cargo.toml".to_string(),
                "--changed".to_string(),
            ])
            .unwrap(),
            RunOptions {
                mode: RunMode::Changed,
                mappings_path: DEFAULT_FATE_MAPPINGS_PATH.to_string(),
                target_filters: vec![],
                context: FateContext {
                    samples_root: Some("tests/fate".to_string()),
                    oracle_ffmpeg: Some("Cargo.toml".to_string()),
                },
                execution_mode: ExecutionMode::Execute,
            }
        );

        assert_eq!(
            parse_run_options(&["--dry-run".to_string(), "--changed".to_string()]).unwrap(),
            RunOptions {
                mode: RunMode::Changed,
                mappings_path: DEFAULT_FATE_MAPPINGS_PATH.to_string(),
                target_filters: vec![],
                context: FateContext::default(),
                execution_mode: ExecutionMode::DryRun,
            }
        );

        assert_eq!(
            parse_run_options(&[
                "--component".to_string(),
                "avformat-rawvideo-demuxer".to_string(),
                "--component".to_string(),
                "avformat-rawvideo-muxer".to_string(),
                "--component".to_string(),
                "avformat-rawvideo-demuxer".to_string(),
                "--dry-run".to_string(),
            ])
            .unwrap(),
            RunOptions {
                mode: RunMode::Components(vec![
                    "avformat-rawvideo-demuxer".to_string(),
                    "avformat-rawvideo-muxer".to_string(),
                ]),
                mappings_path: DEFAULT_FATE_MAPPINGS_PATH.to_string(),
                target_filters: vec![],
                context: FateContext::default(),
                execution_mode: ExecutionMode::DryRun,
            }
        );

        assert_eq!(
            parse_run_options(&[
                "--target".to_string(),
                "local-a".to_string(),
                "--target".to_string(),
                "local-b".to_string(),
                "--target".to_string(),
                "local-a".to_string(),
                "--component".to_string(),
                "fate-runner".to_string(),
            ])
            .unwrap(),
            RunOptions {
                mode: RunMode::Components(vec!["fate-runner".to_string()]),
                mappings_path: DEFAULT_FATE_MAPPINGS_PATH.to_string(),
                target_filters: vec!["local-a".to_string(), "local-b".to_string()],
                context: FateContext::default(),
                execution_mode: ExecutionMode::Execute,
            }
        );
    }

    #[test]
    fn rejects_ambiguous_or_incomplete_run_options() {
        assert_eq!(
            parse_run_options(&[]).unwrap_err(),
            "missing --component <id> or --changed"
        );
        assert_eq!(
            parse_run_options(&[
                "--changed".to_string(),
                "--component".to_string(),
                "fate-runner".to_string(),
            ])
            .unwrap_err(),
            "choose either --component <id> or --changed, not both"
        );
        assert_eq!(
            parse_run_options(&["--mappings".to_string()]).unwrap_err(),
            "missing value for --mappings"
        );
        assert_eq!(
            parse_run_options(&["--samples".to_string()]).unwrap_err(),
            "missing value for --samples"
        );
        assert_eq!(
            parse_run_options(&["--oracle-ffmpeg".to_string()]).unwrap_err(),
            "missing value for --oracle-ffmpeg"
        );
        assert_eq!(
            parse_run_options(&["--target".to_string()]).unwrap_err(),
            "missing value for --target"
        );
        assert_eq!(
            parse_run_options(&[
                "--target".to_string(),
                " ".to_string(),
                "--changed".to_string(),
            ])
            .unwrap_err(),
            "target filter must not be empty"
        );
    }

    #[test]
    fn parses_mapping_options_for_listing_and_prerequisite_checking() {
        assert_eq!(
            parse_mapping_options(&[]).unwrap(),
            MappingOptions {
                mappings_path: DEFAULT_FATE_MAPPINGS_PATH.to_string(),
                target_filters: vec![],
                context: FateContext::default(),
                check_prerequisites: false,
            }
        );

        assert_eq!(
            parse_mapping_options(&[
                "--check-prereqs".to_string(),
                "--mappings".to_string(),
                "custom.map".to_string(),
                "--target".to_string(),
                "local-unit".to_string(),
                "--target".to_string(),
                "local-unit".to_string(),
                "--samples".to_string(),
                "tests/fate".to_string(),
                "--oracle-ffmpeg".to_string(),
                "Cargo.toml".to_string(),
            ])
            .unwrap(),
            MappingOptions {
                mappings_path: "custom.map".to_string(),
                target_filters: vec!["local-unit".to_string()],
                context: FateContext {
                    samples_root: Some("tests/fate".to_string()),
                    oracle_ffmpeg: Some("Cargo.toml".to_string()),
                },
                check_prerequisites: true,
            }
        );
    }

    #[test]
    fn rejects_invalid_mapping_options() {
        assert_eq!(
            parse_mapping_options(&["--mappings".to_string()]).unwrap_err(),
            "missing value for --mappings"
        );
        assert_eq!(
            parse_mapping_options(&["--samples".to_string()]).unwrap_err(),
            "missing value for --samples"
        );
        assert_eq!(
            parse_mapping_options(&["--oracle-ffmpeg".to_string()]).unwrap_err(),
            "missing value for --oracle-ffmpeg"
        );
        assert_eq!(
            parse_mapping_options(&["--target".to_string()]).unwrap_err(),
            "missing value for --target"
        );
        assert_eq!(
            parse_mapping_options(&["--target".to_string(), "".to_string()]).unwrap_err(),
            "target filter must not be empty"
        );
        assert_eq!(
            parse_mapping_options(&["--changed".to_string()]).unwrap_err(),
            "unsupported mappings argument `--changed`"
        );
    }

    #[test]
    fn parses_fate_mapping_rows() {
        let component_ids = component_ids_from_ledger(&ledger(&["fate-runner", "avutil-error"]));
        let mappings = parse_fate_mappings(
            "\
# comments and blank lines are ignored

fate-runner | local-self-test | . | cargo | test | -p | fate-runner
avutil-error|error-unit|.|cargo|test|-p|avutil|error
",
            &component_ids,
        )
        .unwrap();

        assert_eq!(
            mappings,
            vec![
                FateMapping {
                    component_id: "fate-runner".to_string(),
                    target: "local-self-test".to_string(),
                    workdir: ".".to_string(),
                    program: "cargo".to_string(),
                    env: vec![],
                    args: vec![
                        "test".to_string(),
                        "-p".to_string(),
                        "fate-runner".to_string(),
                    ],
                },
                FateMapping {
                    component_id: "avutil-error".to_string(),
                    target: "error-unit".to_string(),
                    workdir: ".".to_string(),
                    program: "cargo".to_string(),
                    env: vec![],
                    args: vec![
                        "test".to_string(),
                        "-p".to_string(),
                        "avutil".to_string(),
                        "error".to_string(),
                    ],
                },
            ]
        );
    }

    #[test]
    fn rejects_invalid_fate_mapping_rows() {
        let component_ids = component_ids_from_ledger(&ledger(&["fate-runner"]));

        assert!(
            parse_fate_mappings("fate-runner|missing-fields", &component_ids)
                .unwrap_err()
                .contains("expected component|target|workdir|program|args...")
        );
        assert!(
            parse_fate_mappings("unknown|target|.|cargo|test", &component_ids)
                .unwrap_err()
                .contains("unknown ledger component `unknown`")
        );
        assert!(parse_fate_mappings(
            "fate-runner|target|.|cargo|test\nfate-runner|target|.|cargo|test",
            &component_ids,
        )
        .unwrap_err()
        .contains("duplicate target `target`"));
        assert!(
            parse_fate_mappings("fate-runner|target|.|cargo|env:BAD|test", &component_ids,)
                .unwrap_err()
                .contains("environment assignments must use env:NAME=value")
        );
        assert!(
            parse_fate_mappings("fate-runner|target|.|cargo|env:=value|test", &component_ids,)
                .unwrap_err()
                .contains("non-empty env:NAME=value")
        );
        assert!(parse_fate_mappings(
            "fate-runner|target|.|cargo|env:FOO=one|env:FOO=two|test",
            &component_ids,
        )
        .unwrap_err()
        .contains("duplicate environment assignment for `FOO`"));
    }

    #[test]
    fn rejects_unknown_or_malformed_mapping_placeholders() {
        let component_ids = component_ids_from_ledger(&ledger(&["fate-runner"]));

        assert!(
            parse_fate_mappings("fate-runner|target|{sample}|cargo|test", &component_ids)
                .unwrap_err()
                .contains(
                    "unknown placeholder `{sample}`; supported placeholders are: {samples}, {oracle_ffmpeg}"
                )
        );
        assert!(
            parse_fate_mappings("fate-runner|target|.|{oracle}|test", &component_ids)
                .unwrap_err()
                .contains("unknown placeholder `{oracle}`")
        );
        assert!(
            parse_fate_mappings("fate-runner|target|.|cargo|{samples", &component_ids)
                .unwrap_err()
                .contains("unclosed `{` placeholder delimiter")
        );
        assert!(
            parse_fate_mappings("fate-runner|target|.|cargo|samples}", &component_ids)
                .unwrap_err()
                .contains("unmatched `}` placeholder delimiter")
        );
    }

    #[test]
    fn formats_mapping_commands_for_diagnostics() {
        let mapping = FateMapping {
            component_id: "fate-runner".to_string(),
            target: "local-self-test".to_string(),
            workdir: ".".to_string(),
            program: "cargo".to_string(),
            env: vec![],
            args: vec![
                "test".to_string(),
                "-p".to_string(),
                "fate-runner".to_string(),
            ],
        };

        assert_eq!(
            format_mapping_command(&mapping),
            "(cd . && cargo test -p fate-runner)"
        );
    }

    #[test]
    fn parses_formats_and_resolves_mapping_environment_assignments() {
        let component_ids =
            component_ids_from_ledger(&ledger(&["fftools-ffmpeg-rawvideo-file-output"]));
        let mappings = parse_fate_mappings(
            "fftools-ffmpeg-rawvideo-file-output|oracle-rawvideo|.|cargo|env:FFMPEG_ORACLE={oracle_ffmpeg}|test|-p|fftools|--test|rawvideo_oracle|--|--ignored",
            &component_ids,
        )
        .unwrap();
        let context = FateContext {
            samples_root: None,
            oracle_ffmpeg: Some("Cargo.toml".to_string()),
        };
        let resolved = resolve_fate_mapping(&mappings[0], &context).unwrap();

        assert_eq!(
            resolved.env,
            vec![("FFMPEG_ORACLE".to_string(), "Cargo.toml".to_string())]
        );
        assert_eq!(
            format_mapping_command(&resolved),
            "(cd . && FFMPEG_ORACLE=Cargo.toml cargo test -p fftools --test rawvideo_oracle -- --ignored)"
        );
    }

    #[test]
    fn resolves_mapping_prerequisite_placeholders() {
        let mapping = FateMapping {
            component_id: "avformat-wav-demuxer".to_string(),
            target: "sample-framecrc".to_string(),
            workdir: "{samples}/audio".to_string(),
            program: "{oracle_ffmpeg}".to_string(),
            env: vec![("FFMPEG_ORACLE".to_string(), "{oracle_ffmpeg}".to_string())],
            args: vec![
                "-i".to_string(),
                "{samples}/audio/test.wav".to_string(),
                "-f".to_string(),
                "framecrc".to_string(),
                "-".to_string(),
            ],
        };
        let context = FateContext {
            samples_root: Some(".".to_string()),
            oracle_ffmpeg: Some("Cargo.toml".to_string()),
        };

        assert_eq!(
            resolve_fate_mapping(&mapping, &context).unwrap(),
            FateMapping {
                component_id: "avformat-wav-demuxer".to_string(),
                target: "sample-framecrc".to_string(),
                workdir: "./audio".to_string(),
                program: "Cargo.toml".to_string(),
                env: vec![("FFMPEG_ORACLE".to_string(), "Cargo.toml".to_string())],
                args: vec![
                    "-i".to_string(),
                    "./audio/test.wav".to_string(),
                    "-f".to_string(),
                    "framecrc".to_string(),
                    "-".to_string(),
                ],
            }
        );
    }

    #[test]
    fn resolves_mapping_prerequisites_from_environment_without_flags() {
        let mapping = FateMapping {
            component_id: "avformat-wav-demuxer".to_string(),
            target: "sample-framecrc".to_string(),
            workdir: "{samples}/audio".to_string(),
            program: "{oracle_ffmpeg}".to_string(),
            env: vec![("FFMPEG_ORACLE".to_string(), "{oracle_ffmpeg}".to_string())],
            args: vec!["{samples}/audio/test.wav".to_string()],
        };
        let env_var = |name: &str| match name {
            "FATE_SAMPLES" => Some("env/fate-samples".to_string()),
            "FFMPEG_ORACLE" => Some("env/ffmpeg".to_string()),
            _ => None,
        };
        let is_dir = |path: &str| path == "env/fate-samples";
        let is_file = |path: &str| path == "env/ffmpeg";

        assert_eq!(
            resolve_fate_mapping_with(
                &mapping,
                &FateContext::default(),
                &env_var,
                &is_dir,
                &is_file
            )
            .unwrap(),
            FateMapping {
                component_id: "avformat-wav-demuxer".to_string(),
                target: "sample-framecrc".to_string(),
                workdir: "env/fate-samples/audio".to_string(),
                program: "env/ffmpeg".to_string(),
                env: vec![("FFMPEG_ORACLE".to_string(), "env/ffmpeg".to_string())],
                args: vec!["env/fate-samples/audio/test.wav".to_string()],
            }
        );
    }

    #[test]
    fn falls_back_to_standard_prerequisite_paths_when_present() {
        let mapping = FateMapping {
            component_id: "avformat-wav-demuxer".to_string(),
            target: "sample-framecrc".to_string(),
            workdir: "{samples}".to_string(),
            program: "{oracle_ffmpeg}".to_string(),
            env: vec![],
            args: vec!["-version".to_string()],
        };
        let env_var = |_name: &str| None;
        let is_dir = |path: &str| path == "third_party/fate-samples";
        let is_file = |path: &str| path == "third_party/ffmpeg-oracle/build/bin/ffmpeg.exe";
        let cwd = std::env::current_dir().unwrap();
        let expected_samples = cwd.join("third_party/fate-samples").display().to_string();
        let expected_ffmpeg = cwd
            .join("third_party/ffmpeg-oracle/build/bin/ffmpeg.exe")
            .display()
            .to_string();

        assert_eq!(
            resolve_fate_mapping_with(
                &mapping,
                &FateContext::default(),
                &env_var,
                &is_dir,
                &is_file
            )
            .unwrap(),
            FateMapping {
                component_id: "avformat-wav-demuxer".to_string(),
                target: "sample-framecrc".to_string(),
                workdir: expected_samples,
                program: expected_ffmpeg,
                env: vec![],
                args: vec!["-version".to_string()],
            }
        );
    }

    #[test]
    fn invalid_environment_prerequisite_paths_are_reported() {
        let mapping = FateMapping {
            component_id: "avformat-wav-demuxer".to_string(),
            target: "sample-framecrc".to_string(),
            workdir: "{samples}".to_string(),
            program: "cargo".to_string(),
            env: vec![],
            args: vec![],
        };
        let env_var = |name: &str| match name {
            "FATE_SAMPLES" => Some("Cargo.toml".to_string()),
            _ => None,
        };
        let is_dir = |_path: &str| false;
        let is_file = |_path: &str| false;

        assert!(resolve_fate_mapping_with(
            &mapping,
            &FateContext::default(),
            &env_var,
            &is_dir,
            &is_file
        )
        .unwrap_err()
        .contains("requires FATE_SAMPLES `Cargo.toml` to be an existing directory"));
    }

    #[test]
    fn reports_missing_or_invalid_mapping_prerequisites() {
        let mapping = FateMapping {
            component_id: "avformat-wav-demuxer".to_string(),
            target: "sample-framecrc".to_string(),
            workdir: ".".to_string(),
            program: "cargo".to_string(),
            env: vec![],
            args: vec!["{samples}/audio/test.wav".to_string()],
        };

        let no_env = |_name: &str| None;
        let no_dir = |_path: &str| false;
        let no_file = |_path: &str| false;

        assert!(resolve_fate_mapping_with(
            &mapping,
            &FateContext::default(),
            &no_env,
            &no_dir,
            &no_file
        )
        .unwrap_err()
        .contains("references {samples} but no samples path was provided"));

        let context = FateContext {
            samples_root: Some("Cargo.toml".to_string()),
            oracle_ffmpeg: Some("Cargo.toml".to_string()),
        };
        assert!(resolve_fate_mapping(&mapping, &context)
            .unwrap_err()
            .contains("requires --samples `Cargo.toml` to be an existing directory"));

        let mapping = FateMapping {
            component_id: "avformat-wav-demuxer".to_string(),
            target: "sample-framecrc".to_string(),
            workdir: ".".to_string(),
            program: "{oracle_ffmpeg}".to_string(),
            env: vec![],
            args: vec!["-version".to_string()],
        };
        let context = FateContext {
            samples_root: Some("tests/fate".to_string()),
            oracle_ffmpeg: Some("tests/fate".to_string()),
        };
        assert!(resolve_fate_mapping(&mapping, &context)
            .unwrap_err()
            .contains("requires --oracle-ffmpeg `tests/fate` to be an existing file"));
    }

    #[test]
    fn reports_selected_components_without_mappings() {
        let mappings = vec![FateMapping {
            component_id: "fate-runner".to_string(),
            target: "local-self-test".to_string(),
            workdir: ".".to_string(),
            program: "cargo".to_string(),
            env: vec![],
            args: vec![
                "test".to_string(),
                "-p".to_string(),
                "fate-runner".to_string(),
            ],
        }];

        assert_eq!(
            components_without_mappings(
                &["fate-runner".to_string(), "avutil-error".to_string()],
                &mappings,
            ),
            vec!["avutil-error".to_string()]
        );
    }

    #[test]
    fn target_filters_keep_exact_mapping_targets_and_preserve_order() {
        let mappings = vec![
            FateMapping {
                component_id: "fate-runner".to_string(),
                target: "local-self-test".to_string(),
                workdir: ".".to_string(),
                program: "cargo".to_string(),
                env: vec![],
                args: vec![],
            },
            FateMapping {
                component_id: "fate-runner".to_string(),
                target: "upstream-sample".to_string(),
                workdir: "{samples}".to_string(),
                program: "{oracle_ffmpeg}".to_string(),
                env: vec![],
                args: vec![],
            },
            FateMapping {
                component_id: "avutil-error".to_string(),
                target: "local-error-unit".to_string(),
                workdir: ".".to_string(),
                program: "cargo".to_string(),
                env: vec![],
                args: vec![],
            },
        ];

        assert_eq!(
            filter_mappings_by_targets(
                mappings.clone(),
                &[
                    "local-error-unit".to_string(),
                    "local-self-test".to_string()
                ]
            ),
            vec![mappings[0].clone(), mappings[2].clone()]
        );
        assert_eq!(filter_mappings_by_targets(mappings.clone(), &[]), mappings);
        assert!(filter_mappings_by_targets(mappings, &["missing-target".to_string()]).is_empty());
    }

    #[test]
    fn target_filters_make_unmatched_selected_components_missing() {
        let mappings = vec![
            FateMapping {
                component_id: "fate-runner".to_string(),
                target: "local-self-test".to_string(),
                workdir: ".".to_string(),
                program: "cargo".to_string(),
                env: vec![],
                args: vec![],
            },
            FateMapping {
                component_id: "fate-runner".to_string(),
                target: "upstream-sample".to_string(),
                workdir: "{samples}".to_string(),
                program: "{oracle_ffmpeg}".to_string(),
                env: vec![],
                args: vec![],
            },
        ];
        let filtered = filter_mappings_by_targets(mappings, &["upstream-sample".to_string()]);

        assert!(components_without_mappings(&["fate-runner".to_string()], &filtered).is_empty());
        assert_eq!(
            components_without_mappings(&["avutil-error".to_string()], &filtered),
            vec!["avutil-error".to_string()]
        );
    }

    #[test]
    fn mapping_report_lists_unresolved_commands_by_default() {
        let mappings = vec![FateMapping {
            component_id: "avformat-wav-demuxer".to_string(),
            target: "sample-framecrc".to_string(),
            workdir: "{samples}/audio".to_string(),
            program: "{oracle_ffmpeg}".to_string(),
            env: vec![],
            args: vec![
                "-i".to_string(),
                "{samples}/audio/test.wav".to_string(),
                "-f".to_string(),
                "framecrc".to_string(),
                "-".to_string(),
            ],
        }];

        assert_eq!(
            fate_mapping_report_lines(&mappings, &FateContext::default(), false).unwrap(),
            vec![
                "avformat-wav-demuxer:sample-framecrc -> (cd {samples}/audio && {oracle_ffmpeg} -i {samples}/audio/test.wav -f framecrc -)".to_string()
            ]
        );
    }

    #[test]
    fn mapping_report_can_check_all_prerequisites() {
        let mappings = vec![FateMapping {
            component_id: "avformat-wav-demuxer".to_string(),
            target: "sample-framecrc".to_string(),
            workdir: "{samples}".to_string(),
            program: "{oracle_ffmpeg}".to_string(),
            env: vec![],
            args: vec!["-version".to_string()],
        }];

        let no_env = |_name: &str| None;
        let no_dir = |_path: &str| false;
        let no_file = |_path: &str| false;

        assert!(fate_mapping_report_lines_with(
            &mappings,
            &FateContext::default(),
            true,
            &no_env,
            &no_dir,
            &no_file
        )
        .unwrap_err()
        .contains("references {samples} but no samples path was provided"));

        let context = FateContext {
            samples_root: Some(".".to_string()),
            oracle_ffmpeg: Some("Cargo.toml".to_string()),
        };
        assert_eq!(
            fate_mapping_report_lines(&mappings, &context, true).unwrap(),
            vec![
                "avformat-wav-demuxer:sample-framecrc -> (cd . && Cargo.toml -version)".to_string()
            ]
        );
    }

    #[test]
    fn dry_run_resolves_mapping_without_executing_command() {
        let mapping = FateMapping {
            component_id: "fate-runner".to_string(),
            target: "nonexistent-program".to_string(),
            workdir: ".".to_string(),
            program: "definitely-not-a-real-fate-command".to_string(),
            env: vec![],
            args: vec!["--would-fail-if-executed".to_string()],
        };

        assert_eq!(
            run_fate_mapping(&mapping, &FateContext::default(), ExecutionMode::DryRun),
            Ok(())
        );
    }

    #[test]
    fn dry_run_still_validates_mapping_prerequisites() {
        let mapping = FateMapping {
            component_id: "avformat-wav-demuxer".to_string(),
            target: "sample-framecrc".to_string(),
            workdir: ".".to_string(),
            program: "cargo".to_string(),
            env: vec![],
            args: vec!["{samples}/audio/test.wav".to_string()],
        };

        let no_env = |_name: &str| None;
        let no_dir = |_path: &str| false;
        let no_file = |_path: &str| false;

        assert!(run_fate_mapping_with(
            &mapping,
            &FateContext::default(),
            ExecutionMode::DryRun,
            &no_env,
            &no_dir,
            &no_file
        )
        .unwrap_err()
        .contains("references {samples} but no samples path was provided"));
    }
}
