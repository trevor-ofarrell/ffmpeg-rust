use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Quiet,
    Panic,
    Fatal,
    Error,
    Warning,
    Info,
    Verbose,
    Debug,
    Trace,
}

impl LogLevel {
    pub const fn as_ffmpeg_value(self) -> i32 {
        match self {
            Self::Quiet => -8,
            Self::Panic => 0,
            Self::Fatal => 8,
            Self::Error => 16,
            Self::Warning => 24,
            Self::Info => 32,
            Self::Verbose => 40,
            Self::Debug => 48,
            Self::Trace => 56,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Quiet => "quiet",
            Self::Panic => "panic",
            Self::Fatal => "fatal",
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Info => "info",
            Self::Verbose => "verbose",
            Self::Debug => "debug",
            Self::Trace => "trace",
        }
    }

    pub const fn from_ffmpeg_value(value: i32) -> Option<Self> {
        match value {
            -8 => Some(Self::Quiet),
            0 => Some(Self::Panic),
            8 => Some(Self::Fatal),
            16 => Some(Self::Error),
            24 => Some(Self::Warning),
            32 => Some(Self::Info),
            40 => Some(Self::Verbose),
            48 => Some(Self::Debug),
            56 => Some(Self::Trace),
            _ => None,
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        if name.eq_ignore_ascii_case(Self::Quiet.name()) {
            Some(Self::Quiet)
        } else if name.eq_ignore_ascii_case(Self::Panic.name()) {
            Some(Self::Panic)
        } else if name.eq_ignore_ascii_case(Self::Fatal.name()) {
            Some(Self::Fatal)
        } else if name.eq_ignore_ascii_case(Self::Error.name()) {
            Some(Self::Error)
        } else if name.eq_ignore_ascii_case(Self::Warning.name()) {
            Some(Self::Warning)
        } else if name.eq_ignore_ascii_case(Self::Info.name()) {
            Some(Self::Info)
        } else if name.eq_ignore_ascii_case(Self::Verbose.name()) {
            Some(Self::Verbose)
        } else if name.eq_ignore_ascii_case(Self::Debug.name()) {
            Some(Self::Debug)
        } else if name.eq_ignore_ascii_case(Self::Trace.name()) {
            Some(Self::Trace)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LogFlags {
    bits: u32,
}

impl LogFlags {
    pub const SKIP_REPEATED: Self = Self { bits: 0x0001 };
    pub const PRINT_LEVEL: Self = Self { bits: 0x0002 };
    pub const PRINT_TIME: Self = Self { bits: 0x0004 };
    pub const PRINT_DATETIME: Self = Self { bits: 0x0008 };

    const KNOWN_BITS: u32 = Self::SKIP_REPEATED.bits
        | Self::PRINT_LEVEL.bits
        | Self::PRINT_TIME.bits
        | Self::PRINT_DATETIME.bits;

    pub const fn empty() -> Self {
        Self { bits: 0 }
    }

    pub const fn all() -> Self {
        Self {
            bits: Self::KNOWN_BITS,
        }
    }

    pub const fn from_bits_truncate(bits: u32) -> Self {
        Self {
            bits: bits & Self::KNOWN_BITS,
        }
    }

    pub const fn bits(self) -> u32 {
        self.bits
    }

    pub const fn is_empty(self) -> bool {
        self.bits == 0
    }

    pub const fn contains(self, other: Self) -> bool {
        (self.bits & other.bits) == other.bits
    }

    pub const fn intersects(self, other: Self) -> bool {
        (self.bits & other.bits) != 0
    }

    pub fn insert(&mut self, other: Self) {
        self.bits |= other.bits;
        self.bits &= Self::KNOWN_BITS;
    }

    pub fn remove(&mut self, other: Self) {
        self.bits &= !other.bits;
    }

    pub fn set(&mut self, other: Self, enabled: bool) {
        if enabled {
            self.insert(other);
        } else {
            self.remove(other);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogColorMode {
    #[default]
    Never,
    Always,
}

pub const AV_LOG_FORCE_COLOR_ENV: &str = "AV_LOG_FORCE_COLOR";
pub const AV_LOG_FORCE_NOCOLOR_ENV: &str = "AV_LOG_FORCE_NOCOLOR";

impl LogColorMode {
    pub fn from_ffmpeg_env() -> Self {
        Self::from_ffmpeg_env_vars(|name| std::env::var_os(name).is_some())
    }

    pub fn from_ffmpeg_env_vars(mut is_set: impl FnMut(&str) -> bool) -> Self {
        if is_set(AV_LOG_FORCE_NOCOLOR_ENV) {
            Self::Never
        } else if is_set(AV_LOG_FORCE_COLOR_ENV) {
            Self::Always
        } else {
            Self::Never
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LogFormatOptions {
    flags: LogFlags,
    color_mode: LogColorMode,
}

impl Default for LogFormatOptions {
    fn default() -> Self {
        Self::new(LogFlags::PRINT_LEVEL)
    }
}

impl LogFormatOptions {
    pub const fn new(flags: LogFlags) -> Self {
        Self {
            flags,
            color_mode: LogColorMode::Never,
        }
    }

    pub const fn with_color_mode(self, color_mode: LogColorMode) -> Self {
        Self {
            flags: self.flags,
            color_mode,
        }
    }

    pub fn with_ffmpeg_env_color(self) -> Self {
        self.with_color_mode(LogColorMode::from_ffmpeg_env())
    }

    pub fn with_ffmpeg_env_color_vars(self, is_set: impl FnMut(&str) -> bool) -> Self {
        self.with_color_mode(LogColorMode::from_ffmpeg_env_vars(is_set))
    }

    pub const fn flags(self) -> LogFlags {
        self.flags
    }

    pub const fn color_mode(self) -> LogColorMode {
        self.color_mode
    }
}

impl From<LogFlags> for LogFormatOptions {
    fn from(flags: LogFlags) -> Self {
        Self::new(flags)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LogTimestamp {
    unix_micros: i64,
}

impl LogTimestamp {
    pub const fn from_unix_micros(unix_micros: i64) -> Self {
        Self { unix_micros }
    }

    pub fn from_system_time(time: SystemTime) -> Option<Self> {
        match time.duration_since(UNIX_EPOCH) {
            Ok(duration) => duration_to_unix_micros(duration),
            Err(error) => duration_before_epoch_to_unix_micros(error.duration()),
        }
        .map(Self::from_unix_micros)
    }

    pub fn now_utc() -> Option<Self> {
        Self::from_system_time(SystemTime::now())
    }

    pub const fn unix_micros(self) -> i64 {
        self.unix_micros
    }

    pub fn format_time_utc(self) -> String {
        let (_, _, _, hour, minute, second, micros) = self.parts_utc();
        format!("{hour:02}:{minute:02}:{second:02}.{micros:06}")
    }

    pub fn format_datetime_utc(self) -> String {
        let (year, month, day, hour, minute, second, micros) = self.parts_utc();
        format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02}.{micros:06}")
    }

    fn parts_utc(self) -> (i64, i64, i64, i64, i64, i64, i64) {
        const MICROS_PER_SECOND: i64 = 1_000_000;
        const SECONDS_PER_DAY: i64 = 86_400;

        let seconds = self.unix_micros.div_euclid(MICROS_PER_SECOND);
        let micros = self.unix_micros.rem_euclid(MICROS_PER_SECOND);
        let days = seconds.div_euclid(SECONDS_PER_DAY);
        let seconds_of_day = seconds.rem_euclid(SECONDS_PER_DAY);
        let hour = seconds_of_day / 3_600;
        let minute = (seconds_of_day % 3_600) / 60;
        let second = seconds_of_day % 60;
        let (year, month, day) = civil_from_unix_days(days);

        (year, month, day, hour, minute, second, micros)
    }
}

fn duration_to_unix_micros(duration: Duration) -> Option<i64> {
    let micros = i128::from(duration.as_secs())
        .checked_mul(1_000_000)?
        .checked_add(i128::from(duration.subsec_micros()))?;
    i64::try_from(micros).ok()
}

fn duration_before_epoch_to_unix_micros(duration: Duration) -> Option<i64> {
    let fractional_micros = if duration.subsec_nanos() == 0 {
        0
    } else {
        (i128::from(duration.subsec_nanos()) + 999) / 1_000
    };
    let micros = i128::from(duration.as_secs())
        .checked_mul(1_000_000)?
        .checked_add(fractional_micros)?;
    i64::try_from(-micros).ok()
}

fn civil_from_unix_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = (if z >= 0 { z } else { z - 146_096 }).div_euclid(146_097);
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_param = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_param + 2) / 5 + 1;
    let month = month_param + if month_param < 10 { 3 } else { -9 };
    let year = year + if month <= 2 { 1 } else { 0 };

    (year, month, day)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogRecord {
    level: LogLevel,
    target: String,
    message: String,
    timestamp: Option<LogTimestamp>,
    kind: LogRecordKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LogRecordKind {
    Message,
    RepetitionSummary { count: usize },
}

impl LogRecord {
    pub fn new(level: LogLevel, target: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            level,
            target: target.into(),
            message: message.into(),
            timestamp: None,
            kind: LogRecordKind::Message,
        }
    }

    pub fn with_timestamp(mut self, timestamp: LogTimestamp) -> Self {
        self.timestamp = Some(timestamp);
        self
    }

    pub fn with_current_timestamp(self) -> Option<Self> {
        LogTimestamp::now_utc().map(|timestamp| self.with_timestamp(timestamp))
    }

    fn repetition_summary(count: usize) -> Self {
        Self {
            level: LogLevel::Info,
            target: String::new(),
            message: format!("Last message repeated {count} times"),
            timestamp: None,
            kind: LogRecordKind::RepetitionSummary { count },
        }
    }

    pub fn level(&self) -> LogLevel {
        self.level
    }

    pub fn target(&self) -> &str {
        &self.target
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn timestamp(&self) -> Option<LogTimestamp> {
        self.timestamp
    }

    pub fn is_repetition_summary(&self) -> bool {
        matches!(self.kind, LogRecordKind::RepetitionSummary { .. })
    }

    pub fn repetition_count(&self) -> Option<usize> {
        match self.kind {
            LogRecordKind::RepetitionSummary { count } => Some(count),
            LogRecordKind::Message => None,
        }
    }

    pub fn format_line(&self) -> String {
        self.format_line_with_flags(LogFlags::PRINT_LEVEL)
    }

    pub fn format_line_with_flags(&self, flags: LogFlags) -> String {
        self.format_line_with_options(LogFormatOptions::new(flags))
    }

    pub fn format_line_with_options(&self, options: LogFormatOptions) -> String {
        let line = self.format_plain_line_with_flags(options.flags());
        if matches!(options.color_mode(), LogColorMode::Always) {
            if let Some(color_code) = self.ansi_color_code() {
                return format!("{color_code}{line}\x1b[0m");
            }
        }
        line
    }

    fn format_plain_line_with_flags(&self, flags: LogFlags) -> String {
        if self.is_repetition_summary() {
            return self.message.clone();
        }

        let time_prefix = if flags.contains(LogFlags::PRINT_DATETIME) {
            self.timestamp
                .map(|timestamp| format!("[{}] ", timestamp.format_datetime_utc()))
                .unwrap_or_default()
        } else if flags.contains(LogFlags::PRINT_TIME) {
            self.timestamp
                .map(|timestamp| format!("[{}] ", timestamp.format_time_utc()))
                .unwrap_or_default()
        } else {
            String::new()
        };
        let prefix = if flags.contains(LogFlags::PRINT_LEVEL) {
            format!("[{}] ", self.level.name())
        } else {
            String::new()
        };

        if self.target.is_empty() {
            format!("{}{}{}", time_prefix, prefix, self.message)
        } else {
            format!("{}{}{}: {}", time_prefix, prefix, self.target, self.message)
        }
    }

    fn ansi_color_code(&self) -> Option<&'static str> {
        match self.level {
            LogLevel::Panic | LogLevel::Fatal | LogLevel::Error => Some("\x1b[31m"),
            LogLevel::Warning => Some("\x1b[33m"),
            LogLevel::Quiet
            | LogLevel::Info
            | LogLevel::Verbose
            | LogLevel::Debug
            | LogLevel::Trace => None,
        }
    }

    fn same_message_as(&self, other: &Self, flags: LogFlags) -> bool {
        self.kind == LogRecordKind::Message
            && other.kind == LogRecordKind::Message
            && self.level == other.level
            && self.target == other.target
            && self.message == other.message
            && (!flags.intersects(LogFlags::PRINT_TIME | LogFlags::PRINT_DATETIME)
                || self.timestamp == other.timestamp)
    }
}

impl core::ops::BitOr for LogFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self::from_bits_truncate(self.bits | rhs.bits)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RepeatedLogState {
    record: LogRecord,
    count: usize,
}

#[derive(Clone)]
struct LogCallback {
    callback: Arc<dyn Fn(&LogRecord) + Send + Sync + 'static>,
}

impl LogCallback {
    fn new<F>(callback: F) -> Self
    where
        F: Fn(&LogRecord) + Send + Sync + 'static,
    {
        Self {
            callback: Arc::new(callback),
        }
    }

    fn call(&self, record: &LogRecord) {
        (self.callback)(record);
    }
}

impl core::fmt::Debug for LogCallback {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("LogCallback { .. }")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EmitResult {
    first_index: usize,
    submitted_index: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct Logger {
    level: LogLevel,
    flags: LogFlags,
    records: Vec<LogRecord>,
    repeated: Option<RepeatedLogState>,
    callback: Option<LogCallback>,
}

impl Default for Logger {
    fn default() -> Self {
        Self::new(LogLevel::Info)
    }
}

impl Logger {
    pub fn new(level: LogLevel) -> Self {
        Self::new_with_flags(level, LogFlags::PRINT_LEVEL)
    }

    pub fn new_with_flags(level: LogLevel, flags: LogFlags) -> Self {
        Self {
            level,
            flags,
            records: Vec::new(),
            repeated: None,
            callback: None,
        }
    }

    pub fn level(&self) -> LogLevel {
        self.level
    }

    pub fn set_level(&mut self, level: LogLevel) {
        self.level = level;
    }

    pub fn flags(&self) -> LogFlags {
        self.flags
    }

    pub fn set_flags(&mut self, flags: LogFlags) {
        let flags = LogFlags::from_bits_truncate(flags.bits());
        if self.flags.contains(LogFlags::SKIP_REPEATED) && !flags.contains(LogFlags::SKIP_REPEATED)
        {
            self.flush_repeated();
        }
        self.flags = flags;
    }

    pub fn set_flag(&mut self, flag: LogFlags, enabled: bool) {
        let mut flags = self.flags;
        flags.set(flag, enabled);
        self.set_flags(flags);
    }

    pub fn enabled(&self, level: LogLevel) -> bool {
        self.level != LogLevel::Quiet && level != LogLevel::Quiet && level <= self.level
    }

    pub fn set_callback<F>(&mut self, callback: F)
    where
        F: Fn(&LogRecord) + Send + Sync + 'static,
    {
        self.callback = Some(LogCallback::new(callback));
    }

    pub fn clear_callback(&mut self) -> bool {
        self.callback.take().is_some()
    }

    pub fn has_callback(&self) -> bool {
        self.callback.is_some()
    }

    pub fn log(&mut self, record: LogRecord) -> bool {
        if self.enabled(record.level()) {
            let emitted = self.emit_record(record);
            self.dispatch_emitted(emitted.first_index);
            true
        } else {
            false
        }
    }

    pub fn log_with_callback<F>(&mut self, record: LogRecord, callback: F) -> bool
    where
        F: FnOnce(&LogRecord),
    {
        if self.enabled(record.level()) {
            let emitted = self.emit_record(record);
            self.dispatch_emitted(emitted.first_index);
            if let Some(record_index) = emitted.submitted_index {
                callback(&self.records[record_index]);
            }
            true
        } else {
            false
        }
    }

    pub fn records(&self) -> &[LogRecord] {
        &self.records
    }

    pub fn formatted_records(&self) -> Vec<String> {
        self.formatted_records_with_options(LogFormatOptions::new(self.flags))
    }

    pub fn formatted_records_with_options(&self, options: LogFormatOptions) -> Vec<String> {
        let mut formatted: Vec<_> = self
            .records
            .iter()
            .map(|record| record.format_line_with_options(options))
            .collect();
        if let Some(summary) = self.pending_repetition_summary() {
            formatted.push(summary.format_line_with_options(options));
        }
        formatted
    }

    pub fn flush_repeated(&mut self) -> bool {
        if let Some(index) = self.flush_repeated_internal() {
            self.dispatch_record(index);
            true
        } else {
            false
        }
    }

    pub fn clear(&mut self) {
        self.records.clear();
        self.repeated = None;
    }

    pub fn take_records(&mut self) -> Vec<LogRecord> {
        self.flush_repeated();
        core::mem::take(&mut self.records)
    }

    fn emit_record(&mut self, record: LogRecord) -> EmitResult {
        let first_index = self.records.len();
        if self.flags.contains(LogFlags::SKIP_REPEATED) {
            if let Some(repeated) = &mut self.repeated {
                if repeated.record.same_message_as(&record, self.flags) {
                    repeated.count = repeated.count.saturating_add(1);
                    return EmitResult {
                        first_index,
                        submitted_index: None,
                    };
                }
            }

            self.flush_repeated_internal();
            self.repeated = Some(RepeatedLogState {
                record: record.clone(),
                count: 0,
            });
        }

        self.records.push(record);
        EmitResult {
            first_index,
            submitted_index: Some(self.records.len() - 1),
        }
    }

    fn flush_repeated_internal(&mut self) -> Option<usize> {
        if let Some(repeated) = self.repeated.take() {
            if repeated.count > 0 {
                self.records
                    .push(LogRecord::repetition_summary(repeated.count));
                return Some(self.records.len() - 1);
            }
        }
        None
    }

    fn dispatch_emitted(&self, first_index: usize) {
        if self.callback.is_some() {
            for index in first_index..self.records.len() {
                self.dispatch_record(index);
            }
        }
    }

    fn dispatch_record(&self, index: usize) {
        if let Some(callback) = &self.callback {
            callback.call(&self.records[index]);
        }
    }

    fn pending_repetition_summary(&self) -> Option<LogRecord> {
        self.repeated.as_ref().and_then(|repeated| {
            (repeated.count > 0).then(|| LogRecord::repetition_summary(repeated.count))
        })
    }
}

static GLOBAL_LOGGER: OnceLock<Mutex<Logger>> = OnceLock::new();

fn global_logger() -> &'static Mutex<Logger> {
    GLOBAL_LOGGER.get_or_init(|| Mutex::new(Logger::default()))
}

pub fn global_log_level() -> LogLevel {
    global_logger().lock().unwrap().level()
}

pub fn set_global_log_level(level: LogLevel) {
    global_logger().lock().unwrap().set_level(level);
}

pub fn global_log_flags() -> LogFlags {
    global_logger().lock().unwrap().flags()
}

pub fn set_global_log_flags(flags: LogFlags) {
    global_logger().lock().unwrap().set_flags(flags);
}

pub fn set_global_log_flag(flag: LogFlags, enabled: bool) {
    global_logger().lock().unwrap().set_flag(flag, enabled);
}

pub fn set_global_log_callback<F>(callback: F)
where
    F: Fn(&LogRecord) + Send + Sync + 'static,
{
    global_logger().lock().unwrap().set_callback(callback);
}

pub fn clear_global_log_callback() -> bool {
    global_logger().lock().unwrap().clear_callback()
}

pub fn global_log(record: LogRecord) -> bool {
    global_logger().lock().unwrap().log(record)
}

pub fn flush_global_log_repeated() -> bool {
    global_logger().lock().unwrap().flush_repeated()
}

pub fn global_formatted_log_records() -> Vec<String> {
    global_logger().lock().unwrap().formatted_records()
}

pub fn global_formatted_log_records_with_options(options: LogFormatOptions) -> Vec<String> {
    global_logger()
        .lock()
        .unwrap()
        .formatted_records_with_options(options)
}

pub fn clear_global_log_records() {
    global_logger().lock().unwrap().clear();
}

pub fn take_global_log_records() -> Vec<LogRecord> {
    global_logger().lock().unwrap().take_records()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    static GLOBAL_LOGGER_TEST_LOCK: Mutex<()> = Mutex::new(());

    fn reset_global_logger_for_tests(level: LogLevel, flags: LogFlags) {
        let mut logger = global_logger().lock().unwrap();
        *logger = Logger::new_with_flags(level, flags);
    }

    #[test]
    fn logger_filters_by_level() {
        let mut logger = Logger::new(LogLevel::Warning);

        assert!(!logger.log(LogRecord::new(LogLevel::Info, "ffmpeg", "ignored")));
        assert!(logger.log(LogRecord::new(LogLevel::Error, "ffmpeg", "kept")));

        assert_eq!(logger.records().len(), 1);
        assert_eq!(logger.records()[0].message(), "kept");
    }

    #[test]
    fn quiet_suppresses_all_records() {
        let mut logger = Logger::new(LogLevel::Quiet);

        assert!(!logger.log(LogRecord::new(LogLevel::Quiet, "ffmpeg", "ignored")));
        assert!(!logger.log(LogRecord::new(LogLevel::Fatal, "ffmpeg", "ignored")));

        assert!(logger.records().is_empty());
    }

    #[test]
    fn log_levels_match_ffmpeg_values_and_names() {
        let levels = [
            (LogLevel::Quiet, -8, "quiet"),
            (LogLevel::Panic, 0, "panic"),
            (LogLevel::Fatal, 8, "fatal"),
            (LogLevel::Error, 16, "error"),
            (LogLevel::Warning, 24, "warning"),
            (LogLevel::Info, 32, "info"),
            (LogLevel::Verbose, 40, "verbose"),
            (LogLevel::Debug, 48, "debug"),
            (LogLevel::Trace, 56, "trace"),
        ];

        for (level, value, name) in levels {
            assert_eq!(level.as_ffmpeg_value(), value);
            assert_eq!(level.name(), name);
            assert_eq!(LogLevel::from_ffmpeg_value(value), Some(level));
            assert_eq!(LogLevel::from_name(name), Some(level));
            assert_eq!(LogLevel::from_name(&name.to_uppercase()), Some(level));
        }

        assert_eq!(LogLevel::from_ffmpeg_value(4), None);
        assert_eq!(LogLevel::from_name("warn"), None);
    }

    #[test]
    fn log_flags_track_known_ffmpeg_bits() {
        assert_eq!(LogFlags::SKIP_REPEATED.bits(), 1);
        assert_eq!(LogFlags::PRINT_LEVEL.bits(), 2);
        assert_eq!(LogFlags::PRINT_TIME.bits(), 4);
        assert_eq!(LogFlags::PRINT_DATETIME.bits(), 8);
        assert_eq!(LogFlags::all().bits(), 15);
        assert!(LogFlags::empty().is_empty());

        let truncated = LogFlags::from_bits_truncate(0xffff);
        assert_eq!(truncated, LogFlags::all());
        assert!(truncated.contains(LogFlags::SKIP_REPEATED));
        assert!(truncated.contains(LogFlags::PRINT_LEVEL));
        assert!(truncated.contains(LogFlags::PRINT_TIME));
        assert!(truncated.contains(LogFlags::PRINT_DATETIME));
        assert!(LogFlags::PRINT_TIME.intersects(LogFlags::PRINT_TIME | LogFlags::PRINT_DATETIME));
        assert!(!LogFlags::PRINT_LEVEL.intersects(LogFlags::PRINT_TIME | LogFlags::PRINT_DATETIME));
    }

    #[test]
    fn log_timestamps_format_utc_time_and_datetime() {
        let timestamp = LogTimestamp::from_unix_micros(1_704_112_705_123_456);

        assert_eq!(timestamp.unix_micros(), 1_704_112_705_123_456);
        assert_eq!(timestamp.format_time_utc(), "12:38:25.123456");
        assert_eq!(
            timestamp.format_datetime_utc(),
            "2024-01-01 12:38:25.123456"
        );

        let before_epoch = LogTimestamp::from_unix_micros(-1);
        assert_eq!(
            before_epoch.format_datetime_utc(),
            "1969-12-31 23:59:59.999999"
        );
    }

    #[test]
    fn log_timestamps_convert_system_time_to_unix_micros() {
        let after_epoch = UNIX_EPOCH + Duration::new(1, 234_567_000);
        assert_eq!(
            LogTimestamp::from_system_time(after_epoch).map(LogTimestamp::unix_micros),
            Some(1_234_567)
        );

        let positive_submicro = UNIX_EPOCH + Duration::from_nanos(999);
        assert_eq!(
            LogTimestamp::from_system_time(positive_submicro).map(LogTimestamp::unix_micros),
            Some(0)
        );

        let before_epoch = UNIX_EPOCH - Duration::new(1, 234_567_000);
        assert_eq!(
            LogTimestamp::from_system_time(before_epoch).map(LogTimestamp::unix_micros),
            Some(-1_234_567)
        );

        assert_eq!(
            duration_before_epoch_to_unix_micros(Duration::from_nanos(1)),
            Some(-1)
        );
    }

    #[test]
    fn log_timestamps_capture_current_system_time() {
        let before = LogTimestamp::from_system_time(SystemTime::now()).unwrap();
        let timestamp = LogTimestamp::now_utc().unwrap();
        let after = LogTimestamp::from_system_time(SystemTime::now()).unwrap();

        assert!(before <= timestamp);
        assert!(timestamp <= after);

        let record = LogRecord::new(LogLevel::Info, "ffmpeg", "ready")
            .with_current_timestamp()
            .unwrap();
        assert!(record.timestamp().is_some());
    }

    #[test]
    fn record_formats_level_target_and_message() {
        assert_eq!(
            LogRecord::new(LogLevel::Warning, "decoder", "damaged packet").format_line(),
            "[warning] decoder: damaged packet"
        );
        assert_eq!(
            LogRecord::new(LogLevel::Info, "", "ready").format_line(),
            "[info] ready"
        );
    }

    #[test]
    fn record_formatting_respects_time_and_datetime_flags() {
        let timestamp = LogTimestamp::from_unix_micros(1_704_112_705_123_456);
        let record =
            LogRecord::new(LogLevel::Error, "demuxer", "bad header").with_timestamp(timestamp);

        assert_eq!(record.timestamp(), Some(timestamp));
        assert_eq!(
            record.format_line_with_flags(LogFlags::PRINT_TIME | LogFlags::PRINT_LEVEL),
            "[12:38:25.123456] [error] demuxer: bad header"
        );
        assert_eq!(
            record.format_line_with_flags(LogFlags::PRINT_DATETIME | LogFlags::PRINT_LEVEL),
            "[2024-01-01 12:38:25.123456] [error] demuxer: bad header"
        );
        assert_eq!(
            record.format_line_with_flags(
                LogFlags::PRINT_TIME | LogFlags::PRINT_DATETIME | LogFlags::PRINT_LEVEL
            ),
            "[2024-01-01 12:38:25.123456] [error] demuxer: bad header"
        );
        assert_eq!(
            LogRecord::new(LogLevel::Info, "ffmpeg", "ready")
                .format_line_with_flags(LogFlags::PRINT_TIME | LogFlags::PRINT_LEVEL),
            "[info] ffmpeg: ready"
        );
    }

    #[test]
    fn record_formatting_respects_print_level_flag() {
        let record = LogRecord::new(LogLevel::Error, "demuxer", "bad header");

        assert_eq!(
            record.format_line_with_flags(LogFlags::PRINT_LEVEL),
            "[error] demuxer: bad header"
        );
        assert_eq!(
            record.format_line_with_flags(LogFlags::empty()),
            "demuxer: bad header"
        );
        assert_eq!(
            LogRecord::new(LogLevel::Info, "", "ready").format_line_with_flags(LogFlags::empty()),
            "ready"
        );
    }

    #[test]
    fn record_formatting_supports_explicit_color_options() {
        let color_options =
            LogFormatOptions::new(LogFlags::PRINT_LEVEL).with_color_mode(LogColorMode::Always);
        let plain_options =
            LogFormatOptions::new(LogFlags::PRINT_LEVEL).with_color_mode(LogColorMode::Never);

        assert_eq!(
            LogRecord::new(LogLevel::Error, "demuxer", "bad header")
                .format_line_with_options(color_options),
            "\x1b[31m[error] demuxer: bad header\x1b[0m"
        );
        assert_eq!(
            LogRecord::new(LogLevel::Fatal, "decoder", "unrecoverable")
                .format_line_with_options(color_options),
            "\x1b[31m[fatal] decoder: unrecoverable\x1b[0m"
        );
        assert_eq!(
            LogRecord::new(LogLevel::Warning, "decoder", "damaged packet")
                .format_line_with_options(color_options),
            "\x1b[33m[warning] decoder: damaged packet\x1b[0m"
        );
        assert_eq!(
            LogRecord::new(LogLevel::Info, "ffmpeg", "ready")
                .format_line_with_options(color_options),
            "[info] ffmpeg: ready"
        );
        assert_eq!(
            LogRecord::new(LogLevel::Warning, "decoder", "damaged packet")
                .format_line_with_options(plain_options),
            "[warning] decoder: damaged packet"
        );
        assert_eq!(
            LogRecord::repetition_summary(2).format_line_with_options(color_options),
            "Last message repeated 2 times"
        );
    }

    #[test]
    fn color_mode_resolves_ffmpeg_force_color_env_vars() {
        assert_eq!(
            LogColorMode::from_ffmpeg_env_vars(|name| name == AV_LOG_FORCE_COLOR_ENV),
            LogColorMode::Always
        );
        assert_eq!(
            LogColorMode::from_ffmpeg_env_vars(|_| false),
            LogColorMode::Never
        );

        let options = LogFormatOptions::new(LogFlags::PRINT_LEVEL)
            .with_ffmpeg_env_color_vars(|name| name == AV_LOG_FORCE_COLOR_ENV);
        assert_eq!(options.color_mode(), LogColorMode::Always);
        assert_eq!(
            LogRecord::new(LogLevel::Error, "demuxer", "bad header")
                .format_line_with_options(options),
            "\x1b[31m[error] demuxer: bad header\x1b[0m"
        );
    }

    #[test]
    fn color_mode_force_nocolor_env_wins_over_force_color() {
        let mut checked = Vec::new();
        let mode = LogColorMode::from_ffmpeg_env_vars(|name| {
            checked.push(name.to_owned());
            name == AV_LOG_FORCE_NOCOLOR_ENV || name == AV_LOG_FORCE_COLOR_ENV
        });

        assert_eq!(mode, LogColorMode::Never);
        assert_eq!(checked, [AV_LOG_FORCE_NOCOLOR_ENV.to_owned()]);

        let options = LogFormatOptions::new(LogFlags::PRINT_LEVEL)
            .with_ffmpeg_env_color_vars(|name| name == AV_LOG_FORCE_NOCOLOR_ENV);
        assert_eq!(options.color_mode(), LogColorMode::Never);
        assert_eq!(
            LogRecord::new(LogLevel::Error, "demuxer", "bad header")
                .format_line_with_options(options),
            "[error] demuxer: bad header"
        );
    }

    #[test]
    fn logger_formats_records_with_configured_flags() {
        let mut logger = Logger::new_with_flags(LogLevel::Info, LogFlags::empty());

        assert_eq!(logger.flags(), LogFlags::empty());
        assert!(logger.log(LogRecord::new(LogLevel::Info, "ffmpeg", "ready")));
        assert_eq!(logger.formatted_records(), ["ffmpeg: ready"]);

        logger.set_flag(LogFlags::PRINT_LEVEL, true);
        assert_eq!(logger.flags(), LogFlags::PRINT_LEVEL);
        assert_eq!(logger.formatted_records(), ["[info] ffmpeg: ready"]);

        logger.set_flags(LogFlags::from_bits_truncate(0xffff));
        assert_eq!(logger.flags(), LogFlags::all());

        logger.set_flag(LogFlags::PRINT_LEVEL, false);
        assert!(!logger.flags().contains(LogFlags::PRINT_LEVEL));
        assert_eq!(logger.formatted_records(), ["ffmpeg: ready"]);
    }

    #[test]
    fn logger_formats_records_with_explicit_color_options() {
        let mut flags = LogFlags::PRINT_LEVEL;
        flags.insert(LogFlags::SKIP_REPEATED);
        let mut logger = Logger::new_with_flags(LogLevel::Warning, flags);
        let repeated = LogRecord::new(LogLevel::Warning, "decoder", "damaged packet");
        let options = LogFormatOptions::new(logger.flags()).with_color_mode(LogColorMode::Always);

        assert!(logger.log(repeated.clone()));
        assert!(logger.log(repeated));
        assert_eq!(
            logger.formatted_records_with_options(options),
            [
                "\x1b[33m[warning] decoder: damaged packet\x1b[0m".to_owned(),
                "Last message repeated 1 times".to_owned()
            ]
        );

        assert!(logger.log(LogRecord::new(LogLevel::Error, "demuxer", "bad header")));
        assert_eq!(
            logger.formatted_records_with_options(options),
            [
                "\x1b[33m[warning] decoder: damaged packet\x1b[0m".to_owned(),
                "Last message repeated 1 times".to_owned(),
                "\x1b[31m[error] demuxer: bad header\x1b[0m".to_owned()
            ]
        );
    }

    #[test]
    fn skip_repeated_flag_compresses_consecutive_messages_until_flush() {
        let mut flags = LogFlags::PRINT_LEVEL;
        flags.insert(LogFlags::SKIP_REPEATED);
        let mut logger = Logger::new_with_flags(LogLevel::Warning, flags);
        let record = LogRecord::new(LogLevel::Warning, "decoder", "damaged packet");

        assert!(logger.log(record.clone()));
        assert!(logger.log(record.clone()));
        assert!(logger.log(record));

        assert_eq!(logger.records().len(), 1);
        assert_eq!(
            logger.formatted_records(),
            [
                "[warning] decoder: damaged packet".to_owned(),
                "Last message repeated 2 times".to_owned()
            ]
        );
        assert!(logger.flush_repeated());
        assert_eq!(logger.records().len(), 2);
        assert!(logger.records()[1].is_repetition_summary());
        assert_eq!(logger.records()[1].repetition_count(), Some(2));
        assert_eq!(
            logger.formatted_records(),
            [
                "[warning] decoder: damaged packet".to_owned(),
                "Last message repeated 2 times".to_owned()
            ]
        );
        assert!(!logger.flush_repeated());
    }

    #[test]
    fn repeated_summary_is_emitted_before_different_message() {
        let mut flags = LogFlags::PRINT_LEVEL;
        flags.insert(LogFlags::SKIP_REPEATED);
        let mut logger = Logger::new_with_flags(LogLevel::Info, flags);
        let repeated = LogRecord::new(LogLevel::Error, "demuxer", "bad header");

        assert!(logger.log(repeated.clone()));
        assert!(logger.log(repeated));
        assert!(logger.log(LogRecord::new(
            LogLevel::Warning,
            "decoder",
            "damaged packet"
        )));

        assert_eq!(
            logger.formatted_records(),
            [
                "[error] demuxer: bad header".to_owned(),
                "Last message repeated 1 times".to_owned(),
                "[warning] decoder: damaged packet".to_owned()
            ]
        );
        assert!(logger.records()[1].is_repetition_summary());
        assert_eq!(logger.records()[1].repetition_count(), Some(1));
    }

    #[test]
    fn repeated_summary_is_dropped_by_clear_and_flushed_by_take_or_flag_change() {
        let mut flags = LogFlags::PRINT_LEVEL;
        flags.insert(LogFlags::SKIP_REPEATED);
        let repeated = LogRecord::new(LogLevel::Error, "ffmpeg", "bad packet");

        let mut logger = Logger::new_with_flags(LogLevel::Info, flags);
        assert!(logger.log(repeated.clone()));
        assert!(logger.log(repeated.clone()));
        logger.clear();
        assert!(logger.records().is_empty());
        assert!(logger.formatted_records().is_empty());

        assert!(logger.log(repeated.clone()));
        assert!(logger.log(repeated.clone()));
        let records = logger.take_records();
        assert_eq!(records.len(), 2);
        assert!(records[1].is_repetition_summary());
        assert!(logger.records().is_empty());
        assert!(logger.formatted_records().is_empty());

        assert!(logger.log(repeated.clone()));
        assert!(logger.log(repeated.clone()));
        logger.set_flag(LogFlags::SKIP_REPEATED, false);
        assert_eq!(logger.records().len(), 2);
        assert!(logger.records()[1].is_repetition_summary());
        assert!(logger.log(repeated));
        assert_eq!(logger.records().len(), 3);
    }

    #[test]
    fn repeated_records_are_preserved_without_skip_repeated_flag() {
        let mut logger = Logger::new(LogLevel::Info);
        let record = LogRecord::new(LogLevel::Warning, "decoder", "damaged packet");

        assert!(logger.log(record.clone()));
        assert!(logger.log(record));

        assert_eq!(logger.records().len(), 2);
        assert_eq!(
            logger.formatted_records(),
            [
                "[warning] decoder: damaged packet".to_owned(),
                "[warning] decoder: damaged packet".to_owned()
            ]
        );
    }

    #[test]
    fn repeated_comparison_respects_printed_timestamp_flags() {
        let first = LogRecord::new(LogLevel::Warning, "decoder", "damaged packet")
            .with_timestamp(LogTimestamp::from_unix_micros(1_704_112_705_000_000));
        let second = LogRecord::new(LogLevel::Warning, "decoder", "damaged packet")
            .with_timestamp(LogTimestamp::from_unix_micros(1_704_112_706_000_000));

        let mut flags = LogFlags::PRINT_LEVEL;
        flags.insert(LogFlags::SKIP_REPEATED);
        let mut logger = Logger::new_with_flags(LogLevel::Warning, flags);
        assert!(logger.log(first.clone()));
        assert!(logger.log(second.clone()));
        assert_eq!(logger.records().len(), 1);
        assert_eq!(
            logger.formatted_records(),
            [
                "[warning] decoder: damaged packet".to_owned(),
                "Last message repeated 1 times".to_owned()
            ]
        );

        let mut flags_with_time = flags;
        flags_with_time.insert(LogFlags::PRINT_TIME);
        let mut logger = Logger::new_with_flags(LogLevel::Warning, flags_with_time);
        assert!(logger.log(first));
        assert!(logger.log(second));
        assert_eq!(logger.records().len(), 2);
        assert_eq!(
            logger.formatted_records(),
            [
                "[12:38:25.000000] [warning] decoder: damaged packet".to_owned(),
                "[12:38:26.000000] [warning] decoder: damaged packet".to_owned()
            ]
        );
    }

    #[test]
    fn callback_runs_only_for_accepted_records() {
        let mut logger = Logger::new(LogLevel::Error);
        let mut seen = Vec::new();

        assert!(!logger.log_with_callback(
            LogRecord::new(LogLevel::Info, "ffmpeg", "ignored"),
            |record| seen.push(record.format_line())
        ));
        assert!(logger.log_with_callback(
            LogRecord::new(LogLevel::Error, "ffmpeg", "kept"),
            |record| seen.push(record.format_line())
        ));

        assert_eq!(seen, ["[error] ffmpeg: kept"]);
        assert_eq!(logger.records().len(), 1);
    }

    #[test]
    fn installed_callback_receives_emitted_records_until_cleared() {
        let mut logger = Logger::new(LogLevel::Warning);
        let seen = Arc::new(Mutex::new(Vec::new()));
        let callback_seen = Arc::clone(&seen);

        assert!(!logger.has_callback());
        logger.set_callback(move |record| {
            callback_seen.lock().unwrap().push(record.format_line());
        });
        assert!(logger.has_callback());

        assert!(!logger.log(LogRecord::new(LogLevel::Info, "ffmpeg", "ignored")));
        assert!(logger.log(LogRecord::new(LogLevel::Warning, "ffmpeg", "kept")));
        assert_eq!(seen.lock().unwrap().as_slice(), ["[warning] ffmpeg: kept"]);

        assert!(logger.clear_callback());
        assert!(!logger.has_callback());
        assert!(!logger.clear_callback());
        assert!(logger.log(LogRecord::new(LogLevel::Error, "ffmpeg", "not callbacked")));
        assert_eq!(seen.lock().unwrap().as_slice(), ["[warning] ffmpeg: kept"]);
        assert_eq!(logger.records().len(), 2);
    }

    #[test]
    fn installed_callback_observes_repeat_summary_when_materialized() {
        let mut flags = LogFlags::PRINT_LEVEL;
        flags.insert(LogFlags::SKIP_REPEATED);
        let mut logger = Logger::new_with_flags(LogLevel::Info, flags);
        let seen = Arc::new(Mutex::new(Vec::new()));
        let callback_seen = Arc::clone(&seen);
        logger.set_callback(move |record| {
            callback_seen.lock().unwrap().push(record.format_line());
        });
        let repeated = LogRecord::new(LogLevel::Warning, "decoder", "damaged packet");

        assert!(logger.log(repeated.clone()));
        assert!(logger.log(repeated));
        assert_eq!(
            seen.lock().unwrap().as_slice(),
            ["[warning] decoder: damaged packet"]
        );

        assert!(logger.log(LogRecord::new(LogLevel::Error, "demuxer", "bad header")));
        assert_eq!(
            seen.lock().unwrap().as_slice(),
            [
                "[warning] decoder: damaged packet",
                "Last message repeated 1 times",
                "[error] demuxer: bad header",
            ]
        );

        assert!(logger.log(LogRecord::new(LogLevel::Info, "muxer", "flushable")));
        assert!(logger.log(LogRecord::new(LogLevel::Info, "muxer", "flushable")));
        assert!(logger.flush_repeated());
        assert_eq!(
            seen.lock().unwrap().last().map(String::as_str),
            Some("Last message repeated 1 times")
        );
    }

    #[test]
    fn callback_runs_only_for_emitted_repeated_records() {
        let mut flags = LogFlags::PRINT_LEVEL;
        flags.insert(LogFlags::SKIP_REPEATED);
        let mut logger = Logger::new_with_flags(LogLevel::Error, flags);
        let mut seen = Vec::new();
        let record = LogRecord::new(LogLevel::Error, "ffmpeg", "bad packet");

        assert!(logger.log_with_callback(record.clone(), |record| seen.push(record.format_line())));
        assert!(logger.log_with_callback(record, |record| seen.push(record.format_line())));

        assert_eq!(seen, ["[error] ffmpeg: bad packet"]);
        assert_eq!(logger.formatted_records().len(), 2);
    }

    #[test]
    fn global_logger_filters_and_takes_records() {
        let _guard = GLOBAL_LOGGER_TEST_LOCK.lock().unwrap();
        reset_global_logger_for_tests(LogLevel::Warning, LogFlags::PRINT_LEVEL);

        assert_eq!(global_log_level(), LogLevel::Warning);
        assert_eq!(global_log_flags(), LogFlags::PRINT_LEVEL);
        assert!(!global_log(LogRecord::new(
            LogLevel::Info,
            "ffmpeg",
            "ignored"
        )));
        assert!(global_log(LogRecord::new(
            LogLevel::Error,
            "ffmpeg",
            "kept"
        )));
        assert_eq!(global_formatted_log_records(), ["[error] ffmpeg: kept"]);
        assert_eq!(
            global_formatted_log_records_with_options(
                LogFormatOptions::new(global_log_flags()).with_color_mode(LogColorMode::Always)
            ),
            ["\x1b[31m[error] ffmpeg: kept\x1b[0m"]
        );

        let records = take_global_log_records();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].message(), "kept");
        assert!(global_formatted_log_records().is_empty());

        reset_global_logger_for_tests(LogLevel::Info, LogFlags::PRINT_LEVEL);
    }

    #[test]
    fn global_logger_shared_flags_flush_repeated_state() {
        let _guard = GLOBAL_LOGGER_TEST_LOCK.lock().unwrap();
        let mut flags = LogFlags::PRINT_LEVEL;
        flags.insert(LogFlags::SKIP_REPEATED);
        reset_global_logger_for_tests(LogLevel::Info, flags);

        let repeated = LogRecord::new(LogLevel::Warning, "decoder", "damaged packet");
        assert!(global_log(repeated.clone()));
        assert!(global_log(repeated));
        assert_eq!(global_formatted_log_records().len(), 2);

        set_global_log_flag(LogFlags::SKIP_REPEATED, false);
        assert!(!global_log_flags().contains(LogFlags::SKIP_REPEATED));
        let records = take_global_log_records();
        assert_eq!(records.len(), 2);
        assert_eq!(records[1].repetition_count(), Some(1));

        set_global_log_flag(LogFlags::SKIP_REPEATED, true);
        let repeated = LogRecord::new(LogLevel::Error, "decoder", "corrupt packet");
        assert!(global_log(repeated.clone()));
        assert!(global_log(repeated));
        assert!(flush_global_log_repeated());
        assert!(!flush_global_log_repeated());
        let records = take_global_log_records();
        assert_eq!(records.len(), 2);
        assert!(records[1].is_repetition_summary());
        assert_eq!(records[1].repetition_count(), Some(1));

        reset_global_logger_for_tests(LogLevel::Info, LogFlags::PRINT_LEVEL);
    }

    #[test]
    fn global_logger_installed_callback_receives_emitted_records() {
        let _guard = GLOBAL_LOGGER_TEST_LOCK.lock().unwrap();
        let mut flags = LogFlags::PRINT_LEVEL;
        flags.insert(LogFlags::SKIP_REPEATED);
        reset_global_logger_for_tests(LogLevel::Info, flags);
        let seen = Arc::new(Mutex::new(Vec::new()));
        let callback_seen = Arc::clone(&seen);
        set_global_log_callback(move |record| {
            callback_seen.lock().unwrap().push(record.format_line());
        });

        assert!(!global_log(LogRecord::new(
            LogLevel::Debug,
            "decoder",
            "hidden"
        )));
        let repeated = LogRecord::new(LogLevel::Warning, "decoder", "damaged packet");
        assert!(global_log(repeated.clone()));
        assert!(global_log(repeated));
        assert_eq!(
            seen.lock().unwrap().as_slice(),
            ["[warning] decoder: damaged packet"]
        );

        assert!(global_log(LogRecord::new(
            LogLevel::Error,
            "demuxer",
            "bad header"
        )));
        assert_eq!(
            seen.lock().unwrap().as_slice(),
            [
                "[warning] decoder: damaged packet",
                "Last message repeated 1 times",
                "[error] demuxer: bad header",
            ]
        );

        assert!(clear_global_log_callback());
        assert!(!clear_global_log_callback());
        assert!(global_log(LogRecord::new(
            LogLevel::Error,
            "demuxer",
            "after clear"
        )));
        assert_eq!(seen.lock().unwrap().len(), 3);

        reset_global_logger_for_tests(LogLevel::Info, LogFlags::PRINT_LEVEL);
    }

    #[test]
    fn repetition_summary_records_format_without_level_prefix() {
        let summary = LogRecord::repetition_summary(3);

        assert!(summary.is_repetition_summary());
        assert_eq!(summary.repetition_count(), Some(3));
        assert_eq!(summary.format_line(), "Last message repeated 3 times");
        assert_eq!(
            summary.format_line_with_flags(LogFlags::PRINT_LEVEL),
            "Last message repeated 3 times"
        );
    }

    #[test]
    fn set_level_clear_and_take_records_control_buffer() {
        let mut logger = Logger::new(LogLevel::Error);

        assert!(!logger.enabled(LogLevel::Info));
        logger.set_level(LogLevel::Info);
        assert!(logger.enabled(LogLevel::Info));

        assert!(logger.log(LogRecord::new(LogLevel::Info, "ffmpeg", "one")));
        assert!(logger.log(LogRecord::new(LogLevel::Error, "ffmpeg", "two")));

        let records = logger.take_records();
        assert_eq!(records.len(), 2);
        assert!(logger.records().is_empty());

        assert!(logger.log(LogRecord::new(LogLevel::Fatal, "ffmpeg", "three")));
        logger.clear();
        assert!(logger.records().is_empty());
    }
}
