use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use avutil::{
    AvOptionRanges, ChannelLayout, ChannelLayoutSpec, Dictionary, MatchMode, OptionChild,
    OptionConstant, OptionDefinition, OptionEntryMatch, OptionFlags, OptionKind, OptionSearchFlags,
    OptionSerializeFlags, OptionSet, OptionValue, PixelFormat, Rational, RgbaColor, SampleFormat,
    SetMode,
};

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 libavutil oracle under third_party/ffmpeg-oracle/wsl"]
fn libavutil_option_helpers_match_current_model() {
    let repo_root = repo_root();
    let oracle_root = oracle_root(&repo_root);
    let include_dir = oracle_root.join("wsl/include");
    let libavutil = oracle_root.join("wsl/lib/libavutil.a");

    assert!(
        include_dir.join("libavutil/opt.h").is_file(),
        "missing pinned FFmpeg libavutil option headers under `{}`",
        include_dir.display()
    );
    assert!(
        libavutil.is_file(),
        "missing pinned FFmpeg libavutil static library `{}`",
        libavutil.display()
    );

    let work_dir = repo_root.join("target/oracle/avutil-options");
    fs::create_dir_all(&work_dir).expect("create avutil-options oracle work dir");
    let source = work_dir.join("options_oracle.c");
    let executable = work_dir.join("options_oracle");
    fs::write(&source, oracle_c_source()).expect("write avutil-options oracle C source");

    let stdout = compile_and_run_oracle(&include_dir, &libavutil, &source, &executable);
    let oracle = parse_oracle_output(&stdout);
    let expected = expected_rows();

    assert_eq!(
        oracle.keys().collect::<Vec<_>>(),
        expected.keys().collect::<Vec<_>>(),
        "oracle row set diverged"
    );

    for (name, expected_fields) in expected {
        assert_eq!(
            row_fields(&oracle, &name),
            expected_fields.as_slice(),
            "{name} diverged"
        );
    }
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 source/build cache; set FFMPEG_FATE_BUILD_DIR or run scripts/bootstrap_ffmpeg_oracle_wsl.sh"]
fn upstream_fate_opt_passes() {
    let output = if cfg!(windows) {
        let script = match env::var("FFMPEG_FATE_BUILD_DIR") {
            Ok(build_dir) => {
                let build_dir = if build_dir.starts_with('/') || build_dir.starts_with('~') {
                    build_dir
                } else {
                    to_wsl_path(Path::new(&build_dir))
                };
                format!(
                    "test -d {0} || {{ echo 'missing FFmpeg FATE build dir: {0}' >&2; exit 66; }}; make -C {0} fate-opt",
                    shell_quote(&build_dir)
                )
            }
            Err(_) => concat!(
                "build_dir=\"${FFMPEGRUST_ORACLE_WORK:-$HOME/.cache/ffmpegrust/ffmpeg-oracle-n8.1.1}/build\"; ",
                "test -d \"$build_dir\" || { echo \"missing FFmpeg FATE build dir: $build_dir\" >&2; exit 66; }; ",
                "make -C \"$build_dir\" fate-opt"
            )
            .to_string(),
        };
        Command::new("wsl")
            .args(["-d", "Ubuntu", "--exec", "bash", "-lc", &script])
            .output()
            .expect("run upstream FFmpeg fate-opt through WSL")
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
            .arg("fate-opt")
            .output()
            .unwrap_or_else(|err| {
                panic!(
                    "run upstream FFmpeg fate-opt in `{}`: {err}",
                    build_dir.display()
                )
            })
    };

    assert!(
        output.status.success(),
        "upstream FFmpeg fate-opt failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn expected_rows() -> BTreeMap<String, Vec<String>> {
    let mut rows = BTreeMap::new();
    insert_row(
        &mut rows,
        "flags",
        [
            OptionFlags::ENCODING_PARAM.bits().to_string(),
            OptionFlags::DECODING_PARAM.bits().to_string(),
            OptionFlags::AUDIO_PARAM.bits().to_string(),
            OptionFlags::VIDEO_PARAM.bits().to_string(),
            OptionFlags::SUBTITLE_PARAM.bits().to_string(),
            OptionFlags::EXPORT.bits().to_string(),
            OptionFlags::READONLY.bits().to_string(),
            OptionFlags::BSF_PARAM.bits().to_string(),
            OptionFlags::RUNTIME_PARAM.bits().to_string(),
            OptionFlags::FILTERING_PARAM.bits().to_string(),
            OptionFlags::DEPRECATED.bits().to_string(),
            OptionFlags::CHILD_CONSTS.bits().to_string(),
            OptionFlags::all().bits().to_string(),
        ],
    );
    insert_row(
        &mut rows,
        "types",
        [
            "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15", "16",
            "17", "18", "19", "20", "65536",
        ],
    );
    insert_row(
        &mut rows,
        "search-flags",
        [
            OptionSearchFlags::CHILDREN.bits().to_string(),
            OptionSearchFlags::FAKE_OBJ.bits().to_string(),
            OptionSearchFlags::ALLOW_NULL.bits().to_string(),
            OptionSearchFlags::ARRAY_REPLACE.bits().to_string(),
        ],
    );
    insert_row(
        &mut rows,
        "serialize-flags",
        [
            OptionSerializeFlags::SKIP_DEFAULTS.bits().to_string(),
            OptionSerializeFlags::OPT_FLAGS_EXACT.bits().to_string(),
            OptionSerializeFlags::SEARCH_CHILDREN.bits().to_string(),
        ],
    );

    let mut options = sample_options();
    rows.insert(
        "next:order".to_string(),
        options
            .avoption_entries()
            .into_iter()
            .map(|entry| entry.name().to_owned())
            .collect(),
    );
    insert_row(
        &mut rows,
        "find:root",
        [
            entry_name(options.find_avoption(
                "threads",
                None,
                OptionFlags::ENCODING_PARAM,
                OptionSearchFlags::empty(),
            )),
            entry_name(options.find_avoption(
                "THREADS",
                None,
                OptionFlags::empty(),
                OptionSearchFlags::empty(),
            )),
            entry_name(options.find_avoption(
                "fast",
                None,
                OptionFlags::empty(),
                OptionSearchFlags::empty(),
            )),
            entry_name(options.find_avoption(
                "fast",
                Some("PRESET"),
                OptionFlags::ENCODING_PARAM,
                OptionSearchFlags::empty(),
            )),
            entry_name(options.find_avoption(
                "slow",
                Some("preset"),
                OptionFlags::empty(),
                OptionSearchFlags::empty(),
            )),
            entry_name(options.find_avoption(
                "exported",
                None,
                OptionFlags::from_bits_truncate(
                    OptionFlags::EXPORT.bits() | OptionFlags::READONLY.bits(),
                ),
                OptionSearchFlags::empty(),
            )),
            entry_name(options.find_avoption(
                "exported",
                None,
                OptionFlags::VIDEO_PARAM,
                OptionSearchFlags::empty(),
            )),
        ],
    );
    rows.insert("state:defaults".to_string(), state_fields(&options));
    insert_row(
        &mut rows,
        "get:defaults",
        [
            ret_value(options.get_avoption_string("threads")),
            ret_value(options.get_avoption_string("bitexact")),
            ret_value(options.get_avoption_string("quality")),
            ret_value(options.get_avoption_string("aspect_ratio")),
            ret_value(options.get_avoption_string("metadata")),
            ret_value(options.get_avoption_string("preset_level")),
        ],
    );
    insert_row(
        &mut rows,
        "get:errors",
        [
            ret_value(options.get_avoption_string("THREADS")),
            ret_value(options.get_avoption_string("fast")),
        ],
    );
    insert_row(
        &mut rows,
        "get:allow-null",
        [
            ret_value(options.get_avoption_string("nullable_metadata")),
            ret_value(
                options.get_avoption_string_with_flags(
                    "nullable_metadata",
                    OptionSearchFlags::empty(),
                ),
            ),
            ret_nullable_value(options.get_avoption_string_nullable_with_flags(
                "nullable_metadata",
                OptionSearchFlags::ALLOW_NULL,
            )),
            ret_nullable_value(options.get_avoption_string_nullable_with_flags(
                "metadata",
                OptionSearchFlags::ALLOW_NULL,
            )),
            ret_nullable_value(options.get_avoption_string_nullable_with_flags(
                "nullable_metadata",
                OptionSearchFlags::from_bits_truncate(
                    OptionSearchFlags::ALLOW_NULL.bits() | OptionSearchFlags::FAKE_OBJ.bits(),
                ),
            )),
        ],
    );
    insert_row(
        &mut rows,
        "query-ranges:root",
        [
            ret_ranges(options.query_avoption_ranges("threads")),
            ret_ranges(options.query_avoption_ranges("bitexact")),
            ret_ranges(options.query_avoption_ranges("quality")),
            ret_ranges(options.query_avoption_ranges("aspect_ratio")),
            ret_ranges(options.query_avoption_ranges("metadata")),
            ret_ranges(options.query_avoption_ranges("preset_level")),
            ret_ranges(options.query_avoption_ranges("exported")),
            ret_ranges(options.query_avoption_ranges("THREADS")),
            ret_ranges(options.query_avoption_ranges("fast")),
        ],
    );

    let mut options_with_child = sample_options_with_child();
    insert_row(
        &mut rows,
        "find:children",
        [
            entry_target_name(options_with_child.find_avoption(
                "threads",
                None,
                OptionFlags::empty(),
                OptionSearchFlags::CHILDREN,
            )),
            entry_target_name(options_with_child.find_avoption(
                "threads",
                None,
                OptionFlags::ENCODING_PARAM,
                OptionSearchFlags::CHILDREN,
            )),
            entry_target_name(options_with_child.find_avoption(
                "child_only",
                None,
                OptionFlags::DECODING_PARAM,
                OptionSearchFlags::CHILDREN,
            )),
            entry_target_name(options_with_child.find_avoption(
                "child_only",
                None,
                OptionFlags::empty(),
                OptionSearchFlags::empty(),
            )),
        ],
    );
    insert_row(
        &mut rows,
        "get:children",
        [
            ret_value(
                options_with_child
                    .get_avoption_string_with_flags("child_only", OptionSearchFlags::empty()),
            ),
            ret_value(
                options_with_child
                    .get_avoption_string_with_flags("child_only", OptionSearchFlags::CHILDREN),
            ),
            ret_value(
                options_with_child
                    .get_avoption_string_with_flags("threads", OptionSearchFlags::CHILDREN),
            ),
            ret_value(
                options_with_child
                    .get_avoption_string_with_flags("threads", OptionSearchFlags::FAKE_OBJ),
            ),
        ],
    );
    insert_row(
        &mut rows,
        "ret:set-children",
        [
            ret(options_with_child.set_avoption_from_str_with_flags(
                "child_only",
                "7",
                OptionSearchFlags::empty(),
            )),
            ret(options_with_child.set_avoption_from_str_with_flags(
                "child_only",
                "7",
                OptionSearchFlags::CHILDREN,
            )),
            ret(options_with_child.set_avoption_from_str_with_flags(
                "threads",
                "9",
                OptionSearchFlags::CHILDREN,
            )),
            ret(options_with_child.set_avoption_from_str_with_flags(
                "child_readonly",
                "4",
                OptionSearchFlags::CHILDREN,
            )),
            ret(options_with_child.set_avoption_from_str_with_flags(
                "threads",
                "10",
                OptionSearchFlags::FAKE_OBJ,
            )),
        ],
    );
    rows.insert(
        "state:children-after-set".to_string(),
        child_state_fields(&options_with_child),
    );

    let mut dict_options = sample_options();
    let mut dict = Dictionary::new();
    for (key, value) in [
        ("threads", "11"),
        ("unknown", "first"),
        ("bitexact", "true"),
        ("unknown", "second"),
        ("metadata", "from-dict"),
    ] {
        dict.set_with_mode(key, value, MatchMode::CaseSensitive, SetMode::AllowMultiple)
            .unwrap();
    }
    let mut dict_result = vec![ret(
        dict_options.set_avoptions_from_dict(&mut dict, OptionSearchFlags::empty())
    )];
    dict_result.extend(dict_fields(&dict));
    rows.insert("ret:set-dict-root".to_string(), dict_result);
    rows.insert(
        "state:set-dict-root".to_string(),
        state_fields(&dict_options),
    );

    let mut dict_child_options = sample_options_with_child();
    let mut child_dict = Dictionary::new();
    for (key, value) in [
        ("threads", "9"),
        ("child_only", "6"),
        ("quality", "0.25"),
        ("unknown", "value"),
    ] {
        child_dict
            .set_with_mode(key, value, MatchMode::CaseSensitive, SetMode::AllowMultiple)
            .unwrap();
    }
    let mut child_dict_result = vec![ret(
        dict_child_options.set_avoptions_from_dict(&mut child_dict, OptionSearchFlags::CHILDREN)
    )];
    child_dict_result.extend(dict_fields(&child_dict));
    rows.insert("ret:set-dict-children".to_string(), child_dict_result);
    rows.insert(
        "state:set-dict-children".to_string(),
        child_dict_state_fields(&dict_child_options),
    );

    let mut dict_error_options = sample_options();
    let mut error_dict = Dictionary::new();
    for (key, value) in [
        ("threads", "13"),
        ("bitexact", "maybe"),
        ("unknown", "later"),
    ] {
        error_dict
            .set_with_mode(key, value, MatchMode::CaseSensitive, SetMode::AllowMultiple)
            .unwrap();
    }
    let mut error_dict_result = vec![ret(
        dict_error_options.set_avoptions_from_dict(&mut error_dict, OptionSearchFlags::empty())
    )];
    error_dict_result.extend(dict_fields(&error_dict));
    rows.insert("ret:set-dict-error".to_string(), error_dict_result);
    rows.insert(
        "state:set-dict-error".to_string(),
        state_fields(&dict_error_options),
    );

    let mut copy_source = sample_options_with_child();
    copy_source.set_avoption_from_str("threads", "12").unwrap();
    copy_source
        .set_avoption_from_str("bitexact", "true")
        .unwrap();
    copy_source
        .set_avoption_from_str("quality", "0.875")
        .unwrap();
    copy_source
        .set_avoption_from_str("aspect_ratio", "3/2")
        .unwrap();
    copy_source
        .set_avoption_from_str("metadata", "source")
        .unwrap();
    copy_source
        .set_avoption_from_str("preset_level", "slow")
        .unwrap();
    copy_source
        .set_child_from_str("decoder", "threads", "11")
        .unwrap();
    copy_source
        .set_child_from_str("decoder", "child_only", "6")
        .unwrap();

    let mut copy_destination = sample_options_with_child();
    copy_destination
        .set_avoption_from_str("threads", "3")
        .unwrap();
    copy_destination
        .set_avoption_from_str("quality", "0.125")
        .unwrap();
    copy_destination
        .set_avoption_from_str("aspect_ratio", "4/3")
        .unwrap();
    copy_destination
        .set_avoption_from_str("metadata", "destination")
        .unwrap();
    copy_destination
        .set_avoption_from_str("preset_level", "fast")
        .unwrap();
    copy_destination
        .set_child_from_str("decoder", "threads", "14")
        .unwrap();
    copy_destination
        .set_child_from_str("decoder", "child_only", "4")
        .unwrap();

    let copy_ret = copy_destination.copy_avoptions_from(&copy_source);
    insert_row(&mut rows, "ret:copy-root", [ret(copy_ret)]);
    rows.insert(
        "state:copy-root-src".to_string(),
        copy_state_fields(&copy_source),
    );
    rows.insert(
        "state:copy-root-dst".to_string(),
        copy_state_fields(&copy_destination),
    );
    copy_source
        .set_avoption_from_str("metadata", "mutated-source")
        .unwrap();
    rows.insert(
        "state:copy-root-dst-after-src-mutate".to_string(),
        copy_state_fields(&copy_destination),
    );

    let child_source = copy_source.child("decoder").unwrap().options().clone();
    let mut child_destination = copy_destination.child("decoder").unwrap().options().clone();
    let copy_child_ret = child_destination.copy_avoptions_from(&child_source);
    insert_row(&mut rows, "ret:copy-child", [ret(copy_child_ret)]);
    rows.insert(
        "state:copy-child-dst".to_string(),
        child_option_state_fields(&child_destination),
    );

    let mut mismatch_destination = OptionSet::new();
    mismatch_destination
        .define(
            OptionDefinition::new("other", OptionKind::Bool, OptionValue::Bool(false), "").unwrap(),
        )
        .unwrap();
    insert_row(
        &mut rows,
        "ret:copy-class-mismatch",
        [ret(mismatch_destination.copy_avoptions_from(&copy_source))],
    );

    let mut string_named = sample_options();
    insert_row(
        &mut rows,
        "ret:set-from-string-named",
        [ret_count(string_named.set_avoptions_from_string(
            "threads=7:quality=0.25:metadata=from-string",
            &[],
            "=",
            ":",
        ))],
    );
    rows.insert(
        "state:set-from-string-named".to_string(),
        state_fields(&string_named),
    );

    let mut string_shorthand = sample_options();
    insert_row(
        &mut rows,
        "ret:set-from-string-shorthand",
        [ret_count(string_shorthand.set_avoptions_from_string(
            " 9 : yes : metadata = shorthand ",
            &["threads", "bitexact"],
            "=",
            ":",
        ))],
    );
    rows.insert(
        "state:set-from-string-shorthand".to_string(),
        state_fields(&string_shorthand),
    );

    let mut string_after_named_error = sample_options();
    insert_row(
        &mut rows,
        "ret:set-from-string-after-named-error",
        [ret_count(
            string_after_named_error.set_avoptions_from_string(
                "10:quality=0.75:no",
                &["threads", "bitexact"],
                "=",
                ":",
            ),
        )],
    );
    rows.insert(
        "state:set-from-string-after-named-error".to_string(),
        state_fields(&string_after_named_error),
    );

    let mut string_after_explicit_shorthand_error = sample_options();
    insert_row(
        &mut rows,
        "ret:set-from-string-after-explicit-shorthand-error",
        [ret_count(
            string_after_explicit_shorthand_error.set_avoptions_from_string(
                "threads=10:yes:quality=0.75",
                &["threads", "bitexact"],
                "=",
                ":",
            ),
        )],
    );
    rows.insert(
        "state:set-from-string-after-explicit-shorthand-error".to_string(),
        state_fields(&string_after_explicit_shorthand_error),
    );

    let mut string_set_error = sample_options();
    insert_row(
        &mut rows,
        "ret:set-from-string-set-error",
        [ret_count(string_set_error.set_avoptions_from_string(
            "threads=11:bitexact=maybe",
            &[],
            "=",
            ":",
        ))],
    );
    rows.insert(
        "state:set-from-string-set-error".to_string(),
        state_fields(&string_set_error),
    );

    let mut string_not_found = sample_options();
    insert_row(
        &mut rows,
        "ret:set-from-string-not-found",
        [ret_count(string_not_found.set_avoptions_from_string(
            "threads=12:unknown=1",
            &[],
            "=",
            ":",
        ))],
    );
    rows.insert(
        "state:set-from-string-not-found".to_string(),
        state_fields(&string_not_found),
    );

    let mut string_no_shorthand = sample_options();
    insert_row(
        &mut rows,
        "ret:set-from-string-no-shorthand",
        [ret_count(string_no_shorthand.set_avoptions_from_string(
            "12",
            &[],
            "=",
            ":",
        ))],
    );
    rows.insert(
        "state:set-from-string-no-shorthand".to_string(),
        state_fields(&string_no_shorthand),
    );

    let mut string_invalid_separators = sample_options();
    insert_row(
        &mut rows,
        "ret:set-from-string-invalid-separators",
        [
            ret_count(string_invalid_separators.set_avoptions_from_string(
                "threads=7",
                &[],
                "",
                ":",
            )),
            ret_count(string_invalid_separators.set_avoptions_from_string(
                "threads=7",
                &[],
                "=",
                "",
            )),
            ret_count(string_invalid_separators.set_avoptions_from_string(
                "threads=7",
                &[],
                ":=",
                ":",
            )),
        ],
    );

    let mut string_empty_pairs_multi = sample_options();
    insert_row(
        &mut rows,
        "ret:set-from-string-empty-pairs-multi",
        [ret_count(
            string_empty_pairs_multi.set_avoptions_from_string(
                "threads=7:quality=0.25",
                &[],
                "=",
                "",
            ),
        )],
    );
    rows.insert(
        "state:set-from-string-empty-pairs-multi".to_string(),
        state_fields(&string_empty_pairs_multi),
    );

    let mut string_empty_pairs_embedded_equals = sample_options();
    insert_row(
        &mut rows,
        "ret:set-from-string-empty-pairs-embedded-equals",
        [ret_count(
            string_empty_pairs_embedded_equals.set_avoptions_from_string(
                "metadata=title=clip",
                &[],
                "=",
                "",
            ),
        )],
    );
    rows.insert(
        "state:set-from-string-empty-pairs-embedded-equals".to_string(),
        state_fields(&string_empty_pairs_embedded_equals),
    );

    let mut string_trailing_pair_sep = sample_options();
    insert_row(
        &mut rows,
        "ret:set-from-string-trailing-pair-sep",
        [ret_count(
            string_trailing_pair_sep.set_avoptions_from_string("threads=7:", &[], "=", ":"),
        )],
    );
    rows.insert(
        "state:set-from-string-trailing-pair-sep".to_string(),
        state_fields(&string_trailing_pair_sep),
    );

    let mut string_shorthand_overflow = sample_options();
    insert_row(
        &mut rows,
        "ret:set-from-string-shorthand-overflow",
        [ret_count(
            string_shorthand_overflow.set_avoptions_from_string(
                "9:yes:15",
                &["threads", "bitexact"],
                "=",
                ":",
            ),
        )],
    );
    rows.insert(
        "state:set-from-string-shorthand-overflow".to_string(),
        state_fields(&string_shorthand_overflow),
    );

    let mut string_escaped = sample_options();
    insert_row(
        &mut rows,
        "ret:set-from-string-escaped",
        [ret_count(string_escaped.set_avoptions_from_string(
            "metadata=title\\:clip\\=one\\\\two:threads=14:preset_level=slow",
            &[],
            "=",
            ":",
        ))],
    );
    rows.insert(
        "state:set-from-string-escaped".to_string(),
        state_fields(&string_escaped),
    );

    let mut string_quoted = sample_options();
    insert_row(
        &mut rows,
        "ret:set-from-string-quoted",
        [ret_count(string_quoted.set_avoptions_from_string(
            "metadata=' title : clip = one ':threads=15",
            &[],
            "=",
            ":",
        ))],
    );
    insert_row(
        &mut rows,
        "ret:set-from-string-empty-key",
        [ret_count(sample_options().set_avoptions_from_string(
            "=7",
            &[],
            "=",
            ":",
        ))],
    );
    insert_row(
        &mut rows,
        "ret:set-from-string-empty-value",
        [ret_count(sample_options().set_avoptions_from_string(
            "metadata=",
            &[],
            "=",
            ":",
        ))],
    );
    let mut string_empty_value = sample_options();
    let _ = string_empty_value.set_avoptions_from_string("metadata=", &[], "=", ":");
    rows.insert(
        "state:set-from-string-empty-value".to_string(),
        state_fields(&string_empty_value),
    );
    insert_row(
        &mut rows,
        "ret:set-from-string-unclosed-quote",
        [ret_count(sample_options().set_avoptions_from_string(
            "metadata='title",
            &[],
            "=",
            ":",
        ))],
    );
    insert_row(
        &mut rows,
        "ret:set-from-string-quote-escape",
        [ret_count(sample_options().set_avoptions_from_string(
            "metadata='\\''x'",
            &[],
            "=",
            ":",
        ))],
    );
    rows.insert(
        "state:set-from-string-quoted".to_string(),
        state_fields(&string_quoted),
    );
    let mut string_quoted_escape = string_quoted.clone();
    assert!(string_quoted_escape
        .set_avoptions_from_string("metadata='\\''x'", &[], "=", ":")
        .is_ok());
    rows.insert(
        "state:set-from-string-quote-escape".to_string(),
        state_fields(&string_quoted_escape),
    );

    let serialize_defaults = sample_options();
    insert_row(
        &mut rows,
        "serialize:defaults",
        [
            ret_serialize(serialize_defaults.serialize_avoptions(
                OptionFlags::empty(),
                OptionSerializeFlags::empty(),
                '=',
                ',',
            )),
            ret_serialize(serialize_defaults.serialize_avoptions(
                OptionFlags::ENCODING_PARAM,
                OptionSerializeFlags::OPT_FLAGS_EXACT,
                '=',
                ',',
            )),
            ret_serialize(serialize_defaults.serialize_avoptions(
                OptionFlags::empty(),
                OptionSerializeFlags::OPT_FLAGS_EXACT,
                '=',
                ',',
            )),
            ret_serialize(serialize_defaults.serialize_avoptions(
                OptionFlags::EXPORT,
                OptionSerializeFlags::empty(),
                '=',
                ',',
            )),
        ],
    );

    let mut serialize_changed = sample_options();
    serialize_changed
        .set_avoption_from_str("threads", "8")
        .unwrap();
    serialize_changed
        .set_avoption_from_str("bitexact", "true")
        .unwrap();
    serialize_changed
        .set_avoption_from_str("metadata", "title=clip,segment\\one")
        .unwrap();
    serialize_changed
        .set_avoption_from_str("preset_level", "slow")
        .unwrap();
    insert_row(
        &mut rows,
        "serialize:skip-defaults",
        [ret_serialize(serialize_changed.serialize_avoptions(
            OptionFlags::empty(),
            OptionSerializeFlags::SKIP_DEFAULTS,
            '=',
            ',',
        ))],
    );

    let serialize_children = sample_options_with_child();
    insert_row(
        &mut rows,
        "serialize:children",
        [
            ret_serialize(serialize_children.serialize_avoptions(
                OptionFlags::empty(),
                OptionSerializeFlags::SEARCH_CHILDREN,
                '=',
                ',',
            )),
            ret_serialize(serialize_children.serialize_avoptions(
                OptionFlags::DECODING_PARAM,
                OptionSerializeFlags::SEARCH_CHILDREN,
                '=',
                ',',
            )),
        ],
    );
    insert_row(
        &mut rows,
        "serialize:invalid-separators",
        [
            ret_serialize(serialize_defaults.serialize_avoptions(
                OptionFlags::empty(),
                OptionSerializeFlags::empty(),
                '=',
                '=',
            )),
            ret_serialize(serialize_defaults.serialize_avoptions(
                OptionFlags::empty(),
                OptionSerializeFlags::empty(),
                '\\',
                ',',
            )),
            ret_serialize(serialize_defaults.serialize_avoptions(
                OptionFlags::empty(),
                OptionSerializeFlags::empty(),
                '=',
                '\0',
            )),
        ],
    );

    let exact_error_results = [
        ret(options.set_avoption_from_str("THREADS", "9")),
        ret(options.set_avoption_from_str("preset_level", "SLOW")),
        ret(options.set_avoption_from_str("fast", "2")),
    ];
    insert_row(&mut rows, "ret:set-exact-errors", exact_error_results);
    rows.insert(
        "state:after-exact-errors".to_string(),
        state_fields(&options),
    );

    let set_results = [
        ret(options.set_avoption_from_str("threads", "8")),
        ret(options.set_avoption_from_str("bitexact", "yes")),
        ret(options.set_avoption_from_str("quality", "0.75")),
        ret(options.set_avoption_from_str("aspect_ratio", "4/3")),
        ret(options.set_avoption_from_str("metadata", "title=clip")),
        ret(options.set_avoption_from_str("preset_level", "slow")),
    ];
    insert_row(&mut rows, "ret:set-supported", set_results);
    rows.insert("state:set-supported".to_string(), state_fields(&options));
    insert_row(
        &mut rows,
        "get:set-supported",
        [
            ret_value(options.get_avoption_string("threads")),
            ret_value(options.get_avoption_string("bitexact")),
            ret_value(options.get_avoption_string("quality")),
            ret_value(options.get_avoption_string("aspect_ratio")),
            ret_value(options.get_avoption_string("metadata")),
            ret_value(options.get_avoption_string("preset_level")),
        ],
    );

    let mut expression_options = sample_options();
    insert_row(
        &mut rows,
        "ret:set-expressions",
        [
            ret(expression_options.set_avoption_from_str("threads", " 2 * 3 ")),
            ret(expression_options.set_avoption_from_str("quality", "500m")),
            ret(expression_options.set_avoption_from_str("aspect_ratio", "1+1/2")),
            ret(expression_options.set_avoption_from_str("preset_level", "slow+2")),
        ],
    );
    rows.insert(
        "state:set-expressions".to_string(),
        state_fields(&expression_options),
    );
    insert_row(
        &mut rows,
        "get:set-expressions",
        [
            ret_value(expression_options.get_avoption_string("threads")),
            ret_value(expression_options.get_avoption_string("quality")),
            ret_value(expression_options.get_avoption_string("aspect_ratio")),
            ret_value(expression_options.get_avoption_string("preset_level")),
        ],
    );
    insert_row(
        &mut rows,
        "ret:set-expression-errors",
        [
            ret(expression_options.set_avoption_from_str("threads", "1K")),
            ret(expression_options.set_avoption_from_str("quality", "2*")),
        ],
    );
    rows.insert(
        "state:after-expression-errors".to_string(),
        state_fields(&expression_options),
    );

    let mut expression_constants = expression_options.clone();
    insert_row(
        &mut rows,
        "ret:set-expression-constants",
        [ret(
            expression_constants.set_avoption_from_str("threads", "PI")
        )],
    );
    insert_row(
        &mut rows,
        "get:set-expression-constants",
        [ret_value(
            expression_constants.get_avoption_string("threads"),
        )],
    );
    rows.insert(
        "state:set-expression-constants".to_string(),
        state_fields(&expression_constants),
    );

    let mut typed_options = sample_options();
    insert_row(
        &mut rows,
        "ret:set-typed",
        [
            ret(typed_options.set_avoption_int("threads", 21)),
            ret(typed_options.set_avoption_int("bitexact", 1)),
            ret(typed_options.set_avoption_double("quality", 0.625)),
            ret(typed_options.set_avoption_q("aspect_ratio", Rational::new(3, 2).unwrap())),
            ret(typed_options.set_avoption_int("preset_level", 6)),
        ],
    );
    rows.insert("state:set-typed".to_string(), state_fields(&typed_options));
    insert_row(
        &mut rows,
        "get:set-typed",
        [
            ret_i64(typed_options.get_avoption_int("threads")),
            ret_f64(typed_options.get_avoption_double("quality")),
            ret_q(typed_options.get_avoption_q("aspect_ratio")),
            ret_i64(typed_options.get_avoption_int("bitexact")),
            ret_f64(typed_options.get_avoption_double("threads")),
            ret_q(typed_options.get_avoption_q("threads")),
            ret_i64(typed_options.get_avoption_int("quality")),
        ],
    );
    insert_row(
        &mut rows,
        "ret:set-typed-errors",
        [
            ret(typed_options.set_avoption_int("metadata", 1)),
            ret(typed_options.set_avoption_int("threads", 128)),
            ret(typed_options.set_avoption_int("exported", 1)),
        ],
    );
    insert_row(
        &mut rows,
        "get:typed-errors",
        [
            ret_i64(typed_options.get_avoption_int("metadata")),
            ret_i64(typed_options.get_avoption_int("missing")),
        ],
    );
    rows.insert(
        "state:after-typed-errors".to_string(),
        state_fields(&typed_options),
    );

    let mut typed_children = sample_options_with_child();
    insert_row(
        &mut rows,
        "ret:set-typed-children",
        [
            ret(typed_children.set_avoption_int_with_flags(
                "threads",
                9,
                OptionSearchFlags::CHILDREN,
            )),
            ret(typed_children.set_avoption_int_with_flags(
                "child_only",
                7,
                OptionSearchFlags::CHILDREN,
            )),
            ret(typed_children.set_avoption_int_with_flags(
                "threads",
                10,
                OptionSearchFlags::FAKE_OBJ,
            )),
        ],
    );
    insert_row(
        &mut rows,
        "get:typed-children",
        [
            ret_i64(
                typed_children.get_avoption_int_with_flags("threads", OptionSearchFlags::CHILDREN),
            ),
            ret_i64(
                typed_children
                    .get_avoption_int_with_flags("child_only", OptionSearchFlags::CHILDREN),
            ),
            ret_i64(
                typed_children.get_avoption_int_with_flags("threads", OptionSearchFlags::FAKE_OBJ),
            ),
        ],
    );
    rows.insert(
        "state:set-typed-children".to_string(),
        child_state_fields(&typed_children),
    );

    let mut duration_set = duration_options();
    let ret_duration_15 = ret(duration_set.set_avoption_from_str("timeout", "1.5"));
    let duration_after_15 = duration_value(&duration_set, "timeout").to_string();
    let ret_duration_clock = ret(duration_set.set_avoption_from_str("timeout", "00:01:02.250"));
    let duration_after_clock = duration_value(&duration_set, "timeout").to_string();
    let ret_duration_ms = ret(duration_set.set_avoption_from_str("timeout", "1500ms"));
    let duration_after_ms = duration_value(&duration_set, "timeout").to_string();
    let ret_duration_us = ret(duration_set.set_avoption_from_str("timeout", "42us"));
    let duration_after_us = duration_value(&duration_set, "timeout").to_string();
    insert_row(
        &mut rows,
        "ret:set-duration-strings",
        [
            ret_duration_15,
            ret_duration_clock,
            ret_duration_ms,
            ret_duration_us,
        ],
    );
    insert_row(
        &mut rows,
        "state:set-duration-strings",
        [
            duration_after_15,
            duration_after_clock,
            duration_after_ms,
            duration_after_us,
        ],
    );
    insert_row(
        &mut rows,
        "get:set-duration-strings",
        [ret_value(duration_set.get_avoption_string("timeout"))],
    );
    insert_row(
        &mut rows,
        "ret:set-duration-errors",
        [
            ret(duration_set.set_avoption_from_str("timeout", "bad")),
            ret(duration_set.set_avoption_from_str("timeout", "-1")),
        ],
    );
    insert_row(
        &mut rows,
        "state:after-duration-errors",
        [duration_value(&duration_set, "timeout").to_string()],
    );

    let mut typed_duration_options = duration_options();
    insert_row(
        &mut rows,
        "ret:set-duration-typed",
        [ret(
            typed_duration_options.set_avoption_int("timeout", 90_500_000)
        )],
    );
    insert_row(
        &mut rows,
        "state:set-duration-typed",
        [duration_value(&typed_duration_options, "timeout").to_string()],
    );
    insert_row(
        &mut rows,
        "get:set-duration-typed",
        [
            ret_i64(typed_duration_options.get_avoption_int("timeout")),
            ret_f64(typed_duration_options.get_avoption_double("timeout")),
            ret_q(typed_duration_options.get_avoption_q("timeout")),
            ret_value(typed_duration_options.get_avoption_string("timeout")),
        ],
    );
    insert_row(
        &mut rows,
        "query-ranges:duration",
        [
            ret_ranges(typed_duration_options.query_avoption_ranges("timeout")),
            ret_ranges(typed_duration_options.query_avoption_ranges("missing")),
        ],
    );

    let image_defaults = image_size_options();
    insert_row(
        &mut rows,
        "state:image-size-defaults",
        image_size_state_fields(&image_defaults),
    );
    insert_row(
        &mut rows,
        "get:image-size-defaults",
        [
            ret_image_size(image_defaults.get_avoption_image_size("size")),
            ret_value(image_defaults.get_avoption_string("size")),
        ],
    );

    let mut image_set = image_size_options();
    let ret_640 = ret(image_set.set_avoption_from_str("size", "640x480"));
    let after_640 = image_size_value(&image_set, "size");
    let ret_hd720 = ret(image_set.set_avoption_from_str("size", "hd720"));
    let after_hd720 = image_size_value(&image_set, "size");
    let ret_none = ret(image_set.set_avoption_from_str("size", "none"));
    let after_none = image_size_value(&image_set, "size");
    insert_row(
        &mut rows,
        "ret:set-image-size-strings",
        [ret_640, ret_hd720, ret_none],
    );
    insert_row(
        &mut rows,
        "state:set-image-size-strings",
        [
            after_640.0.to_string(),
            after_640.1.to_string(),
            after_hd720.0.to_string(),
            after_hd720.1.to_string(),
            after_none.0.to_string(),
            after_none.1.to_string(),
        ],
    );
    insert_row(
        &mut rows,
        "get:set-image-size-strings",
        [
            ret_image_size(image_set.get_avoption_image_size("size")),
            ret_value(image_set.get_avoption_string("size")),
        ],
    );
    insert_row(
        &mut rows,
        "ret:set-image-size-errors",
        [
            ret(image_set.set_avoption_from_str("size", "bad")),
            ret(image_set.set_avoption_from_str("size", "0x480")),
        ],
    );
    insert_row(
        &mut rows,
        "state:after-image-size-errors",
        image_size_state_fields(&image_set),
    );

    let mut typed_image_options = image_size_options();
    insert_row(
        &mut rows,
        "ret:set-image-size-typed",
        [
            ret(typed_image_options.set_avoption_image_size("size", 800, 600)),
            ret(typed_image_options.set_avoption_image_size("size", -1, 480)),
            ret(typed_image_options.set_avoption_image_size("scalar", 1, 1)),
            ret(typed_image_options.set_avoption_int("size", 10)),
        ],
    );
    insert_row(
        &mut rows,
        "state:set-image-size-typed",
        image_size_state_fields(&typed_image_options),
    );
    insert_row(
        &mut rows,
        "get:set-image-size-typed",
        [
            ret_image_size(typed_image_options.get_avoption_image_size("size")),
            ret_value(typed_image_options.get_avoption_string("size")),
            ret_image_size(typed_image_options.get_avoption_image_size("scalar")),
            ret_i64(typed_image_options.get_avoption_int("size")),
        ],
    );
    insert_row(
        &mut rows,
        "query-ranges:image-size",
        [
            ret_ranges(typed_image_options.query_avoption_ranges("size")),
            ret_ranges(typed_image_options.query_avoption_ranges("missing")),
        ],
    );

    let mut image_child_options = image_size_parent_with_child_options();
    insert_row(
        &mut rows,
        "get:image-size-children",
        [
            ret_image_size(
                image_child_options
                    .get_avoption_image_size_with_flags("child_size", OptionSearchFlags::empty()),
            ),
            ret_image_size(
                image_child_options
                    .get_avoption_image_size_with_flags("child_size", OptionSearchFlags::CHILDREN),
            ),
            ret_value(
                image_child_options
                    .get_avoption_string_with_flags("child_size", OptionSearchFlags::CHILDREN),
            ),
        ],
    );
    insert_row(
        &mut rows,
        "ret:set-image-size-children",
        [
            ret(image_child_options.set_avoption_image_size_with_flags(
                "child_size",
                800,
                600,
                OptionSearchFlags::empty(),
            )),
            ret(image_child_options.set_avoption_image_size_with_flags(
                "child_size",
                800,
                600,
                OptionSearchFlags::CHILDREN,
            )),
            ret(image_child_options.set_avoption_image_size_with_flags(
                "child_size",
                1024,
                768,
                OptionSearchFlags::FAKE_OBJ,
            )),
        ],
    );
    rows.insert(
        "state:image-size-children".to_string(),
        image_size_child_state_fields(&image_child_options),
    );
    insert_row(
        &mut rows,
        "get:set-image-size-children",
        [
            ret_image_size(
                image_child_options
                    .get_avoption_image_size_with_flags("child_size", OptionSearchFlags::CHILDREN),
            ),
            ret_value(
                image_child_options
                    .get_avoption_string_with_flags("child_size", OptionSearchFlags::CHILDREN),
            ),
        ],
    );

    let pixel_defaults = pixel_format_options();
    insert_row(
        &mut rows,
        "state:pixel-format-defaults",
        pixel_format_state_fields(&pixel_defaults),
    );
    insert_row(
        &mut rows,
        "get:pixel-format-defaults",
        [
            ret_pixel_format(pixel_defaults.get_avoption_pixel_format("pix_fmt")),
            ret_value(pixel_defaults.get_avoption_string("pix_fmt")),
            ret_i64(pixel_defaults.get_avoption_int("pix_fmt")),
        ],
    );

    let mut pixel_set = pixel_format_options();
    let ret_rgb24 = ret(pixel_set.set_avoption_from_str("pix_fmt", "rgb24"));
    let after_rgb24 = pixel_format_value(&pixel_set, "pix_fmt");
    let ret_gray = ret(pixel_set.set_avoption_from_str("pix_fmt", "gray"));
    let after_gray = pixel_format_value(&pixel_set, "pix_fmt");
    let ret_none = ret(pixel_set.set_avoption_from_str("pix_fmt", "none"));
    let after_none = pixel_format_value(&pixel_set, "pix_fmt");
    let ret_numeric = ret(pixel_set.set_avoption_from_str("pix_fmt", "0x3"));
    let after_numeric = pixel_format_value(&pixel_set, "pix_fmt");
    let ret_high_name = ret(pixel_set.set_avoption_from_str("pix_fmt", "gbrap32le"));
    let after_high_name = pixel_format_value(&pixel_set, "pix_fmt");
    let ret_high_numeric = ret(pixel_set.set_avoption_from_str("pix_fmt", "259"));
    let after_high_numeric = pixel_format_value(&pixel_set, "pix_fmt");
    let ret_hardware_name = ret(pixel_set.set_avoption_from_str("pix_fmt", "vaapi"));
    let after_hardware_name = pixel_format_value(&pixel_set, "pix_fmt");
    let ret_hardware_numeric = ret(pixel_set.set_avoption_from_str("pix_fmt", "227"));
    let after_hardware_numeric = pixel_format_value(&pixel_set, "pix_fmt");
    let ret_hardware_last = ret(pixel_set.set_avoption_from_str("pix_fmt", "ohcodec"));
    let after_hardware_last = pixel_format_value(&pixel_set, "pix_fmt");
    insert_row(
        &mut rows,
        "ret:set-pixel-format-strings",
        [
            ret_rgb24,
            ret_gray,
            ret_none,
            ret_numeric,
            ret_high_name,
            ret_high_numeric,
            ret_hardware_name,
            ret_hardware_numeric,
            ret_hardware_last,
        ],
    );
    insert_row(
        &mut rows,
        "state:set-pixel-format-strings",
        [
            pixel_format_field(after_rgb24),
            pixel_format_field(after_gray),
            pixel_format_field(after_none),
            pixel_format_field(after_numeric),
            pixel_format_field(after_high_name),
            pixel_format_field(after_high_numeric),
            pixel_format_field(after_hardware_name),
            pixel_format_field(after_hardware_numeric),
            pixel_format_field(after_hardware_last),
        ],
    );
    insert_row(
        &mut rows,
        "get:set-pixel-format-strings",
        [ret_value(pixel_set.get_avoption_string("pix_fmt"))],
    );
    insert_row(
        &mut rows,
        "ret:set-pixel-format-errors",
        [
            ret(pixel_set.set_avoption_from_str("pix_fmt", "bad")),
            ret(pixel_set.set_avoption_from_str("pix_fmt", "267")),
            ret(pixel_set.set_avoption_from_str("pix_fmt", "-1")),
        ],
    );
    insert_row(
        &mut rows,
        "state:after-pixel-format-errors",
        pixel_format_state_fields(&pixel_set),
    );

    let mut typed_pixel_options = pixel_format_options();
    insert_row(
        &mut rows,
        "ret:set-pixel-format-typed",
        [
            ret(typed_pixel_options.set_avoption_pixel_format("pix_fmt", Some(PixelFormat::Rgb24))),
            ret(typed_pixel_options.set_avoption_pixel_format("pix_fmt", None)),
            ret(typed_pixel_options.set_avoption_pixel_format("scalar", Some(PixelFormat::Rgb24))),
            ret(typed_pixel_options.set_avoption_int("pix_fmt", 3)),
            ret(typed_pixel_options.set_avoption_int("pix_fmt", 267)),
            ret(typed_pixel_options.set_avoption_int("scalar", 6)),
            ret(typed_pixel_options.set_avoption_pixel_format("pix_fmt", Some(PixelFormat::Vaapi))),
            ret(typed_pixel_options.set_avoption_int("pix_fmt", 266)),
        ],
    );
    insert_row(
        &mut rows,
        "state:set-pixel-format-typed",
        pixel_format_state_fields(&typed_pixel_options),
    );
    insert_row(
        &mut rows,
        "get:set-pixel-format-typed",
        [
            ret_pixel_format(typed_pixel_options.get_avoption_pixel_format("pix_fmt")),
            ret_i64(typed_pixel_options.get_avoption_int("pix_fmt")),
            ret_f64(typed_pixel_options.get_avoption_double("pix_fmt")),
            ret_q(typed_pixel_options.get_avoption_q("pix_fmt")),
            ret_value(typed_pixel_options.get_avoption_string("pix_fmt")),
            ret_pixel_format(typed_pixel_options.get_avoption_pixel_format("scalar")),
        ],
    );
    insert_row(
        &mut rows,
        "query-ranges:pixel-format",
        [
            ret_ranges(typed_pixel_options.query_avoption_ranges("pix_fmt")),
            ret_ranges(typed_pixel_options.query_avoption_ranges("missing")),
        ],
    );

    let sample_defaults = sample_format_options();
    insert_row(
        &mut rows,
        "state:sample-format-defaults",
        sample_format_state_fields(&sample_defaults),
    );
    insert_row(
        &mut rows,
        "get:sample-format-defaults",
        [
            ret_sample_format(sample_defaults.get_avoption_sample_format("sample_fmt")),
            ret_value(sample_defaults.get_avoption_string("sample_fmt")),
            ret_i64(sample_defaults.get_avoption_int("sample_fmt")),
        ],
    );

    let mut sample_set = sample_format_options();
    let ret_fltp = ret(sample_set.set_avoption_from_str("sample_fmt", "fltp"));
    let after_fltp = sample_format_value(&sample_set, "sample_fmt");
    let ret_none = ret(sample_set.set_avoption_from_str("sample_fmt", "none"));
    let after_none = sample_format_value(&sample_set, "sample_fmt");
    let ret_numeric = ret(sample_set.set_avoption_from_str("sample_fmt", "0x4"));
    let after_numeric = sample_format_value(&sample_set, "sample_fmt");
    insert_row(
        &mut rows,
        "ret:set-sample-format-strings",
        [ret_fltp, ret_none, ret_numeric],
    );
    insert_row(
        &mut rows,
        "state:set-sample-format-strings",
        [
            sample_format_field(after_fltp),
            sample_format_field(after_none),
            sample_format_field(after_numeric),
        ],
    );
    insert_row(
        &mut rows,
        "get:set-sample-format-strings",
        [ret_value(sample_set.get_avoption_string("sample_fmt"))],
    );
    insert_row(
        &mut rows,
        "ret:set-sample-format-errors",
        [
            ret(sample_set.set_avoption_from_str("sample_fmt", "bad")),
            ret(sample_set.set_avoption_from_str("sample_fmt", "12")),
            ret(sample_set.set_avoption_from_str("sample_fmt", "-1")),
        ],
    );
    insert_row(
        &mut rows,
        "state:after-sample-format-errors",
        sample_format_state_fields(&sample_set),
    );

    let mut typed_sample_options = sample_format_options();
    insert_row(
        &mut rows,
        "ret:set-sample-format-typed",
        [
            ret(typed_sample_options
                .set_avoption_sample_format("sample_fmt", Some(SampleFormat::S32P))),
            ret(typed_sample_options.set_avoption_sample_format("sample_fmt", None)),
            ret(typed_sample_options.set_avoption_sample_format("scalar", Some(SampleFormat::S16))),
            ret(typed_sample_options.set_avoption_int("sample_fmt", 10)),
            ret(typed_sample_options.set_avoption_int("sample_fmt", 12)),
            ret(typed_sample_options.set_avoption_int("scalar", 6)),
        ],
    );
    insert_row(
        &mut rows,
        "state:set-sample-format-typed",
        sample_format_state_fields(&typed_sample_options),
    );
    insert_row(
        &mut rows,
        "get:set-sample-format-typed",
        [
            ret_sample_format(typed_sample_options.get_avoption_sample_format("sample_fmt")),
            ret_i64(typed_sample_options.get_avoption_int("sample_fmt")),
            ret_f64(typed_sample_options.get_avoption_double("sample_fmt")),
            ret_q(typed_sample_options.get_avoption_q("sample_fmt")),
            ret_value(typed_sample_options.get_avoption_string("sample_fmt")),
            ret_sample_format(typed_sample_options.get_avoption_sample_format("scalar")),
        ],
    );
    insert_row(
        &mut rows,
        "query-ranges:sample-format",
        [
            ret_ranges(typed_sample_options.query_avoption_ranges("sample_fmt")),
            ret_ranges(typed_sample_options.query_avoption_ranges("missing")),
        ],
    );

    let channel_layout_defaults = channel_layout_options();
    insert_row(
        &mut rows,
        "state:channel-layout-defaults",
        channel_layout_state_fields(&channel_layout_defaults),
    );
    insert_row(
        &mut rows,
        "get:channel-layout-defaults",
        [
            ret_channel_layout(channel_layout_defaults.get_avoption_channel_layout("layout")),
            ret_value(channel_layout_defaults.get_avoption_string("layout")),
            ret_i64(channel_layout_defaults.get_avoption_int("layout")),
        ],
    );

    let mut channel_layout_set = channel_layout_options();
    let ret_mono = ret(channel_layout_set.set_avoption_from_str("layout", "mono"));
    let after_mono = channel_layout_value(&channel_layout_set, "layout");
    let ret_five_one = ret(channel_layout_set.set_avoption_from_str("layout", "5.1"));
    let after_five_one = channel_layout_value(&channel_layout_set, "layout");
    let ret_unspecified = ret(channel_layout_set.set_avoption_from_str("layout", "2C"));
    let after_unspecified = channel_layout_value(&channel_layout_set, "layout");
    insert_row(
        &mut rows,
        "ret:set-channel-layout-strings",
        [ret_mono, ret_five_one, ret_unspecified],
    );
    insert_row(
        &mut rows,
        "state:set-channel-layout-strings",
        [
            after_mono.describe(),
            after_five_one.describe(),
            after_unspecified.describe(),
        ],
    );
    insert_row(
        &mut rows,
        "get:set-channel-layout-strings",
        [ret_value(channel_layout_set.get_avoption_string("layout"))],
    );
    insert_row(
        &mut rows,
        "ret:set-channel-layout-errors",
        [
            ret(channel_layout_set.set_avoption_from_str("layout", "bad")),
            ret(channel_layout_set.set_avoption_from_str("layout", "0")),
        ],
    );
    insert_row(
        &mut rows,
        "state:after-channel-layout-errors",
        channel_layout_state_fields(&channel_layout_set),
    );

    let mut typed_channel_layout_options = channel_layout_options();
    insert_row(
        &mut rows,
        "ret:set-channel-layout-typed",
        [
            ret(typed_channel_layout_options.set_avoption_channel_layout(
                "layout",
                ChannelLayoutSpec::native(ChannelLayout::mono()),
            )),
            ret(typed_channel_layout_options.set_avoption_channel_layout(
                "scalar",
                ChannelLayoutSpec::native(ChannelLayout::stereo()),
            )),
            ret(typed_channel_layout_options.set_avoption_int("layout", 2)),
            ret(typed_channel_layout_options.set_avoption_int("layout", 0)),
            ret(typed_channel_layout_options.set_avoption_int("scalar", 6)),
        ],
    );
    insert_row(
        &mut rows,
        "state:set-channel-layout-typed",
        channel_layout_state_fields(&typed_channel_layout_options),
    );
    insert_row(
        &mut rows,
        "get:set-channel-layout-typed",
        [
            ret_channel_layout(typed_channel_layout_options.get_avoption_channel_layout("layout")),
            ret_i64(typed_channel_layout_options.get_avoption_int("layout")),
            ret_f64(typed_channel_layout_options.get_avoption_double("layout")),
            ret_q(typed_channel_layout_options.get_avoption_q("layout")),
            ret_value(typed_channel_layout_options.get_avoption_string("layout")),
            ret_channel_layout(typed_channel_layout_options.get_avoption_channel_layout("scalar")),
        ],
    );
    insert_row(
        &mut rows,
        "query-ranges:channel-layout",
        [
            ret_ranges(typed_channel_layout_options.query_avoption_ranges("layout")),
            ret_ranges(typed_channel_layout_options.query_avoption_ranges("missing")),
        ],
    );

    let binary_defaults = binary_options();
    insert_row(
        &mut rows,
        "state:binary-defaults",
        binary_state_fields(&binary_defaults),
    );
    insert_row(
        &mut rows,
        "get:binary-defaults",
        [
            ret_value(binary_defaults.get_avoption_string("blob")),
            ret_i64(binary_defaults.get_avoption_int("blob")),
        ],
    );
    insert_row(
        &mut rows,
        "get:binary-allow-null",
        [
            ret_value(binary_defaults.get_avoption_string("nullable_blob")),
            ret_nullable_value(binary_defaults.get_avoption_string_nullable_with_flags(
                "nullable_blob",
                OptionSearchFlags::ALLOW_NULL,
            )),
            ret_nullable_value(
                binary_defaults
                    .get_avoption_string_nullable_with_flags("blob", OptionSearchFlags::ALLOW_NULL),
            ),
            ret_nullable_value(binary_defaults.get_avoption_string_nullable_with_flags(
                "nullable_blob",
                OptionSearchFlags::from_bits_truncate(
                    OptionSearchFlags::ALLOW_NULL.bits() | OptionSearchFlags::FAKE_OBJ.bits(),
                ),
            )),
        ],
    );

    let mut binary_set = binary_options();
    let ret_hex = ret(binary_set.set_avoption_from_str("blob", "0f10Aa"));
    let after_hex = binary_value(&binary_set, "blob");
    let ret_empty = ret(binary_set.set_avoption_from_str("blob", ""));
    let after_empty = binary_value(&binary_set, "blob");
    let ret_dead = ret(binary_set.set_avoption_from_str("blob", "deAd"));
    let after_dead = binary_value(&binary_set, "blob");
    insert_row(
        &mut rows,
        "ret:set-binary-strings",
        [ret_hex, ret_empty, ret_dead],
    );
    insert_row(
        &mut rows,
        "state:set-binary-strings",
        [
            after_hex.len().to_string(),
            binary_field(&after_hex),
            after_empty.len().to_string(),
            binary_field(&after_empty),
            after_dead.len().to_string(),
            binary_field(&after_dead),
        ],
    );
    insert_row(
        &mut rows,
        "get:set-binary-strings",
        [ret_value(binary_set.get_avoption_string("blob"))],
    );
    let ret_odd = ret(binary_set.set_avoption_from_str("blob", "abc"));
    binary_set.set_avoption_from_str("blob", "beef").unwrap();
    let ret_non_hex = ret(binary_set.set_avoption_from_str("blob", "0g"));
    insert_row(&mut rows, "ret:set-binary-errors", [ret_odd, ret_non_hex]);
    insert_row(
        &mut rows,
        "state:after-binary-errors",
        binary_state_fields(&binary_set),
    );

    let mut typed_binary_options = binary_options();
    let ret_typed = ret(typed_binary_options.set_avoption_binary("blob", &[0xDE, 0xAD]));
    let after_typed = binary_value(&typed_binary_options, "blob");
    let ret_typed_empty = ret(typed_binary_options.set_avoption_binary("blob", &[]));
    let after_typed_empty = binary_value(&typed_binary_options, "blob");
    insert_row(
        &mut rows,
        "ret:set-binary-typed",
        [
            ret_typed,
            ret_typed_empty,
            ret(typed_binary_options.set_avoption_binary("scalar", &[1])),
            ret(typed_binary_options.set_avoption_int("blob", 2)),
            ret(typed_binary_options.set_avoption_int("blob", 0)),
            ret(typed_binary_options.set_avoption_int("scalar", 6)),
        ],
    );
    insert_row(
        &mut rows,
        "state:set-binary-typed",
        [
            after_typed.len().to_string(),
            binary_field(&after_typed),
            after_typed_empty.len().to_string(),
            binary_field(&after_typed_empty),
            binary_value(&typed_binary_options, "blob")
                .len()
                .to_string(),
            binary_field(&binary_value(&typed_binary_options, "blob")),
            int_value(&typed_binary_options, "scalar").to_string(),
        ],
    );
    insert_row(
        &mut rows,
        "get:set-binary-typed",
        [
            ret_value(typed_binary_options.get_avoption_string("blob")),
            ret_i64(typed_binary_options.get_avoption_int("blob")),
            ret_q(typed_binary_options.get_avoption_q("blob")),
            ret_value(typed_binary_options.get_avoption_string("scalar")),
            ret_value(typed_binary_options.get_avoption_string("missing")),
        ],
    );
    insert_row(
        &mut rows,
        "query-ranges:binary",
        [
            ret_ranges(typed_binary_options.query_avoption_ranges("blob")),
            ret_ranges(typed_binary_options.query_avoption_ranges("missing")),
        ],
    );

    let dictionary_defaults = dictionary_options();
    rows.insert(
        "state:dictionary-defaults".to_string(),
        dictionary_state_fields(&dictionary_defaults),
    );
    insert_row(
        &mut rows,
        "get:dictionary-defaults",
        [
            ret_value(dictionary_defaults.get_avoption_string("dict")),
            ret_dictionary(dictionary_defaults.get_avoption_dictionary("dict")),
            ret_value(dictionary_defaults.get_avoption_string("empty")),
            ret_dictionary(dictionary_defaults.get_avoption_dictionary("empty")),
            ret_i64(dictionary_defaults.get_avoption_int("dict")),
        ],
    );
    insert_row(
        &mut rows,
        "get:dictionary-allow-null",
        [
            ret_value(dictionary_defaults.get_avoption_string("empty")),
            ret_nullable_value(
                dictionary_defaults.get_avoption_string_nullable_with_flags(
                    "empty",
                    OptionSearchFlags::ALLOW_NULL,
                ),
            ),
            ret_nullable_value(
                dictionary_defaults
                    .get_avoption_string_nullable_with_flags("dict", OptionSearchFlags::ALLOW_NULL),
            ),
            ret_nullable_value(dictionary_defaults.get_avoption_string_nullable_with_flags(
                "empty",
                OptionSearchFlags::from_bits_truncate(
                    OptionSearchFlags::ALLOW_NULL.bits() | OptionSearchFlags::FAKE_OBJ.bits(),
                ),
            )),
        ],
    );

    let mut dictionary_set = dictionary_options();
    let ret_escaped =
        ret(dictionary_set.set_avoption_from_str("dict", "artist=rust:comment=hello\\:there"));
    let after_escaped = dictionary_value(&dictionary_set, "dict");
    let ret_empty = ret(dictionary_set.set_avoption_from_str("dict", ""));
    let after_empty = dictionary_value(&dictionary_set, "dict");
    let ret_quoted = ret(dictionary_set.set_avoption_from_str("dict", "quoted='a:b':space=trim "));
    let after_quoted = dictionary_value(&dictionary_set, "dict");
    insert_row(
        &mut rows,
        "ret:set-dictionary-strings",
        [ret_escaped, ret_empty, ret_quoted],
    );
    rows.insert(
        "state:set-dictionary-strings".to_string(),
        dictionary_sequence_fields([&after_escaped, &after_empty, &after_quoted]),
    );
    insert_row(
        &mut rows,
        "get:set-dictionary-strings",
        [ret_value(dictionary_set.get_avoption_string("dict"))],
    );

    let ret_duplicate =
        ret(dictionary_set.set_avoption_from_str("dict", "artist=rust:ARTIST=override"));
    let after_duplicate = dictionary_value(&dictionary_set, "dict");
    insert_row(&mut rows, "ret:set-dictionary-duplicate", [ret_duplicate]);
    rows.insert(
        "state:set-dictionary-duplicate".to_string(),
        dictionary_sequence_fields([&after_duplicate]),
    );
    insert_row(
        &mut rows,
        "get:set-dictionary-duplicate",
        [ret_value(dictionary_set.get_avoption_string("dict"))],
    );

    let before_dictionary_errors = dictionary_value(&dictionary_set, "dict");
    let ret_missing_separator = ret(dictionary_set.set_avoption_from_str("dict", "missing"));
    let ret_empty_value = ret(dictionary_set.set_avoption_from_str("dict", "key="));
    insert_row(
        &mut rows,
        "ret:set-dictionary-errors",
        [ret_missing_separator, ret_empty_value],
    );
    rows.insert(
        "state:after-dictionary-errors".to_string(),
        dictionary_sequence_fields([
            &before_dictionary_errors,
            &dictionary_value(&dictionary_set, "dict"),
        ]),
    );

    let mut typed_dictionary_options = dictionary_options();
    let mut typed_dict = Dictionary::new();
    typed_dict.set("typed", "one").unwrap();
    typed_dict.set("note", "two:three").unwrap();
    let ret_typed = ret(typed_dictionary_options.set_avoption_dictionary("dict", &typed_dict));
    let after_typed = dictionary_value(&typed_dictionary_options, "dict");
    let empty_dict = Dictionary::new();
    let ret_typed_empty =
        ret(typed_dictionary_options.set_avoption_dictionary("dict", &empty_dict));
    let after_typed_empty = dictionary_value(&typed_dictionary_options, "dict");
    insert_row(
        &mut rows,
        "ret:set-dictionary-typed",
        [
            ret_typed,
            ret_typed_empty,
            ret(typed_dictionary_options.set_avoption_dictionary("scalar", &typed_dict)),
            ret(typed_dictionary_options.set_avoption_int("dict", 2)),
            ret(typed_dictionary_options.set_avoption_int("dict", 0)),
            ret(typed_dictionary_options.set_avoption_int("scalar", 6)),
        ],
    );
    let final_typed_dict = dictionary_value(&typed_dictionary_options, "dict");
    rows.insert("state:set-dictionary-typed".to_string(), {
        let mut fields =
            dictionary_sequence_fields([&after_typed, &after_typed_empty, &final_typed_dict]);
        fields.push(int_value(&typed_dictionary_options, "scalar").to_string());
        fields
    });
    insert_row(
        &mut rows,
        "get:set-dictionary-typed",
        [
            ret_value(typed_dictionary_options.get_avoption_string("dict")),
            ret_dictionary(typed_dictionary_options.get_avoption_dictionary("dict")),
            ret_i64(typed_dictionary_options.get_avoption_int("dict")),
            ret_q(typed_dictionary_options.get_avoption_q("dict")),
            ret_value(typed_dictionary_options.get_avoption_string("scalar")),
            ret_dictionary(typed_dictionary_options.get_avoption_dictionary("scalar")),
            ret_value(typed_dictionary_options.get_avoption_string("missing")),
        ],
    );
    insert_row(
        &mut rows,
        "query-ranges:dictionary",
        [
            ret_ranges(typed_dictionary_options.query_avoption_ranges("dict")),
            ret_ranges(typed_dictionary_options.query_avoption_ranges("missing")),
        ],
    );

    let array_defaults = array_options();
    rows.insert(
        "state:array-defaults".to_string(),
        array_state_fields(&array_defaults),
    );
    insert_row(
        &mut rows,
        "get:array-defaults",
        [
            ret_value(array_defaults.get_avoption_string("ints")),
            ret_value(array_defaults.get_avoption_string("words")),
            ret_array_size(array_defaults.get_avoption_array_size("ints")),
            ret_array_values(array_defaults.get_avoption_array("ints", 0, 2)),
            ret_i64(array_defaults.get_avoption_int("ints")),
        ],
    );

    let mut array_set = array_options();
    let ret_set_ints = ret(array_set.set_avoption_from_str("ints", "3,4"));
    let ret_set_words =
        ret(array_set.set_avoption_from_str("words", "left,right\\,inner,slash\\\\tail"));
    insert_row(
        &mut rows,
        "ret:set-array-strings",
        [ret_set_ints, ret_set_words],
    );
    rows.insert(
        "state:set-array-strings".to_string(),
        array_state_fields(&array_set),
    );
    insert_row(
        &mut rows,
        "get:set-array-strings",
        [
            ret_value(array_set.get_avoption_string("ints")),
            ret_value(array_set.get_avoption_string("words")),
        ],
    );
    insert_row(
        &mut rows,
        "ret:set-array-errors",
        [
            ret(array_set.set_avoption_from_str("ints", "7,11")),
            ret(array_set.set_avoption_from_str("words", "a,b,c,d")),
        ],
    );
    rows.insert(
        "state:after-array-errors".to_string(),
        array_state_fields(&array_set),
    );

    let mut array_required = array_options();
    insert_row(
        &mut rows,
        "ret:set-array-required-min",
        [ret(array_required.set_avoption_from_str("required", "9"))],
    );
    rows.insert(
        "state:after-array-required-min".to_string(),
        array_required_fields(&array_required),
    );

    let mut typed_array_options = array_options();
    typed_array_options
        .set_avoption_from_str("ints", "3,4")
        .unwrap();
    let ret_insert = ret(typed_array_options.set_avoption_array(
        "ints",
        1,
        &[OptionValue::Int(8)],
        OptionSearchFlags::empty(),
    ));
    let ret_replace = ret(typed_array_options.set_avoption_array(
        "ints",
        1,
        &[OptionValue::Int(5)],
        OptionSearchFlags::ARRAY_REPLACE,
    ));
    let ret_remove =
        ret(typed_array_options.remove_avoption_array("ints", 0, 1, OptionSearchFlags::empty()));
    insert_row(
        &mut rows,
        "ret:set-array-typed",
        [
            ret_insert,
            ret_replace,
            ret_remove,
            ret(typed_array_options.set_avoption_array(
                "ints",
                0,
                &[OptionValue::String("bad".to_owned())],
                OptionSearchFlags::empty(),
            )),
            ret(typed_array_options.remove_avoption_array(
                "ints",
                0,
                3,
                OptionSearchFlags::empty(),
            )),
            ret_array_size(typed_array_options.get_avoption_array_size("scalar")),
            ret_array_values(typed_array_options.get_avoption_array("ints", 2, 1)),
        ],
    );
    rows.insert(
        "state:set-array-typed".to_string(),
        array_state_fields(&typed_array_options),
    );
    insert_row(
        &mut rows,
        "get:set-array-typed",
        [
            ret_array_size(typed_array_options.get_avoption_array_size("ints")),
            ret_array_values(typed_array_options.get_avoption_array("ints", 0, 2)),
            ret_value(typed_array_options.get_avoption_string("ints")),
        ],
    );

    let mut typed_int_string_array_options = array_options();
    typed_int_string_array_options
        .set_avoption_from_str("ints", "3,4")
        .unwrap();
    let ret_int_string_insert = ret(typed_int_string_array_options.set_avoption_array(
        "ints",
        1,
        &[OptionValue::String("6".to_owned())],
        OptionSearchFlags::empty(),
    ));
    let ret_int_string_replace = ret(typed_int_string_array_options.set_avoption_array(
        "ints",
        2,
        &[OptionValue::String("9".to_owned())],
        OptionSearchFlags::ARRAY_REPLACE,
    ));
    let ret_int_string_remove = ret(typed_int_string_array_options.remove_avoption_array(
        "ints",
        0,
        1,
        OptionSearchFlags::empty(),
    ));
    let ret_int_string_bad = ret(typed_int_string_array_options.set_avoption_array(
        "ints",
        0,
        &[OptionValue::String("bad".to_owned())],
        OptionSearchFlags::empty(),
    ));
    insert_row(
        &mut rows,
        "ret:set-array-int-string-typed",
        [
            ret_int_string_insert,
            ret_int_string_replace,
            ret_int_string_remove,
            ret_int_string_bad,
        ],
    );
    rows.insert(
        "state:set-array-int-string-typed".to_string(),
        array_state_fields(&typed_int_string_array_options),
    );
    insert_row(
        &mut rows,
        "get:set-array-int-string-typed",
        [
            ret_array_size(typed_int_string_array_options.get_avoption_array_size("ints")),
            ret_array_values(typed_int_string_array_options.get_avoption_array("ints", 0, 2)),
            ret_array_strings(
                typed_int_string_array_options.get_avoption_array_strings("ints", 0, 2),
            ),
            ret_value(typed_int_string_array_options.get_avoption_string("ints")),
        ],
    );

    let mut typed_int_numeric_array_options = array_options();
    typed_int_numeric_array_options
        .set_avoption_from_str("ints", "3,4")
        .unwrap();
    let ret_int_double_insert = ret(typed_int_numeric_array_options.set_avoption_array(
        "ints",
        1,
        &[OptionValue::Float(6.0)],
        OptionSearchFlags::empty(),
    ));
    let ret_int_q_replace = ret(typed_int_numeric_array_options.set_avoption_array(
        "ints",
        2,
        &[OptionValue::Rational(Rational::new(9, 1).unwrap())],
        OptionSearchFlags::ARRAY_REPLACE,
    ));
    let ret_int_numeric_remove = ret(typed_int_numeric_array_options.remove_avoption_array(
        "ints",
        0,
        1,
        OptionSearchFlags::empty(),
    ));
    let ret_int_q_bad = ret(typed_int_numeric_array_options.set_avoption_array(
        "ints",
        0,
        &[OptionValue::Rational(Rational::new(11, 1).unwrap())],
        OptionSearchFlags::empty(),
    ));
    insert_row(
        &mut rows,
        "ret:set-array-int-numeric-typed",
        [
            ret_int_double_insert,
            ret_int_q_replace,
            ret_int_numeric_remove,
            ret_int_q_bad,
        ],
    );
    rows.insert(
        "state:set-array-int-numeric-typed".to_string(),
        array_state_fields(&typed_int_numeric_array_options),
    );
    insert_row(
        &mut rows,
        "get:set-array-int-numeric-typed",
        [
            ret_array_size(typed_int_numeric_array_options.get_avoption_array_size("ints")),
            ret_array_values(typed_int_numeric_array_options.get_avoption_array("ints", 0, 2)),
            ret_array_doubles(
                typed_int_numeric_array_options.get_avoption_array_doubles("ints", 0, 2),
            ),
            ret_array_rationals(
                typed_int_numeric_array_options.get_avoption_array_rationals("ints", 0, 2),
            ),
            ret_value(typed_int_numeric_array_options.get_avoption_string("ints")),
        ],
    );

    let mut zero_count_array_options = array_options();
    zero_count_array_options
        .set_avoption_from_str("ints", "3,4")
        .unwrap();
    let ret_zero_insert = ret(zero_count_array_options.set_avoption_array(
        "ints",
        2,
        &[],
        OptionSearchFlags::empty(),
    ));
    let ret_zero_replace = ret(zero_count_array_options.set_avoption_array(
        "ints",
        2,
        &[],
        OptionSearchFlags::ARRAY_REPLACE,
    ));
    let ret_zero_remove = ret(zero_count_array_options.remove_avoption_array(
        "ints",
        2,
        0,
        OptionSearchFlags::empty(),
    ));
    insert_row(
        &mut rows,
        "ret:set-array-zero-count",
        [
            ret_zero_insert,
            ret_zero_replace,
            ret_zero_remove,
            ret_array_values(zero_count_array_options.get_avoption_array("ints", 3, 0)),
        ],
    );
    rows.insert(
        "state:set-array-zero-count".to_string(),
        array_state_fields(&zero_count_array_options),
    );
    insert_row(
        &mut rows,
        "get:set-array-zero-count",
        [
            ret_array_size(zero_count_array_options.get_avoption_array_size("ints")),
            ret_array_values(zero_count_array_options.get_avoption_array("ints", 0, 2)),
            ret_array_values(zero_count_array_options.get_avoption_array("ints", 2, 0)),
            ret_array_strings(zero_count_array_options.get_avoption_array_strings("ints", 2, 0)),
            ret_array_doubles(zero_count_array_options.get_avoption_array_doubles("ints", 2, 0)),
            ret_array_rationals(
                zero_count_array_options.get_avoption_array_rationals("ints", 2, 0),
            ),
            ret_value(zero_count_array_options.get_avoption_string("ints")),
        ],
    );

    let mut typed_string_array_options = array_options();
    typed_string_array_options
        .set_avoption_from_str("words", "left,right\\,inner")
        .unwrap();
    let ret_string_insert = ret(typed_string_array_options.set_avoption_array(
        "words",
        1,
        &[OptionValue::String("middle,comma".to_owned())],
        OptionSearchFlags::empty(),
    ));
    let ret_string_replace = ret(typed_string_array_options.set_avoption_array(
        "words",
        2,
        &[OptionValue::String("tail\\slash".to_owned())],
        OptionSearchFlags::ARRAY_REPLACE,
    ));
    let ret_string_remove = ret(typed_string_array_options.remove_avoption_array(
        "words",
        0,
        1,
        OptionSearchFlags::empty(),
    ));
    insert_row(
        &mut rows,
        "ret:set-array-string-typed",
        [ret_string_insert, ret_string_replace, ret_string_remove],
    );
    rows.insert(
        "state:set-array-string-typed".to_string(),
        array_state_fields(&typed_string_array_options),
    );
    insert_row(
        &mut rows,
        "get:set-array-string-typed",
        [
            ret_array_size(typed_string_array_options.get_avoption_array_size("words")),
            ret_array_values(typed_string_array_options.get_avoption_array("words", 0, 2)),
            ret_value(typed_string_array_options.get_avoption_string("words")),
        ],
    );
    insert_row(
        &mut rows,
        "query-ranges:array",
        [
            ret_ranges(typed_array_options.query_avoption_ranges("ints")),
            ret_ranges(typed_array_options.query_avoption_ranges("missing")),
        ],
    );

    let video_defaults = video_rate_options();
    insert_row(
        &mut rows,
        "state:video-rate-defaults",
        video_rate_state_fields(&video_defaults),
    );
    insert_row(
        &mut rows,
        "get:video-rate-defaults",
        [
            ret_q(video_defaults.get_avoption_video_rate("rate")),
            ret_value(video_defaults.get_avoption_string("rate")),
            ret_q(video_defaults.get_avoption_video_rate("scalar")),
        ],
    );

    let mut video_set = video_rate_options();
    let ret_ntsc = ret(video_set.set_avoption_from_str("rate", "ntsc"));
    let after_ntsc = video_rate_value(&video_set, "rate");
    let ret_film = ret(video_set.set_avoption_from_str("rate", "film"));
    let after_film = video_rate_value(&video_set, "rate");
    let ret_fraction = ret(video_set.set_avoption_from_str("rate", "30000/1001"));
    let after_fraction = video_rate_value(&video_set, "rate");
    let ret_integer = ret(video_set.set_avoption_from_str("rate", "25"));
    let after_integer = video_rate_value(&video_set, "rate");
    insert_row(
        &mut rows,
        "ret:set-video-rate-strings",
        [ret_ntsc, ret_film, ret_fraction, ret_integer],
    );
    insert_row(
        &mut rows,
        "state:set-video-rate-strings",
        [
            format!("{}/{}", after_ntsc.num(), after_ntsc.den()),
            format!("{}/{}", after_film.num(), after_film.den()),
            format!("{}/{}", after_fraction.num(), after_fraction.den()),
            format!("{}/{}", after_integer.num(), after_integer.den()),
        ],
    );
    insert_row(
        &mut rows,
        "get:set-video-rate-strings",
        [ret_value(video_set.get_avoption_string("rate"))],
    );
    insert_row(
        &mut rows,
        "ret:set-video-rate-errors",
        [
            ret(video_set.set_avoption_from_str("rate", "bad")),
            ret(video_set.set_avoption_from_str("rate", "0")),
            ret(video_set.set_avoption_from_str("rate", "-25")),
            ret(video_set.set_avoption_from_str("rate", "121")),
        ],
    );
    insert_row(
        &mut rows,
        "state:after-video-rate-errors",
        video_rate_state_fields(&video_set),
    );

    let mut typed_video_options = video_rate_options();
    insert_row(
        &mut rows,
        "ret:set-video-rate-typed",
        [
            ret(typed_video_options.set_avoption_video_rate("rate", Rational::new(50, 1).unwrap())),
            ret(typed_video_options.set_avoption_video_rate("rate", Rational::ZERO)),
            ret(typed_video_options.set_avoption_video_rate("scalar", Rational::ONE)),
            ret(typed_video_options.set_avoption_q("rate", Rational::new(60, 1).unwrap())),
            ret(typed_video_options.set_avoption_int("rate", 75)),
        ],
    );
    insert_row(
        &mut rows,
        "state:set-video-rate-typed",
        video_rate_state_fields(&typed_video_options),
    );
    insert_row(
        &mut rows,
        "get:set-video-rate-typed",
        [
            ret_q(typed_video_options.get_avoption_video_rate("rate")),
            ret_q(typed_video_options.get_avoption_q("rate")),
            ret_i64(typed_video_options.get_avoption_int("rate")),
            ret_value(typed_video_options.get_avoption_string("rate")),
            ret_q(typed_video_options.get_avoption_video_rate("scalar")),
        ],
    );
    insert_row(
        &mut rows,
        "query-ranges:video-rate",
        [
            ret_ranges(typed_video_options.query_avoption_ranges("rate")),
            ret_ranges(typed_video_options.query_avoption_ranges("missing")),
        ],
    );

    let color_defaults = color_options();
    insert_row(
        &mut rows,
        "state:color-defaults",
        color_state_fields(&color_defaults),
    );
    insert_row(
        &mut rows,
        "get:color-defaults",
        [
            ret_value(color_defaults.get_avoption_string("color")),
            ret_i64(color_defaults.get_avoption_int("color")),
            ret_i64(color_defaults.get_avoption_int("scalar")),
        ],
    );

    let mut color_set = color_options();
    let ret_blue = ret(color_set.set_avoption_from_str("color", "Blue@0.5"));
    let after_blue = color_value(&color_set, "color");
    let ret_hex = ret(color_set.set_avoption_from_str("color", "#112233"));
    let after_hex = color_value(&color_set, "color");
    let ret_hex_alpha = ret(color_set.set_avoption_from_str("color", "0x11223344"));
    let after_hex_alpha = color_value(&color_set, "color");
    insert_row(
        &mut rows,
        "ret:set-color-strings",
        [ret_blue, ret_hex, ret_hex_alpha],
    );
    insert_row(
        &mut rows,
        "state:set-color-strings",
        [
            color_field(after_blue),
            color_field(after_hex),
            color_field(after_hex_alpha),
        ],
    );
    insert_row(
        &mut rows,
        "get:set-color-strings",
        [ret_value(color_set.get_avoption_string("color"))],
    );
    insert_row(
        &mut rows,
        "ret:set-color-errors",
        [
            ret(color_set.set_avoption_from_str("color", "not-a-color")),
            ret(color_set.set_avoption_from_str("color", "red@2")),
        ],
    );
    insert_row(
        &mut rows,
        "state:after-color-errors",
        color_state_fields(&color_set),
    );

    let mut typed_color_options = color_options();
    insert_row(
        &mut rows,
        "ret:set-color-typed",
        [
            ret(typed_color_options.set_avoption_int("color", 10)),
            ret(typed_color_options.set_avoption_int("color", 0)),
            ret(typed_color_options.set_avoption_int("scalar", 6)),
        ],
    );
    insert_row(
        &mut rows,
        "state:set-color-typed",
        color_state_fields(&typed_color_options),
    );
    insert_row(
        &mut rows,
        "get:set-color-typed",
        [
            ret_i64(typed_color_options.get_avoption_int("color")),
            ret_f64(typed_color_options.get_avoption_double("color")),
            ret_q(typed_color_options.get_avoption_q("color")),
            ret_value(typed_color_options.get_avoption_string("color")),
            ret_i64(typed_color_options.get_avoption_int("scalar")),
        ],
    );
    insert_row(
        &mut rows,
        "query-ranges:color",
        [
            ret_ranges(typed_color_options.query_avoption_ranges("color")),
            ret_ranges(typed_color_options.query_avoption_ranges("missing")),
        ],
    );

    let error_results = [
        ret(options.set_avoption_from_str("bitexact", "maybe")),
        ret(options.set_avoption_from_str("exported", "6")),
    ];
    insert_row(&mut rows, "ret:set-errors", error_results);
    rows.insert("state:after-errors".to_string(), state_fields(&options));

    rows
}

fn sample_options() -> OptionSet {
    let mut options = OptionSet::new();
    options
        .define(
            OptionDefinition::new_with_flags(
                "threads",
                OptionKind::Int { min: 1, max: 64 },
                OptionValue::Int(1),
                "worker count",
                OptionFlags::ENCODING_PARAM,
            )
            .unwrap(),
        )
        .unwrap();
    options
        .define(
            OptionDefinition::new_with_flags(
                "bitexact",
                OptionKind::Bool,
                OptionValue::Bool(false),
                "bit-exact output",
                OptionFlags::ENCODING_PARAM,
            )
            .unwrap(),
        )
        .unwrap();
    options
        .define(
            OptionDefinition::new_with_flags(
                "quality",
                OptionKind::Float { min: 0.0, max: 1.0 },
                OptionValue::Float(0.5),
                "quality",
                OptionFlags::ENCODING_PARAM,
            )
            .unwrap(),
        )
        .unwrap();
    options
        .define(
            OptionDefinition::new_with_flags(
                "aspect_ratio",
                OptionKind::Rational {
                    min: Rational::ONE,
                    max: Rational::new(16, 9).unwrap(),
                },
                OptionValue::Rational(Rational::ONE),
                "aspect ratio",
                OptionFlags::ENCODING_PARAM,
            )
            .unwrap(),
        )
        .unwrap();
    options
        .define(
            OptionDefinition::new_with_flags(
                "metadata",
                OptionKind::String { allow_empty: false },
                OptionValue::String("default".to_owned()),
                "metadata",
                OptionFlags::ENCODING_PARAM,
            )
            .unwrap(),
        )
        .unwrap();
    options
        .define(
            OptionDefinition::new_with_flags(
                "nullable_metadata",
                OptionKind::String { allow_empty: true },
                OptionValue::NullString,
                "nullable metadata",
                OptionFlags::ENCODING_PARAM,
            )
            .unwrap(),
        )
        .unwrap();
    options
        .define(
            OptionDefinition::new_with_flags_and_unit(
                "preset_level",
                OptionKind::Int { min: 0, max: 10 },
                OptionValue::Int(0),
                "preset level",
                OptionFlags::ENCODING_PARAM,
                Some("PRESET"),
            )
            .unwrap(),
        )
        .unwrap();
    options
        .define_constant(
            OptionConstant::new_with_flags(
                "PRESET",
                "fast",
                OptionValue::Int(2),
                "fast preset",
                OptionFlags::ENCODING_PARAM,
            )
            .unwrap(),
        )
        .unwrap();
    options
        .define_constant(
            OptionConstant::new_with_flags(
                "PRESET",
                "slow",
                OptionValue::Int(8),
                "slow preset",
                OptionFlags::ENCODING_PARAM,
            )
            .unwrap(),
        )
        .unwrap();
    options
        .define_with_current_value(
            OptionDefinition::new_with_flags(
                "exported",
                OptionKind::Int { min: 0, max: 8 },
                OptionValue::Int(4),
                "read-only exported value",
                OptionFlags::from_bits_truncate(
                    OptionFlags::EXPORT.bits() | OptionFlags::READONLY.bits(),
                ),
            )
            .unwrap(),
            OptionValue::Int(0),
        )
        .unwrap();
    options
}

fn sample_options_with_child() -> OptionSet {
    let mut options = sample_options();
    let mut child_options = OptionSet::new();
    child_options
        .define(
            OptionDefinition::new_with_flags(
                "threads",
                OptionKind::Int { min: 1, max: 16 },
                OptionValue::Int(2),
                "child worker count",
                OptionFlags::DECODING_PARAM,
            )
            .unwrap(),
        )
        .unwrap();
    child_options
        .define(
            OptionDefinition::new_with_flags(
                "child_only",
                OptionKind::Int { min: 0, max: 10 },
                OptionValue::Int(5),
                "child-only value",
                OptionFlags::DECODING_PARAM,
            )
            .unwrap(),
        )
        .unwrap();
    child_options
        .define(
            OptionDefinition::new_with_flags(
                "child_readonly",
                OptionKind::Int { min: 0, max: 10 },
                OptionValue::Int(0),
                "child read-only value",
                OptionFlags::from_bits_truncate(
                    OptionFlags::DECODING_PARAM.bits() | OptionFlags::READONLY.bits(),
                ),
            )
            .unwrap(),
        )
        .unwrap();
    options
        .define_child(OptionChild::new("decoder", child_options, "decoder options").unwrap())
        .unwrap();
    options
}

fn duration_options() -> OptionSet {
    let mut options = OptionSet::new();
    options
        .define(
            OptionDefinition::new_with_flags(
                "timeout",
                OptionKind::Duration {
                    min: 0,
                    max: 7_200_000_000,
                },
                OptionValue::Duration(0),
                "timeout",
                OptionFlags::ENCODING_PARAM,
            )
            .unwrap(),
        )
        .unwrap();
    options
}

fn image_size_options() -> OptionSet {
    let mut options = OptionSet::new();
    options
        .define(
            OptionDefinition::new_with_flags(
                "size",
                OptionKind::ImageSize,
                OptionValue::ImageSize {
                    width: 320,
                    height: 240,
                },
                "image size",
                OptionFlags::ENCODING_PARAM,
            )
            .unwrap(),
        )
        .unwrap();
    options
        .define(
            OptionDefinition::new_with_flags(
                "scalar",
                OptionKind::Int { min: 0, max: 10 },
                OptionValue::Int(4),
                "scalar",
                OptionFlags::ENCODING_PARAM,
            )
            .unwrap(),
        )
        .unwrap();
    options
}

fn image_size_parent_with_child_options() -> OptionSet {
    let mut parent = OptionSet::new();
    let mut child = OptionSet::new();
    child
        .define(
            OptionDefinition::new_with_flags(
                "child_size",
                OptionKind::ImageSize,
                OptionValue::ImageSize {
                    width: 320,
                    height: 240,
                },
                "child image size",
                OptionFlags::DECODING_PARAM,
            )
            .unwrap(),
        )
        .unwrap();
    parent
        .define_child(OptionChild::new("decoder", child, "decoder options").unwrap())
        .unwrap();
    parent
}

fn pixel_format_options() -> OptionSet {
    let mut options = OptionSet::new();
    options
        .define(
            OptionDefinition::new_with_flags(
                "pix_fmt",
                OptionKind::PixelFormat { min: -1, max: 266 },
                OptionValue::PixelFormat(Some(PixelFormat::Yuv420p)),
                "pixel format",
                OptionFlags::ENCODING_PARAM,
            )
            .unwrap(),
        )
        .unwrap();
    options
        .define(
            OptionDefinition::new_with_flags(
                "scalar",
                OptionKind::Int { min: 0, max: 10 },
                OptionValue::Int(4),
                "scalar",
                OptionFlags::ENCODING_PARAM,
            )
            .unwrap(),
        )
        .unwrap();
    options
}

fn sample_format_options() -> OptionSet {
    let mut options = OptionSet::new();
    options
        .define(
            OptionDefinition::new_with_flags(
                "sample_fmt",
                OptionKind::SampleFormat { min: -1, max: 11 },
                OptionValue::SampleFormat(Some(SampleFormat::S16)),
                "sample format",
                OptionFlags::ENCODING_PARAM,
            )
            .unwrap(),
        )
        .unwrap();
    options
        .define(
            OptionDefinition::new_with_flags(
                "scalar",
                OptionKind::Int { min: 0, max: 10 },
                OptionValue::Int(4),
                "scalar",
                OptionFlags::ENCODING_PARAM,
            )
            .unwrap(),
        )
        .unwrap();
    options
}

fn channel_layout_options() -> OptionSet {
    let mut options = OptionSet::new();
    options
        .define(
            OptionDefinition::new_with_flags(
                "layout",
                OptionKind::ChannelLayout,
                OptionValue::ChannelLayout(ChannelLayoutSpec::native(ChannelLayout::stereo())),
                "channel layout",
                OptionFlags::ENCODING_PARAM,
            )
            .unwrap(),
        )
        .unwrap();
    options
        .define(
            OptionDefinition::new_with_flags(
                "scalar",
                OptionKind::Int { min: 0, max: 10 },
                OptionValue::Int(4),
                "scalar",
                OptionFlags::ENCODING_PARAM,
            )
            .unwrap(),
        )
        .unwrap();
    options
}

fn binary_options() -> OptionSet {
    let mut options = OptionSet::new();
    options
        .define(
            OptionDefinition::new_with_flags(
                "blob",
                OptionKind::Binary,
                OptionValue::Binary(vec![0x00, 0x01, 0xAA, 0xFF]),
                "binary data",
                OptionFlags::ENCODING_PARAM,
            )
            .unwrap(),
        )
        .unwrap();
    options
        .define(
            OptionDefinition::new_with_flags(
                "nullable_blob",
                OptionKind::Binary,
                OptionValue::NullBinary,
                "nullable binary",
                OptionFlags::ENCODING_PARAM,
            )
            .unwrap(),
        )
        .unwrap();
    options
        .define(
            OptionDefinition::new_with_flags(
                "scalar",
                OptionKind::Int { min: 0, max: 10 },
                OptionValue::Int(4),
                "scalar",
                OptionFlags::ENCODING_PARAM,
            )
            .unwrap(),
        )
        .unwrap();
    options
}

fn dictionary_options() -> OptionSet {
    let mut default_dict = Dictionary::new();
    default_dict.set("title", "clip").unwrap();
    default_dict.set("note", "hello:world").unwrap();

    let mut options = OptionSet::new();
    options
        .define(
            OptionDefinition::new_with_flags(
                "dict",
                OptionKind::Dictionary,
                OptionValue::Dictionary(default_dict),
                "dictionary data",
                OptionFlags::ENCODING_PARAM,
            )
            .unwrap(),
        )
        .unwrap();
    options
        .define(
            OptionDefinition::new_with_flags(
                "empty",
                OptionKind::Dictionary,
                OptionValue::NullDictionary,
                "empty dictionary",
                OptionFlags::ENCODING_PARAM,
            )
            .unwrap(),
        )
        .unwrap();
    options
        .define(
            OptionDefinition::new_with_flags(
                "scalar",
                OptionKind::Int { min: 0, max: 10 },
                OptionValue::Int(4),
                "scalar",
                OptionFlags::ENCODING_PARAM,
            )
            .unwrap(),
        )
        .unwrap();
    options
}

fn array_options() -> OptionSet {
    let mut options = OptionSet::new();
    options
        .define(
            OptionDefinition::new_with_flags(
                "ints",
                OptionKind::array(OptionKind::Int { min: 0, max: 10 }, 0, Some(4), ',').unwrap(),
                OptionValue::Array(vec![OptionValue::Int(1), OptionValue::Int(2)]),
                "integer array",
                OptionFlags::ENCODING_PARAM,
            )
            .unwrap(),
        )
        .unwrap();
    options
        .define(
            OptionDefinition::new_with_flags(
                "words",
                OptionKind::array(OptionKind::String { allow_empty: true }, 0, Some(3), ',')
                    .unwrap(),
                OptionValue::Array(vec![
                    OptionValue::String("alpha".to_owned()),
                    OptionValue::String("beta,gamma".to_owned()),
                ]),
                "string array",
                OptionFlags::ENCODING_PARAM,
            )
            .unwrap(),
        )
        .unwrap();
    options
        .define(
            OptionDefinition::new_with_flags(
                "required",
                OptionKind::array(OptionKind::Int { min: 0, max: 10 }, 2, Some(3), ',').unwrap(),
                OptionValue::Array(vec![OptionValue::Int(3), OptionValue::Int(4)]),
                "required integer array",
                OptionFlags::ENCODING_PARAM,
            )
            .unwrap(),
        )
        .unwrap();
    options
        .define(
            OptionDefinition::new_with_flags(
                "scalar",
                OptionKind::Int { min: 0, max: 10 },
                OptionValue::Int(4),
                "scalar",
                OptionFlags::ENCODING_PARAM,
            )
            .unwrap(),
        )
        .unwrap();
    options
}

fn video_rate_options() -> OptionSet {
    let mut options = OptionSet::new();
    options
        .define(
            OptionDefinition::new_with_flags(
                "rate",
                OptionKind::VideoRate {
                    min: Rational::ONE,
                    max: Rational::new(120, 1).unwrap(),
                },
                OptionValue::VideoRate(Rational::new(25, 1).unwrap()),
                "video rate",
                OptionFlags::ENCODING_PARAM,
            )
            .unwrap(),
        )
        .unwrap();
    options
        .define(
            OptionDefinition::new_with_flags(
                "scalar",
                OptionKind::Int { min: 0, max: 10 },
                OptionValue::Int(4),
                "scalar",
                OptionFlags::ENCODING_PARAM,
            )
            .unwrap(),
        )
        .unwrap();
    options
}

fn color_options() -> OptionSet {
    let mut options = OptionSet::new();
    options
        .define(
            OptionDefinition::new_with_flags(
                "color",
                OptionKind::Color,
                OptionValue::Color(RgbaColor::from_rgba([0xFF, 0x00, 0x00, 0xFF])),
                "color",
                OptionFlags::ENCODING_PARAM,
            )
            .unwrap(),
        )
        .unwrap();
    options
        .define(
            OptionDefinition::new_with_flags(
                "scalar",
                OptionKind::Int { min: 0, max: 10 },
                OptionValue::Int(4),
                "scalar",
                OptionFlags::ENCODING_PARAM,
            )
            .unwrap(),
        )
        .unwrap();
    options
}

fn state_fields(options: &OptionSet) -> Vec<String> {
    vec![
        int_value(options, "threads").to_string(),
        bool_int(bool_value(options, "bitexact")).to_string(),
        format_float(float_value(options, "quality")),
        rational_value(options, "aspect_ratio"),
        string_value(options, "metadata").to_string(),
        int_value(options, "preset_level").to_string(),
    ]
}

fn child_state_fields(options: &OptionSet) -> Vec<String> {
    vec![
        int_value(options, "threads").to_string(),
        child_int_value(options, "decoder", "threads").to_string(),
        child_int_value(options, "decoder", "child_only").to_string(),
        child_int_value(options, "decoder", "child_readonly").to_string(),
    ]
}

fn child_dict_state_fields(options: &OptionSet) -> Vec<String> {
    vec![
        int_value(options, "threads").to_string(),
        child_int_value(options, "decoder", "threads").to_string(),
        child_int_value(options, "decoder", "child_only").to_string(),
        format_float(float_value(options, "quality")),
    ]
}

fn copy_state_fields(options: &OptionSet) -> Vec<String> {
    let mut fields = state_fields(options);
    fields.push(child_int_value(options, "decoder", "threads").to_string());
    fields.push(child_int_value(options, "decoder", "child_only").to_string());
    fields.push(child_int_value(options, "decoder", "child_readonly").to_string());
    fields
}

fn child_option_state_fields(options: &OptionSet) -> Vec<String> {
    vec![
        int_value(options, "threads").to_string(),
        int_value(options, "child_only").to_string(),
        int_value(options, "child_readonly").to_string(),
    ]
}

fn image_size_state_fields(options: &OptionSet) -> [String; 3] {
    let (width, height) = image_size_value(options, "size");
    [
        width.to_string(),
        height.to_string(),
        int_value(options, "scalar").to_string(),
    ]
}

fn image_size_child_state_fields(options: &OptionSet) -> Vec<String> {
    match options
        .child("decoder")
        .and_then(|child| child.options().get("child_size"))
    {
        Some(OptionValue::ImageSize { width, height }) => {
            vec![width.to_string(), height.to_string()]
        }
        _ => panic!("missing decoder child_size option"),
    }
}

fn pixel_format_state_fields(options: &OptionSet) -> [String; 2] {
    [
        pixel_format_field(pixel_format_value(options, "pix_fmt")),
        int_value(options, "scalar").to_string(),
    ]
}

fn sample_format_state_fields(options: &OptionSet) -> [String; 2] {
    [
        sample_format_field(sample_format_value(options, "sample_fmt")),
        int_value(options, "scalar").to_string(),
    ]
}

fn channel_layout_state_fields(options: &OptionSet) -> [String; 2] {
    [
        channel_layout_value(options, "layout").describe(),
        int_value(options, "scalar").to_string(),
    ]
}

fn binary_state_fields(options: &OptionSet) -> [String; 3] {
    let binary = binary_value(options, "blob");
    [
        binary.len().to_string(),
        binary_field(&binary),
        int_value(options, "scalar").to_string(),
    ]
}

fn dictionary_state_fields(options: &OptionSet) -> Vec<String> {
    let mut fields = dict_fields(&dictionary_value(options, "dict"));
    fields.push(dictionary_value(options, "empty").len().to_string());
    fields.push(int_value(options, "scalar").to_string());
    fields
}

fn dictionary_sequence_fields<const N: usize>(values: [&Dictionary; N]) -> Vec<String> {
    let mut fields = Vec::new();
    for value in values {
        fields.extend(dict_fields(value));
    }
    fields
}

fn array_state_fields(options: &OptionSet) -> Vec<String> {
    let mut fields = array_fields(&array_value(options, "ints"));
    fields.extend(array_fields(&array_value(options, "words")));
    fields.extend(array_fields(&array_value(options, "required")));
    fields.push(int_value(options, "scalar").to_string());
    fields
}

fn array_required_fields(options: &OptionSet) -> Vec<String> {
    array_fields(&array_value(options, "required"))
}

fn array_fields(values: &[OptionValue]) -> Vec<String> {
    let mut fields = vec![values.len().to_string()];
    fields.extend(values.iter().map(array_value_field));
    fields
}

fn video_rate_state_fields(options: &OptionSet) -> [String; 3] {
    let rate = video_rate_value(options, "rate");
    [
        rate.num().to_string(),
        rate.den().to_string(),
        int_value(options, "scalar").to_string(),
    ]
}

fn color_state_fields(options: &OptionSet) -> [String; 5] {
    let rgba = color_value(options, "color").rgba();
    [
        rgba[0].to_string(),
        rgba[1].to_string(),
        rgba[2].to_string(),
        rgba[3].to_string(),
        int_value(options, "scalar").to_string(),
    ]
}

fn dict_fields(dict: &Dictionary) -> Vec<String> {
    let mut fields = vec![dict.len().to_string()];
    fields.extend(
        dict.entries()
            .iter()
            .map(|entry| format!("{}={}", entry.key(), entry.value())),
    );
    fields
}

fn int_value(options: &OptionSet, name: &str) -> i64 {
    match options.get(name) {
        Some(OptionValue::Int(value)) => *value,
        other => panic!("expected int option `{name}`, got {other:?}"),
    }
}

fn child_int_value(options: &OptionSet, child_name: &str, name: &str) -> i64 {
    match options.get_child_option(child_name, name) {
        Ok(OptionValue::Int(value)) => *value,
        other => panic!("expected child int option `{child_name}.{name}`, got {other:?}"),
    }
}

fn duration_value(options: &OptionSet, name: &str) -> i64 {
    match options.get(name) {
        Some(OptionValue::Duration(value)) => *value,
        other => panic!("expected duration option `{name}`, got {other:?}"),
    }
}

fn image_size_value(options: &OptionSet, name: &str) -> (i32, i32) {
    match options.get(name) {
        Some(OptionValue::ImageSize { width, height }) => (*width, *height),
        other => panic!("expected image-size option `{name}`, got {other:?}"),
    }
}

fn pixel_format_value(options: &OptionSet, name: &str) -> Option<PixelFormat> {
    match options.get(name) {
        Some(OptionValue::PixelFormat(value)) => *value,
        other => panic!("expected pixel-format option `{name}`, got {other:?}"),
    }
}

fn pixel_format_field(value: Option<PixelFormat>) -> String {
    pixel_format_index(value).to_string()
}

fn pixel_format_index(value: Option<PixelFormat>) -> i32 {
    match value {
        None => -1,
        Some(PixelFormat::Yuv420p) => 0,
        Some(PixelFormat::Yuyv422) => 1,
        Some(PixelFormat::Rgb24) => 2,
        Some(PixelFormat::Bgr24) => 3,
        Some(PixelFormat::Yuv422p) => 4,
        Some(PixelFormat::Yuv444p) => 5,
        Some(PixelFormat::Yuv410p) => 6,
        Some(PixelFormat::Yuv411p) => 7,
        Some(PixelFormat::Gray8) => 8,
        Some(PixelFormat::MonoWhite) => 9,
        Some(PixelFormat::MonoBlack) => 10,
        Some(PixelFormat::Pal8) => 11,
        Some(PixelFormat::YuvJ420p) => 12,
        Some(PixelFormat::YuvJ422p) => 13,
        Some(PixelFormat::YuvJ444p) => 14,
        Some(PixelFormat::Uyvy422) => 15,
        Some(PixelFormat::Uyyvyy411) => 16,
        Some(PixelFormat::Bgr8) => 17,
        Some(PixelFormat::Bgr4) => 18,
        Some(PixelFormat::Bgr4Byte) => 19,
        Some(PixelFormat::Rgb8) => 20,
        Some(PixelFormat::Rgb4) => 21,
        Some(PixelFormat::Rgb4Byte) => 22,
        Some(PixelFormat::Nv12) => 23,
        Some(PixelFormat::Nv21) => 24,
        Some(PixelFormat::Vaapi) => 44,
        Some(PixelFormat::Dxva2Vld) => 51,
        Some(PixelFormat::Vdpau) => 98,
        Some(PixelFormat::Qsv) => 114,
        Some(PixelFormat::Mmal) => 115,
        Some(PixelFormat::D3d11VaVld) => 116,
        Some(PixelFormat::Cuda) => 117,
        Some(PixelFormat::VideoToolboxVld) => 157,
        Some(PixelFormat::MediaCodec) => 164,
        Some(PixelFormat::D3d11) => 171,
        Some(PixelFormat::DrmPrime) => 178,
        Some(PixelFormat::OpenCl) => 179,
        Some(PixelFormat::Vulkan) => 190,
        Some(PixelFormat::D3d12) => 227,
        Some(PixelFormat::Amf) => 249,
        Some(PixelFormat::Gbrap32Le) => 257,
        Some(PixelFormat::Yuv444p10MsbLe) => 259,
        Some(PixelFormat::OhCodec) => 266,
        Some(format) => panic!("unsupported bounded pixel format `{}`", format.name()),
    }
}

fn sample_format_value(options: &OptionSet, name: &str) -> Option<SampleFormat> {
    match options.get(name) {
        Some(OptionValue::SampleFormat(value)) => *value,
        other => panic!("expected sample-format option `{name}`, got {other:?}"),
    }
}

fn sample_format_field(value: Option<SampleFormat>) -> String {
    sample_format_index(value).to_string()
}

fn sample_format_index(value: Option<SampleFormat>) -> i32 {
    match value {
        None => -1,
        Some(SampleFormat::U8) => 0,
        Some(SampleFormat::S16) => 1,
        Some(SampleFormat::S32) => 2,
        Some(SampleFormat::Flt) => 3,
        Some(SampleFormat::Dbl) => 4,
        Some(SampleFormat::U8P) => 5,
        Some(SampleFormat::S16P) => 6,
        Some(SampleFormat::S32P) => 7,
        Some(SampleFormat::FltP) => 8,
        Some(SampleFormat::DblP) => 9,
        Some(SampleFormat::S64) => 10,
        Some(SampleFormat::S64P) => 11,
    }
}

fn channel_layout_value(options: &OptionSet, name: &str) -> ChannelLayoutSpec {
    match options.get(name) {
        Some(OptionValue::ChannelLayout(value)) => value.clone(),
        other => panic!("expected channel-layout option `{name}`, got {other:?}"),
    }
}

fn binary_value(options: &OptionSet, name: &str) -> Vec<u8> {
    match options.get(name) {
        Some(OptionValue::NullBinary) => Vec::new(),
        Some(OptionValue::Binary(value)) => value.clone(),
        other => panic!("expected binary option `{name}`, got {other:?}"),
    }
}

fn dictionary_value(options: &OptionSet, name: &str) -> Dictionary {
    match options.get(name) {
        Some(OptionValue::NullDictionary) => Dictionary::new(),
        Some(OptionValue::Dictionary(value)) => value.clone(),
        other => panic!("expected dictionary option `{name}`, got {other:?}"),
    }
}

fn array_value(options: &OptionSet, name: &str) -> Vec<OptionValue> {
    match options.get(name) {
        Some(OptionValue::Array(value)) => value.clone(),
        other => panic!("expected array option `{name}`, got {other:?}"),
    }
}

fn array_value_field(value: &OptionValue) -> String {
    match value {
        OptionValue::Bool(value) => bool_int(*value).to_owned(),
        OptionValue::Int(value) => value.to_string(),
        OptionValue::Duration(value) => value.to_string(),
        OptionValue::ImageSize { width, height } => format!("{width}x{height}"),
        OptionValue::PixelFormat(value) => pixel_format_field(*value),
        OptionValue::SampleFormat(value) => sample_format_field(*value),
        OptionValue::ChannelLayout(value) => value.describe(),
        OptionValue::VideoRate(value) | OptionValue::Rational(value) => {
            format!("{}/{}", value.num(), value.den())
        }
        OptionValue::Color(value) => {
            let rgba = value.rgba();
            format!(
                "0x{:02x}{:02x}{:02x}{:02x}",
                rgba[0], rgba[1], rgba[2], rgba[3]
            )
        }
        OptionValue::NullBinary => "<null>".to_owned(),
        OptionValue::Binary(value) => binary_field(value),
        OptionValue::NullDictionary => "<null>".to_owned(),
        OptionValue::Dictionary(value) => value
            .to_pairs_string('=', ':')
            .expect("dictionary AVOption values use valid separators"),
        OptionValue::Array(values) => format!("nested:{}", values.len()),
        OptionValue::Float(value) => format!("{value:.6}"),
        OptionValue::String(value) => value.clone(),
        OptionValue::NullString => "<null>".to_owned(),
    }
}

fn binary_field(value: &[u8]) -> String {
    let mut formatted = String::with_capacity(value.len() * 2);
    for byte in value {
        use std::fmt::Write as _;
        let _ = write!(&mut formatted, "{byte:02X}");
    }
    formatted
}

fn video_rate_value(options: &OptionSet, name: &str) -> Rational {
    match options.get(name) {
        Some(OptionValue::VideoRate(value)) => *value,
        other => panic!("expected video-rate option `{name}`, got {other:?}"),
    }
}

fn color_value(options: &OptionSet, name: &str) -> RgbaColor {
    match options.get(name) {
        Some(OptionValue::Color(value)) => *value,
        other => panic!("expected color option `{name}`, got {other:?}"),
    }
}

fn color_field(value: RgbaColor) -> String {
    let rgba = value.rgba();
    format!("{}:{}:{}:{}", rgba[0], rgba[1], rgba[2], rgba[3])
}

fn bool_value(options: &OptionSet, name: &str) -> bool {
    match options.get(name) {
        Some(OptionValue::Bool(value)) => *value,
        other => panic!("expected bool option `{name}`, got {other:?}"),
    }
}

fn float_value(options: &OptionSet, name: &str) -> f64 {
    match options.get(name) {
        Some(OptionValue::Float(value)) => *value,
        other => panic!("expected float option `{name}`, got {other:?}"),
    }
}

fn rational_value(options: &OptionSet, name: &str) -> String {
    match options.get(name) {
        Some(OptionValue::Rational(value)) => format!("{}/{}", value.num(), value.den()),
        other => panic!("expected rational option `{name}`, got {other:?}"),
    }
}

fn string_value<'a>(options: &'a OptionSet, name: &str) -> &'a str {
    match options.get(name) {
        Some(OptionValue::String(value)) => value,
        other => panic!("expected string option `{name}`, got {other:?}"),
    }
}

fn bool_int(value: bool) -> &'static str {
    if value {
        "1"
    } else {
        "0"
    }
}

fn entry_name(entry: Option<OptionEntryMatch<'_>>) -> String {
    entry
        .map(|entry| entry.name().to_owned())
        .unwrap_or_else(|| "<null>".to_owned())
}

fn entry_target_name(entry: Option<OptionEntryMatch<'_>>) -> String {
    entry
        .map(|entry| format!("{}:{}", entry.child_name().unwrap_or("root"), entry.name()))
        .unwrap_or_else(|| "<null>".to_owned())
}

fn format_float(value: f64) -> String {
    let mut text = format!("{value:.17}");
    if text.contains('.') {
        while text.ends_with('0') {
            text.pop();
        }
        if text.ends_with('.') {
            text.push('0');
        }
    }
    text
}

fn ret(result: avutil::AvResult<()>) -> String {
    match result {
        Ok(()) => "0".to_owned(),
        Err(err) => err
            .code()
            .map(|code| code.raw().to_string())
            .unwrap_or_else(|| "no-code".to_owned()),
    }
}

fn ret_count(result: avutil::AvResult<usize>) -> String {
    match result {
        Ok(count) => count.to_string(),
        Err(err) => err
            .code()
            .map(|code| code.raw().to_string())
            .unwrap_or_else(|| "no-code".to_owned()),
    }
}

fn ret_array_size(result: avutil::AvResult<usize>) -> String {
    match result {
        Ok(count) => format!("0:{count}"),
        Err(err) => format!(
            "{}:0",
            err.code()
                .map(|code| code.raw().to_string())
                .unwrap_or_else(|| "no-code".to_owned())
        ),
    }
}

fn ret_array_values(result: avutil::AvResult<Vec<OptionValue>>) -> String {
    match result {
        Ok(values) => {
            let mut fields = vec!["0".to_owned(), values.len().to_string()];
            fields.extend(values.iter().map(array_value_field));
            fields.join(":")
        }
        Err(err) => format!(
            "{}:0",
            err.code()
                .map(|code| code.raw().to_string())
                .unwrap_or_else(|| "no-code".to_owned())
        ),
    }
}

fn ret_array_strings(result: avutil::AvResult<Vec<String>>) -> String {
    match result {
        Ok(values) => {
            let mut fields = vec!["0".to_owned(), values.len().to_string()];
            fields.extend(values);
            fields.join(":")
        }
        Err(err) => format!(
            "{}:0",
            err.code()
                .map(|code| code.raw().to_string())
                .unwrap_or_else(|| "no-code".to_owned())
        ),
    }
}

fn ret_array_doubles(result: avutil::AvResult<Vec<f64>>) -> String {
    match result {
        Ok(values) => {
            let mut fields = vec!["0".to_owned(), values.len().to_string()];
            fields.extend(values.into_iter().map(format_c_g17));
            fields.join(":")
        }
        Err(err) => format!(
            "{}:0",
            err.code()
                .map(|code| code.raw().to_string())
                .unwrap_or_else(|| "no-code".to_owned())
        ),
    }
}

fn ret_array_rationals(result: avutil::AvResult<Vec<Rational>>) -> String {
    match result {
        Ok(values) => {
            let mut fields = vec!["0".to_owned(), values.len().to_string()];
            fields.extend(
                values
                    .into_iter()
                    .map(|value| format!("{}/{}", value.num(), value.den())),
            );
            fields.join(":")
        }
        Err(err) => format!(
            "{}:0",
            err.code()
                .map(|code| code.raw().to_string())
                .unwrap_or_else(|| "no-code".to_owned())
        ),
    }
}

fn ret_value(result: avutil::AvResult<String>) -> String {
    match result {
        Ok(value) => format!("0:{value}"),
        Err(err) => format!(
            "{}:<null>",
            err.code()
                .map(|code| code.raw().to_string())
                .unwrap_or_else(|| "no-code".to_owned())
        ),
    }
}

fn ret_nullable_value(result: avutil::AvResult<Option<String>>) -> String {
    match result {
        Ok(Some(value)) => format!("0:{value}"),
        Ok(None) => "0:<null>".to_owned(),
        Err(err) => format!(
            "{}:<null>",
            err.code()
                .map(|code| code.raw().to_string())
                .unwrap_or_else(|| "no-code".to_owned())
        ),
    }
}

fn ret_i64(result: avutil::AvResult<i64>) -> String {
    match result {
        Ok(value) => format!("0:{value}"),
        Err(err) => format!(
            "{}:0",
            err.code()
                .map(|code| code.raw().to_string())
                .unwrap_or_else(|| "no-code".to_owned())
        ),
    }
}

fn ret_f64(result: avutil::AvResult<f64>) -> String {
    match result {
        Ok(value) => format!("0:{}", format_c_g17(value)),
        Err(err) => format!(
            "{}:0",
            err.code()
                .map(|code| code.raw().to_string())
                .unwrap_or_else(|| "no-code".to_owned())
        ),
    }
}

fn ret_q(result: avutil::AvResult<Rational>) -> String {
    match result {
        Ok(value) => format!("0:{}/{}", value.num(), value.den()),
        Err(err) => format!(
            "{}:0/1",
            err.code()
                .map(|code| code.raw().to_string())
                .unwrap_or_else(|| "no-code".to_owned())
        ),
    }
}

fn ret_image_size(result: avutil::AvResult<(i32, i32)>) -> String {
    match result {
        Ok((width, height)) => format!("0:{width}x{height}"),
        Err(err) => format!(
            "{}:0x0",
            err.code()
                .map(|code| code.raw().to_string())
                .unwrap_or_else(|| "no-code".to_owned())
        ),
    }
}

fn ret_pixel_format(result: avutil::AvResult<Option<PixelFormat>>) -> String {
    match result {
        Ok(value) => format!("0:{}", pixel_format_index(value)),
        Err(err) => format!(
            "{}:-1",
            err.code()
                .map(|code| code.raw().to_string())
                .unwrap_or_else(|| "no-code".to_owned())
        ),
    }
}

fn ret_sample_format(result: avutil::AvResult<Option<SampleFormat>>) -> String {
    match result {
        Ok(value) => format!("0:{}", sample_format_index(value)),
        Err(err) => format!(
            "{}:-1",
            err.code()
                .map(|code| code.raw().to_string())
                .unwrap_or_else(|| "no-code".to_owned())
        ),
    }
}

fn ret_channel_layout(result: avutil::AvResult<ChannelLayoutSpec>) -> String {
    match result {
        Ok(value) => format!("0:{}", value.describe()),
        Err(err) => format!(
            "{}:0 channels",
            err.code()
                .map(|code| code.raw().to_string())
                .unwrap_or_else(|| "no-code".to_owned())
        ),
    }
}

fn ret_dictionary(result: avutil::AvResult<Dictionary>) -> String {
    match result {
        Ok(value) => {
            let mut fields = vec!["0".to_owned(), value.len().to_string()];
            fields.extend(
                value
                    .entries()
                    .iter()
                    .map(|entry| format!("{}={}", entry.key(), entry.value())),
            );
            fields.join(":")
        }
        Err(err) => format!(
            "{}:0",
            err.code()
                .map(|code| code.raw().to_string())
                .unwrap_or_else(|| "no-code".to_owned())
        ),
    }
}

fn ret_serialize(result: avutil::AvResult<String>) -> String {
    ret_value(result)
}

fn ret_ranges(result: avutil::AvResult<AvOptionRanges>) -> String {
    match result {
        Ok(ranges) => {
            let first = ranges.ranges().first().expect("one default range");
            format!(
                "{}:{}:{}:{}:{}:{}:{}:{}",
                ranges.nb_components(),
                ranges.nb_ranges(),
                ranges.nb_components(),
                format_c_g17(first.value_min()),
                format_c_g17(first.value_max()),
                format_c_g17(first.component_min()),
                format_c_g17(first.component_max()),
                i32::from(first.is_range())
            )
        }
        Err(err) => format!(
            "{}:<null>",
            err.code()
                .map(|code| code.raw().to_string())
                .unwrap_or_else(|| "no-code".to_owned())
        ),
    }
}

fn format_c_g17(value: f64) -> String {
    value.to_string()
}

fn insert_row<const N: usize>(
    rows: &mut BTreeMap<String, Vec<String>>,
    name: &str,
    fields: [impl ToString; N],
) {
    rows.insert(
        name.to_owned(),
        fields.into_iter().map(|field| field.to_string()).collect(),
    );
}

fn row_fields<'a>(rows: &'a BTreeMap<String, Vec<String>>, name: &str) -> &'a [String] {
    rows.get(name)
        .unwrap_or_else(|| panic!("missing oracle row `{name}`"))
}

fn parse_oracle_output(stdout: &str) -> BTreeMap<String, Vec<String>> {
    let mut rows = BTreeMap::new();
    for line in stdout.lines() {
        let fields: Vec<_> = line.split('|').map(str::to_owned).collect();
        assert!(fields.len() >= 2, "invalid oracle row `{line}`");
        let name = fields[0].clone();
        assert!(
            rows.insert(name, fields[1..].to_vec()).is_none(),
            "duplicate oracle row `{line}`"
        );
    }
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
            .expect("run WSL libavutil options oracle")
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
            .expect("run libavutil options oracle")
    };

    assert!(
        output.status.success(),
        "libavutil options oracle failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout).expect("oracle stdout should be UTF-8")
}

fn oracle_c_source() -> &'static str {
    r#"#include <inttypes.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include <libavutil/avutil.h>
#include <libavutil/channel_layout.h>
#include <libavutil/dict.h>
#include <libavutil/mem.h>
#include <libavutil/opt.h>
#include <libavutil/pixfmt.h>
#include <libavutil/rational.h>
#include <libavutil/samplefmt.h>

#define ROW_INT(name, value) printf("%s|%d\n", name, (int)(value))

typedef struct ChildOptions {
    const AVClass *av_class;
    int64_t threads;
    int64_t child_only;
    int64_t child_readonly;
} ChildOptions;

typedef struct TestOptions {
    const AVClass *av_class;
    ChildOptions child;
    int64_t threads;
    int bitexact;
    double quality;
    AVRational aspect_ratio;
    char *metadata;
    char *nullable_metadata;
    int64_t preset_level;
    int64_t exported;
} TestOptions;

typedef struct DurationOptions {
    const AVClass *av_class;
    int64_t timeout;
} DurationOptions;

typedef struct ImageSizeOptions {
    const AVClass *av_class;
    int size[2];
    int64_t scalar;
} ImageSizeOptions;

typedef struct ImageSizeChildOptions {
    const AVClass *av_class;
    int child_size[2];
} ImageSizeChildOptions;

typedef struct ImageSizeParentOptions {
    const AVClass *av_class;
    ImageSizeChildOptions child;
} ImageSizeParentOptions;

typedef struct PixelFormatOptions {
    const AVClass *av_class;
    enum AVPixelFormat pix_fmt;
    int64_t scalar;
} PixelFormatOptions;

typedef struct SampleFormatOptions {
    const AVClass *av_class;
    enum AVSampleFormat sample_fmt;
    int64_t scalar;
} SampleFormatOptions;

typedef struct ChannelLayoutOptions {
    const AVClass *av_class;
    AVChannelLayout layout;
    int64_t scalar;
} ChannelLayoutOptions;

typedef struct BinaryOptions {
    const AVClass *av_class;
    uint8_t *blob;
    int blob_size;
    uint8_t *nullable_blob;
    int nullable_blob_size;
    int64_t scalar;
} BinaryOptions;

typedef struct DictionaryOptions {
    const AVClass *av_class;
    AVDictionary *dict;
    AVDictionary *empty;
    int64_t scalar;
} DictionaryOptions;

typedef struct ArrayOptions {
    const AVClass *av_class;
    int64_t *ints;
    unsigned ints_count;
    char **words;
    unsigned words_count;
    int64_t *required;
    unsigned required_count;
    int64_t scalar;
} ArrayOptions;

typedef struct VideoRateOptions {
    const AVClass *av_class;
    AVRational rate;
    int64_t scalar;
} VideoRateOptions;

typedef struct ColorOptions {
    const AVClass *av_class;
    uint8_t color[4];
    int64_t scalar;
} ColorOptions;

static const AVOption child_options[] = {
    { "threads", "child worker count", offsetof(ChildOptions, threads),
      AV_OPT_TYPE_INT64, { .i64 = 2 }, 1, 16, AV_OPT_FLAG_DECODING_PARAM },
    { "child_only", "child-only value", offsetof(ChildOptions, child_only),
      AV_OPT_TYPE_INT64, { .i64 = 5 }, 0, 10, AV_OPT_FLAG_DECODING_PARAM },
    { "child_readonly", "child read-only value", offsetof(ChildOptions, child_readonly),
      AV_OPT_TYPE_INT64, { .i64 = 0 }, 0, 10, AV_OPT_FLAG_DECODING_PARAM | AV_OPT_FLAG_READONLY },
    { NULL }
};

static const AVClass child_class = {
    .class_name = "rust-options-oracle-child",
    .item_name = av_default_item_name,
    .option = child_options,
    .version = LIBAVUTIL_VERSION_INT,
};

static void *test_child_next(void *obj, void *prev) {
    TestOptions *ctx = (TestOptions *)obj;

    if (prev)
        return NULL;
    return &ctx->child;
}

static const AVOption test_options[] = {
    { "threads", "worker count", offsetof(TestOptions, threads),
      AV_OPT_TYPE_INT64, { .i64 = 1 }, 1, 64, AV_OPT_FLAG_ENCODING_PARAM },
    { "bitexact", "bit-exact output", offsetof(TestOptions, bitexact),
      AV_OPT_TYPE_BOOL, { .i64 = 0 }, 0, 1, AV_OPT_FLAG_ENCODING_PARAM },
    { "quality", "quality", offsetof(TestOptions, quality),
      AV_OPT_TYPE_DOUBLE, { .dbl = 0.5 }, 0.0, 1.0, AV_OPT_FLAG_ENCODING_PARAM },
    { "aspect_ratio", "aspect ratio", offsetof(TestOptions, aspect_ratio),
      AV_OPT_TYPE_RATIONAL, { .dbl = 1.0 }, 1.0, 16.0 / 9.0, AV_OPT_FLAG_ENCODING_PARAM },
    { "metadata", "metadata", offsetof(TestOptions, metadata),
      AV_OPT_TYPE_STRING, { .str = "default" }, 0, 0, AV_OPT_FLAG_ENCODING_PARAM },
    { "nullable_metadata", "nullable metadata", offsetof(TestOptions, nullable_metadata),
      AV_OPT_TYPE_STRING, { .str = NULL }, 0, 0, AV_OPT_FLAG_ENCODING_PARAM },
    { "preset_level", "preset level", offsetof(TestOptions, preset_level),
      AV_OPT_TYPE_INT64, { .i64 = 0 }, 0, 10, AV_OPT_FLAG_ENCODING_PARAM, "PRESET" },
    { "fast", "fast preset", 0,
      AV_OPT_TYPE_CONST, { .i64 = 2 }, 0, 0, AV_OPT_FLAG_ENCODING_PARAM, "PRESET" },
    { "slow", "slow preset", 0,
      AV_OPT_TYPE_CONST, { .i64 = 8 }, 0, 0, AV_OPT_FLAG_ENCODING_PARAM, "PRESET" },
    { "exported", "read-only exported value", offsetof(TestOptions, exported),
      AV_OPT_TYPE_INT64, { .i64 = 4 }, 0, 8, AV_OPT_FLAG_EXPORT | AV_OPT_FLAG_READONLY },
    { NULL }
};

static const AVClass test_class = {
    .class_name = "rust-options-oracle",
    .item_name = av_default_item_name,
    .option = test_options,
    .version = LIBAVUTIL_VERSION_INT,
    .child_next = test_child_next,
};

static const AVOption duration_options[] = {
    { "timeout", "timeout", offsetof(DurationOptions, timeout),
      AV_OPT_TYPE_DURATION, { .i64 = 0 }, 0, 7200000000LL, AV_OPT_FLAG_ENCODING_PARAM },
    { NULL }
};

static const AVClass duration_class = {
    .class_name = "rust-options-oracle-duration",
    .item_name = av_default_item_name,
    .option = duration_options,
    .version = LIBAVUTIL_VERSION_INT,
};

static const AVOption image_size_options[] = {
    { "size", "image size", offsetof(ImageSizeOptions, size),
      AV_OPT_TYPE_IMAGE_SIZE, { .str = "320x240" }, 0, 0, AV_OPT_FLAG_ENCODING_PARAM },
    { "scalar", "scalar", offsetof(ImageSizeOptions, scalar),
      AV_OPT_TYPE_INT64, { .i64 = 4 }, 0, 10, AV_OPT_FLAG_ENCODING_PARAM },
    { NULL }
};

static const AVOption image_size_child_options[] = {
    { "child_size", "child image size", offsetof(ImageSizeChildOptions, child_size),
      AV_OPT_TYPE_IMAGE_SIZE, { .str = "320x240" }, 0, 0, AV_OPT_FLAG_DECODING_PARAM },
    { NULL }
};

static const AVOption image_size_parent_options[] = {
    { NULL }
};

static void *image_size_parent_child_next(void *obj, void *prev) {
    ImageSizeParentOptions *ctx = (ImageSizeParentOptions *)obj;

    if (prev)
        return NULL;
    return &ctx->child;
}

static const AVOption pixel_format_options[] = {
    { "pix_fmt", "pixel format", offsetof(PixelFormatOptions, pix_fmt),
      AV_OPT_TYPE_PIXEL_FMT, { .i64 = AV_PIX_FMT_YUV420P }, AV_PIX_FMT_NONE, AV_PIX_FMT_NB - 1, AV_OPT_FLAG_ENCODING_PARAM },
    { "scalar", "scalar", offsetof(PixelFormatOptions, scalar),
      AV_OPT_TYPE_INT64, { .i64 = 4 }, 0, 10, AV_OPT_FLAG_ENCODING_PARAM },
    { NULL }
};

static const AVOption sample_format_options[] = {
    { "sample_fmt", "sample format", offsetof(SampleFormatOptions, sample_fmt),
      AV_OPT_TYPE_SAMPLE_FMT, { .i64 = AV_SAMPLE_FMT_S16 }, AV_SAMPLE_FMT_NONE, AV_SAMPLE_FMT_NB - 1, AV_OPT_FLAG_ENCODING_PARAM },
    { "scalar", "scalar", offsetof(SampleFormatOptions, scalar),
      AV_OPT_TYPE_INT64, { .i64 = 4 }, 0, 10, AV_OPT_FLAG_ENCODING_PARAM },
    { NULL }
};

static const AVOption channel_layout_options[] = {
    { "layout", "channel layout", offsetof(ChannelLayoutOptions, layout),
      AV_OPT_TYPE_CHLAYOUT, { .str = "stereo" }, 0, 0, AV_OPT_FLAG_ENCODING_PARAM },
    { "scalar", "scalar", offsetof(ChannelLayoutOptions, scalar),
      AV_OPT_TYPE_INT64, { .i64 = 4 }, 0, 10, AV_OPT_FLAG_ENCODING_PARAM },
    { NULL }
};

static const AVOption binary_options[] = {
    { "blob", "binary data", offsetof(BinaryOptions, blob),
      AV_OPT_TYPE_BINARY, { .str = "0001aaff" }, 0, 0, AV_OPT_FLAG_ENCODING_PARAM },
    { "nullable_blob", "nullable binary", offsetof(BinaryOptions, nullable_blob),
      AV_OPT_TYPE_BINARY, { .str = NULL }, 0, 0, AV_OPT_FLAG_ENCODING_PARAM },
    { "scalar", "scalar", offsetof(BinaryOptions, scalar),
      AV_OPT_TYPE_INT64, { .i64 = 4 }, 0, 10, AV_OPT_FLAG_ENCODING_PARAM },
    { NULL }
};

static const AVOption dictionary_options[] = {
    { "dict", "dictionary data", offsetof(DictionaryOptions, dict),
      AV_OPT_TYPE_DICT, { .str = "title=clip:note=hello\\:world" }, 0, 0, AV_OPT_FLAG_ENCODING_PARAM },
    { "empty", "empty dictionary", offsetof(DictionaryOptions, empty),
      AV_OPT_TYPE_DICT, { .str = NULL }, 0, 0, AV_OPT_FLAG_ENCODING_PARAM },
    { "scalar", "scalar", offsetof(DictionaryOptions, scalar),
      AV_OPT_TYPE_INT64, { .i64 = 4 }, 0, 10, AV_OPT_FLAG_ENCODING_PARAM },
    { NULL }
};

static const AVOptionArrayDef ints_array_def = {
    .def = "1,2",
    .size_min = 0,
    .size_max = 4,
    .sep = ',',
};

static const AVOptionArrayDef words_array_def = {
    .def = "alpha,beta\\,gamma",
    .size_min = 0,
    .size_max = 3,
    .sep = ',',
};

static const AVOptionArrayDef required_array_def = {
    .def = "3,4",
    .size_min = 2,
    .size_max = 3,
    .sep = ',',
};

static const AVOption array_options[] = {
    { "ints", "integer array", offsetof(ArrayOptions, ints),
      AV_OPT_TYPE_INT64 | AV_OPT_TYPE_FLAG_ARRAY, { .arr = &ints_array_def }, 0, 10, AV_OPT_FLAG_ENCODING_PARAM },
    { "words", "string array", offsetof(ArrayOptions, words),
      AV_OPT_TYPE_STRING | AV_OPT_TYPE_FLAG_ARRAY, { .arr = &words_array_def }, 0, 0, AV_OPT_FLAG_ENCODING_PARAM },
    { "required", "required integer array", offsetof(ArrayOptions, required),
      AV_OPT_TYPE_INT64 | AV_OPT_TYPE_FLAG_ARRAY, { .arr = &required_array_def }, 0, 10, AV_OPT_FLAG_ENCODING_PARAM },
    { "scalar", "scalar", offsetof(ArrayOptions, scalar),
      AV_OPT_TYPE_INT64, { .i64 = 4 }, 0, 10, AV_OPT_FLAG_ENCODING_PARAM },
    { NULL }
};

static const AVClass image_size_class = {
    .class_name = "rust-options-oracle-image-size",
    .item_name = av_default_item_name,
    .option = image_size_options,
    .version = LIBAVUTIL_VERSION_INT,
};

static const AVClass image_size_child_class = {
    .class_name = "rust-options-oracle-image-size-child",
    .item_name = av_default_item_name,
    .option = image_size_child_options,
    .version = LIBAVUTIL_VERSION_INT,
};

static const AVClass image_size_parent_class = {
    .class_name = "rust-options-oracle-image-size-parent",
    .item_name = av_default_item_name,
    .option = image_size_parent_options,
    .version = LIBAVUTIL_VERSION_INT,
    .child_next = image_size_parent_child_next,
};

static const AVClass pixel_format_class = {
    .class_name = "rust-options-oracle-pixel-format",
    .item_name = av_default_item_name,
    .option = pixel_format_options,
    .version = LIBAVUTIL_VERSION_INT,
};

static const AVClass sample_format_class = {
    .class_name = "rust-options-oracle-sample-format",
    .item_name = av_default_item_name,
    .option = sample_format_options,
    .version = LIBAVUTIL_VERSION_INT,
};

static const AVClass channel_layout_class = {
    .class_name = "rust-options-oracle-channel-layout",
    .item_name = av_default_item_name,
    .option = channel_layout_options,
    .version = LIBAVUTIL_VERSION_INT,
};

static const AVClass binary_class = {
    .class_name = "rust-options-oracle-binary",
    .item_name = av_default_item_name,
    .option = binary_options,
    .version = LIBAVUTIL_VERSION_INT,
};

static const AVClass dictionary_class = {
    .class_name = "rust-options-oracle-dictionary",
    .item_name = av_default_item_name,
    .option = dictionary_options,
    .version = LIBAVUTIL_VERSION_INT,
};

static const AVClass array_class = {
    .class_name = "rust-options-oracle-array",
    .item_name = av_default_item_name,
    .option = array_options,
    .version = LIBAVUTIL_VERSION_INT,
};

static const AVOption video_rate_options[] = {
    { "rate", "video rate", offsetof(VideoRateOptions, rate),
      AV_OPT_TYPE_VIDEO_RATE, { .str = "25" }, 1, 120, AV_OPT_FLAG_ENCODING_PARAM },
    { "scalar", "scalar", offsetof(VideoRateOptions, scalar),
      AV_OPT_TYPE_INT64, { .i64 = 4 }, 0, 10, AV_OPT_FLAG_ENCODING_PARAM },
    { NULL }
};

static const AVClass video_rate_class = {
    .class_name = "rust-options-oracle-video-rate",
    .item_name = av_default_item_name,
    .option = video_rate_options,
    .version = LIBAVUTIL_VERSION_INT,
};

static const AVOption color_options[] = {
    { "color", "color", offsetof(ColorOptions, color),
      AV_OPT_TYPE_COLOR, { .str = "red" }, 0, 0, AV_OPT_FLAG_ENCODING_PARAM },
    { "scalar", "scalar", offsetof(ColorOptions, scalar),
      AV_OPT_TYPE_INT64, { .i64 = 4 }, 0, 10, AV_OPT_FLAG_ENCODING_PARAM },
    { NULL }
};

static const AVClass color_class = {
    .class_name = "rust-options-oracle-color",
    .item_name = av_default_item_name,
    .option = color_options,
    .version = LIBAVUTIL_VERSION_INT,
};

static void init_context(TestOptions *ctx) {
    memset(ctx, 0, sizeof(*ctx));
    ctx->av_class = &test_class;
    ctx->child.av_class = &child_class;
    av_opt_set_defaults(ctx);
    av_opt_set_defaults(&ctx->child);
}

static void init_duration_context(DurationOptions *ctx) {
    memset(ctx, 0, sizeof(*ctx));
    ctx->av_class = &duration_class;
    av_opt_set_defaults(ctx);
}

static void init_image_size_context(ImageSizeOptions *ctx) {
    memset(ctx, 0, sizeof(*ctx));
    ctx->av_class = &image_size_class;
    av_opt_set_defaults(ctx);
}

static void init_image_size_parent_context(ImageSizeParentOptions *ctx) {
    memset(ctx, 0, sizeof(*ctx));
    ctx->av_class = &image_size_parent_class;
    ctx->child.av_class = &image_size_child_class;
    av_opt_set_defaults(ctx);
    av_opt_set_defaults(&ctx->child);
}

static void init_pixel_format_context(PixelFormatOptions *ctx) {
    memset(ctx, 0, sizeof(*ctx));
    ctx->av_class = &pixel_format_class;
    av_opt_set_defaults(ctx);
}

static void init_sample_format_context(SampleFormatOptions *ctx) {
    memset(ctx, 0, sizeof(*ctx));
    ctx->av_class = &sample_format_class;
    av_opt_set_defaults(ctx);
}

static void init_channel_layout_context(ChannelLayoutOptions *ctx) {
    memset(ctx, 0, sizeof(*ctx));
    ctx->av_class = &channel_layout_class;
    av_opt_set_defaults(ctx);
}

static void init_binary_context(BinaryOptions *ctx) {
    memset(ctx, 0, sizeof(*ctx));
    ctx->av_class = &binary_class;
    av_opt_set_defaults(ctx);
}

static void init_dictionary_context(DictionaryOptions *ctx) {
    memset(ctx, 0, sizeof(*ctx));
    ctx->av_class = &dictionary_class;
    av_opt_set_defaults(ctx);
}

static void init_array_context(ArrayOptions *ctx) {
    memset(ctx, 0, sizeof(*ctx));
    ctx->av_class = &array_class;
    av_opt_set_defaults(ctx);
}

static void init_video_rate_context(VideoRateOptions *ctx) {
    memset(ctx, 0, sizeof(*ctx));
    ctx->av_class = &video_rate_class;
    av_opt_set_defaults(ctx);
}

static void init_color_context(ColorOptions *ctx) {
    memset(ctx, 0, sizeof(*ctx));
    ctx->av_class = &color_class;
    av_opt_set_defaults(ctx);
}

static void print_flags(void) {
    printf("flags|%d|%d|%d|%d|%d|%d|%d|%d|%d|%d|%d|%d|%d\n",
           AV_OPT_FLAG_ENCODING_PARAM,
           AV_OPT_FLAG_DECODING_PARAM,
           AV_OPT_FLAG_AUDIO_PARAM,
           AV_OPT_FLAG_VIDEO_PARAM,
           AV_OPT_FLAG_SUBTITLE_PARAM,
           AV_OPT_FLAG_EXPORT,
           AV_OPT_FLAG_READONLY,
           AV_OPT_FLAG_BSF_PARAM,
           AV_OPT_FLAG_RUNTIME_PARAM,
           AV_OPT_FLAG_FILTERING_PARAM,
           AV_OPT_FLAG_DEPRECATED,
           AV_OPT_FLAG_CHILD_CONSTS,
           AV_OPT_FLAG_ENCODING_PARAM |
           AV_OPT_FLAG_DECODING_PARAM |
           AV_OPT_FLAG_AUDIO_PARAM |
           AV_OPT_FLAG_VIDEO_PARAM |
           AV_OPT_FLAG_SUBTITLE_PARAM |
           AV_OPT_FLAG_EXPORT |
           AV_OPT_FLAG_READONLY |
           AV_OPT_FLAG_BSF_PARAM |
           AV_OPT_FLAG_RUNTIME_PARAM |
           AV_OPT_FLAG_FILTERING_PARAM |
           AV_OPT_FLAG_DEPRECATED |
           AV_OPT_FLAG_CHILD_CONSTS);
}

static void print_types(void) {
    printf("types|%d|%d|%d|%d|%d|%d|%d|%d|%d|%d|%d|%d|%d|%d|%d|%d|%d|%d|%d|%d|%d\n",
           AV_OPT_TYPE_FLAGS,
           AV_OPT_TYPE_INT,
           AV_OPT_TYPE_INT64,
           AV_OPT_TYPE_DOUBLE,
           AV_OPT_TYPE_FLOAT,
           AV_OPT_TYPE_STRING,
           AV_OPT_TYPE_RATIONAL,
           AV_OPT_TYPE_BINARY,
           AV_OPT_TYPE_DICT,
           AV_OPT_TYPE_UINT64,
           AV_OPT_TYPE_CONST,
           AV_OPT_TYPE_IMAGE_SIZE,
           AV_OPT_TYPE_PIXEL_FMT,
           AV_OPT_TYPE_SAMPLE_FMT,
           AV_OPT_TYPE_VIDEO_RATE,
           AV_OPT_TYPE_DURATION,
           AV_OPT_TYPE_COLOR,
           AV_OPT_TYPE_BOOL,
           AV_OPT_TYPE_CHLAYOUT,
           AV_OPT_TYPE_UINT,
           AV_OPT_TYPE_FLAG_ARRAY);
}

static void print_search_flags(void) {
    printf("search-flags|%d|%d|%d|%d\n",
           AV_OPT_SEARCH_CHILDREN,
           AV_OPT_SEARCH_FAKE_OBJ,
           AV_OPT_ALLOW_NULL,
           AV_OPT_ARRAY_REPLACE);
}

static void print_serialize_flags(void) {
    printf("serialize-flags|%d|%d|%d\n",
           AV_OPT_SERIALIZE_SKIP_DEFAULTS,
           AV_OPT_SERIALIZE_OPT_FLAGS_EXACT,
           AV_OPT_SERIALIZE_SEARCH_CHILDREN);
}

static const char *option_name_or_null(const AVOption *option) {
    return option ? option->name : "<null>";
}

static void print_next_order(const TestOptions *ctx) {
    const AVOption *option = NULL;

    printf("next:order");
    while ((option = av_opt_next(ctx, option))) {
        printf("|%s", option->name);
    }
    printf("\n");
}

static void print_find_rows(const TestOptions *ctx) {
    printf("find:root|%s|%s|%s|%s|%s|%s|%s\n",
           option_name_or_null(av_opt_find(ctx, "threads", NULL, AV_OPT_FLAG_ENCODING_PARAM, 0)),
           option_name_or_null(av_opt_find(ctx, "THREADS", NULL, 0, 0)),
           option_name_or_null(av_opt_find(ctx, "fast", NULL, 0, 0)),
           option_name_or_null(av_opt_find(ctx, "fast", "PRESET", AV_OPT_FLAG_ENCODING_PARAM, 0)),
           option_name_or_null(av_opt_find(ctx, "slow", "preset", 0, 0)),
           option_name_or_null(av_opt_find(ctx, "exported", NULL, AV_OPT_FLAG_EXPORT | AV_OPT_FLAG_READONLY, 0)),
           option_name_or_null(av_opt_find(ctx, "exported", NULL, AV_OPT_FLAG_VIDEO_PARAM, 0)));
}

static const char *target_name_or_null(const TestOptions *ctx, const void *target) {
    if (target == ctx)
        return "root";
    if (target == &ctx->child)
        return "decoder";
    return "<null>";
}

static void print_find2_value(const TestOptions *ctx, const char *name, int flags, int search_flags) {
    void *target = NULL;
    const AVOption *option = av_opt_find2((void *)ctx, name, NULL, flags, search_flags, &target);

    if (!option) {
        printf("|<null>");
    } else {
        printf("|%s:%s", target_name_or_null(ctx, target), option->name);
    }
}

static void print_find_children_row(const TestOptions *ctx) {
    printf("find:children");
    print_find2_value(ctx, "threads", 0, AV_OPT_SEARCH_CHILDREN);
    print_find2_value(ctx, "threads", AV_OPT_FLAG_ENCODING_PARAM, AV_OPT_SEARCH_CHILDREN);
    print_find2_value(ctx, "child_only", AV_OPT_FLAG_DECODING_PARAM, AV_OPT_SEARCH_CHILDREN);
    print_find2_value(ctx, "child_only", 0, 0);
    printf("\n");
}

static void print_state(const char *name, const TestOptions *ctx) {
    printf("%s|%" PRId64 "|%d|%.17g|%d/%d|%s|%" PRId64 "\n",
           name,
           ctx->threads,
           ctx->bitexact,
           ctx->quality,
           ctx->aspect_ratio.num,
           ctx->aspect_ratio.den,
           ctx->metadata ? ctx->metadata : "<null>",
           ctx->preset_level);
}

static void print_child_state(const char *name, const TestOptions *ctx) {
    printf("%s|%" PRId64 "|%" PRId64 "|%" PRId64 "|%" PRId64 "\n",
           name,
           ctx->threads,
           ctx->child.threads,
           ctx->child.child_only,
           ctx->child.child_readonly);
}

static void print_child_dict_state(const char *name, const TestOptions *ctx) {
    printf("%s|%" PRId64 "|%" PRId64 "|%" PRId64 "|%.17g\n",
           name,
           ctx->threads,
           ctx->child.threads,
           ctx->child.child_only,
           ctx->quality);
}

static void print_copy_state(const char *name, const TestOptions *ctx) {
    printf("%s|%" PRId64 "|%d|%.17g|%d/%d|%s|%" PRId64
           "|%" PRId64 "|%" PRId64 "|%" PRId64 "\n",
           name,
           ctx->threads,
           ctx->bitexact,
           ctx->quality,
           ctx->aspect_ratio.num,
           ctx->aspect_ratio.den,
           ctx->metadata ? ctx->metadata : "<null>",
           ctx->preset_level,
           ctx->child.threads,
           ctx->child.child_only,
           ctx->child.child_readonly);
}

static void print_child_option_state(const char *name, const ChildOptions *ctx) {
    printf("%s|%" PRId64 "|%" PRId64 "|%" PRId64 "\n",
           name,
           ctx->threads,
           ctx->child_only,
           ctx->child_readonly);
}

static void print_dict_entries(const AVDictionary *dict) {
    const AVDictionaryEntry *entry = NULL;

    printf("|%d", av_dict_count(dict));
    while ((entry = av_dict_iterate(dict, entry))) {
        printf("|%s=%s", entry->key, entry->value);
    }
}

static void print_get_value(const void *ctx, const char *name) {
    uint8_t *value = NULL;
    int ret = av_opt_get((void *)ctx, name, 0, &value);
    printf("|%d:%s", ret, ret >= 0 && value ? (const char *)value : "<null>");
    av_free(value);
}

static void print_get_value_flags(const void *ctx, const char *name, int search_flags) {
    uint8_t *value = NULL;
    int ret = av_opt_get((void *)ctx, name, search_flags, &value);
    printf("|%d:%s", ret, ret >= 0 && value ? (const char *)value : "<null>");
    av_free(value);
}

static void print_get_int_value(const void *ctx, const char *name, int search_flags) {
    int64_t value = 0;
    int ret = av_opt_get_int((void *)ctx, name, search_flags, &value);
    printf("|%d:%" PRId64, ret, value);
}

static void print_get_double_value(const void *ctx, const char *name, int search_flags) {
    double value = 0.0;
    int ret = av_opt_get_double((void *)ctx, name, search_flags, &value);
    printf("|%d:%.17g", ret, value);
}

static void print_get_q_value(const void *ctx, const char *name, int search_flags) {
    AVRational value = { 0, 1 };
    int ret = av_opt_get_q((void *)ctx, name, search_flags, &value);
    printf("|%d:%d/%d", ret, value.num, value.den);
}

static void print_get_image_size_value(const void *ctx, const char *name, int search_flags) {
    int width = 0;
    int height = 0;
    int ret = av_opt_get_image_size((void *)ctx, name, search_flags, &width, &height);
    printf("|%d:%dx%d", ret, width, height);
}

static void print_get_pixel_format_value(const void *ctx, const char *name, int search_flags) {
    enum AVPixelFormat value = AV_PIX_FMT_NONE;
    int ret = av_opt_get_pixel_fmt((void *)ctx, name, search_flags, &value);
    printf("|%d:%d", ret, value);
}

static void print_get_sample_format_value(const void *ctx, const char *name, int search_flags) {
    enum AVSampleFormat value = AV_SAMPLE_FMT_NONE;
    int ret = av_opt_get_sample_fmt((void *)ctx, name, search_flags, &value);
    printf("|%d:%d", ret, value);
}

static void describe_channel_layout(const AVChannelLayout *layout, char *buffer, size_t size) {
    int ret = av_channel_layout_describe(layout, buffer, size);
    if (ret < 0) {
        snprintf(buffer, size, "<err:%d>", ret);
    }
}

static void print_get_channel_layout_value(const void *ctx, const char *name, int search_flags) {
    AVChannelLayout value = { 0 };
    char desc[256] = { 0 };
    int ret = av_opt_get_chlayout((void *)ctx, name, search_flags, &value);
    describe_channel_layout(&value, desc, sizeof(desc));
    printf("|%d:%s", ret, desc);
    av_channel_layout_uninit(&value);
}

static void print_get_dict_value(const void *ctx, const char *name, int search_flags) {
    AVDictionary *value = NULL;
    const AVDictionaryEntry *entry = NULL;
    int ret = av_opt_get_dict_val((void *)ctx, name, search_flags, &value);

    printf("|%d", ret);
    if (ret >= 0) {
        printf(":%d", av_dict_count(value));
        while ((entry = av_dict_iterate(value, entry))) {
            printf(":%s=%s", entry->key, entry->value);
        }
    } else {
        printf(":0");
    }
    av_dict_free(&value);
}

static void print_get_video_rate_value(const void *ctx, const char *name, int search_flags) {
    AVRational value = { 0, 1 };
    int ret = av_opt_get_video_rate((void *)ctx, name, search_flags, &value);
    printf("|%d:%d/%d", ret, value.num, value.den);
}

static void print_get_row(const char *name, const TestOptions *ctx) {
    printf("%s", name);
    print_get_value(ctx, "threads");
    print_get_value(ctx, "bitexact");
    print_get_value(ctx, "quality");
    print_get_value(ctx, "aspect_ratio");
    print_get_value(ctx, "metadata");
    print_get_value(ctx, "preset_level");
    printf("\n");
}

static void print_get_errors(const TestOptions *ctx) {
    printf("get:errors");
    print_get_value(ctx, "THREADS");
    print_get_value(ctx, "fast");
    printf("\n");
}

static void print_get_allow_null_row(const TestOptions *ctx) {
    printf("get:allow-null");
    print_get_value(ctx, "nullable_metadata");
    print_get_value_flags(ctx, "nullable_metadata", 0);
    print_get_value_flags(ctx, "nullable_metadata", AV_OPT_ALLOW_NULL);
    print_get_value_flags(ctx, "metadata", AV_OPT_ALLOW_NULL);
    print_get_value_flags(ctx, "nullable_metadata", AV_OPT_ALLOW_NULL | AV_OPT_SEARCH_FAKE_OBJ);
    printf("\n");
}

static void print_get_children_row(const TestOptions *ctx) {
    printf("get:children");
    print_get_value_flags(ctx, "child_only", 0);
    print_get_value_flags(ctx, "child_only", AV_OPT_SEARCH_CHILDREN);
    print_get_value_flags(ctx, "threads", AV_OPT_SEARCH_CHILDREN);
    print_get_value_flags(ctx, "threads", AV_OPT_SEARCH_FAKE_OBJ);
    printf("\n");
}

static void print_query_range_value(const void *ctx, const char *name) {
    AVOptionRanges *ranges = NULL;
    int ret = av_opt_query_ranges(&ranges, (void *)ctx, name, 0);
    if (ret < 0 || !ranges || !ranges->range ||
        ranges->nb_ranges <= 0 || ranges->nb_components <= 0 ||
        !ranges->range[0]) {
        printf("|%d:<null>", ret);
    } else {
        const AVOptionRange *range = ranges->range[0];
        printf("|%d:%d:%d:%.17g:%.17g:%.17g:%.17g:%d",
               ret,
               ranges->nb_ranges,
               ranges->nb_components,
               range->value_min,
               range->value_max,
               range->component_min,
               range->component_max,
               range->is_range);
    }
    av_opt_freep_ranges(&ranges);
}

static void print_query_ranges_row(const TestOptions *ctx) {
    printf("query-ranges:root");
    print_query_range_value(ctx, "threads");
    print_query_range_value(ctx, "bitexact");
    print_query_range_value(ctx, "quality");
    print_query_range_value(ctx, "aspect_ratio");
    print_query_range_value(ctx, "metadata");
    print_query_range_value(ctx, "preset_level");
    print_query_range_value(ctx, "exported");
    print_query_range_value(ctx, "THREADS");
    print_query_range_value(ctx, "fast");
    printf("\n");
}

static void print_set_children_row(TestOptions *ctx) {
    int ret_child_only_root = av_opt_set(ctx, "child_only", "7", 0);
    int ret_child_only_child = av_opt_set(ctx, "child_only", "7", AV_OPT_SEARCH_CHILDREN);
    int ret_threads_child = av_opt_set(ctx, "threads", "9", AV_OPT_SEARCH_CHILDREN);
    int ret_child_readonly = av_opt_set(ctx, "child_readonly", "4", AV_OPT_SEARCH_CHILDREN);
    int ret_threads_fake = av_opt_set(ctx, "threads", "10", AV_OPT_SEARCH_FAKE_OBJ);

    printf("ret:set-children|%d|%d|%d|%d|%d\n",
           ret_child_only_root,
           ret_child_only_child,
           ret_threads_child,
           ret_child_readonly,
           ret_threads_fake);
}

static void print_set_dict_rows(void) {
    TestOptions root_ctx;
    TestOptions child_ctx;
    TestOptions error_ctx;
    AVDictionary *root_dict = NULL;
    AVDictionary *child_dict = NULL;
    AVDictionary *error_dict = NULL;
    int ret;

    init_context(&root_ctx);
    av_dict_set(&root_dict, "threads", "11", AV_DICT_MULTIKEY);
    av_dict_set(&root_dict, "unknown", "first", AV_DICT_MULTIKEY);
    av_dict_set(&root_dict, "bitexact", "true", AV_DICT_MULTIKEY);
    av_dict_set(&root_dict, "unknown", "second", AV_DICT_MULTIKEY);
    av_dict_set(&root_dict, "metadata", "from-dict", AV_DICT_MULTIKEY);
    ret = av_opt_set_dict2(&root_ctx, &root_dict, 0);
    printf("ret:set-dict-root|%d", ret);
    print_dict_entries(root_dict);
    printf("\n");
    print_state("state:set-dict-root", &root_ctx);
    av_dict_free(&root_dict);
    av_opt_free(&root_ctx);
    av_opt_free(&root_ctx.child);

    init_context(&child_ctx);
    av_dict_set(&child_dict, "threads", "9", AV_DICT_MULTIKEY);
    av_dict_set(&child_dict, "child_only", "6", AV_DICT_MULTIKEY);
    av_dict_set(&child_dict, "quality", "0.25", AV_DICT_MULTIKEY);
    av_dict_set(&child_dict, "unknown", "value", AV_DICT_MULTIKEY);
    ret = av_opt_set_dict2(&child_ctx, &child_dict, AV_OPT_SEARCH_CHILDREN);
    printf("ret:set-dict-children|%d", ret);
    print_dict_entries(child_dict);
    printf("\n");
    print_child_dict_state("state:set-dict-children", &child_ctx);
    av_dict_free(&child_dict);
    av_opt_free(&child_ctx);
    av_opt_free(&child_ctx.child);

    init_context(&error_ctx);
    av_dict_set(&error_dict, "threads", "13", AV_DICT_MULTIKEY);
    av_dict_set(&error_dict, "bitexact", "maybe", AV_DICT_MULTIKEY);
    av_dict_set(&error_dict, "unknown", "later", AV_DICT_MULTIKEY);
    ret = av_opt_set_dict2(&error_ctx, &error_dict, 0);
    printf("ret:set-dict-error|%d", ret);
    print_dict_entries(error_dict);
    printf("\n");
    print_state("state:set-dict-error", &error_ctx);
    av_dict_free(&error_dict);
    av_opt_free(&error_ctx);
    av_opt_free(&error_ctx.child);
}

static void print_copy_rows(void) {
    TestOptions source;
    TestOptions destination;
    ChildOptions child_source;
    ChildOptions child_destination;
    ChildOptions mismatch_destination;
    int ret;

    init_context(&source);
    init_context(&destination);

    av_opt_set(&source, "threads", "12", 0);
    av_opt_set(&source, "bitexact", "true", 0);
    av_opt_set(&source, "quality", "0.875", 0);
    av_opt_set(&source, "aspect_ratio", "3/2", 0);
    av_opt_set(&source, "metadata", "source", 0);
    av_opt_set(&source, "preset_level", "slow", 0);
    av_opt_set(&source.child, "threads", "11", 0);
    av_opt_set(&source.child, "child_only", "6", 0);

    av_opt_set(&destination, "threads", "3", 0);
    av_opt_set(&destination, "quality", "0.125", 0);
    av_opt_set(&destination, "aspect_ratio", "4/3", 0);
    av_opt_set(&destination, "metadata", "destination", 0);
    av_opt_set(&destination, "preset_level", "fast", 0);
    av_opt_set(&destination.child, "threads", "14", 0);
    av_opt_set(&destination.child, "child_only", "4", 0);

    ret = av_opt_copy(&destination, &source);
    printf("ret:copy-root|%d\n", ret);
    print_copy_state("state:copy-root-src", &source);
    print_copy_state("state:copy-root-dst", &destination);
    av_opt_set(&source, "metadata", "mutated-source", 0);
    print_copy_state("state:copy-root-dst-after-src-mutate", &destination);

    memset(&child_source, 0, sizeof(child_source));
    memset(&child_destination, 0, sizeof(child_destination));
    child_source.av_class = &child_class;
    child_destination.av_class = &child_class;
    av_opt_set_defaults(&child_source);
    av_opt_set_defaults(&child_destination);
    av_opt_set(&child_source, "threads", "11", 0);
    av_opt_set(&child_source, "child_only", "6", 0);
    av_opt_set(&child_destination, "threads", "14", 0);
    av_opt_set(&child_destination, "child_only", "4", 0);
    ret = av_opt_copy(&child_destination, &child_source);
    printf("ret:copy-child|%d\n", ret);
    print_child_option_state("state:copy-child-dst", &child_destination);

    memset(&mismatch_destination, 0, sizeof(mismatch_destination));
    mismatch_destination.av_class = &child_class;
    av_opt_set_defaults(&mismatch_destination);
    ret = av_opt_copy(&mismatch_destination, &source);
    printf("ret:copy-class-mismatch|%d\n", ret);

    av_opt_free(&source);
    av_opt_free(&source.child);
    av_opt_free(&destination);
    av_opt_free(&destination.child);
}

static void print_typed_get_set_rows(void) {
    TestOptions ctx;
    int ret_threads;
    int ret_bitexact;
    int ret_quality;
    int ret_aspect;
    int ret_preset;
    int ret_metadata_number;
    int ret_threads_range;
    int ret_exported_readonly;
    int ret_child_threads;
    int ret_child_only;
    int ret_threads_fake;

    init_context(&ctx);
    ret_threads = av_opt_set_int(&ctx, "threads", 21, 0);
    ret_bitexact = av_opt_set_int(&ctx, "bitexact", 1, 0);
    ret_quality = av_opt_set_double(&ctx, "quality", 0.625, 0);
    ret_aspect = av_opt_set_q(&ctx, "aspect_ratio", (AVRational){ 3, 2 }, 0);
    ret_preset = av_opt_set_int(&ctx, "preset_level", 6, 0);
    printf("ret:set-typed|%d|%d|%d|%d|%d\n",
           ret_threads,
           ret_bitexact,
           ret_quality,
           ret_aspect,
           ret_preset);
    print_state("state:set-typed", &ctx);
    printf("get:set-typed");
    print_get_int_value(&ctx, "threads", 0);
    print_get_double_value(&ctx, "quality", 0);
    print_get_q_value(&ctx, "aspect_ratio", 0);
    print_get_int_value(&ctx, "bitexact", 0);
    print_get_double_value(&ctx, "threads", 0);
    print_get_q_value(&ctx, "threads", 0);
    print_get_int_value(&ctx, "quality", 0);
    printf("\n");

    ret_metadata_number = av_opt_set_int(&ctx, "metadata", 1, 0);
    ret_threads_range = av_opt_set_int(&ctx, "threads", 128, 0);
    ret_exported_readonly = av_opt_set_int(&ctx, "exported", 1, 0);
    printf("ret:set-typed-errors|%d|%d|%d\n",
           ret_metadata_number,
           ret_threads_range,
           ret_exported_readonly);
    printf("get:typed-errors");
    print_get_int_value(&ctx, "metadata", 0);
    print_get_int_value(&ctx, "missing", 0);
    printf("\n");
    print_state("state:after-typed-errors", &ctx);
    av_opt_free(&ctx);
    av_opt_free(&ctx.child);

    init_context(&ctx);
    ret_child_threads = av_opt_set_int(&ctx, "threads", 9, AV_OPT_SEARCH_CHILDREN);
    ret_child_only = av_opt_set_int(&ctx, "child_only", 7, AV_OPT_SEARCH_CHILDREN);
    ret_threads_fake = av_opt_set_int(&ctx, "threads", 10, AV_OPT_SEARCH_FAKE_OBJ);
    printf("ret:set-typed-children|%d|%d|%d\n",
           ret_child_threads,
           ret_child_only,
           ret_threads_fake);
    printf("get:typed-children");
    print_get_int_value(&ctx, "threads", AV_OPT_SEARCH_CHILDREN);
    print_get_int_value(&ctx, "child_only", AV_OPT_SEARCH_CHILDREN);
    print_get_int_value(&ctx, "threads", AV_OPT_SEARCH_FAKE_OBJ);
    printf("\n");
    print_child_state("state:set-typed-children", &ctx);
    av_opt_free(&ctx);
    av_opt_free(&ctx.child);
}

static void print_duration_state(const char *name, const DurationOptions *ctx) {
    printf("%s|%" PRId64 "\n", name, ctx->timeout);
}

static void print_duration_rows(void) {
    DurationOptions ctx;
    int ret_15;
    int ret_clock;
    int ret_ms;
    int ret_us;
    int ret_bad;
    int ret_range;
    int ret_typed;
    int64_t after_15;
    int64_t after_clock;
    int64_t after_ms;
    int64_t after_us;

    init_duration_context(&ctx);
    ret_15 = av_opt_set(&ctx, "timeout", "1.5", 0);
    after_15 = ctx.timeout;
    ret_clock = av_opt_set(&ctx, "timeout", "00:01:02.250", 0);
    after_clock = ctx.timeout;
    ret_ms = av_opt_set(&ctx, "timeout", "1500ms", 0);
    after_ms = ctx.timeout;
    ret_us = av_opt_set(&ctx, "timeout", "42us", 0);
    after_us = ctx.timeout;
    printf("ret:set-duration-strings|%d|%d|%d|%d\n",
           ret_15, ret_clock, ret_ms, ret_us);
    printf("state:set-duration-strings|%" PRId64 "|%" PRId64 "|%" PRId64 "|%" PRId64 "\n",
           after_15, after_clock, after_ms, after_us);
    printf("get:set-duration-strings");
    print_get_value(&ctx, "timeout");
    printf("\n");

    ret_bad = av_opt_set(&ctx, "timeout", "bad", 0);
    ret_range = av_opt_set(&ctx, "timeout", "-1", 0);
    printf("ret:set-duration-errors|%d|%d\n", ret_bad, ret_range);
    print_duration_state("state:after-duration-errors", &ctx);

    init_duration_context(&ctx);
    ret_typed = av_opt_set_int(&ctx, "timeout", 90500000, 0);
    printf("ret:set-duration-typed|%d\n", ret_typed);
    print_duration_state("state:set-duration-typed", &ctx);
    printf("get:set-duration-typed");
    print_get_int_value(&ctx, "timeout", 0);
    print_get_double_value(&ctx, "timeout", 0);
    print_get_q_value(&ctx, "timeout", 0);
    print_get_value(&ctx, "timeout");
    printf("\n");

    printf("query-ranges:duration");
    print_query_range_value(&ctx, "timeout");
    print_query_range_value(&ctx, "missing");
    printf("\n");
}

static void print_image_size_state(const char *name, const ImageSizeOptions *ctx) {
    printf("%s|%d|%d|%" PRId64 "\n", name, ctx->size[0], ctx->size[1], ctx->scalar);
}

static void print_image_size_rows(void) {
    ImageSizeOptions ctx;
    int ret_640;
    int ret_hd720;
    int ret_none;
    int ret_bad;
    int ret_zero;
    int ret_typed;
    int ret_negative;
    int ret_wrong_type;
    int ret_numeric;
    int after_640[2];
    int after_hd720[2];
    int after_none[2];

    init_image_size_context(&ctx);
    print_image_size_state("state:image-size-defaults", &ctx);
    printf("get:image-size-defaults");
    print_get_image_size_value(&ctx, "size", 0);
    print_get_value(&ctx, "size");
    printf("\n");

    ret_640 = av_opt_set(&ctx, "size", "640x480", 0);
    after_640[0] = ctx.size[0];
    after_640[1] = ctx.size[1];
    ret_hd720 = av_opt_set(&ctx, "size", "hd720", 0);
    after_hd720[0] = ctx.size[0];
    after_hd720[1] = ctx.size[1];
    ret_none = av_opt_set(&ctx, "size", "none", 0);
    after_none[0] = ctx.size[0];
    after_none[1] = ctx.size[1];
    printf("ret:set-image-size-strings|%d|%d|%d\n", ret_640, ret_hd720, ret_none);
    printf("state:set-image-size-strings|%d|%d|%d|%d|%d|%d\n",
           after_640[0], after_640[1],
           after_hd720[0], after_hd720[1],
           after_none[0], after_none[1]);
    printf("get:set-image-size-strings");
    print_get_image_size_value(&ctx, "size", 0);
    print_get_value(&ctx, "size");
    printf("\n");

    ret_bad = av_opt_set(&ctx, "size", "bad", 0);
    ret_zero = av_opt_set(&ctx, "size", "0x480", 0);
    printf("ret:set-image-size-errors|%d|%d\n", ret_bad, ret_zero);
    print_image_size_state("state:after-image-size-errors", &ctx);

    init_image_size_context(&ctx);
    ret_typed = av_opt_set_image_size(&ctx, "size", 800, 600, 0);
    ret_negative = av_opt_set_image_size(&ctx, "size", -1, 480, 0);
    ret_wrong_type = av_opt_set_image_size(&ctx, "scalar", 1, 1, 0);
    ret_numeric = av_opt_set_int(&ctx, "size", 10, 0);
    printf("ret:set-image-size-typed|%d|%d|%d|%d\n",
           ret_typed, ret_negative, ret_wrong_type, ret_numeric);
    print_image_size_state("state:set-image-size-typed", &ctx);
    printf("get:set-image-size-typed");
    print_get_image_size_value(&ctx, "size", 0);
    print_get_value(&ctx, "size");
    print_get_image_size_value(&ctx, "scalar", 0);
    print_get_int_value(&ctx, "size", 0);
    printf("\n");

    printf("query-ranges:image-size");
    print_query_range_value(&ctx, "size");
    print_query_range_value(&ctx, "missing");
    printf("\n");
}

static void print_image_size_child_rows(void) {
    ImageSizeParentOptions ctx;
    int ret_no_flags;
    int ret_child;
    int ret_fake;

    init_image_size_parent_context(&ctx);
    printf("get:image-size-children");
    print_get_image_size_value(&ctx, "child_size", 0);
    print_get_image_size_value(&ctx, "child_size", AV_OPT_SEARCH_CHILDREN);
    print_get_value_flags(&ctx, "child_size", AV_OPT_SEARCH_CHILDREN);
    printf("\n");

    ret_no_flags = av_opt_set_image_size(&ctx, "child_size", 800, 600, 0);
    ret_child = av_opt_set_image_size(&ctx, "child_size", 800, 600,
                                      AV_OPT_SEARCH_CHILDREN);
    ret_fake = av_opt_set_image_size(&ctx, "child_size", 1024, 768,
                                     AV_OPT_SEARCH_FAKE_OBJ);
    printf("ret:set-image-size-children|%d|%d|%d\n",
           ret_no_flags, ret_child, ret_fake);
    printf("state:image-size-children|%d|%d\n",
           ctx.child.child_size[0], ctx.child.child_size[1]);
    printf("get:set-image-size-children");
    print_get_image_size_value(&ctx, "child_size", AV_OPT_SEARCH_CHILDREN);
    print_get_value_flags(&ctx, "child_size", AV_OPT_SEARCH_CHILDREN);
    printf("\n");
}

static void print_pixel_format_state(const char *name, const PixelFormatOptions *ctx) {
    printf("%s|%d|%" PRId64 "\n", name, ctx->pix_fmt, ctx->scalar);
}

static void print_pixel_format_rows(void) {
    PixelFormatOptions ctx;
    int ret_rgb24;
    int ret_gray;
    int ret_none;
    int ret_numeric;
    int ret_high_name;
    int ret_high_numeric;
    int ret_hardware_name;
    int ret_hardware_numeric;
    int ret_hardware_last;
    int ret_bad;
    int ret_out_of_range;
    int ret_negative_numeric;
    int ret_typed;
    int ret_typed_none;
    int ret_wrong_type;
    int ret_int;
    int ret_int_range;
    int ret_scalar;
    int ret_hardware_typed;
    int ret_hardware_int;
    enum AVPixelFormat after_rgb24;
    enum AVPixelFormat after_gray;
    enum AVPixelFormat after_none;
    enum AVPixelFormat after_numeric;
    enum AVPixelFormat after_high_name;
    enum AVPixelFormat after_high_numeric;
    enum AVPixelFormat after_hardware_name;
    enum AVPixelFormat after_hardware_numeric;
    enum AVPixelFormat after_hardware_last;

    init_pixel_format_context(&ctx);
    print_pixel_format_state("state:pixel-format-defaults", &ctx);
    printf("get:pixel-format-defaults");
    print_get_pixel_format_value(&ctx, "pix_fmt", 0);
    print_get_value(&ctx, "pix_fmt");
    print_get_int_value(&ctx, "pix_fmt", 0);
    printf("\n");

    ret_rgb24 = av_opt_set(&ctx, "pix_fmt", "rgb24", 0);
    after_rgb24 = ctx.pix_fmt;
    ret_gray = av_opt_set(&ctx, "pix_fmt", "gray", 0);
    after_gray = ctx.pix_fmt;
    ret_none = av_opt_set(&ctx, "pix_fmt", "none", 0);
    after_none = ctx.pix_fmt;
    ret_numeric = av_opt_set(&ctx, "pix_fmt", "0x3", 0);
    after_numeric = ctx.pix_fmt;
    ret_high_name = av_opt_set(&ctx, "pix_fmt", "gbrap32le", 0);
    after_high_name = ctx.pix_fmt;
    ret_high_numeric = av_opt_set(&ctx, "pix_fmt", "259", 0);
    after_high_numeric = ctx.pix_fmt;
    ret_hardware_name = av_opt_set(&ctx, "pix_fmt", "vaapi", 0);
    after_hardware_name = ctx.pix_fmt;
    ret_hardware_numeric = av_opt_set(&ctx, "pix_fmt", "227", 0);
    after_hardware_numeric = ctx.pix_fmt;
    ret_hardware_last = av_opt_set(&ctx, "pix_fmt", "ohcodec", 0);
    after_hardware_last = ctx.pix_fmt;
    printf("ret:set-pixel-format-strings|%d|%d|%d|%d|%d|%d|%d|%d|%d\n",
           ret_rgb24, ret_gray, ret_none, ret_numeric, ret_high_name, ret_high_numeric,
           ret_hardware_name, ret_hardware_numeric, ret_hardware_last);
    printf("state:set-pixel-format-strings|%d|%d|%d|%d|%d|%d|%d|%d|%d\n",
           after_rgb24, after_gray, after_none, after_numeric, after_high_name,
           after_high_numeric, after_hardware_name, after_hardware_numeric, after_hardware_last);
    printf("get:set-pixel-format-strings");
    print_get_value(&ctx, "pix_fmt");
    printf("\n");

    ret_bad = av_opt_set(&ctx, "pix_fmt", "bad", 0);
    ret_out_of_range = av_opt_set(&ctx, "pix_fmt", "267", 0);
    ret_negative_numeric = av_opt_set(&ctx, "pix_fmt", "-1", 0);
    printf("ret:set-pixel-format-errors|%d|%d|%d\n",
           ret_bad, ret_out_of_range, ret_negative_numeric);
    print_pixel_format_state("state:after-pixel-format-errors", &ctx);

    init_pixel_format_context(&ctx);
    ret_typed = av_opt_set_pixel_fmt(&ctx, "pix_fmt", AV_PIX_FMT_RGB24, 0);
    ret_typed_none = av_opt_set_pixel_fmt(&ctx, "pix_fmt", AV_PIX_FMT_NONE, 0);
    ret_wrong_type = av_opt_set_pixel_fmt(&ctx, "scalar", AV_PIX_FMT_RGB24, 0);
    ret_int = av_opt_set_int(&ctx, "pix_fmt", AV_PIX_FMT_BGR24, 0);
    ret_int_range = av_opt_set_int(&ctx, "pix_fmt", AV_PIX_FMT_NB, 0);
    ret_scalar = av_opt_set_int(&ctx, "scalar", 6, 0);
    ret_hardware_typed = av_opt_set_pixel_fmt(&ctx, "pix_fmt", AV_PIX_FMT_VAAPI, 0);
    ret_hardware_int = av_opt_set_int(&ctx, "pix_fmt", AV_PIX_FMT_OHCODEC, 0);
    printf("ret:set-pixel-format-typed|%d|%d|%d|%d|%d|%d|%d|%d\n",
           ret_typed, ret_typed_none, ret_wrong_type, ret_int, ret_int_range, ret_scalar,
           ret_hardware_typed, ret_hardware_int);
    print_pixel_format_state("state:set-pixel-format-typed", &ctx);
    printf("get:set-pixel-format-typed");
    print_get_pixel_format_value(&ctx, "pix_fmt", 0);
    print_get_int_value(&ctx, "pix_fmt", 0);
    print_get_double_value(&ctx, "pix_fmt", 0);
    print_get_q_value(&ctx, "pix_fmt", 0);
    print_get_value(&ctx, "pix_fmt");
    print_get_pixel_format_value(&ctx, "scalar", 0);
    printf("\n");

    printf("query-ranges:pixel-format");
    print_query_range_value(&ctx, "pix_fmt");
    print_query_range_value(&ctx, "missing");
    printf("\n");
}

static void print_sample_format_state(const char *name, const SampleFormatOptions *ctx) {
    printf("%s|%d|%" PRId64 "\n", name, ctx->sample_fmt, ctx->scalar);
}

static void print_sample_format_rows(void) {
    SampleFormatOptions ctx;
    int ret_fltp;
    int ret_none;
    int ret_numeric;
    int ret_bad;
    int ret_out_of_range;
    int ret_negative_numeric;
    int ret_typed;
    int ret_typed_none;
    int ret_wrong_type;
    int ret_int;
    int ret_int_range;
    int ret_scalar;
    enum AVSampleFormat after_fltp;
    enum AVSampleFormat after_none;
    enum AVSampleFormat after_numeric;

    init_sample_format_context(&ctx);
    print_sample_format_state("state:sample-format-defaults", &ctx);
    printf("get:sample-format-defaults");
    print_get_sample_format_value(&ctx, "sample_fmt", 0);
    print_get_value(&ctx, "sample_fmt");
    print_get_int_value(&ctx, "sample_fmt", 0);
    printf("\n");

    ret_fltp = av_opt_set(&ctx, "sample_fmt", "fltp", 0);
    after_fltp = ctx.sample_fmt;
    ret_none = av_opt_set(&ctx, "sample_fmt", "none", 0);
    after_none = ctx.sample_fmt;
    ret_numeric = av_opt_set(&ctx, "sample_fmt", "0x4", 0);
    after_numeric = ctx.sample_fmt;
    printf("ret:set-sample-format-strings|%d|%d|%d\n",
           ret_fltp, ret_none, ret_numeric);
    printf("state:set-sample-format-strings|%d|%d|%d\n",
           after_fltp, after_none, after_numeric);
    printf("get:set-sample-format-strings");
    print_get_value(&ctx, "sample_fmt");
    printf("\n");

    ret_bad = av_opt_set(&ctx, "sample_fmt", "bad", 0);
    ret_out_of_range = av_opt_set(&ctx, "sample_fmt", "12", 0);
    ret_negative_numeric = av_opt_set(&ctx, "sample_fmt", "-1", 0);
    printf("ret:set-sample-format-errors|%d|%d|%d\n",
           ret_bad, ret_out_of_range, ret_negative_numeric);
    print_sample_format_state("state:after-sample-format-errors", &ctx);

    init_sample_format_context(&ctx);
    ret_typed = av_opt_set_sample_fmt(&ctx, "sample_fmt", AV_SAMPLE_FMT_S32P, 0);
    ret_typed_none = av_opt_set_sample_fmt(&ctx, "sample_fmt", AV_SAMPLE_FMT_NONE, 0);
    ret_wrong_type = av_opt_set_sample_fmt(&ctx, "scalar", AV_SAMPLE_FMT_S16, 0);
    ret_int = av_opt_set_int(&ctx, "sample_fmt", AV_SAMPLE_FMT_S64, 0);
    ret_int_range = av_opt_set_int(&ctx, "sample_fmt", AV_SAMPLE_FMT_NB, 0);
    ret_scalar = av_opt_set_int(&ctx, "scalar", 6, 0);
    printf("ret:set-sample-format-typed|%d|%d|%d|%d|%d|%d\n",
           ret_typed, ret_typed_none, ret_wrong_type, ret_int, ret_int_range, ret_scalar);
    print_sample_format_state("state:set-sample-format-typed", &ctx);
    printf("get:set-sample-format-typed");
    print_get_sample_format_value(&ctx, "sample_fmt", 0);
    print_get_int_value(&ctx, "sample_fmt", 0);
    print_get_double_value(&ctx, "sample_fmt", 0);
    print_get_q_value(&ctx, "sample_fmt", 0);
    print_get_value(&ctx, "sample_fmt");
    print_get_sample_format_value(&ctx, "scalar", 0);
    printf("\n");

    printf("query-ranges:sample-format");
    print_query_range_value(&ctx, "sample_fmt");
    print_query_range_value(&ctx, "missing");
    printf("\n");
}

static void print_channel_layout_state(const char *name, const ChannelLayoutOptions *ctx) {
    char desc[256] = { 0 };
    describe_channel_layout(&ctx->layout, desc, sizeof(desc));
    printf("%s|%s|%" PRId64 "\n", name, desc, ctx->scalar);
}

static void print_channel_layout_rows(void) {
    ChannelLayoutOptions ctx;
    int ret_mono;
    int ret_five_one;
    int ret_unspecified;
    int ret_bad;
    int ret_zero;
    int ret_typed;
    int ret_wrong_type;
    int ret_int_range;
    int ret_int_zero;
    int ret_scalar;
    AVChannelLayout mono = AV_CHANNEL_LAYOUT_MONO;
    AVChannelLayout stereo = AV_CHANNEL_LAYOUT_STEREO;
    char after_mono[256] = { 0 };
    char after_five_one[256] = { 0 };
    char after_unspecified[256] = { 0 };

    init_channel_layout_context(&ctx);
    print_channel_layout_state("state:channel-layout-defaults", &ctx);
    printf("get:channel-layout-defaults");
    print_get_channel_layout_value(&ctx, "layout", 0);
    print_get_value(&ctx, "layout");
    print_get_int_value(&ctx, "layout", 0);
    printf("\n");

    ret_mono = av_opt_set(&ctx, "layout", "mono", 0);
    describe_channel_layout(&ctx.layout, after_mono, sizeof(after_mono));
    ret_five_one = av_opt_set(&ctx, "layout", "5.1", 0);
    describe_channel_layout(&ctx.layout, after_five_one, sizeof(after_five_one));
    ret_unspecified = av_opt_set(&ctx, "layout", "2C", 0);
    describe_channel_layout(&ctx.layout, after_unspecified, sizeof(after_unspecified));
    printf("ret:set-channel-layout-strings|%d|%d|%d\n",
           ret_mono, ret_five_one, ret_unspecified);
    printf("state:set-channel-layout-strings|%s|%s|%s\n",
           after_mono, after_five_one, after_unspecified);
    printf("get:set-channel-layout-strings");
    print_get_value(&ctx, "layout");
    printf("\n");

    ret_bad = av_opt_set(&ctx, "layout", "bad", 0);
    ret_zero = av_opt_set(&ctx, "layout", "0", 0);
    printf("ret:set-channel-layout-errors|%d|%d\n", ret_bad, ret_zero);
    print_channel_layout_state("state:after-channel-layout-errors", &ctx);

    av_channel_layout_uninit(&ctx.layout);
    init_channel_layout_context(&ctx);
    ret_typed = av_opt_set_chlayout(&ctx, "layout", &mono, 0);
    ret_wrong_type = av_opt_set_chlayout(&ctx, "scalar", &stereo, 0);
    ret_int_range = av_opt_set_int(&ctx, "layout", 2, 0);
    ret_int_zero = av_opt_set_int(&ctx, "layout", 0, 0);
    ret_scalar = av_opt_set_int(&ctx, "scalar", 6, 0);
    printf("ret:set-channel-layout-typed|%d|%d|%d|%d|%d\n",
           ret_typed, ret_wrong_type, ret_int_range, ret_int_zero, ret_scalar);
    print_channel_layout_state("state:set-channel-layout-typed", &ctx);
    printf("get:set-channel-layout-typed");
    print_get_channel_layout_value(&ctx, "layout", 0);
    print_get_int_value(&ctx, "layout", 0);
    print_get_double_value(&ctx, "layout", 0);
    print_get_q_value(&ctx, "layout", 0);
    print_get_value(&ctx, "layout");
    print_get_channel_layout_value(&ctx, "scalar", 0);
    printf("\n");

    printf("query-ranges:channel-layout");
    print_query_range_value(&ctx, "layout");
    print_query_range_value(&ctx, "missing");
    printf("\n");
    av_channel_layout_uninit(&ctx.layout);
}

static void print_binary_hex(const uint8_t *data, int len) {
    for (int i = 0; i < len; i++)
        printf("%02X", data[i]);
}

static void print_binary_state(const char *name, const BinaryOptions *ctx) {
    printf("%s|%d|", name, ctx->blob_size);
    print_binary_hex(ctx->blob, ctx->blob_size);
    printf("|%" PRId64 "\n", ctx->scalar);
}

static void print_binary_rows(void) {
    BinaryOptions ctx;
    int ret_hex;
    int ret_empty;
    int ret_dead;
    int ret_odd;
    int ret_non_hex;
    int ret_typed;
    int ret_typed_empty;
    int ret_wrong_type;
    int ret_int_range;
    int ret_int_zero;
    int ret_scalar;
    uint8_t typed[] = { 0xDE, 0xAD };
    int after_hex_len;
    int after_empty_len;
    int after_dead_len;
    int after_typed_len;
    int after_typed_empty_len;
    uint8_t after_hex[8] = { 0 };
    uint8_t after_dead[8] = { 0 };
    uint8_t after_typed[8] = { 0 };

    init_binary_context(&ctx);
    print_binary_state("state:binary-defaults", &ctx);
    printf("get:binary-defaults");
    print_get_value(&ctx, "blob");
    print_get_int_value(&ctx, "blob", 0);
    printf("\n");
    printf("get:binary-allow-null");
    print_get_value(&ctx, "nullable_blob");
    print_get_value_flags(&ctx, "nullable_blob", AV_OPT_ALLOW_NULL);
    print_get_value_flags(&ctx, "blob", AV_OPT_ALLOW_NULL);
    print_get_value_flags(&ctx, "nullable_blob", AV_OPT_ALLOW_NULL | AV_OPT_SEARCH_FAKE_OBJ);
    printf("\n");

    ret_hex = av_opt_set(&ctx, "blob", "0f10Aa", 0);
    after_hex_len = ctx.blob_size;
    memcpy(after_hex, ctx.blob, ctx.blob_size);
    ret_empty = av_opt_set(&ctx, "blob", "", 0);
    after_empty_len = ctx.blob_size;
    ret_dead = av_opt_set(&ctx, "blob", "deAd", 0);
    after_dead_len = ctx.blob_size;
    memcpy(after_dead, ctx.blob, ctx.blob_size);
    printf("ret:set-binary-strings|%d|%d|%d\n", ret_hex, ret_empty, ret_dead);
    printf("state:set-binary-strings|%d|", after_hex_len);
    print_binary_hex(after_hex, after_hex_len);
    printf("|%d||%d|", after_empty_len, after_dead_len);
    print_binary_hex(after_dead, after_dead_len);
    printf("\n");
    printf("get:set-binary-strings");
    print_get_value(&ctx, "blob");
    printf("\n");

    ret_odd = av_opt_set(&ctx, "blob", "abc", 0);
    av_opt_set(&ctx, "blob", "beef", 0);
    ret_non_hex = av_opt_set(&ctx, "blob", "0g", 0);
    printf("ret:set-binary-errors|%d|%d\n", ret_odd, ret_non_hex);
    print_binary_state("state:after-binary-errors", &ctx);

    av_opt_free(&ctx);
    init_binary_context(&ctx);
    ret_typed = av_opt_set_bin(&ctx, "blob", typed, 2, 0);
    after_typed_len = ctx.blob_size;
    memcpy(after_typed, ctx.blob, ctx.blob_size);
    ret_typed_empty = av_opt_set_bin(&ctx, "blob", typed, 0, 0);
    after_typed_empty_len = ctx.blob_size;
    ret_wrong_type = av_opt_set_bin(&ctx, "scalar", typed, 1, 0);
    ret_int_range = av_opt_set_int(&ctx, "blob", 2, 0);
    ret_int_zero = av_opt_set_int(&ctx, "blob", 0, 0);
    ret_scalar = av_opt_set_int(&ctx, "scalar", 6, 0);
    printf("ret:set-binary-typed|%d|%d|%d|%d|%d|%d\n",
           ret_typed, ret_typed_empty, ret_wrong_type, ret_int_range, ret_int_zero, ret_scalar);
    printf("state:set-binary-typed|%d|", after_typed_len);
    print_binary_hex(after_typed, after_typed_len);
    printf("|%d||%d|", after_typed_empty_len, ctx.blob_size);
    print_binary_hex(ctx.blob, ctx.blob_size);
    printf("|%" PRId64 "\n", ctx.scalar);
    printf("get:set-binary-typed");
    print_get_value(&ctx, "blob");
    print_get_int_value(&ctx, "blob", 0);
    print_get_q_value(&ctx, "blob", 0);
    print_get_value(&ctx, "scalar");
    print_get_value(&ctx, "missing");
    printf("\n");

    printf("query-ranges:binary");
    print_query_range_value(&ctx, "blob");
    print_query_range_value(&ctx, "missing");
    printf("\n");
    av_opt_free(&ctx);
}

static void print_dictionary_state(const char *name, const DictionaryOptions *ctx) {
    printf("%s", name);
    print_dict_entries(ctx->dict);
    printf("|%d|%" PRId64 "\n", av_dict_count(ctx->empty), ctx->scalar);
}

static void print_dictionary_sequence(const AVDictionary *dict) {
    print_dict_entries(dict);
}

static void print_dictionary_rows(void) {
    DictionaryOptions ctx;
    AVDictionary *typed = NULL;
    AVDictionary *empty = NULL;
    AVDictionary *after_escaped = NULL;
    AVDictionary *after_empty = NULL;
    AVDictionary *after_quoted = NULL;
    AVDictionary *after_duplicate = NULL;
    AVDictionary *before_errors = NULL;
    AVDictionary *after_typed = NULL;
    AVDictionary *after_typed_empty = NULL;
    int ret_escaped;
    int ret_empty;
    int ret_quoted;
    int ret_duplicate;
    int ret_missing_separator;
    int ret_empty_value;
    int ret_typed;
    int ret_typed_empty;
    int ret_wrong_type;
    int ret_int_range;
    int ret_int_zero;
    int ret_scalar;

    init_dictionary_context(&ctx);
    print_dictionary_state("state:dictionary-defaults", &ctx);
    printf("get:dictionary-defaults");
    print_get_value(&ctx, "dict");
    print_get_dict_value(&ctx, "dict", 0);
    print_get_value(&ctx, "empty");
    print_get_dict_value(&ctx, "empty", 0);
    print_get_int_value(&ctx, "dict", 0);
    printf("\n");
    printf("get:dictionary-allow-null");
    print_get_value(&ctx, "empty");
    print_get_value_flags(&ctx, "empty", AV_OPT_ALLOW_NULL);
    print_get_value_flags(&ctx, "dict", AV_OPT_ALLOW_NULL);
    print_get_value_flags(&ctx, "empty", AV_OPT_ALLOW_NULL | AV_OPT_SEARCH_FAKE_OBJ);
    printf("\n");

    ret_escaped = av_opt_set(&ctx, "dict", "artist=rust:comment=hello\\:there", 0);
    av_dict_copy(&after_escaped, ctx.dict, 0);
    ret_empty = av_opt_set(&ctx, "dict", "", 0);
    av_dict_copy(&after_empty, ctx.dict, 0);
    ret_quoted = av_opt_set(&ctx, "dict", "quoted='a:b':space=trim ", 0);
    av_dict_copy(&after_quoted, ctx.dict, 0);
    printf("ret:set-dictionary-strings|%d|%d|%d\n", ret_escaped, ret_empty, ret_quoted);
    printf("state:set-dictionary-strings");
    print_dictionary_sequence(after_escaped);
    print_dictionary_sequence(after_empty);
    print_dictionary_sequence(after_quoted);
    printf("\n");
    printf("get:set-dictionary-strings");
    print_get_value(&ctx, "dict");
    printf("\n");

    ret_duplicate = av_opt_set(&ctx, "dict", "artist=rust:ARTIST=override", 0);
    av_dict_copy(&after_duplicate, ctx.dict, 0);
    printf("ret:set-dictionary-duplicate|%d\n", ret_duplicate);
    printf("state:set-dictionary-duplicate");
    print_dictionary_sequence(after_duplicate);
    printf("\n");
    printf("get:set-dictionary-duplicate");
    print_get_value(&ctx, "dict");
    printf("\n");

    av_dict_copy(&before_errors, ctx.dict, 0);
    ret_missing_separator = av_opt_set(&ctx, "dict", "missing", 0);
    ret_empty_value = av_opt_set(&ctx, "dict", "key=", 0);
    printf("ret:set-dictionary-errors|%d|%d\n", ret_missing_separator, ret_empty_value);
    printf("state:after-dictionary-errors");
    print_dictionary_sequence(before_errors);
    print_dictionary_sequence(ctx.dict);
    printf("\n");

    av_opt_free(&ctx);
    init_dictionary_context(&ctx);
    av_dict_set(&typed, "typed", "one", 0);
    av_dict_set(&typed, "note", "two:three", 0);
    ret_typed = av_opt_set_dict_val(&ctx, "dict", typed, 0);
    av_dict_copy(&after_typed, ctx.dict, 0);
    ret_typed_empty = av_opt_set_dict_val(&ctx, "dict", empty, 0);
    av_dict_copy(&after_typed_empty, ctx.dict, 0);
    ret_wrong_type = av_opt_set_dict_val(&ctx, "scalar", typed, 0);
    ret_int_range = av_opt_set_int(&ctx, "dict", 2, 0);
    ret_int_zero = av_opt_set_int(&ctx, "dict", 0, 0);
    ret_scalar = av_opt_set_int(&ctx, "scalar", 6, 0);
    printf("ret:set-dictionary-typed|%d|%d|%d|%d|%d|%d\n",
           ret_typed, ret_typed_empty, ret_wrong_type, ret_int_range, ret_int_zero, ret_scalar);
    printf("state:set-dictionary-typed");
    print_dictionary_sequence(after_typed);
    print_dictionary_sequence(after_typed_empty);
    print_dictionary_sequence(ctx.dict);
    printf("|%" PRId64 "\n", ctx.scalar);
    printf("get:set-dictionary-typed");
    print_get_value(&ctx, "dict");
    print_get_dict_value(&ctx, "dict", 0);
    print_get_int_value(&ctx, "dict", 0);
    print_get_q_value(&ctx, "dict", 0);
    print_get_value(&ctx, "scalar");
    print_get_dict_value(&ctx, "scalar", 0);
    print_get_value(&ctx, "missing");
    printf("\n");

    printf("query-ranges:dictionary");
    print_query_range_value(&ctx, "dict");
    print_query_range_value(&ctx, "missing");
    printf("\n");

    av_dict_free(&typed);
    av_dict_free(&after_escaped);
    av_dict_free(&after_empty);
    av_dict_free(&after_quoted);
    av_dict_free(&after_duplicate);
    av_dict_free(&before_errors);
    av_dict_free(&after_typed);
    av_dict_free(&after_typed_empty);
    av_opt_free(&ctx);
}

static void print_int64_array_values(const int64_t *values, unsigned count) {
    printf("|%u", count);
    for (unsigned i = 0; i < count; i++)
        printf("|%" PRId64, values[i]);
}

static void print_string_array_values(char *const *values, unsigned count) {
    printf("|%u", count);
    for (unsigned i = 0; i < count; i++)
        printf("|%s", values[i] ? values[i] : "<null>");
}

static void print_array_state(const char *name, const ArrayOptions *ctx) {
    printf("%s", name);
    print_int64_array_values(ctx->ints, ctx->ints_count);
    print_string_array_values(ctx->words, ctx->words_count);
    print_int64_array_values(ctx->required, ctx->required_count);
    printf("|%" PRId64 "\n", ctx->scalar);
}

static void print_array_required_state(const char *name, const ArrayOptions *ctx) {
    printf("%s", name);
    print_int64_array_values(ctx->required, ctx->required_count);
    printf("\n");
}

static void print_get_array_size_value(const void *ctx, const char *name, int search_flags) {
    unsigned size = 0;
    int ret = av_opt_get_array_size((void *)ctx, name, search_flags, &size);
    printf("|%d:%u", ret, size);
}

static void print_get_array_int64_value(const void *ctx, const char *name,
                                        unsigned start, unsigned count) {
    int64_t values[8] = { 0 };
    int ret = av_opt_get_array((void *)ctx, name, 0, start, count,
                               AV_OPT_TYPE_INT64, values);
    printf("|%d:%u", ret, ret < 0 ? 0 : count);
    if (ret >= 0) {
        for (unsigned i = 0; i < count; i++)
            printf(":%" PRId64, values[i]);
    }
}

static void print_get_array_string_value(const void *ctx, const char *name,
                                         unsigned start, unsigned count) {
    char *values[8] = { 0 };
    int ret = av_opt_get_array((void *)ctx, name, 0, start, count,
                               AV_OPT_TYPE_STRING, values);
    printf("|%d:%u", ret, ret < 0 ? 0 : count);
    if (ret >= 0) {
        for (unsigned i = 0; i < count; i++)
            printf(":%s", values[i] ? values[i] : "<null>");
        for (unsigned i = 0; i < count; i++)
            av_freep(&values[i]);
    }
}

static void print_get_array_double_value(const void *ctx, const char *name,
                                         unsigned start, unsigned count) {
    double values[8] = { 0 };
    int ret = av_opt_get_array((void *)ctx, name, 0, start, count,
                               AV_OPT_TYPE_DOUBLE, values);
    printf("|%d:%u", ret, ret < 0 ? 0 : count);
    if (ret >= 0) {
        for (unsigned i = 0; i < count; i++)
            printf(":%.17g", values[i]);
    }
}

static void print_get_array_q_value(const void *ctx, const char *name,
                                    unsigned start, unsigned count) {
    AVRational values[8] = { 0 };
    int ret = av_opt_get_array((void *)ctx, name, 0, start, count,
                               AV_OPT_TYPE_RATIONAL, values);
    printf("|%d:%u", ret, ret < 0 ? 0 : count);
    if (ret >= 0) {
        for (unsigned i = 0; i < count; i++)
            printf(":%d/%d", values[i].num, values[i].den);
    }
}

static void print_array_rows(void) {
    ArrayOptions ctx;
    int ret_set_ints;
    int ret_set_words;
    int ret_int_range;
    int ret_words_max;
    int ret_required_min;
    int ret_insert;
    int ret_replace;
    int ret_remove;
    int ret_wrong_type;
    int ret_remove_range;
    int ret_string_insert;
    int ret_string_replace;
    int ret_string_remove;
    int ret_int_string_insert;
    int ret_int_string_replace;
    int ret_int_string_remove;
    int ret_int_string_bad;
    int ret_int_double_insert;
    int ret_int_q_replace;
    int ret_int_numeric_remove;
    int ret_int_q_bad;
    int ret_zero_insert;
    int ret_zero_replace;
    int ret_zero_remove;
    int64_t insert_value[] = { 8 };
    int64_t replace_value[] = { 5 };
    const char *bad_value[] = { "bad" };
    const char *string_insert[] = { "middle,comma" };
    const char *string_replace[] = { "tail\\slash" };
    const char *int_string_insert[] = { "6" };
    const char *int_string_replace[] = { "9" };
    const char *int_string_bad[] = { "bad" };
    double int_double_insert[] = { 6.0 };
    AVRational int_q_replace[] = { { 9, 1 } };
    AVRational int_q_bad[] = { { 11, 1 } };

    init_array_context(&ctx);
    print_array_state("state:array-defaults", &ctx);
    printf("get:array-defaults");
    print_get_value(&ctx, "ints");
    print_get_value(&ctx, "words");
    print_get_array_size_value(&ctx, "ints", 0);
    print_get_array_int64_value(&ctx, "ints", 0, 2);
    print_get_int_value(&ctx, "ints", 0);
    printf("\n");

    ret_set_ints = av_opt_set(&ctx, "ints", "3,4", 0);
    ret_set_words = av_opt_set(&ctx, "words", "left,right\\,inner,slash\\\\tail", 0);
    printf("ret:set-array-strings|%d|%d\n", ret_set_ints, ret_set_words);
    print_array_state("state:set-array-strings", &ctx);
    printf("get:set-array-strings");
    print_get_value(&ctx, "ints");
    print_get_value(&ctx, "words");
    printf("\n");

    ret_int_range = av_opt_set(&ctx, "ints", "7,11", 0);
    ret_words_max = av_opt_set(&ctx, "words", "a,b,c,d", 0);
    printf("ret:set-array-errors|%d|%d\n", ret_int_range, ret_words_max);
    print_array_state("state:after-array-errors", &ctx);
    av_opt_free(&ctx);

    init_array_context(&ctx);
    ret_required_min = av_opt_set(&ctx, "required", "9", 0);
    printf("ret:set-array-required-min|%d\n", ret_required_min);
    print_array_required_state("state:after-array-required-min", &ctx);
    av_opt_free(&ctx);

    init_array_context(&ctx);
    av_opt_set(&ctx, "ints", "3,4", 0);
    ret_insert = av_opt_set_array(&ctx, "ints", 0, 1, 1,
                                  AV_OPT_TYPE_INT64, insert_value);
    ret_replace = av_opt_set_array(&ctx, "ints", AV_OPT_ARRAY_REPLACE, 1, 1,
                                   AV_OPT_TYPE_INT64, replace_value);
    ret_remove = av_opt_set_array(&ctx, "ints", 0, 0, 1,
                                  AV_OPT_TYPE_INT64, NULL);
    ret_wrong_type = av_opt_set_array(&ctx, "ints", 0, 0, 1,
                                      AV_OPT_TYPE_STRING, bad_value);
    ret_remove_range = av_opt_set_array(&ctx, "ints", 0, 0, 3,
                                        AV_OPT_TYPE_INT64, NULL);
    printf("ret:set-array-typed|%d|%d|%d|%d|%d",
           ret_insert, ret_replace, ret_remove, ret_wrong_type, ret_remove_range);
    print_get_array_size_value(&ctx, "scalar", 0);
    print_get_array_int64_value(&ctx, "ints", 2, 1);
    printf("\n");
    print_array_state("state:set-array-typed", &ctx);
    printf("get:set-array-typed");
    print_get_array_size_value(&ctx, "ints", 0);
    print_get_array_int64_value(&ctx, "ints", 0, 2);
    print_get_value(&ctx, "ints");
    printf("\n");
    av_opt_free(&ctx);

    init_array_context(&ctx);
    av_opt_set(&ctx, "ints", "3,4", 0);
    ret_int_string_insert = av_opt_set_array(&ctx, "ints", 0, 1, 1,
                                             AV_OPT_TYPE_STRING, int_string_insert);
    ret_int_string_replace = av_opt_set_array(&ctx, "ints", AV_OPT_ARRAY_REPLACE, 2, 1,
                                              AV_OPT_TYPE_STRING, int_string_replace);
    ret_int_string_remove = av_opt_set_array(&ctx, "ints", 0, 0, 1,
                                             AV_OPT_TYPE_STRING, NULL);
    ret_int_string_bad = av_opt_set_array(&ctx, "ints", 0, 0, 1,
                                          AV_OPT_TYPE_STRING, int_string_bad);
    printf("ret:set-array-int-string-typed|%d|%d|%d|%d\n",
           ret_int_string_insert, ret_int_string_replace,
           ret_int_string_remove, ret_int_string_bad);
    print_array_state("state:set-array-int-string-typed", &ctx);
    printf("get:set-array-int-string-typed");
    print_get_array_size_value(&ctx, "ints", 0);
    print_get_array_int64_value(&ctx, "ints", 0, 2);
    print_get_array_string_value(&ctx, "ints", 0, 2);
    print_get_value(&ctx, "ints");
    printf("\n");
    av_opt_free(&ctx);

    init_array_context(&ctx);
    av_opt_set(&ctx, "ints", "3,4", 0);
    ret_int_double_insert = av_opt_set_array(&ctx, "ints", 0, 1, 1,
                                             AV_OPT_TYPE_DOUBLE, int_double_insert);
    ret_int_q_replace = av_opt_set_array(&ctx, "ints", AV_OPT_ARRAY_REPLACE, 2, 1,
                                         AV_OPT_TYPE_RATIONAL, int_q_replace);
    ret_int_numeric_remove = av_opt_set_array(&ctx, "ints", 0, 0, 1,
                                              AV_OPT_TYPE_INT64, NULL);
    ret_int_q_bad = av_opt_set_array(&ctx, "ints", 0, 0, 1,
                                     AV_OPT_TYPE_RATIONAL, int_q_bad);
    printf("ret:set-array-int-numeric-typed|%d|%d|%d|%d\n",
           ret_int_double_insert, ret_int_q_replace,
           ret_int_numeric_remove, ret_int_q_bad);
    print_array_state("state:set-array-int-numeric-typed", &ctx);
    printf("get:set-array-int-numeric-typed");
    print_get_array_size_value(&ctx, "ints", 0);
    print_get_array_int64_value(&ctx, "ints", 0, 2);
    print_get_array_double_value(&ctx, "ints", 0, 2);
    print_get_array_q_value(&ctx, "ints", 0, 2);
    print_get_value(&ctx, "ints");
    printf("\n");
    av_opt_free(&ctx);

    init_array_context(&ctx);
    av_opt_set(&ctx, "ints", "3,4", 0);
    ret_zero_insert = av_opt_set_array(&ctx, "ints", 0, 2, 0,
                                       AV_OPT_TYPE_INT64, insert_value);
    ret_zero_replace = av_opt_set_array(&ctx, "ints", AV_OPT_ARRAY_REPLACE, 2, 0,
                                        AV_OPT_TYPE_INT64, replace_value);
    ret_zero_remove = av_opt_set_array(&ctx, "ints", 0, 2, 0,
                                       AV_OPT_TYPE_INT64, NULL);
    printf("ret:set-array-zero-count|%d|%d|%d",
           ret_zero_insert, ret_zero_replace, ret_zero_remove);
    print_get_array_int64_value(&ctx, "ints", 3, 0);
    printf("\n");
    print_array_state("state:set-array-zero-count", &ctx);
    printf("get:set-array-zero-count");
    print_get_array_size_value(&ctx, "ints", 0);
    print_get_array_int64_value(&ctx, "ints", 0, 2);
    print_get_array_int64_value(&ctx, "ints", 2, 0);
    print_get_array_string_value(&ctx, "ints", 2, 0);
    print_get_array_double_value(&ctx, "ints", 2, 0);
    print_get_array_q_value(&ctx, "ints", 2, 0);
    print_get_value(&ctx, "ints");
    printf("\n");
    av_opt_free(&ctx);

    init_array_context(&ctx);
    av_opt_set(&ctx, "words", "left,right\\,inner", 0);
    ret_string_insert = av_opt_set_array(&ctx, "words", 0, 1, 1,
                                         AV_OPT_TYPE_STRING, string_insert);
    ret_string_replace = av_opt_set_array(&ctx, "words", AV_OPT_ARRAY_REPLACE, 2, 1,
                                          AV_OPT_TYPE_STRING, string_replace);
    ret_string_remove = av_opt_set_array(&ctx, "words", 0, 0, 1,
                                         AV_OPT_TYPE_STRING, NULL);
    printf("ret:set-array-string-typed|%d|%d|%d\n",
           ret_string_insert, ret_string_replace, ret_string_remove);
    print_array_state("state:set-array-string-typed", &ctx);
    printf("get:set-array-string-typed");
    print_get_array_size_value(&ctx, "words", 0);
    print_get_array_string_value(&ctx, "words", 0, 2);
    print_get_value(&ctx, "words");
    printf("\n");

    printf("query-ranges:array");
    print_query_range_value(&ctx, "ints");
    print_query_range_value(&ctx, "missing");
    printf("\n");
    av_opt_free(&ctx);
}

static void print_video_rate_state(const char *name, const VideoRateOptions *ctx) {
    printf("%s|%d|%d|%" PRId64 "\n", name, ctx->rate.num, ctx->rate.den, ctx->scalar);
}

static void print_video_rate_rows(void) {
    VideoRateOptions ctx;
    int ret_ntsc;
    int ret_film;
    int ret_fraction;
    int ret_integer;
    int ret_bad;
    int ret_zero;
    int ret_negative;
    int ret_too_high;
    int ret_typed;
    int ret_zero_typed;
    int ret_wrong_type;
    int ret_q;
    int ret_int;
    AVRational after_ntsc;
    AVRational after_film;
    AVRational after_fraction;
    AVRational after_integer;

    init_video_rate_context(&ctx);
    print_video_rate_state("state:video-rate-defaults", &ctx);
    printf("get:video-rate-defaults");
    print_get_video_rate_value(&ctx, "rate", 0);
    print_get_value(&ctx, "rate");
    print_get_video_rate_value(&ctx, "scalar", 0);
    printf("\n");

    ret_ntsc = av_opt_set(&ctx, "rate", "ntsc", 0);
    after_ntsc = ctx.rate;
    ret_film = av_opt_set(&ctx, "rate", "film", 0);
    after_film = ctx.rate;
    ret_fraction = av_opt_set(&ctx, "rate", "30000/1001", 0);
    after_fraction = ctx.rate;
    ret_integer = av_opt_set(&ctx, "rate", "25", 0);
    after_integer = ctx.rate;
    printf("ret:set-video-rate-strings|%d|%d|%d|%d\n",
           ret_ntsc, ret_film, ret_fraction, ret_integer);
    printf("state:set-video-rate-strings|%d/%d|%d/%d|%d/%d|%d/%d\n",
           after_ntsc.num, after_ntsc.den,
           after_film.num, after_film.den,
           after_fraction.num, after_fraction.den,
           after_integer.num, after_integer.den);
    printf("get:set-video-rate-strings");
    print_get_value(&ctx, "rate");
    printf("\n");

    ret_bad = av_opt_set(&ctx, "rate", "bad", 0);
    ret_zero = av_opt_set(&ctx, "rate", "0", 0);
    ret_negative = av_opt_set(&ctx, "rate", "-25", 0);
    ret_too_high = av_opt_set(&ctx, "rate", "121", 0);
    printf("ret:set-video-rate-errors|%d|%d|%d|%d\n",
           ret_bad, ret_zero, ret_negative, ret_too_high);
    print_video_rate_state("state:after-video-rate-errors", &ctx);

    init_video_rate_context(&ctx);
    ret_typed = av_opt_set_video_rate(&ctx, "rate", (AVRational){ 50, 1 }, 0);
    ret_zero_typed = av_opt_set_video_rate(&ctx, "rate", (AVRational){ 0, 1 }, 0);
    ret_wrong_type = av_opt_set_video_rate(&ctx, "scalar", (AVRational){ 1, 1 }, 0);
    ret_q = av_opt_set_q(&ctx, "rate", (AVRational){ 60, 1 }, 0);
    ret_int = av_opt_set_int(&ctx, "rate", 75, 0);
    printf("ret:set-video-rate-typed|%d|%d|%d|%d|%d\n",
           ret_typed, ret_zero_typed, ret_wrong_type, ret_q, ret_int);
    print_video_rate_state("state:set-video-rate-typed", &ctx);
    printf("get:set-video-rate-typed");
    print_get_video_rate_value(&ctx, "rate", 0);
    print_get_q_value(&ctx, "rate", 0);
    print_get_int_value(&ctx, "rate", 0);
    print_get_value(&ctx, "rate");
    print_get_video_rate_value(&ctx, "scalar", 0);
    printf("\n");

    printf("query-ranges:video-rate");
    print_query_range_value(&ctx, "rate");
    print_query_range_value(&ctx, "missing");
    printf("\n");
}

static void print_color_state(const char *name, const ColorOptions *ctx) {
    printf("%s|%d|%d|%d|%d|%" PRId64 "\n",
           name, ctx->color[0], ctx->color[1], ctx->color[2], ctx->color[3], ctx->scalar);
}

static void print_color_rows(void) {
    ColorOptions ctx;
    int ret_blue;
    int ret_hex;
    int ret_hex_alpha;
    int ret_bad;
    int ret_bad_alpha;
    int ret_numeric;
    int ret_zero_numeric;
    int ret_scalar;
    uint8_t after_blue[4];
    uint8_t after_hex[4];
    uint8_t after_hex_alpha[4];

    init_color_context(&ctx);
    print_color_state("state:color-defaults", &ctx);
    printf("get:color-defaults");
    print_get_value(&ctx, "color");
    print_get_int_value(&ctx, "color", 0);
    print_get_int_value(&ctx, "scalar", 0);
    printf("\n");

    ret_blue = av_opt_set(&ctx, "color", "Blue@0.5", 0);
    memcpy(after_blue, ctx.color, sizeof(after_blue));
    ret_hex = av_opt_set(&ctx, "color", "112233", 0);
    memcpy(after_hex, ctx.color, sizeof(after_hex));
    ret_hex_alpha = av_opt_set(&ctx, "color", "0x11223344", 0);
    memcpy(after_hex_alpha, ctx.color, sizeof(after_hex_alpha));
    printf("ret:set-color-strings|%d|%d|%d\n", ret_blue, ret_hex, ret_hex_alpha);
    printf("state:set-color-strings|%d:%d:%d:%d|%d:%d:%d:%d|%d:%d:%d:%d\n",
           after_blue[0], after_blue[1], after_blue[2], after_blue[3],
           after_hex[0], after_hex[1], after_hex[2], after_hex[3],
           after_hex_alpha[0], after_hex_alpha[1], after_hex_alpha[2], after_hex_alpha[3]);
    printf("get:set-color-strings");
    print_get_value(&ctx, "color");
    printf("\n");

    ret_bad = av_opt_set(&ctx, "color", "not-a-color", 0);
    ret_bad_alpha = av_opt_set(&ctx, "color", "red@2", 0);
    printf("ret:set-color-errors|%d|%d\n", ret_bad, ret_bad_alpha);
    print_color_state("state:after-color-errors", &ctx);

    init_color_context(&ctx);
    ret_numeric = av_opt_set_int(&ctx, "color", 10, 0);
    ret_zero_numeric = av_opt_set_int(&ctx, "color", 0, 0);
    ret_scalar = av_opt_set_int(&ctx, "scalar", 6, 0);
    printf("ret:set-color-typed|%d|%d|%d\n", ret_numeric, ret_zero_numeric, ret_scalar);
    print_color_state("state:set-color-typed", &ctx);
    printf("get:set-color-typed");
    print_get_int_value(&ctx, "color", 0);
    print_get_double_value(&ctx, "color", 0);
    print_get_q_value(&ctx, "color", 0);
    print_get_value(&ctx, "color");
    print_get_int_value(&ctx, "scalar", 0);
    printf("\n");

    printf("query-ranges:color");
    print_query_range_value(&ctx, "color");
    print_query_range_value(&ctx, "missing");
    printf("\n");
}

static void print_set_from_string_rows(void) {
    static const char * const shorthand[] = { "threads", "bitexact", NULL };
    TestOptions ctx;
    int ret;
    int ret_empty_key;
    int ret_empty_value;
    int ret_invalid_separator_1;
    int ret_invalid_separator_2;
    int ret_invalid_separator_3;

    init_context(&ctx);
    ret = av_opt_set_from_string(&ctx,
                                 "threads=7:quality=0.25:metadata=from-string",
                                 NULL, "=", ":");
    printf("ret:set-from-string-named|%d\n", ret);
    print_state("state:set-from-string-named", &ctx);
    av_opt_free(&ctx);
    av_opt_free(&ctx.child);

    init_context(&ctx);
    ret = av_opt_set_from_string(&ctx,
                                 " 9 : yes : metadata = shorthand ",
                                 shorthand, "=", ":");
    printf("ret:set-from-string-shorthand|%d\n", ret);
    print_state("state:set-from-string-shorthand", &ctx);
    av_opt_free(&ctx);
    av_opt_free(&ctx.child);

    init_context(&ctx);
    ret = av_opt_set_from_string(&ctx,
                                 "10:quality=0.75:no",
                                 shorthand, "=", ":");
    printf("ret:set-from-string-after-named-error|%d\n", ret);
    print_state("state:set-from-string-after-named-error", &ctx);
    av_opt_free(&ctx);
    av_opt_free(&ctx.child);

    init_context(&ctx);
    ret = av_opt_set_from_string(&ctx,
                                 "threads=10:yes:quality=0.75",
                                 shorthand, "=", ":");
    printf("ret:set-from-string-after-explicit-shorthand-error|%d\n", ret);
    print_state("state:set-from-string-after-explicit-shorthand-error", &ctx);
    av_opt_free(&ctx);
    av_opt_free(&ctx.child);

    init_context(&ctx);
    ret = av_opt_set_from_string(&ctx,
                                 "threads=11:bitexact=maybe",
                                 NULL, "=", ":");
    printf("ret:set-from-string-set-error|%d\n", ret);
    print_state("state:set-from-string-set-error", &ctx);
    av_opt_free(&ctx);
    av_opt_free(&ctx.child);

    init_context(&ctx);
    ret = av_opt_set_from_string(&ctx,
                                 "threads=12:unknown=1",
                                 NULL, "=", ":");
    printf("ret:set-from-string-not-found|%d\n", ret);
    print_state("state:set-from-string-not-found", &ctx);
    av_opt_free(&ctx);
    av_opt_free(&ctx.child);

    init_context(&ctx);
    ret = av_opt_set_from_string(&ctx, "12", NULL, "=", ":");
    printf("ret:set-from-string-no-shorthand|%d\n", ret);
    print_state("state:set-from-string-no-shorthand", &ctx);
    av_opt_free(&ctx);
    av_opt_free(&ctx.child);

    init_context(&ctx);
    ret_invalid_separator_1 = av_opt_set_from_string(&ctx,
                                 "threads=7",
                                 NULL,
                                 "",
                                 ":");
    ret_invalid_separator_2 = av_opt_set_from_string(&ctx,
                                 "threads=7",
                                 NULL,
                                 "=",
                                 "");
    ret_invalid_separator_3 = av_opt_set_from_string(&ctx,
                                 "threads=7",
                                 NULL,
                                 ":=",
                                 ":");
    printf("ret:set-from-string-invalid-separators|%d|%d|%d\n",
           ret_invalid_separator_1,
           ret_invalid_separator_2,
           ret_invalid_separator_3);
    av_opt_free(&ctx);
    av_opt_free(&ctx.child);

    init_context(&ctx);
    ret = av_opt_set_from_string(&ctx,
                                 "9:yes:15",
                                 shorthand,
                                 "=",
                                 ":");
    printf("ret:set-from-string-shorthand-overflow|%d\n", ret);
    print_state("state:set-from-string-shorthand-overflow", &ctx);
    av_opt_free(&ctx);
    av_opt_free(&ctx.child);

    init_context(&ctx);
    ret = av_opt_set_from_string(&ctx,
                                 "metadata=title\\:clip\\=one\\\\two:threads=14:preset_level=slow",
                                 NULL, "=", ":");
    printf("ret:set-from-string-escaped|%d\n", ret);
    print_state("state:set-from-string-escaped", &ctx);
    av_opt_free(&ctx);
    av_opt_free(&ctx.child);

    init_context(&ctx);
    ret = av_opt_set_from_string(&ctx,
                                 "metadata=' title : clip = one ':threads=15",
                                 NULL, "=", ":");
    printf("ret:set-from-string-quoted|%d\n", ret);
    print_state("state:set-from-string-quoted", &ctx);
    ret = av_opt_set_from_string(&ctx,
                                 "metadata='\\''x'",
                                 NULL, "=", ":");
    printf("ret:set-from-string-quote-escape|%d\n", ret);
    print_state("state:set-from-string-quote-escape", &ctx);
    ret_empty_key = av_opt_set_from_string(&ctx, "=7", NULL, "=", ":");
    printf("ret:set-from-string-empty-key|%d\n", ret_empty_key);

    av_opt_free(&ctx);
    av_opt_free(&ctx.child);
    init_context(&ctx);
    ret_empty_value = av_opt_set_from_string(&ctx,
                                            "metadata=",
                                            NULL,
                                            "=",
                                            ":");
    printf("ret:set-from-string-empty-value|%d\n", ret_empty_value);
    print_state("state:set-from-string-empty-value", &ctx);
    ret = av_opt_set_from_string(&ctx,
                                 "metadata='title",
                                 NULL, "=", ":");
    printf("ret:set-from-string-unclosed-quote|%d\n", ret);
    av_opt_free(&ctx);
    av_opt_free(&ctx.child);

    init_context(&ctx);
    ret = av_opt_set_from_string(&ctx,
                                 "threads=7:quality=0.25",
                                 NULL, "=", "");
    printf("ret:set-from-string-empty-pairs-multi|%d\n", ret);
    print_state("state:set-from-string-empty-pairs-multi", &ctx);
    av_opt_free(&ctx);
    av_opt_free(&ctx.child);

    init_context(&ctx);
    ret = av_opt_set_from_string(&ctx,
                                 "metadata=title=clip",
                                 NULL, "=", "");
    printf("ret:set-from-string-empty-pairs-embedded-equals|%d\n", ret);
    print_state("state:set-from-string-empty-pairs-embedded-equals", &ctx);
    av_opt_free(&ctx);
    av_opt_free(&ctx.child);

    init_context(&ctx);
    ret = av_opt_set_from_string(&ctx, "threads=7:", NULL, "=", ":");
    printf("ret:set-from-string-trailing-pair-sep|%d\n", ret);
    print_state("state:set-from-string-trailing-pair-sep", &ctx);
    av_opt_free(&ctx);
    av_opt_free(&ctx.child);
}

static void print_expression_rows(void) {
    TestOptions ctx;
    int ret_threads;
    int ret_quality;
    int ret_aspect;
    int ret_preset;
    int ret_constant;
    int ret_range;
    int ret_parse;

    init_context(&ctx);
    ret_threads = av_opt_set(&ctx, "threads", " 2 * 3 ", 0);
    ret_quality = av_opt_set(&ctx, "quality", "500m", 0);
    ret_aspect = av_opt_set(&ctx, "aspect_ratio", "1+1/2", 0);
    ret_preset = av_opt_set(&ctx, "preset_level", "slow+2", 0);
    printf("ret:set-expressions|%d|%d|%d|%d\n",
           ret_threads, ret_quality, ret_aspect, ret_preset);
    print_state("state:set-expressions", &ctx);
    printf("get:set-expressions");
    print_get_value(&ctx, "threads");
    print_get_value(&ctx, "quality");
    print_get_value(&ctx, "aspect_ratio");
    print_get_value(&ctx, "preset_level");
    printf("\n");

    ret_range = av_opt_set(&ctx, "threads", "1K", 0);
    ret_parse = av_opt_set(&ctx, "quality", "2*", 0);
    printf("ret:set-expression-errors|%d|%d\n", ret_range, ret_parse);
    print_state("state:after-expression-errors", &ctx);
    ret_constant = av_opt_set(&ctx, "threads", "PI", 0);
    printf("ret:set-expression-constants|%d\n", ret_constant);
    print_state("state:set-expression-constants", &ctx);
    printf("get:set-expression-constants");
    print_get_value(&ctx, "threads");
    printf("\n");
    av_opt_free(&ctx);
    av_opt_free(&ctx.child);
}

static void print_serialize_value(TestOptions *ctx, int opt_flags, int flags,
                                  char key_val_sep, char pairs_sep) {
    char *buf = NULL;
    int ret = av_opt_serialize(ctx, opt_flags, flags, &buf, key_val_sep, pairs_sep);

    printf("|%d:%s", ret, ret >= 0 && buf ? buf : "<null>");
    av_free(buf);
}

static void print_serialize_invalid_value(TestOptions *ctx, int opt_flags, int flags,
                                          char key_val_sep, char pairs_sep) {
    char *buf = NULL;
    int ret = av_opt_serialize(ctx, opt_flags, flags, &buf, key_val_sep, pairs_sep);

    printf("|%d:<null>", ret);
    av_free(buf);
}

static void print_serialize_rows(void) {
    TestOptions defaults;
    TestOptions changed;
    TestOptions children;

    init_context(&defaults);
    printf("serialize:defaults");
    print_serialize_value(&defaults, 0, 0, '=', ',');
    print_serialize_value(&defaults, AV_OPT_FLAG_ENCODING_PARAM,
                          AV_OPT_SERIALIZE_OPT_FLAGS_EXACT, '=', ',');
    print_serialize_value(&defaults, 0, AV_OPT_SERIALIZE_OPT_FLAGS_EXACT, '=', ',');
    print_serialize_value(&defaults, AV_OPT_FLAG_EXPORT, 0, '=', ',');
    printf("\n");
    av_opt_free(&defaults);
    av_opt_free(&defaults.child);

    init_context(&changed);
    av_opt_set(&changed, "threads", "8", 0);
    av_opt_set(&changed, "bitexact", "true", 0);
    av_opt_set(&changed, "metadata", "title=clip,segment\\one", 0);
    av_opt_set(&changed, "preset_level", "slow", 0);
    printf("serialize:skip-defaults");
    print_serialize_value(&changed, 0, AV_OPT_SERIALIZE_SKIP_DEFAULTS, '=', ',');
    printf("\n");
    av_opt_free(&changed);
    av_opt_free(&changed.child);

    init_context(&children);
    printf("serialize:children");
    print_serialize_value(&children, 0, AV_OPT_SERIALIZE_SEARCH_CHILDREN, '=', ',');
    print_serialize_value(&children, AV_OPT_FLAG_DECODING_PARAM,
                          AV_OPT_SERIALIZE_SEARCH_CHILDREN, '=', ',');
    printf("\n");
    printf("serialize:invalid-separators");
    print_serialize_invalid_value(&children, 0, 0, '=', '=');
    print_serialize_invalid_value(&children, 0, 0, '\\', ',');
    print_serialize_invalid_value(&children, 0, 0, '=', '\0');
    printf("\n");
    av_opt_free(&children);
    av_opt_free(&children.child);
}

int main(void) {
    TestOptions ctx = { 0 };
    int ret_threads;
    int ret_bitexact;
    int ret_quality;
    int ret_aspect;
    int ret_metadata;
    int ret_preset;
    int ret_upper_threads;
    int ret_upper_preset;
    int ret_const_name;
    int ret_invalid_bool;
    int ret_readonly;

    init_context(&ctx);

    print_flags();
    print_types();
    print_search_flags();
    print_serialize_flags();
    print_next_order(&ctx);
    print_find_rows(&ctx);
    print_state("state:defaults", &ctx);
    print_get_row("get:defaults", &ctx);
    print_get_errors(&ctx);
    print_get_allow_null_row(&ctx);
    print_query_ranges_row(&ctx);
    print_find_children_row(&ctx);
    print_get_children_row(&ctx);
    print_set_children_row(&ctx);
    print_child_state("state:children-after-set", &ctx);
    print_set_dict_rows();
    print_copy_rows();
    print_set_from_string_rows();
    print_expression_rows();
    print_serialize_rows();
    print_typed_get_set_rows();
    print_duration_rows();
    print_image_size_rows();
    print_image_size_child_rows();
    print_pixel_format_rows();
    print_sample_format_rows();
    print_channel_layout_rows();
    print_binary_rows();
    print_dictionary_rows();
    print_array_rows();
    print_video_rate_rows();
    print_color_rows();

    ret_upper_threads = av_opt_set(&ctx, "THREADS", "9", 0);
    ret_upper_preset = av_opt_set(&ctx, "preset_level", "SLOW", 0);
    ret_const_name = av_opt_set(&ctx, "fast", "2", 0);
    printf("ret:set-exact-errors|%d|%d|%d\n",
           ret_upper_threads, ret_upper_preset, ret_const_name);
    print_state("state:after-exact-errors", &ctx);

    ret_threads = av_opt_set(&ctx, "threads", "8", 0);
    ret_bitexact = av_opt_set(&ctx, "bitexact", "yes", 0);
    ret_quality = av_opt_set(&ctx, "quality", "0.75", 0);
    ret_aspect = av_opt_set(&ctx, "aspect_ratio", "4/3", 0);
    ret_metadata = av_opt_set(&ctx, "metadata", "title=clip", 0);
    ret_preset = av_opt_set(&ctx, "preset_level", "slow", 0);
    printf("ret:set-supported|%d|%d|%d|%d|%d|%d\n",
           ret_threads, ret_bitexact, ret_quality, ret_aspect, ret_metadata, ret_preset);
    print_state("state:set-supported", &ctx);
    print_get_row("get:set-supported", &ctx);

    ret_invalid_bool = av_opt_set(&ctx, "bitexact", "maybe", 0);
    ret_readonly = av_opt_set(&ctx, "exported", "6", 0);
    printf("ret:set-errors|%d|%d\n", ret_invalid_bool, ret_readonly);
    print_state("state:after-errors", &ctx);

    av_opt_free(&ctx);
    return 0;
}
"#
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
