use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use avutil::{
    AvLogContextPrefix, AvLogFormatLine, AvLogFormatLine2, LogFlags, LogLevel, LogRecord,
    LogTimestamp,
};

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 libavutil oracle under third_party/ffmpeg-oracle/wsl"]
fn libavutil_logging_constants_and_state_match_current_model() {
    let repo_root = repo_root();
    let oracle_root = oracle_root(&repo_root);
    let include_dir = oracle_root.join("wsl/include");
    let libavutil = oracle_root.join("wsl/lib/libavutil.a");

    assert!(
        include_dir.join("libavutil/log.h").is_file(),
        "missing pinned FFmpeg libavutil headers under `{}`",
        include_dir.display()
    );
    assert!(
        libavutil.is_file(),
        "missing pinned FFmpeg libavutil static library `{}`",
        libavutil.display()
    );

    let work_dir = repo_root.join("target/oracle/avutil-logging");
    fs::create_dir_all(&work_dir).expect("create avutil-logging oracle work dir");
    let source = work_dir.join("logging_oracle.c");
    let executable = work_dir.join("logging_oracle");
    fs::write(&source, oracle_c_source()).expect("write avutil-logging oracle C source");

    let stdout = compile_and_run_oracle(&include_dir, &libavutil, &source, &executable);
    let oracle = parse_oracle_output(&stdout);
    let expected = expected_rows();

    for (name, expected_value) in expected {
        let actual = oracle
            .get(name)
            .unwrap_or_else(|| panic!("missing oracle row `{name}`"));
        let actual = actual
            .parse::<i32>()
            .unwrap_or_else(|err| panic!("invalid value in oracle row `{name}`: {err}"));
        assert_eq!(
            actual, expected_value,
            "logging oracle mismatch for `{name}`"
        );
    }

    let expected_text = expected_text_rows();
    for (name, expected_value) in expected_text {
        let actual = oracle
            .get(name)
            .unwrap_or_else(|| panic!("missing oracle row `{name}`"));
        assert_eq!(
            actual, &expected_value,
            "logging oracle text mismatch for `{name}`"
        );
    }
}

fn expected_rows() -> BTreeMap<&'static str, i32> {
    let all_flags = LogFlags::SKIP_REPEATED
        | LogFlags::PRINT_LEVEL
        | LogFlags::PRINT_TIME
        | LogFlags::PRINT_DATETIME;
    let mut rows = [
        ("AV_LOG_QUIET", LogLevel::Quiet.as_ffmpeg_value()),
        ("AV_LOG_PANIC", LogLevel::Panic.as_ffmpeg_value()),
        ("AV_LOG_FATAL", LogLevel::Fatal.as_ffmpeg_value()),
        ("AV_LOG_ERROR", LogLevel::Error.as_ffmpeg_value()),
        ("AV_LOG_WARNING", LogLevel::Warning.as_ffmpeg_value()),
        ("AV_LOG_INFO", LogLevel::Info.as_ffmpeg_value()),
        ("AV_LOG_VERBOSE", LogLevel::Verbose.as_ffmpeg_value()),
        ("AV_LOG_DEBUG", LogLevel::Debug.as_ffmpeg_value()),
        ("AV_LOG_TRACE", LogLevel::Trace.as_ffmpeg_value()),
        (
            "AV_LOG_MAX_OFFSET",
            LogLevel::Trace.as_ffmpeg_value() - LogLevel::Quiet.as_ffmpeg_value(),
        ),
        (
            "AV_LOG_SKIP_REPEATED",
            LogFlags::SKIP_REPEATED.bits() as i32,
        ),
        ("AV_LOG_PRINT_LEVEL", LogFlags::PRINT_LEVEL.bits() as i32),
        ("AV_LOG_PRINT_TIME", LogFlags::PRINT_TIME.bits() as i32),
        (
            "AV_LOG_PRINT_DATETIME",
            LogFlags::PRINT_DATETIME.bits() as i32,
        ),
        ("default-level", LogLevel::Info.as_ffmpeg_value()),
        ("set-level-quiet", LogLevel::Quiet.as_ffmpeg_value()),
        ("set-level-panic", LogLevel::Panic.as_ffmpeg_value()),
        ("set-level-fatal", LogLevel::Fatal.as_ffmpeg_value()),
        ("set-level-error", LogLevel::Error.as_ffmpeg_value()),
        ("set-level-warning", LogLevel::Warning.as_ffmpeg_value()),
        ("set-level-info", LogLevel::Info.as_ffmpeg_value()),
        ("set-level-verbose", LogLevel::Verbose.as_ffmpeg_value()),
        ("set-level-debug", LogLevel::Debug.as_ffmpeg_value()),
        ("set-level-trace", LogLevel::Trace.as_ffmpeg_value()),
        ("set-flags-empty", LogFlags::empty().bits() as i32),
        (
            "set-flags-skip-repeated",
            LogFlags::SKIP_REPEATED.bits() as i32,
        ),
        ("set-flags-print-level", LogFlags::PRINT_LEVEL.bits() as i32),
        ("set-flags-print-time", LogFlags::PRINT_TIME.bits() as i32),
        (
            "set-flags-print-datetime",
            LogFlags::PRINT_DATETIME.bits() as i32,
        ),
        ("set-flags-all-known", all_flags.bits() as i32),
        ("log-once-first-state", 1),
        ("log-once-first-count", 1),
        ("log-once-first-level", LogLevel::Warning.as_ffmpeg_value()),
        ("log-once-second-state", 1),
        ("log-once-second-count", 2),
        ("log-once-second-level", LogLevel::Debug.as_ffmpeg_value()),
        ("log-once-preseed-state", 1),
        ("log-once-preseed-count", 3),
        ("log-once-preseed-level", LogLevel::Error.as_ffmpeg_value()),
    ]
    .into_iter()
    .collect::<BTreeMap<_, _>>();

    add_format_line2_int_rows(&mut rows);
    add_format_line_int_rows(&mut rows);
    rows
}

fn expected_text_rows() -> BTreeMap<&'static str, String> {
    let mut rows = BTreeMap::new();
    let (plain, _) = rust_format_line2(LogLevel::Warning, "plain", LogFlags::empty(), true, 128);
    rows.insert("format-line2-plain-line", escape_row_text(plain.bytes()));

    let (level, _) =
        rust_format_line2(LogLevel::Warning, "plain", LogFlags::PRINT_LEVEL, true, 128);
    rows.insert("format-line2-level-line", escape_row_text(level.bytes()));

    let (no_prefix, _) =
        rust_format_line2(LogLevel::Error, "after", LogFlags::PRINT_LEVEL, false, 128);
    rows.insert(
        "format-line2-noprefix-line",
        escape_row_text(no_prefix.bytes()),
    );

    let (newline, _) =
        rust_format_line2(LogLevel::Info, "withnl\n", LogFlags::PRINT_LEVEL, true, 128);
    rows.insert(
        "format-line2-newline-line",
        escape_row_text(newline.bytes()),
    );

    let (small, _) = rust_format_line2(LogLevel::Warning, "plain", LogFlags::PRINT_LEVEL, true, 8);
    rows.insert("format-line2-small-line", escape_row_text(small.bytes()));

    let (size1, _) = rust_format_line2(LogLevel::Warning, "plain", LogFlags::PRINT_LEVEL, true, 1);
    rows.insert("format-line2-size1-line", escape_row_text(size1.bytes()));

    let (context_plain, _) =
        rust_format_line2_context(LogLevel::Warning, "ctxmsg", LogFlags::empty(), true, 128);
    rows.insert(
        "format-line2-context-plain-line",
        escape_row_text(context_plain.bytes()),
    );

    let (context_level, _) = rust_format_line2_context(
        LogLevel::Warning,
        "ctxmsg",
        LogFlags::PRINT_LEVEL,
        true,
        128,
    );
    rows.insert(
        "format-line2-context-level-line",
        escape_row_text(context_level.bytes()),
    );

    let (context_no_prefix, _) = rust_format_line2_context(
        LogLevel::Warning,
        "nopfx",
        LogFlags::PRINT_LEVEL,
        false,
        128,
    );
    rows.insert(
        "format-line2-context-noprefix-line",
        escape_row_text(context_no_prefix.bytes()),
    );

    let (context_newline, _) =
        rust_format_line2_context(LogLevel::Info, "withnl\n", LogFlags::PRINT_LEVEL, true, 128);
    rows.insert(
        "format-line2-context-newline-line",
        escape_row_text(context_newline.bytes()),
    );

    let (time, _) = rust_format_line2(LogLevel::Warning, "plain", LogFlags::PRINT_TIME, true, 128);
    rows.insert("format-line2-time-line", escape_row_text(time.bytes()));

    let (datetime_level, _) = rust_format_line2(
        LogLevel::Warning,
        "plain",
        LogFlags::PRINT_DATETIME | LogFlags::PRINT_LEVEL,
        true,
        128,
    );
    rows.insert(
        "format-line2-datetime-level-line",
        escape_row_text(datetime_level.bytes()),
    );

    let (both_level, _) = rust_format_line2(
        LogLevel::Warning,
        "plain",
        LogFlags::PRINT_TIME | LogFlags::PRINT_DATETIME | LogFlags::PRINT_LEVEL,
        true,
        128,
    );
    rows.insert(
        "format-line2-time-datetime-level-line",
        escape_row_text(both_level.bytes()),
    );

    let (context_time_level, _) = rust_format_line2_context(
        LogLevel::Warning,
        "ctxmsg",
        LogFlags::PRINT_TIME | LogFlags::PRINT_LEVEL,
        true,
        128,
    );
    rows.insert(
        "format-line2-context-time-level-line",
        escape_row_text(context_time_level.bytes()),
    );

    let timestamp = LogTimestamp::from_unix_micros(1_704_112_705_123_456);
    let default_time = rust_default_callback_line(LogFlags::PRINT_TIME, Some(timestamp));
    rows.insert(
        "default-callback-time-line",
        escape_row_text(normalize_default_callback_timestamp(&default_time).as_bytes()),
    );
    let default_time_level = rust_default_callback_line(
        LogFlags::PRINT_TIME | LogFlags::PRINT_LEVEL,
        Some(timestamp),
    );
    rows.insert(
        "default-callback-time-level-line",
        escape_row_text(normalize_default_callback_timestamp(&default_time_level).as_bytes()),
    );
    let default_datetime = rust_default_callback_line(LogFlags::PRINT_DATETIME, Some(timestamp));
    rows.insert(
        "default-callback-datetime-line",
        escape_row_text(normalize_default_callback_timestamp(&default_datetime).as_bytes()),
    );
    let default_datetime_level = rust_default_callback_line(
        LogFlags::PRINT_DATETIME | LogFlags::PRINT_LEVEL,
        Some(timestamp),
    );
    rows.insert(
        "default-callback-datetime-level-line",
        escape_row_text(normalize_default_callback_timestamp(&default_datetime_level).as_bytes()),
    );
    let default_both_level = rust_default_callback_line(
        LogFlags::PRINT_TIME | LogFlags::PRINT_DATETIME | LogFlags::PRINT_LEVEL,
        Some(timestamp),
    );
    rows.insert(
        "default-callback-time-datetime-level-line",
        escape_row_text(normalize_default_callback_timestamp(&default_both_level).as_bytes()),
    );

    let (plain, _) = rust_format_line(LogLevel::Warning, "plain", LogFlags::empty(), true, 128);
    rows.insert("format-line-plain-line", escape_row_text(plain.bytes()));

    let (level, _) = rust_format_line(LogLevel::Warning, "plain", LogFlags::PRINT_LEVEL, true, 128);
    rows.insert("format-line-level-line", escape_row_text(level.bytes()));

    let (no_prefix, _) =
        rust_format_line(LogLevel::Error, "after", LogFlags::PRINT_LEVEL, false, 128);
    rows.insert(
        "format-line-noprefix-line",
        escape_row_text(no_prefix.bytes()),
    );

    let (newline, _) =
        rust_format_line(LogLevel::Info, "withnl\n", LogFlags::PRINT_LEVEL, true, 128);
    rows.insert("format-line-newline-line", escape_row_text(newline.bytes()));

    let (small, _) = rust_format_line(LogLevel::Warning, "plain", LogFlags::PRINT_LEVEL, true, 8);
    rows.insert("format-line-small-line", escape_row_text(small.bytes()));

    let (size1, _) = rust_format_line(LogLevel::Warning, "plain", LogFlags::PRINT_LEVEL, true, 1);
    rows.insert("format-line-size1-line", escape_row_text(size1.bytes()));

    let (context_level, _) = rust_format_line_context(
        LogLevel::Warning,
        "ctxmsg",
        LogFlags::PRINT_LEVEL,
        true,
        128,
    );
    rows.insert(
        "format-line-context-level-line",
        escape_row_text(context_level.bytes()),
    );

    let (context_no_prefix, _) = rust_format_line_context(
        LogLevel::Warning,
        "nopfx",
        LogFlags::PRINT_LEVEL,
        false,
        128,
    );
    rows.insert(
        "format-line-context-noprefix-line",
        escape_row_text(context_no_prefix.bytes()),
    );

    let (context_newline, _) =
        rust_format_line_context(LogLevel::Info, "withnl\n", LogFlags::PRINT_LEVEL, true, 128);
    rows.insert(
        "format-line-context-newline-line",
        escape_row_text(context_newline.bytes()),
    );

    let (time_level, _) = rust_format_line(
        LogLevel::Warning,
        "plain",
        LogFlags::PRINT_TIME | LogFlags::PRINT_LEVEL,
        true,
        128,
    );
    rows.insert(
        "format-line-time-level-line",
        escape_row_text(time_level.bytes()),
    );

    rows
}

fn add_format_line2_int_rows(rows: &mut BTreeMap<&'static str, i32>) {
    let (plain, plain_prefix) =
        rust_format_line2(LogLevel::Warning, "plain", LogFlags::empty(), true, 128);
    rows.insert("format-line2-plain-ret", usize_to_i32(plain.full_len()));
    rows.insert("format-line2-plain-prefix", bool_to_i32(plain_prefix));
    rows.insert("format-line2-plain-len", usize_to_i32(plain.bytes().len()));

    let (level, level_prefix) =
        rust_format_line2(LogLevel::Warning, "plain", LogFlags::PRINT_LEVEL, true, 128);
    rows.insert("format-line2-level-ret", usize_to_i32(level.full_len()));
    rows.insert("format-line2-level-prefix", bool_to_i32(level_prefix));
    rows.insert("format-line2-level-len", usize_to_i32(level.bytes().len()));

    let (no_prefix, no_prefix_state) =
        rust_format_line2(LogLevel::Error, "after", LogFlags::PRINT_LEVEL, false, 128);
    rows.insert(
        "format-line2-noprefix-ret",
        usize_to_i32(no_prefix.full_len()),
    );
    rows.insert("format-line2-noprefix-prefix", bool_to_i32(no_prefix_state));
    rows.insert(
        "format-line2-noprefix-len",
        usize_to_i32(no_prefix.bytes().len()),
    );

    let (newline, newline_prefix) =
        rust_format_line2(LogLevel::Info, "withnl\n", LogFlags::PRINT_LEVEL, true, 128);
    rows.insert("format-line2-newline-ret", usize_to_i32(newline.full_len()));
    rows.insert("format-line2-newline-prefix", bool_to_i32(newline_prefix));
    rows.insert(
        "format-line2-newline-len",
        usize_to_i32(newline.bytes().len()),
    );

    let (small, small_prefix) =
        rust_format_line2(LogLevel::Warning, "plain", LogFlags::PRINT_LEVEL, true, 8);
    rows.insert("format-line2-small-ret", usize_to_i32(small.full_len()));
    rows.insert("format-line2-small-prefix", bool_to_i32(small_prefix));
    rows.insert("format-line2-small-len", usize_to_i32(small.bytes().len()));

    let (null_zero, null_zero_prefix) =
        rust_format_line2(LogLevel::Warning, "plain", LogFlags::PRINT_LEVEL, true, 0);
    rows.insert(
        "format-line2-nullzero-ret",
        usize_to_i32(null_zero.full_len()),
    );
    rows.insert(
        "format-line2-nullzero-prefix",
        bool_to_i32(null_zero_prefix),
    );

    let (size1, size1_prefix) =
        rust_format_line2(LogLevel::Warning, "plain", LogFlags::PRINT_LEVEL, true, 1);
    rows.insert("format-line2-size1-ret", usize_to_i32(size1.full_len()));
    rows.insert("format-line2-size1-prefix", bool_to_i32(size1_prefix));
    rows.insert("format-line2-size1-len", usize_to_i32(size1.bytes().len()));

    let (_, context_plain_prefix) =
        rust_format_line2_context(LogLevel::Warning, "ctxmsg", LogFlags::empty(), true, 128);
    rows.insert(
        "format-line2-context-plain-prefix",
        bool_to_i32(context_plain_prefix),
    );

    let (_, context_level_prefix) = rust_format_line2_context(
        LogLevel::Warning,
        "ctxmsg",
        LogFlags::PRINT_LEVEL,
        true,
        128,
    );
    rows.insert(
        "format-line2-context-level-prefix",
        bool_to_i32(context_level_prefix),
    );

    let (_, context_no_prefix_state) = rust_format_line2_context(
        LogLevel::Warning,
        "nopfx",
        LogFlags::PRINT_LEVEL,
        false,
        128,
    );
    rows.insert(
        "format-line2-context-noprefix-prefix",
        bool_to_i32(context_no_prefix_state),
    );

    let (_, context_newline_prefix) =
        rust_format_line2_context(LogLevel::Info, "withnl\n", LogFlags::PRINT_LEVEL, true, 128);
    rows.insert(
        "format-line2-context-newline-prefix",
        bool_to_i32(context_newline_prefix),
    );

    let (time, time_prefix) =
        rust_format_line2(LogLevel::Warning, "plain", LogFlags::PRINT_TIME, true, 128);
    rows.insert("format-line2-time-ret", usize_to_i32(time.full_len()));
    rows.insert("format-line2-time-prefix", bool_to_i32(time_prefix));
    rows.insert("format-line2-time-len", usize_to_i32(time.bytes().len()));

    let (datetime_level, datetime_level_prefix) = rust_format_line2(
        LogLevel::Warning,
        "plain",
        LogFlags::PRINT_DATETIME | LogFlags::PRINT_LEVEL,
        true,
        128,
    );
    rows.insert(
        "format-line2-datetime-level-ret",
        usize_to_i32(datetime_level.full_len()),
    );
    rows.insert(
        "format-line2-datetime-level-prefix",
        bool_to_i32(datetime_level_prefix),
    );
    rows.insert(
        "format-line2-datetime-level-len",
        usize_to_i32(datetime_level.bytes().len()),
    );

    let (both_level, both_level_prefix) = rust_format_line2(
        LogLevel::Warning,
        "plain",
        LogFlags::PRINT_TIME | LogFlags::PRINT_DATETIME | LogFlags::PRINT_LEVEL,
        true,
        128,
    );
    rows.insert(
        "format-line2-time-datetime-level-ret",
        usize_to_i32(both_level.full_len()),
    );
    rows.insert(
        "format-line2-time-datetime-level-prefix",
        bool_to_i32(both_level_prefix),
    );
    rows.insert(
        "format-line2-time-datetime-level-len",
        usize_to_i32(both_level.bytes().len()),
    );

    let (_, context_time_level_prefix) = rust_format_line2_context(
        LogLevel::Warning,
        "ctxmsg",
        LogFlags::PRINT_TIME | LogFlags::PRINT_LEVEL,
        true,
        128,
    );
    rows.insert(
        "format-line2-context-time-level-prefix",
        bool_to_i32(context_time_level_prefix),
    );
}

fn add_format_line_int_rows(rows: &mut BTreeMap<&'static str, i32>) {
    let (plain, plain_prefix) =
        rust_format_line(LogLevel::Warning, "plain", LogFlags::empty(), true, 128);
    rows.insert("format-line-plain-prefix", bool_to_i32(plain_prefix));
    rows.insert("format-line-plain-len", usize_to_i32(plain.bytes().len()));

    let (level, level_prefix) =
        rust_format_line(LogLevel::Warning, "plain", LogFlags::PRINT_LEVEL, true, 128);
    rows.insert("format-line-level-prefix", bool_to_i32(level_prefix));
    rows.insert("format-line-level-len", usize_to_i32(level.bytes().len()));

    let (no_prefix, no_prefix_state) =
        rust_format_line(LogLevel::Error, "after", LogFlags::PRINT_LEVEL, false, 128);
    rows.insert("format-line-noprefix-prefix", bool_to_i32(no_prefix_state));
    rows.insert(
        "format-line-noprefix-len",
        usize_to_i32(no_prefix.bytes().len()),
    );

    let (newline, newline_prefix) =
        rust_format_line(LogLevel::Info, "withnl\n", LogFlags::PRINT_LEVEL, true, 128);
    rows.insert("format-line-newline-prefix", bool_to_i32(newline_prefix));
    rows.insert(
        "format-line-newline-len",
        usize_to_i32(newline.bytes().len()),
    );

    let (small, small_prefix) =
        rust_format_line(LogLevel::Warning, "plain", LogFlags::PRINT_LEVEL, true, 8);
    rows.insert("format-line-small-prefix", bool_to_i32(small_prefix));
    rows.insert("format-line-small-len", usize_to_i32(small.bytes().len()));

    let (size1, size1_prefix) =
        rust_format_line(LogLevel::Warning, "plain", LogFlags::PRINT_LEVEL, true, 1);
    rows.insert("format-line-size1-prefix", bool_to_i32(size1_prefix));
    rows.insert("format-line-size1-len", usize_to_i32(size1.bytes().len()));

    let (_, context_level_prefix) = rust_format_line_context(
        LogLevel::Warning,
        "ctxmsg",
        LogFlags::PRINT_LEVEL,
        true,
        128,
    );
    rows.insert(
        "format-line-context-level-prefix",
        bool_to_i32(context_level_prefix),
    );

    let (_, context_no_prefix_state) = rust_format_line_context(
        LogLevel::Warning,
        "nopfx",
        LogFlags::PRINT_LEVEL,
        false,
        128,
    );
    rows.insert(
        "format-line-context-noprefix-prefix",
        bool_to_i32(context_no_prefix_state),
    );

    let (_, context_newline_prefix) =
        rust_format_line_context(LogLevel::Info, "withnl\n", LogFlags::PRINT_LEVEL, true, 128);
    rows.insert(
        "format-line-context-newline-prefix",
        bool_to_i32(context_newline_prefix),
    );

    let (time_level, time_level_prefix) = rust_format_line(
        LogLevel::Warning,
        "plain",
        LogFlags::PRINT_TIME | LogFlags::PRINT_LEVEL,
        true,
        128,
    );
    rows.insert(
        "format-line-time-level-prefix",
        bool_to_i32(time_level_prefix),
    );
    rows.insert(
        "format-line-time-level-len",
        usize_to_i32(time_level.bytes().len()),
    );
}

fn rust_format_line(
    level: LogLevel,
    message: &str,
    flags: LogFlags,
    initial_prefix: bool,
    line_size: usize,
) -> (AvLogFormatLine, bool) {
    let mut prefix = initial_prefix;
    let line = LogRecord::new(level, "ignored", message)
        .format_av_log_line_null_context(flags, &mut prefix, line_size)
        .expect("bounded av_log_format_line model should support this flag shape");
    (line, prefix)
}

fn rust_format_line_context(
    level: LogLevel,
    message: &str,
    flags: LogFlags,
    initial_prefix: bool,
    line_size: usize,
) -> (AvLogFormatLine, bool) {
    let context = AvLogContextPrefix::new("rustctx", "<ptr>");
    let mut prefix = initial_prefix;
    let line = LogRecord::new(level, "ignored", message)
        .format_av_log_line_context(&context, flags, &mut prefix, line_size)
        .expect("bounded av_log_format_line context model should support this flag shape");
    (line, prefix)
}

fn rust_format_line2(
    level: LogLevel,
    message: &str,
    flags: LogFlags,
    initial_prefix: bool,
    line_size: usize,
) -> (AvLogFormatLine2, bool) {
    let mut prefix = initial_prefix;
    let line = LogRecord::new(level, "ignored", message)
        .format_av_log_line2_null_context(flags, &mut prefix, line_size)
        .expect("bounded av_log_format_line2 model should support this flag shape");
    (line, prefix)
}

fn rust_format_line2_context(
    level: LogLevel,
    message: &str,
    flags: LogFlags,
    initial_prefix: bool,
    line_size: usize,
) -> (AvLogFormatLine2, bool) {
    let context = AvLogContextPrefix::new("rustctx", "<ptr>");
    let mut prefix = initial_prefix;
    let line = LogRecord::new(level, "ignored", message)
        .format_av_log_line2_context(&context, flags, &mut prefix, line_size)
        .expect("bounded av_log_format_line2 context model should support this flag shape");
    (line, prefix)
}

fn rust_default_callback_line(flags: LogFlags, timestamp: Option<LogTimestamp>) -> String {
    let mut record = LogRecord::new(LogLevel::Warning, "ignored", "plain\n");
    if let Some(timestamp) = timestamp {
        record = record.with_timestamp(timestamp);
    }
    record.format_default_callback_line_null_context_with_flags(flags)
}

fn normalize_default_callback_timestamp(line: &str) -> String {
    if is_default_datetime_prefix(line) {
        format!("<datetime> {}", &line[24..])
    } else if is_default_time_prefix(line) {
        format!("<time> {}", &line[13..])
    } else {
        line.to_owned()
    }
}

fn is_default_time_prefix(line: &str) -> bool {
    let bytes = line.as_bytes();
    bytes.len() >= 13
        && bytes[0].is_ascii_digit()
        && bytes[1].is_ascii_digit()
        && bytes[2] == b':'
        && bytes[3].is_ascii_digit()
        && bytes[4].is_ascii_digit()
        && bytes[5] == b':'
        && bytes[6].is_ascii_digit()
        && bytes[7].is_ascii_digit()
        && bytes[8] == b'.'
        && bytes[9].is_ascii_digit()
        && bytes[10].is_ascii_digit()
        && bytes[11].is_ascii_digit()
        && bytes[12] == b' '
}

fn is_default_datetime_prefix(line: &str) -> bool {
    let bytes = line.as_bytes();
    bytes.len() >= 24
        && bytes[0].is_ascii_digit()
        && bytes[1].is_ascii_digit()
        && bytes[2].is_ascii_digit()
        && bytes[3].is_ascii_digit()
        && bytes[4] == b'-'
        && bytes[5].is_ascii_digit()
        && bytes[6].is_ascii_digit()
        && bytes[7] == b'-'
        && bytes[8].is_ascii_digit()
        && bytes[9].is_ascii_digit()
        && bytes[10] == b' '
        && bytes[11].is_ascii_digit()
        && bytes[12].is_ascii_digit()
        && bytes[13] == b':'
        && bytes[14].is_ascii_digit()
        && bytes[15].is_ascii_digit()
        && bytes[16] == b':'
        && bytes[17].is_ascii_digit()
        && bytes[18].is_ascii_digit()
        && bytes[19] == b'.'
        && bytes[20].is_ascii_digit()
        && bytes[21].is_ascii_digit()
        && bytes[22].is_ascii_digit()
        && bytes[23] == b' '
}

fn bool_to_i32(value: bool) -> i32 {
    if value {
        1
    } else {
        0
    }
}

fn usize_to_i32(value: usize) -> i32 {
    i32::try_from(value).expect("oracle row should fit in i32")
}

fn escape_row_text(bytes: &[u8]) -> String {
    let mut escaped = String::new();
    for &byte in bytes {
        match byte {
            b'\n' => escaped.push_str("\\n"),
            b'\r' => escaped.push_str("\\r"),
            b'\t' => escaped.push_str("\\t"),
            b'\\' => escaped.push_str("\\\\"),
            b'|' => escaped.push_str("\\x7c"),
            0x20..=0x7e => escaped.push(char::from(byte)),
            _ => escaped.push_str(&format!("\\x{byte:02x}")),
        }
    }
    escaped
}

fn parse_oracle_output(stdout: &str) -> BTreeMap<String, String> {
    let mut rows = BTreeMap::new();
    for line in stdout.lines() {
        let mut parts = line.splitn(2, '|');
        let name = parts.next().expect("row name").to_string();
        let value = parts
            .next()
            .unwrap_or_else(|| panic!("missing value in oracle row `{line}`"))
            .to_string();
        assert!(
            rows.insert(name, value).is_none(),
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
            .expect("run WSL libavutil logging oracle")
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
            .expect("run libavutil logging oracle")
    };

    assert!(
        output.status.success(),
        "libavutil logging oracle failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout).expect("oracle stdout should be UTF-8")
}

fn oracle_c_source() -> &'static str {
    r#"#include <stdarg.h>
#include <ctype.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <libavutil/log.h>
#include <libavutil/version.h>

#define ROW(name, value) printf("%s|%d\n", name, (int)(value))

static void ROW_STR(const char *name, const char *value) {
    printf("%s|", name);
    if (!value) {
        printf("<null>\n");
        return;
    }
    for (const unsigned char *p = (const unsigned char *)value; *p; p++) {
        if (*p == '\n')
            printf("\\n");
        else if (*p == '\r')
            printf("\\r");
        else if (*p == '\t')
            printf("\\t");
        else if (*p == '\\')
            printf("\\\\");
        else if (*p == '|')
            printf("\\x7c");
        else if (*p < 32 || *p > 126)
            printf("\\x%02x", *p);
        else
            putchar(*p);
    }
    putchar('\n');
}

static void ROW_STR_NORMALIZED_CONTEXT(const char *name, const char *value) {
    char normalized[512];
    const char *marker = value ? strstr(value, " @ 0x") : NULL;
    if (!marker) {
        ROW_STR(name, value);
        return;
    }
    const char *ptr_end = marker + 3;
    while (*ptr_end && *ptr_end != ']')
        ptr_end++;
    size_t head_len = (size_t)(marker - value);
    snprintf(normalized, sizeof(normalized), "%.*s @ <ptr>%s",
             (int)head_len, value, ptr_end);
    ROW_STR(name, normalized);
}

static int has_default_time_prefix(const char *value) {
    return value && strlen(value) >= 13 &&
           isdigit((unsigned char)value[0]) &&
           isdigit((unsigned char)value[1]) &&
           value[2] == ':' &&
           isdigit((unsigned char)value[3]) &&
           isdigit((unsigned char)value[4]) &&
           value[5] == ':' &&
           isdigit((unsigned char)value[6]) &&
           isdigit((unsigned char)value[7]) &&
           value[8] == '.' &&
           isdigit((unsigned char)value[9]) &&
           isdigit((unsigned char)value[10]) &&
           isdigit((unsigned char)value[11]) &&
           value[12] == ' ';
}

static int has_default_datetime_prefix(const char *value) {
    return value && strlen(value) >= 24 &&
           isdigit((unsigned char)value[0]) &&
           isdigit((unsigned char)value[1]) &&
           isdigit((unsigned char)value[2]) &&
           isdigit((unsigned char)value[3]) &&
           value[4] == '-' &&
           isdigit((unsigned char)value[5]) &&
           isdigit((unsigned char)value[6]) &&
           value[7] == '-' &&
           isdigit((unsigned char)value[8]) &&
           isdigit((unsigned char)value[9]) &&
           value[10] == ' ' &&
           isdigit((unsigned char)value[11]) &&
           isdigit((unsigned char)value[12]) &&
           value[13] == ':' &&
           isdigit((unsigned char)value[14]) &&
           isdigit((unsigned char)value[15]) &&
           value[16] == ':' &&
           isdigit((unsigned char)value[17]) &&
           isdigit((unsigned char)value[18]) &&
           value[19] == '.' &&
           isdigit((unsigned char)value[20]) &&
           isdigit((unsigned char)value[21]) &&
           isdigit((unsigned char)value[22]) &&
           value[23] == ' ';
}

static void ROW_STR_NORMALIZED_DEFAULT_TIMESTAMP(const char *name, const char *value) {
    char normalized[1024];
    if (has_default_datetime_prefix(value)) {
        snprintf(normalized, sizeof(normalized), "<datetime> %s", value + 24);
        ROW_STR(name, normalized);
    } else if (has_default_time_prefix(value)) {
        snprintf(normalized, sizeof(normalized), "<time> %s", value + 13);
        ROW_STR(name, normalized);
    } else {
        ROW_STR(name, value);
    }
}

typedef struct TestLogContext {
    const AVClass *av_class;
} TestLogContext;

static const AVClass test_log_class = {
    .class_name = "rustctx",
    .item_name = av_default_item_name,
    .version = LIBAVUTIL_VERSION_INT,
};

static int call_format_line2(void *ptr, char *line, int line_size, int *print_prefix,
                             int level, const char *fmt, ...) {
    va_list vl;
    va_start(vl, fmt);
    int ret = av_log_format_line2(ptr, level, fmt, vl, line, line_size,
                                  print_prefix);
    va_end(vl);
    return ret;
}

static void call_format_line(void *ptr, char *line, int line_size, int *print_prefix,
                             int level, const char *fmt, ...) {
    va_list vl;
    va_start(vl, fmt);
    av_log_format_line(ptr, level, fmt, vl, line, line_size, print_prefix);
    va_end(vl);
}

static int captured_count = 0;
static int captured_level = -999;

static void capture_log_callback(void *ptr, int level, const char *fmt, va_list vl) {
    (void)ptr;
    (void)fmt;
    (void)vl;
    captured_count++;
    captured_level = level;
}

static void print_level_after_set(const char *name, int level) {
    av_log_set_level(level);
    ROW(name, av_log_get_level());
}

static void print_flags_after_set(const char *name, int flags) {
    av_log_set_flags(flags);
    ROW(name, av_log_get_flags());
}

static void print_format_line2_rows(void) {
    char line[128];
    int print_prefix;
    int ret;

    av_log_set_flags(0);
    memset(line, 'X', sizeof(line));
    print_prefix = 1;
    ret = call_format_line2(NULL, line, sizeof(line), &print_prefix,
                            AV_LOG_WARNING, "%s", "plain");
    ROW("format-line2-plain-ret", ret);
    ROW("format-line2-plain-prefix", print_prefix);
    ROW("format-line2-plain-len", strlen(line));
    ROW_STR("format-line2-plain-line", line);

    av_log_set_flags(AV_LOG_PRINT_LEVEL);
    memset(line, 'X', sizeof(line));
    print_prefix = 1;
    ret = call_format_line2(NULL, line, sizeof(line), &print_prefix,
                            AV_LOG_WARNING, "%s", "plain");
    ROW("format-line2-level-ret", ret);
    ROW("format-line2-level-prefix", print_prefix);
    ROW("format-line2-level-len", strlen(line));
    ROW_STR("format-line2-level-line", line);

    memset(line, 'X', sizeof(line));
    print_prefix = 0;
    ret = call_format_line2(NULL, line, sizeof(line), &print_prefix,
                            AV_LOG_ERROR, "%s", "after");
    ROW("format-line2-noprefix-ret", ret);
    ROW("format-line2-noprefix-prefix", print_prefix);
    ROW("format-line2-noprefix-len", strlen(line));
    ROW_STR("format-line2-noprefix-line", line);

    memset(line, 'X', sizeof(line));
    print_prefix = 1;
    ret = call_format_line2(NULL, line, sizeof(line), &print_prefix,
                            AV_LOG_INFO, "%s\n", "withnl");
    ROW("format-line2-newline-ret", ret);
    ROW("format-line2-newline-prefix", print_prefix);
    ROW("format-line2-newline-len", strlen(line));
    ROW_STR("format-line2-newline-line", line);

    char small[8];
    memset(small, 'X', sizeof(small));
    print_prefix = 1;
    ret = call_format_line2(NULL, small, sizeof(small), &print_prefix,
                            AV_LOG_WARNING, "%s", "plain");
    ROW("format-line2-small-ret", ret);
    ROW("format-line2-small-prefix", print_prefix);
    ROW("format-line2-small-len", strlen(small));
    ROW_STR("format-line2-small-line", small);

    print_prefix = 1;
    ret = call_format_line2(NULL, NULL, 0, &print_prefix,
                            AV_LOG_WARNING, "%s", "plain");
    ROW("format-line2-nullzero-ret", ret);
    ROW("format-line2-nullzero-prefix", print_prefix);

    memset(line, 'X', sizeof(line));
    print_prefix = 1;
    ret = call_format_line2(NULL, line, 1, &print_prefix,
                            AV_LOG_WARNING, "%s", "plain");
    ROW("format-line2-size1-ret", ret);
    ROW("format-line2-size1-prefix", print_prefix);
    ROW("format-line2-size1-len", strlen(line));
    ROW_STR("format-line2-size1-line", line);

    av_log_set_flags(AV_LOG_PRINT_TIME);
    memset(line, 'X', sizeof(line));
    print_prefix = 1;
    ret = call_format_line2(NULL, line, sizeof(line), &print_prefix,
                            AV_LOG_WARNING, "%s", "plain");
    ROW("format-line2-time-ret", ret);
    ROW("format-line2-time-prefix", print_prefix);
    ROW("format-line2-time-len", strlen(line));
    ROW_STR("format-line2-time-line", line);

    av_log_set_flags(AV_LOG_PRINT_DATETIME | AV_LOG_PRINT_LEVEL);
    memset(line, 'X', sizeof(line));
    print_prefix = 1;
    ret = call_format_line2(NULL, line, sizeof(line), &print_prefix,
                            AV_LOG_WARNING, "%s", "plain");
    ROW("format-line2-datetime-level-ret", ret);
    ROW("format-line2-datetime-level-prefix", print_prefix);
    ROW("format-line2-datetime-level-len", strlen(line));
    ROW_STR("format-line2-datetime-level-line", line);

    av_log_set_flags(AV_LOG_PRINT_TIME | AV_LOG_PRINT_DATETIME |
                     AV_LOG_PRINT_LEVEL);
    memset(line, 'X', sizeof(line));
    print_prefix = 1;
    ret = call_format_line2(NULL, line, sizeof(line), &print_prefix,
                            AV_LOG_WARNING, "%s", "plain");
    ROW("format-line2-time-datetime-level-ret", ret);
    ROW("format-line2-time-datetime-level-prefix", print_prefix);
    ROW("format-line2-time-datetime-level-len", strlen(line));
    ROW_STR("format-line2-time-datetime-level-line", line);

    TestLogContext ctx = { &test_log_class };

    av_log_set_flags(0);
    memset(line, 'X', sizeof(line));
    print_prefix = 1;
    ret = call_format_line2(&ctx, line, sizeof(line), &print_prefix,
                            AV_LOG_WARNING, "%s", "ctxmsg");
    (void)ret;
    ROW("format-line2-context-plain-prefix", print_prefix);
    ROW_STR_NORMALIZED_CONTEXT("format-line2-context-plain-line", line);

    av_log_set_flags(AV_LOG_PRINT_LEVEL);
    memset(line, 'X', sizeof(line));
    print_prefix = 1;
    ret = call_format_line2(&ctx, line, sizeof(line), &print_prefix,
                            AV_LOG_WARNING, "%s", "ctxmsg");
    (void)ret;
    ROW("format-line2-context-level-prefix", print_prefix);
    ROW_STR_NORMALIZED_CONTEXT("format-line2-context-level-line", line);

    memset(line, 'X', sizeof(line));
    print_prefix = 0;
    ret = call_format_line2(&ctx, line, sizeof(line), &print_prefix,
                            AV_LOG_WARNING, "%s", "nopfx");
    (void)ret;
    ROW("format-line2-context-noprefix-prefix", print_prefix);
    ROW_STR_NORMALIZED_CONTEXT("format-line2-context-noprefix-line", line);

    memset(line, 'X', sizeof(line));
    print_prefix = 1;
    ret = call_format_line2(&ctx, line, sizeof(line), &print_prefix,
                            AV_LOG_INFO, "%s\n", "withnl");
    (void)ret;
    ROW("format-line2-context-newline-prefix", print_prefix);
    ROW_STR_NORMALIZED_CONTEXT("format-line2-context-newline-line", line);

    av_log_set_flags(AV_LOG_PRINT_TIME | AV_LOG_PRINT_LEVEL);
    memset(line, 'X', sizeof(line));
    print_prefix = 1;
    ret = call_format_line2(&ctx, line, sizeof(line), &print_prefix,
                            AV_LOG_WARNING, "%s", "ctxmsg");
    (void)ret;
    ROW("format-line2-context-time-level-prefix", print_prefix);
    ROW_STR_NORMALIZED_CONTEXT("format-line2-context-time-level-line", line);
}

static void print_format_line_rows(void) {
    char line[128];
    int print_prefix;

    av_log_set_flags(0);
    memset(line, 'X', sizeof(line));
    print_prefix = 1;
    call_format_line(NULL, line, sizeof(line), &print_prefix,
                     AV_LOG_WARNING, "%s", "plain");
    ROW("format-line-plain-prefix", print_prefix);
    ROW("format-line-plain-len", strlen(line));
    ROW_STR("format-line-plain-line", line);

    av_log_set_flags(AV_LOG_PRINT_LEVEL);
    memset(line, 'X', sizeof(line));
    print_prefix = 1;
    call_format_line(NULL, line, sizeof(line), &print_prefix,
                     AV_LOG_WARNING, "%s", "plain");
    ROW("format-line-level-prefix", print_prefix);
    ROW("format-line-level-len", strlen(line));
    ROW_STR("format-line-level-line", line);

    memset(line, 'X', sizeof(line));
    print_prefix = 0;
    call_format_line(NULL, line, sizeof(line), &print_prefix,
                     AV_LOG_ERROR, "%s", "after");
    ROW("format-line-noprefix-prefix", print_prefix);
    ROW("format-line-noprefix-len", strlen(line));
    ROW_STR("format-line-noprefix-line", line);

    memset(line, 'X', sizeof(line));
    print_prefix = 1;
    call_format_line(NULL, line, sizeof(line), &print_prefix,
                     AV_LOG_INFO, "%s\n", "withnl");
    ROW("format-line-newline-prefix", print_prefix);
    ROW("format-line-newline-len", strlen(line));
    ROW_STR("format-line-newline-line", line);

    char small[8];
    memset(small, 'X', sizeof(small));
    print_prefix = 1;
    call_format_line(NULL, small, sizeof(small), &print_prefix,
                     AV_LOG_WARNING, "%s", "plain");
    ROW("format-line-small-prefix", print_prefix);
    ROW("format-line-small-len", strlen(small));
    ROW_STR("format-line-small-line", small);

    memset(line, 'X', sizeof(line));
    print_prefix = 1;
    call_format_line(NULL, line, 1, &print_prefix,
                     AV_LOG_WARNING, "%s", "plain");
    ROW("format-line-size1-prefix", print_prefix);
    ROW("format-line-size1-len", strlen(line));
    ROW_STR("format-line-size1-line", line);

    TestLogContext ctx = { &test_log_class };

    av_log_set_flags(AV_LOG_PRINT_LEVEL);
    memset(line, 'X', sizeof(line));
    print_prefix = 1;
    call_format_line(&ctx, line, sizeof(line), &print_prefix,
                     AV_LOG_WARNING, "%s", "ctxmsg");
    ROW("format-line-context-level-prefix", print_prefix);
    ROW_STR_NORMALIZED_CONTEXT("format-line-context-level-line", line);

    memset(line, 'X', sizeof(line));
    print_prefix = 0;
    call_format_line(&ctx, line, sizeof(line), &print_prefix,
                     AV_LOG_WARNING, "%s", "nopfx");
    ROW("format-line-context-noprefix-prefix", print_prefix);
    ROW_STR_NORMALIZED_CONTEXT("format-line-context-noprefix-line", line);

    memset(line, 'X', sizeof(line));
    print_prefix = 1;
    call_format_line(&ctx, line, sizeof(line), &print_prefix,
                     AV_LOG_INFO, "%s\n", "withnl");
    ROW("format-line-context-newline-prefix", print_prefix);
    ROW_STR_NORMALIZED_CONTEXT("format-line-context-newline-line", line);

    av_log_set_flags(AV_LOG_PRINT_TIME | AV_LOG_PRINT_LEVEL);
    memset(line, 'X', sizeof(line));
    print_prefix = 1;
    call_format_line(NULL, line, sizeof(line), &print_prefix,
                     AV_LOG_WARNING, "%s", "plain");
    ROW("format-line-time-level-prefix", print_prefix);
    ROW("format-line-time-level-len", strlen(line));
    ROW_STR("format-line-time-level-line", line);
}

static void print_default_callback_row(const char *name, int flags) {
    char captured[1024];
    FILE *capture = tmpfile();
    if (!capture) {
        ROW_STR(name, "<tmpfile-error>");
        return;
    }

    fflush(stderr);
    int saved_stderr = dup(fileno(stderr));
    if (saved_stderr < 0 || dup2(fileno(capture), fileno(stderr)) < 0) {
        if (saved_stderr >= 0)
            close(saved_stderr);
        fclose(capture);
        ROW_STR(name, "<stderr-redirect-error>");
        return;
    }

    av_log_set_callback(av_log_default_callback);
    av_log_set_level(AV_LOG_TRACE);
    av_log_set_flags(flags);
    av_log(NULL, AV_LOG_WARNING, "%s\n", "plain");
    fflush(stderr);

    dup2(saved_stderr, fileno(stderr));
    close(saved_stderr);

    rewind(capture);
    size_t len = fread(captured, 1, sizeof(captured) - 1, capture);
    captured[len] = '\0';
    fclose(capture);

    ROW_STR_NORMALIZED_DEFAULT_TIMESTAMP(name, captured);
}

static void print_default_callback_rows(void) {
    setenv("AV_LOG_FORCE_NOCOLOR", "1", 1);
    print_default_callback_row("default-callback-time-line", AV_LOG_PRINT_TIME);
    print_default_callback_row("default-callback-time-level-line",
                               AV_LOG_PRINT_TIME | AV_LOG_PRINT_LEVEL);
    print_default_callback_row("default-callback-datetime-line", AV_LOG_PRINT_DATETIME);
    print_default_callback_row("default-callback-datetime-level-line",
                               AV_LOG_PRINT_DATETIME | AV_LOG_PRINT_LEVEL);
    print_default_callback_row("default-callback-time-datetime-level-line",
                               AV_LOG_PRINT_TIME | AV_LOG_PRINT_DATETIME |
                               AV_LOG_PRINT_LEVEL);
}

int main(void) {
    ROW("AV_LOG_QUIET", AV_LOG_QUIET);
    ROW("AV_LOG_PANIC", AV_LOG_PANIC);
    ROW("AV_LOG_FATAL", AV_LOG_FATAL);
    ROW("AV_LOG_ERROR", AV_LOG_ERROR);
    ROW("AV_LOG_WARNING", AV_LOG_WARNING);
    ROW("AV_LOG_INFO", AV_LOG_INFO);
    ROW("AV_LOG_VERBOSE", AV_LOG_VERBOSE);
    ROW("AV_LOG_DEBUG", AV_LOG_DEBUG);
    ROW("AV_LOG_TRACE", AV_LOG_TRACE);
    ROW("AV_LOG_MAX_OFFSET", AV_LOG_MAX_OFFSET);
    ROW("AV_LOG_SKIP_REPEATED", AV_LOG_SKIP_REPEATED);
    ROW("AV_LOG_PRINT_LEVEL", AV_LOG_PRINT_LEVEL);
    ROW("AV_LOG_PRINT_TIME", AV_LOG_PRINT_TIME);
    ROW("AV_LOG_PRINT_DATETIME", AV_LOG_PRINT_DATETIME);

    ROW("default-level", av_log_get_level());
    print_level_after_set("set-level-quiet", AV_LOG_QUIET);
    print_level_after_set("set-level-panic", AV_LOG_PANIC);
    print_level_after_set("set-level-fatal", AV_LOG_FATAL);
    print_level_after_set("set-level-error", AV_LOG_ERROR);
    print_level_after_set("set-level-warning", AV_LOG_WARNING);
    print_level_after_set("set-level-info", AV_LOG_INFO);
    print_level_after_set("set-level-verbose", AV_LOG_VERBOSE);
    print_level_after_set("set-level-debug", AV_LOG_DEBUG);
    print_level_after_set("set-level-trace", AV_LOG_TRACE);

    print_flags_after_set("set-flags-empty", 0);
    print_flags_after_set("set-flags-skip-repeated", AV_LOG_SKIP_REPEATED);
    print_flags_after_set("set-flags-print-level", AV_LOG_PRINT_LEVEL);
    print_flags_after_set("set-flags-print-time", AV_LOG_PRINT_TIME);
    print_flags_after_set("set-flags-print-datetime", AV_LOG_PRINT_DATETIME);
    print_flags_after_set("set-flags-all-known",
                          AV_LOG_SKIP_REPEATED | AV_LOG_PRINT_LEVEL |
                          AV_LOG_PRINT_TIME | AV_LOG_PRINT_DATETIME);
    print_format_line2_rows();
    print_format_line_rows();
    print_default_callback_rows();
    av_log_set_callback(capture_log_callback);
    int once_state = 0;
    av_log_once(NULL, AV_LOG_WARNING, AV_LOG_DEBUG, &once_state, "%s", "once");
    ROW("log-once-first-state", once_state);
    ROW("log-once-first-count", captured_count);
    ROW("log-once-first-level", captured_level);
    av_log_once(NULL, AV_LOG_WARNING, AV_LOG_DEBUG, &once_state, "%s", "once");
    ROW("log-once-second-state", once_state);
    ROW("log-once-second-count", captured_count);
    ROW("log-once-second-level", captured_level);
    int preseed_state = 7;
    av_log_once(NULL, AV_LOG_INFO, AV_LOG_ERROR, &preseed_state, "%s", "preseed");
    ROW("log-once-preseed-state", preseed_state);
    ROW("log-once-preseed-count", captured_count);
    ROW("log-once-preseed-level", captured_level);
    av_log_set_callback(av_log_default_callback);
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
