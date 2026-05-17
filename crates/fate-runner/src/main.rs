use std::env;
use std::fs;
use std::process::Command;

struct PathRule {
    path: &'static str,
    exact_ids: &'static [&'static str],
    id_prefixes: &'static [&'static str],
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

fn run_component(args: Vec<String>) -> Result<(), String> {
    if args.first().map(String::as_str) == Some("--changed") {
        return run_changed_components();
    }

    let mut component = None;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--component" => component = iter.next().cloned(),
            other => return Err(format!("unsupported run argument `{other}`")),
        }
    }

    let component = component.ok_or_else(|| "missing --component <id>".to_string())?;
    let ids = load_component_ids()?;
    if !ids.iter().any(|id| id == &component) {
        return Err(format!("unknown ledger component `{component}`"));
    }

    Err(format!(
        "component `{component}` has no runnable FATE mapping in the current slice"
    ))
}

fn run_changed_components() -> Result<(), String> {
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

    Err(format!(
        "no runnable FATE mappings exist yet for changed components: {}",
        components.join(", ")
    ))
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
    path.starts_with("crates/") || path.starts_with("tests/fate/")
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
    eprintln!("usage: fate-runner list | run --component <id> | run --changed");
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
        ]));
        let paths = vec![
            "crates\\fate-runner\\src\\main.rs".to_string(),
            "crates/avformat/src/rawvideo.rs".to_string(),
            "crates/avutil/src/error.rs".to_string(),
            "crates/avformat/src/rawvideo.rs".to_string(),
        ];

        assert_eq!(
            changed_components(&component_ids, &paths),
            vec![
                "avutil-error".to_string(),
                "avformat-rawvideo-demuxer".to_string(),
                "avformat-rawvideo-muxer".to_string(),
                "fate-runner".to_string(),
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
    fn unmapped_relevant_paths_report_crate_files_but_ignore_docs() {
        let paths = vec![
            "docs/architecture.md".to_string(),
            "crates/swscale/src/lib.rs".to_string(),
            "tests/fate/README.md".to_string(),
        ];

        assert_eq!(
            unmapped_relevant_paths(&paths),
            vec!["crates/swscale/src/lib.rs".to_string()]
        );
    }
}
