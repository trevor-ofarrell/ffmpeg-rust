use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

const DEFAULT_FATE_MAPPINGS_PATH: &str = "tests/fate/mappings.txt";

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
    args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RunOptions {
    mode: RunMode,
    mappings_path: String,
    context: FateContext,
    execution_mode: ExecutionMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MappingOptions {
    mappings_path: String,
    context: FateContext,
    check_prerequisites: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RunMode {
    Component(String),
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
        path: "crates/avutil/src/rational.rs",
        exact_ids: &["avutil-rational"],
        id_prefixes: &[],
    },
    PathRule {
        path: "crates/avutil/src/timebase.rs",
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
        path: "crates/avutil/src/samplefmt.rs",
        exact_ids: &["avutil-sample-format"],
        id_prefixes: &[],
    },
    PathRule {
        path: "crates/avutil/src/channel_layout.rs",
        exact_ids: &["avutil-channel-layout"],
        id_prefixes: &[],
    },
    PathRule {
        path: "crates/avutil/src/hash.rs",
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
        path: "crates/fftools/src/option_parser.rs",
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
    let mappings = load_fate_mappings(&options.mappings_path, &component_ids)?;
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
        context,
        check_prerequisites,
    })
}

fn run_component(args: Vec<String>) -> Result<(), String> {
    let options = parse_run_options(&args)?;
    let RunOptions {
        mode,
        mappings_path,
        context,
        execution_mode,
    } = options;

    match mode {
        RunMode::Component(component) => {
            let ids = load_component_ids()?;
            if !ids.iter().any(|id| id == &component) {
                return Err(format!("unknown ledger component `{component}`"));
            }
            run_mapped_components(&ids, &[component], &mappings_path, &context, execution_mode)
        }
        RunMode::Changed => run_changed_components(&mappings_path, &context, execution_mode),
    }
}

fn parse_run_options(args: &[String]) -> Result<RunOptions, String> {
    let mut mode = None;
    let mut mappings_path = DEFAULT_FATE_MAPPINGS_PATH.to_string();
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
                set_run_mode(&mut mode, RunMode::Component(component.clone()))?;
            }
            "--mappings" => {
                mappings_path = iter
                    .next()
                    .ok_or_else(|| "missing value for --mappings".to_string())?
                    .clone();
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
        context,
        execution_mode,
    })
}

fn set_run_mode(mode: &mut Option<RunMode>, new_mode: RunMode) -> Result<(), String> {
    if mode.is_some() {
        return Err("choose either --component <id> or --changed, not both".to_string());
    }
    *mode = Some(new_mode);
    Ok(())
}

fn run_changed_components(
    mappings_path: &str,
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
        context,
        execution_mode,
    )
}

fn run_mapped_components(
    component_ids: &[String],
    selected_components: &[String],
    mappings_path: &str,
    context: &FateContext,
    execution_mode: ExecutionMode,
) -> Result<(), String> {
    let mappings = load_fate_mappings(mappings_path, component_ids)?;
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
    mappings
        .iter()
        .map(|mapping| {
            let mapping = if check_prerequisites {
                resolve_fate_mapping(mapping, context)?
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

        let key = (component_id.to_string(), target.to_string());
        if seen_targets.iter().any(|seen| seen == &key) {
            return Err(format!(
                "invalid FATE mapping at line {line_number}: duplicate target `{target}` for component `{component_id}`"
            ));
        }
        seen_targets.push(key);

        let args = fields[4..]
            .iter()
            .map(|arg| (*arg).to_string())
            .collect::<Vec<_>>();

        mappings.push(FateMapping {
            component_id: component_id.to_string(),
            target: target.to_string(),
            workdir: normalize_path(workdir),
            program: program.to_string(),
            args,
        });
    }

    Ok(mappings)
}

fn run_fate_mapping(
    mapping: &FateMapping,
    context: &FateContext,
    execution_mode: ExecutionMode,
) -> Result<(), String> {
    let mapping = resolve_fate_mapping(mapping, context)?;

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

fn resolve_fate_mapping(
    mapping: &FateMapping,
    context: &FateContext,
) -> Result<FateMapping, String> {
    Ok(FateMapping {
        component_id: mapping.component_id.clone(),
        target: mapping.target.clone(),
        workdir: replace_mapping_placeholders(mapping, &mapping.workdir, context)?,
        program: replace_mapping_placeholders(mapping, &mapping.program, context)?,
        args: mapping
            .args
            .iter()
            .map(|arg| replace_mapping_placeholders(mapping, arg, context))
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn replace_mapping_placeholders(
    mapping: &FateMapping,
    value: &str,
    context: &FateContext,
) -> Result<String, String> {
    let mut value = value.to_string();

    if value.contains("{samples}") {
        let samples_root = required_samples_root(mapping, context)?;
        value = value.replace("{samples}", samples_root);
    }

    if value.contains("{oracle_ffmpeg}") {
        let oracle_ffmpeg = required_oracle_ffmpeg(mapping, context)?;
        value = value.replace("{oracle_ffmpeg}", oracle_ffmpeg);
    }

    Ok(value)
}

fn required_samples_root<'a>(
    mapping: &FateMapping,
    context: &'a FateContext,
) -> Result<&'a str, String> {
    let samples_root = context.samples_root.as_deref().ok_or_else(|| {
        format!(
            "FATE mapping {}:{} references {{samples}} but --samples <path> was not provided",
            mapping.component_id, mapping.target
        )
    })?;

    if !Path::new(samples_root).is_dir() {
        return Err(format!(
            "FATE mapping {}:{} requires --samples `{samples_root}` to be an existing directory",
            mapping.component_id, mapping.target
        ));
    }

    Ok(samples_root)
}

fn required_oracle_ffmpeg<'a>(
    mapping: &FateMapping,
    context: &'a FateContext,
) -> Result<&'a str, String> {
    let oracle_ffmpeg = context.oracle_ffmpeg.as_deref().ok_or_else(|| {
        format!(
            "FATE mapping {}:{} references {{oracle_ffmpeg}} but --oracle-ffmpeg <path> was not provided",
            mapping.component_id, mapping.target
        )
    })?;

    if !Path::new(oracle_ffmpeg).is_file() {
        return Err(format!(
            "FATE mapping {}:{} requires --oracle-ffmpeg `{oracle_ffmpeg}` to be an existing file",
            mapping.component_id, mapping.target
        ));
    }

    Ok(oracle_ffmpeg)
}

fn format_mapping_command(mapping: &FateMapping) -> String {
    let mut parts = Vec::with_capacity(mapping.args.len() + 1);
    parts.push(mapping.program.as_str());
    parts.extend(mapping.args.iter().map(String::as_str));
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
    path.starts_with("crates/")
        || path.starts_with("tests/fate/")
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
        "usage: fate-runner list | mappings [--check-prereqs] [--mappings <path>] [--samples <path>] [--oracle-ffmpeg <path>] | run [--dry-run] [--mappings <path>] [--samples <path>] [--oracle-ffmpeg <path>] --component <id> | run [--dry-run] [--mappings <path>] [--samples <path>] [--oracle-ffmpeg <path>] --changed"
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
                "crates/fftools/src/io_plan.rs".to_string(),
                "crates/fftools/src/ffmpeg.rs".to_string(),
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
        let selected_components =
            changed_components(&component_ids, &["xtask/src/main.rs".to_string()]);

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
            "crates/swscale/src/lib.rs".to_string(),
            "tests/fate/README.md".to_string(),
            "fuzz/Cargo.toml".to_string(),
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
                mode: RunMode::Component("fate-runner".to_string()),
                mappings_path: "custom.map".to_string(),
                context: FateContext::default(),
                execution_mode: ExecutionMode::Execute,
            }
        );

        assert_eq!(
            parse_run_options(&["--changed".to_string()]).unwrap(),
            RunOptions {
                mode: RunMode::Changed,
                mappings_path: DEFAULT_FATE_MAPPINGS_PATH.to_string(),
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
                context: FateContext::default(),
                execution_mode: ExecutionMode::DryRun,
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
    }

    #[test]
    fn parses_mapping_options_for_listing_and_prerequisite_checking() {
        assert_eq!(
            parse_mapping_options(&[]).unwrap(),
            MappingOptions {
                mappings_path: DEFAULT_FATE_MAPPINGS_PATH.to_string(),
                context: FateContext::default(),
                check_prerequisites: false,
            }
        );

        assert_eq!(
            parse_mapping_options(&[
                "--check-prereqs".to_string(),
                "--mappings".to_string(),
                "custom.map".to_string(),
                "--samples".to_string(),
                "tests/fate".to_string(),
                "--oracle-ffmpeg".to_string(),
                "Cargo.toml".to_string(),
            ])
            .unwrap(),
            MappingOptions {
                mappings_path: "custom.map".to_string(),
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
    }

    #[test]
    fn formats_mapping_commands_for_diagnostics() {
        let mapping = FateMapping {
            component_id: "fate-runner".to_string(),
            target: "local-self-test".to_string(),
            workdir: ".".to_string(),
            program: "cargo".to_string(),
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
    fn resolves_mapping_prerequisite_placeholders() {
        let mapping = FateMapping {
            component_id: "avformat-wav-demuxer".to_string(),
            target: "sample-framecrc".to_string(),
            workdir: "{samples}/audio".to_string(),
            program: "{oracle_ffmpeg}".to_string(),
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
    fn reports_missing_or_invalid_mapping_prerequisites() {
        let mapping = FateMapping {
            component_id: "avformat-wav-demuxer".to_string(),
            target: "sample-framecrc".to_string(),
            workdir: ".".to_string(),
            program: "cargo".to_string(),
            args: vec!["{samples}/audio/test.wav".to_string()],
        };

        assert!(resolve_fate_mapping(&mapping, &FateContext::default())
            .unwrap_err()
            .contains("references {samples} but --samples <path> was not provided"));

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
    fn mapping_report_lists_unresolved_commands_by_default() {
        let mappings = vec![FateMapping {
            component_id: "avformat-wav-demuxer".to_string(),
            target: "sample-framecrc".to_string(),
            workdir: "{samples}/audio".to_string(),
            program: "{oracle_ffmpeg}".to_string(),
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
            args: vec!["-version".to_string()],
        }];

        assert!(
            fate_mapping_report_lines(&mappings, &FateContext::default(), true)
                .unwrap_err()
                .contains("references {samples} but --samples <path> was not provided")
        );

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
            args: vec!["{samples}/audio/test.wav".to_string()],
        };

        assert!(
            run_fate_mapping(&mapping, &FateContext::default(), ExecutionMode::DryRun)
                .unwrap_err()
                .contains("references {samples} but --samples <path> was not provided")
        );
    }
}
