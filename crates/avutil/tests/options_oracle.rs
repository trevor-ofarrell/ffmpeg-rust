use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use avutil::{
    AvOptionRanges, Dictionary, MatchMode, OptionChild, OptionConstant, OptionDefinition,
    OptionEntryMatch, OptionFlags, OptionKind, OptionSearchFlags, OptionSerializeFlags, OptionSet,
    OptionValue, PixelFormat, Rational, RgbaColor, SampleFormat, SetMode,
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
    rows.insert(
        "state:set-from-string-quoted".to_string(),
        state_fields(&string_quoted),
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
    insert_row(
        &mut rows,
        "ret:set-pixel-format-strings",
        [ret_rgb24, ret_gray, ret_none, ret_numeric],
    );
    insert_row(
        &mut rows,
        "state:set-pixel-format-strings",
        [
            pixel_format_field(after_rgb24),
            pixel_format_field(after_gray),
            pixel_format_field(after_none),
            pixel_format_field(after_numeric),
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
            ret(pixel_set.set_avoption_from_str("pix_fmt", "25")),
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
            ret(typed_pixel_options.set_avoption_int("pix_fmt", 25)),
            ret(typed_pixel_options.set_avoption_int("scalar", 6)),
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

fn pixel_format_options() -> OptionSet {
    let mut options = OptionSet::new();
    options
        .define(
            OptionDefinition::new_with_flags(
                "pix_fmt",
                OptionKind::PixelFormat { min: -1, max: 24 },
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

static const AVOption pixel_format_options[] = {
    { "pix_fmt", "pixel format", offsetof(PixelFormatOptions, pix_fmt),
      AV_OPT_TYPE_PIXEL_FMT, { .i64 = AV_PIX_FMT_YUV420P }, AV_PIX_FMT_NONE, 24, AV_OPT_FLAG_ENCODING_PARAM },
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

static const AVClass image_size_class = {
    .class_name = "rust-options-oracle-image-size",
    .item_name = av_default_item_name,
    .option = image_size_options,
    .version = LIBAVUTIL_VERSION_INT,
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
    printf("search-flags|%d|%d\n",
           AV_OPT_SEARCH_CHILDREN,
           AV_OPT_SEARCH_FAKE_OBJ);
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

static void print_pixel_format_state(const char *name, const PixelFormatOptions *ctx) {
    printf("%s|%d|%" PRId64 "\n", name, ctx->pix_fmt, ctx->scalar);
}

static void print_pixel_format_rows(void) {
    PixelFormatOptions ctx;
    int ret_rgb24;
    int ret_gray;
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
    enum AVPixelFormat after_rgb24;
    enum AVPixelFormat after_gray;
    enum AVPixelFormat after_none;
    enum AVPixelFormat after_numeric;

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
    printf("ret:set-pixel-format-strings|%d|%d|%d|%d\n",
           ret_rgb24, ret_gray, ret_none, ret_numeric);
    printf("state:set-pixel-format-strings|%d|%d|%d|%d\n",
           after_rgb24, after_gray, after_none, after_numeric);
    printf("get:set-pixel-format-strings");
    print_get_value(&ctx, "pix_fmt");
    printf("\n");

    ret_bad = av_opt_set(&ctx, "pix_fmt", "bad", 0);
    ret_out_of_range = av_opt_set(&ctx, "pix_fmt", "25", 0);
    ret_negative_numeric = av_opt_set(&ctx, "pix_fmt", "-1", 0);
    printf("ret:set-pixel-format-errors|%d|%d|%d\n",
           ret_bad, ret_out_of_range, ret_negative_numeric);
    print_pixel_format_state("state:after-pixel-format-errors", &ctx);

    init_pixel_format_context(&ctx);
    ret_typed = av_opt_set_pixel_fmt(&ctx, "pix_fmt", AV_PIX_FMT_RGB24, 0);
    ret_typed_none = av_opt_set_pixel_fmt(&ctx, "pix_fmt", AV_PIX_FMT_NONE, 0);
    ret_wrong_type = av_opt_set_pixel_fmt(&ctx, "scalar", AV_PIX_FMT_RGB24, 0);
    ret_int = av_opt_set_int(&ctx, "pix_fmt", AV_PIX_FMT_BGR24, 0);
    ret_int_range = av_opt_set_int(&ctx, "pix_fmt", 25, 0);
    ret_scalar = av_opt_set_int(&ctx, "scalar", 6, 0);
    printf("ret:set-pixel-format-typed|%d|%d|%d|%d|%d|%d\n",
           ret_typed, ret_typed_none, ret_wrong_type, ret_int, ret_int_range, ret_scalar);
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
    av_opt_free(&ctx);
    av_opt_free(&ctx.child);
}

static void print_expression_rows(void) {
    TestOptions ctx;
    int ret_threads;
    int ret_quality;
    int ret_aspect;
    int ret_preset;
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
    print_pixel_format_rows();
    print_sample_format_rows();
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
