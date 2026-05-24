use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use avutil::{
    AvOptionRanges, OptionConstant, OptionDefinition, OptionEntryMatch, OptionFlags, OptionKind,
    OptionSearchFlags, OptionSet, OptionValue, Rational,
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
        .define(
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

fn int_value(options: &OptionSet, name: &str) -> i64 {
    match options.get(name) {
        Some(OptionValue::Int(value)) => *value,
        other => panic!("expected int option `{name}`, got {other:?}"),
    }
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

#include <libavutil/avutil.h>
#include <libavutil/mem.h>
#include <libavutil/opt.h>
#include <libavutil/rational.h>

#define ROW_INT(name, value) printf("%s|%d\n", name, (int)(value))

typedef struct TestOptions {
    const AVClass *av_class;
    int64_t threads;
    int bitexact;
    double quality;
    AVRational aspect_ratio;
    char *metadata;
    int64_t preset_level;
    int64_t exported;
} TestOptions;

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
};

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

static void print_get_value(const TestOptions *ctx, const char *name) {
    uint8_t *value = NULL;
    int ret = av_opt_get(ctx, name, 0, &value);
    printf("|%d:%s", ret, ret >= 0 && value ? (const char *)value : "<null>");
    av_free(value);
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

static void print_query_range_value(const TestOptions *ctx, const char *name) {
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

    ctx.av_class = &test_class;
    av_opt_set_defaults(&ctx);

    print_flags();
    print_types();
    print_search_flags();
    print_next_order(&ctx);
    print_find_rows(&ctx);
    print_state("state:defaults", &ctx);
    print_get_row("get:defaults", &ctx);
    print_get_errors(&ctx);
    print_query_ranges_row(&ctx);

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
