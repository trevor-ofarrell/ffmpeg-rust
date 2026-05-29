use std::ffi::OsStr;
use std::io::IsTerminal;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::error::AvResult;

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

    const fn permits_prefix_fields(self) -> bool {
        !matches!(self, Self::Quiet)
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

    pub const fn from_bits_retain(bits: u32) -> Self {
        Self { bits }
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
pub struct LogOnceState {
    raw: i32,
}

impl LogOnceState {
    pub const fn new() -> Self {
        Self { raw: 0 }
    }

    pub const fn from_raw(raw: i32) -> Self {
        Self { raw }
    }

    pub const fn raw(self) -> i32 {
        self.raw
    }

    pub const fn has_logged(self) -> bool {
        self.raw != 0
    }

    fn mark_logged(&mut self) {
        self.raw = 1;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogColorMode {
    #[default]
    Never,
    Basic,
    Always,
}

pub const AV_LOG_FORCE_COLOR_ENV: &str = "AV_LOG_FORCE_COLOR";
pub const AV_LOG_FORCE_NOCOLOR_ENV: &str = "AV_LOG_FORCE_NOCOLOR";
const DEFAULT_CALLBACK_CONTEXT_COLOR: &str = "\x1b[48;5;0m\x1b[38;5;250m";
const DEFAULT_CALLBACK_WARNING_COLOR: &str = "\x1b[48;5;0m\x1b[38;5;226m";
const DEFAULT_CALLBACK_FATAL_COLOR: &str = "\x1b[48;5;0m\x1b[38;5;208m";
const DEFAULT_CALLBACK_ERROR_COLOR: &str = "\x1b[48;5;0m\x1b[38;5;196m";
const DEFAULT_CALLBACK_PANIC_COLOR: &str = "\x1b[48;5;52m\x1b[38;5;196m";
const DEFAULT_CALLBACK_BASIC_CONTEXT_COLOR: &str = "\x1b[0;39m";
const DEFAULT_CALLBACK_BASIC_ERROR_COLOR: &str = "\x1b[1;31m";
const DEFAULT_CALLBACK_BASIC_FATAL_COLOR: &str = "\x1b[4;31m";
const DEFAULT_CALLBACK_BASIC_WARNING_COLOR: &str = "\x1b[0;33m";

fn colorize(color_code: &str, text: &str) -> String {
    format!("{color_code}{text}\x1b[0m")
}

impl LogColorMode {
    pub fn from_ffmpeg_env() -> Self {
        let term = std::env::var_os("TERM");
        Self::from_ffmpeg_env_vars_stderr_and_term(
            |name| std::env::var_os(name).is_some(),
            std::io::stderr().is_terminal(),
            term.as_deref(),
        )
    }

    pub fn from_ffmpeg_env_vars(is_set: impl FnMut(&str) -> bool) -> Self {
        Self::from_ffmpeg_env_vars_and_stderr(is_set, false)
    }

    pub fn from_ffmpeg_env_vars_and_stderr(
        mut is_set: impl FnMut(&str) -> bool,
        stderr_is_terminal: bool,
    ) -> Self {
        if is_set(AV_LOG_FORCE_NOCOLOR_ENV) {
            Self::Never
        } else if is_set(AV_LOG_FORCE_COLOR_ENV) || stderr_is_terminal {
            Self::Always
        } else {
            Self::Never
        }
    }

    pub fn from_ffmpeg_env_vars_stderr_and_term(
        mut is_set: impl FnMut(&str) -> bool,
        stderr_is_terminal: bool,
        term: Option<&OsStr>,
    ) -> Self {
        if is_set(AV_LOG_FORCE_NOCOLOR_ENV) {
            Self::Never
        } else if is_set(AV_LOG_FORCE_COLOR_ENV) {
            Self::Always
        } else if !stderr_is_terminal {
            Self::Never
        } else if let Some(term) = term {
            if term.to_string_lossy().contains("256color") {
                Self::Always
            } else {
                Self::Basic
            }
        } else {
            Self::Never
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DefaultCallbackColorState {
    resolved: Option<LogColorMode>,
}

impl DefaultCallbackColorState {
    pub const fn new() -> Self {
        Self { resolved: None }
    }

    pub const fn from_resolved(color_mode: LogColorMode) -> Self {
        Self {
            resolved: Some(color_mode),
        }
    }

    pub const fn cached_mode(self) -> Option<LogColorMode> {
        self.resolved
    }

    pub fn resolve_with(&mut self, resolver: impl FnOnce() -> LogColorMode) -> LogColorMode {
        match self.resolved {
            Some(color_mode) => color_mode,
            None => {
                let color_mode = resolver();
                self.resolved = Some(color_mode);
                color_mode
            }
        }
    }

    pub fn resolve_ffmpeg_env(&mut self) -> LogColorMode {
        self.resolve_with(LogColorMode::from_ffmpeg_env)
    }

    pub fn reset(&mut self) {
        self.resolved = None;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DefaultCallbackPrefixState {
    print_prefix: bool,
}

impl Default for DefaultCallbackPrefixState {
    fn default() -> Self {
        Self::new()
    }
}

impl DefaultCallbackPrefixState {
    pub const fn new() -> Self {
        Self { print_prefix: true }
    }

    pub const fn from_print_prefix(print_prefix: bool) -> Self {
        Self { print_prefix }
    }

    pub const fn print_prefix(self) -> bool {
        self.print_prefix
    }

    pub fn set_print_prefix(&mut self, print_prefix: bool) {
        self.print_prefix = print_prefix;
    }

    pub fn reset(&mut self) {
        self.print_prefix = true;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LogFormatOptions {
    flags: LogFlags,
    color_mode: LogColorMode,
    default_callback_time_zone: LogDefaultCallbackTimeZone,
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
            default_callback_time_zone: LogDefaultCallbackTimeZone::FixedOffsetSeconds(0),
        }
    }

    pub const fn with_color_mode(self, color_mode: LogColorMode) -> Self {
        Self {
            flags: self.flags,
            color_mode,
            default_callback_time_zone: self.default_callback_time_zone,
        }
    }

    pub const fn with_default_callback_time_offset_seconds(
        self,
        default_callback_time_offset_seconds: i32,
    ) -> Self {
        Self {
            flags: self.flags,
            color_mode: self.color_mode,
            default_callback_time_zone: LogDefaultCallbackTimeZone::FixedOffsetSeconds(
                default_callback_time_offset_seconds,
            ),
        }
    }

    pub const fn with_default_callback_time_zone(
        self,
        default_callback_time_zone: LogDefaultCallbackTimeZone,
    ) -> Self {
        Self {
            flags: self.flags,
            color_mode: self.color_mode,
            default_callback_time_zone,
        }
    }

    pub fn with_ffmpeg_env_color(self) -> Self {
        self.with_color_mode(LogColorMode::from_ffmpeg_env())
    }

    pub fn with_ffmpeg_env_color_vars(self, is_set: impl FnMut(&str) -> bool) -> Self {
        self.with_color_mode(LogColorMode::from_ffmpeg_env_vars(is_set))
    }

    pub fn with_ffmpeg_env_color_vars_and_stderr(
        self,
        is_set: impl FnMut(&str) -> bool,
        stderr_is_terminal: bool,
    ) -> Self {
        self.with_color_mode(LogColorMode::from_ffmpeg_env_vars_and_stderr(
            is_set,
            stderr_is_terminal,
        ))
    }

    pub fn with_ffmpeg_env_color_vars_stderr_and_term(
        self,
        is_set: impl FnMut(&str) -> bool,
        stderr_is_terminal: bool,
        term: Option<&OsStr>,
    ) -> Self {
        self.with_color_mode(LogColorMode::from_ffmpeg_env_vars_stderr_and_term(
            is_set,
            stderr_is_terminal,
            term,
        ))
    }

    pub fn with_default_callback_color_state(self, state: &mut DefaultCallbackColorState) -> Self {
        self.with_color_mode(state.resolve_ffmpeg_env())
    }

    pub fn with_default_callback_color_state_and_resolver(
        self,
        state: &mut DefaultCallbackColorState,
        resolver: impl FnOnce() -> LogColorMode,
    ) -> Self {
        self.with_color_mode(state.resolve_with(resolver))
    }

    pub const fn flags(self) -> LogFlags {
        self.flags
    }

    pub const fn color_mode(self) -> LogColorMode {
        self.color_mode
    }

    pub const fn default_callback_time_offset_seconds(self) -> i32 {
        match self.default_callback_time_zone {
            LogDefaultCallbackTimeZone::FixedOffsetSeconds(offset_seconds) => offset_seconds,
            LogDefaultCallbackTimeZone::PosixDst {
                standard_offset_seconds,
                ..
            } => standard_offset_seconds,
        }
    }

    pub const fn default_callback_time_zone(self) -> LogDefaultCallbackTimeZone {
        self.default_callback_time_zone
    }
}

impl From<LogFlags> for LogFormatOptions {
    fn from(flags: LogFlags) -> Self {
        Self::new(flags)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogDefaultCallbackTimeZone {
    FixedOffsetSeconds(i32),
    PosixDst {
        standard_offset_seconds: i32,
        daylight_offset_seconds: i32,
        start: PosixDstTransition,
        end: PosixDstTransition,
    },
}

impl LogDefaultCallbackTimeZone {
    pub const fn fixed_offset_seconds(offset_seconds: i32) -> Self {
        Self::FixedOffsetSeconds(offset_seconds)
    }

    pub const fn posix_dst(
        standard_offset_seconds: i32,
        daylight_offset_seconds: i32,
        start: PosixDstTransition,
        end: PosixDstTransition,
    ) -> Self {
        Self::PosixDst {
            standard_offset_seconds,
            daylight_offset_seconds,
            start,
            end,
        }
    }

    pub fn offset_seconds_for_timestamp(self, timestamp: LogTimestamp) -> Option<i32> {
        match self {
            Self::FixedOffsetSeconds(offset_seconds) => Some(offset_seconds),
            Self::PosixDst {
                standard_offset_seconds,
                daylight_offset_seconds,
                start,
                end,
            } => {
                let (year, _, _, _, _, _, _) = timestamp.parts_utc();
                let start_utc = start
                    .local_unix_seconds_for_year(year)?
                    .checked_sub(i64::from(standard_offset_seconds))?;
                let end_utc = end
                    .local_unix_seconds_for_year(year)?
                    .checked_sub(i64::from(daylight_offset_seconds))?;
                let utc_seconds = timestamp.unix_micros().div_euclid(1_000_000);

                let is_daylight = if start_utc <= end_utc {
                    utc_seconds >= start_utc && utc_seconds < end_utc
                } else {
                    utc_seconds >= start_utc || utc_seconds < end_utc
                };
                Some(if is_daylight {
                    daylight_offset_seconds
                } else {
                    standard_offset_seconds
                })
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PosixDstTransition {
    month: u8,
    week: u8,
    weekday: u8,
    seconds: u32,
}

impl PosixDstTransition {
    pub const fn month_week_weekday(month: u8, week: u8, weekday: u8) -> Option<Self> {
        Self::month_week_weekday_time(month, week, weekday, 2 * 3_600)
    }

    pub const fn month_week_weekday_time(
        month: u8,
        week: u8,
        weekday: u8,
        seconds: u32,
    ) -> Option<Self> {
        if month == 0 || month > 12 || week == 0 || week > 5 || weekday > 6 || seconds >= 86_400 {
            return None;
        }
        Some(Self {
            month,
            week,
            weekday,
            seconds,
        })
    }

    pub const fn month(self) -> u8 {
        self.month
    }

    pub const fn week(self) -> u8 {
        self.week
    }

    pub const fn weekday(self) -> u8 {
        self.weekday
    }

    pub const fn seconds(self) -> u32 {
        self.seconds
    }

    fn local_unix_seconds_for_year(self, year: i64) -> Option<i64> {
        let month = i64::from(self.month);
        let first_day = unix_days_from_civil(year, month, 1)?;
        let first_weekday = i64::from(weekday_from_unix_days(first_day));
        let target_weekday = i64::from(self.weekday);
        let days_in_month = days_in_month(year, month)?;
        let day = if self.week == 5 {
            let last_day = unix_days_from_civil(year, month, days_in_month)?;
            let last_weekday = i64::from(weekday_from_unix_days(last_day));
            days_in_month - (last_weekday - target_weekday).rem_euclid(7)
        } else {
            1 + (target_weekday - first_weekday).rem_euclid(7) + 7 * (i64::from(self.week) - 1)
        };
        if day < 1 || day > days_in_month {
            return None;
        }
        unix_days_from_civil(year, month, day)?
            .checked_mul(86_400)?
            .checked_add(i64::from(self.seconds))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvLogFormatLine {
    line: Vec<u8>,
    truncated: bool,
}

impl AvLogFormatLine {
    pub fn new(line: Vec<u8>, truncated: bool) -> Self {
        Self { line, truncated }
    }

    pub fn bytes(&self) -> &[u8] {
        &self.line
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.line
    }

    pub fn as_utf8(&self) -> Option<&str> {
        std::str::from_utf8(&self.line).ok()
    }

    pub fn line_lossy(&self) -> String {
        String::from_utf8_lossy(&self.line).into_owned()
    }

    pub const fn truncated(&self) -> bool {
        self.truncated
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvLogFormatLine2 {
    line: Vec<u8>,
    full_len: usize,
    truncated: bool,
}

impl AvLogFormatLine2 {
    pub fn new(line: Vec<u8>, full_len: usize, truncated: bool) -> Self {
        Self {
            line,
            full_len,
            truncated,
        }
    }

    pub fn bytes(&self) -> &[u8] {
        &self.line
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.line
    }

    pub fn as_utf8(&self) -> Option<&str> {
        std::str::from_utf8(&self.line).ok()
    }

    pub fn line_lossy(&self) -> String {
        String::from_utf8_lossy(&self.line).into_owned()
    }

    pub const fn full_len(&self) -> usize {
        self.full_len
    }

    pub const fn truncated(&self) -> bool {
        self.truncated
    }
}

impl From<AvLogFormatLine2> for AvLogFormatLine {
    fn from(line2: AvLogFormatLine2) -> Self {
        let truncated = line2.truncated();
        Self::new(line2.into_bytes(), truncated)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvLogContextPrefix {
    item_name: String,
    address: String,
}

impl AvLogContextPrefix {
    pub fn new(item_name: impl Into<String>, address: impl Into<String>) -> Self {
        Self {
            item_name: item_name.into(),
            address: address.into(),
        }
    }

    pub fn item_name(&self) -> &str {
        &self.item_name
    }

    pub fn address(&self) -> &str {
        &self.address
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

    pub fn format_default_callback_time_utc(self) -> String {
        let (_, _, _, hour, minute, second, micros) = self.parts_utc();
        let millis = micros / 1_000;
        format!("{hour:02}:{minute:02}:{second:02}.{millis:03}")
    }

    pub fn format_default_callback_datetime_utc(self) -> String {
        let (year, month, day, hour, minute, second, micros) = self.parts_utc();
        let millis = micros / 1_000;
        format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02}.{millis:03}")
    }

    pub fn format_default_callback_time_with_offset_seconds(
        self,
        offset_seconds: i32,
    ) -> Option<String> {
        let (_, _, _, hour, minute, second, micros) =
            self.parts_with_offset_seconds(offset_seconds)?;
        let millis = micros / 1_000;
        Some(format!("{hour:02}:{minute:02}:{second:02}.{millis:03}"))
    }

    pub fn format_default_callback_datetime_with_offset_seconds(
        self,
        offset_seconds: i32,
    ) -> Option<String> {
        let (year, month, day, hour, minute, second, micros) =
            self.parts_with_offset_seconds(offset_seconds)?;
        let millis = micros / 1_000;
        Some(format!(
            "{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02}.{millis:03}"
        ))
    }

    pub fn format_default_callback_time_with_time_zone(
        self,
        time_zone: LogDefaultCallbackTimeZone,
    ) -> Option<String> {
        self.format_default_callback_time_with_offset_seconds(
            time_zone.offset_seconds_for_timestamp(self)?,
        )
    }

    pub fn format_default_callback_datetime_with_time_zone(
        self,
        time_zone: LogDefaultCallbackTimeZone,
    ) -> Option<String> {
        self.format_default_callback_datetime_with_offset_seconds(
            time_zone.offset_seconds_for_timestamp(self)?,
        )
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

    fn parts_with_offset_seconds(
        self,
        offset_seconds: i32,
    ) -> Option<(i64, i64, i64, i64, i64, i64, i64)> {
        let offset_micros = i64::from(offset_seconds).checked_mul(1_000_000)?;
        Some(Self::from_unix_micros(self.unix_micros.checked_add(offset_micros)?).parts_utc())
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

fn unix_days_from_civil(year: i64, month: i64, day: i64) -> Option<i64> {
    if !(1..=12).contains(&month) {
        return None;
    }
    let days_in_month = days_in_month(year, month)?;
    if day < 1 || day > days_in_month {
        return None;
    }

    let adjusted_year = year - if month <= 2 { 1 } else { 0 };
    let era = (if adjusted_year >= 0 {
        adjusted_year
    } else {
        adjusted_year - 399
    })
    .div_euclid(400);
    let year_of_era = adjusted_year - era * 400;
    let month_param = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * month_param + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    Some(era * 146_097 + day_of_era - 719_468)
}

fn days_in_month(year: i64, month: i64) -> Option<i64> {
    Some(match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => return None,
    })
}

fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn weekday_from_unix_days(days: i64) -> u8 {
    (days + 4).rem_euclid(7) as u8
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogRecord {
    level: LogLevel,
    raw_level: i32,
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
            raw_level: level.as_ffmpeg_value(),
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

    pub fn with_level(mut self, level: LogLevel) -> Self {
        self.level = level;
        self.raw_level = level.as_ffmpeg_value();
        self
    }

    pub fn with_raw_level(mut self, raw_level: i32) -> Self {
        self.raw_level = raw_level;
        if let Some(level) = LogLevel::from_ffmpeg_value(raw_level) {
            self.level = level;
        }
        self
    }

    fn repetition_summary(count: usize) -> Self {
        Self {
            level: LogLevel::Info,
            raw_level: LogLevel::Info.as_ffmpeg_value(),
            target: String::new(),
            message: format!("Last message repeated {count} times"),
            timestamp: None,
            kind: LogRecordKind::RepetitionSummary { count },
        }
    }

    pub fn level(&self) -> LogLevel {
        self.level
    }

    pub fn raw_level(&self) -> i32 {
        self.raw_level
    }

    pub fn known_level(&self) -> Option<LogLevel> {
        LogLevel::from_ffmpeg_value(self.raw_level)
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
        if matches!(
            options.color_mode(),
            LogColorMode::Basic | LogColorMode::Always
        ) {
            if let Some(color_code) = self.ansi_color_code() {
                return format!("{color_code}{line}\x1b[0m");
            }
        }
        line
    }

    pub fn format_default_callback_line_null_context_with_flags(&self, flags: LogFlags) -> String {
        self.format_default_callback_line_with_context(None, LogFormatOptions::new(flags))
    }

    pub fn format_default_callback_line_context_with_flags(
        &self,
        context: &AvLogContextPrefix,
        flags: LogFlags,
    ) -> String {
        self.format_default_callback_line_with_context(Some(context), LogFormatOptions::new(flags))
    }

    pub fn format_default_callback_line_null_context_with_options(
        &self,
        options: LogFormatOptions,
    ) -> String {
        self.format_default_callback_line_with_context(None, options)
    }

    pub fn format_default_callback_line_context_with_options(
        &self,
        context: &AvLogContextPrefix,
        options: LogFormatOptions,
    ) -> String {
        self.format_default_callback_line_with_context(Some(context), options)
    }

    pub fn format_default_callback_line_null_context_with_state(
        &self,
        flags: LogFlags,
        state: &mut DefaultCallbackPrefixState,
    ) -> String {
        self.format_default_callback_line_null_context_with_options_and_state(
            LogFormatOptions::new(flags),
            state,
        )
    }

    pub fn format_default_callback_line_context_with_state(
        &self,
        context: &AvLogContextPrefix,
        flags: LogFlags,
        state: &mut DefaultCallbackPrefixState,
    ) -> String {
        self.format_default_callback_line_context_with_options_and_state(
            context,
            LogFormatOptions::new(flags),
            state,
        )
    }

    pub fn format_default_callback_line_null_context_with_options_and_state(
        &self,
        options: LogFormatOptions,
        state: &mut DefaultCallbackPrefixState,
    ) -> String {
        self.format_default_callback_line_with_context_and_state(None, options, state)
    }

    pub fn format_default_callback_line_context_with_options_and_state(
        &self,
        context: &AvLogContextPrefix,
        options: LogFormatOptions,
        state: &mut DefaultCallbackPrefixState,
    ) -> String {
        self.format_default_callback_line_with_context_and_state(Some(context), options, state)
    }

    fn format_default_callback_line_with_context(
        &self,
        context: Option<&AvLogContextPrefix>,
        options: LogFormatOptions,
    ) -> String {
        self.format_default_callback_line_with_context_and_prefix(context, options, true)
            .0
    }

    fn format_default_callback_line_with_context_and_state(
        &self,
        context: Option<&AvLogContextPrefix>,
        options: LogFormatOptions,
        state: &mut DefaultCallbackPrefixState,
    ) -> String {
        let (line, next_print_prefix) = self.format_default_callback_line_with_context_and_prefix(
            context,
            options,
            state.print_prefix(),
        );
        state.set_print_prefix(next_print_prefix);
        line
    }

    fn format_default_callback_line_with_context_and_prefix(
        &self,
        context: Option<&AvLogContextPrefix>,
        options: LogFormatOptions,
        print_prefix: bool,
    ) -> (String, bool) {
        if self.is_repetition_summary() {
            return (format!("    {}\n", self.message), true);
        }

        let flags = options.flags();
        let severity_color = match options.color_mode() {
            LogColorMode::Never => None,
            LogColorMode::Basic => self.default_callback_basic_ansi_color_code(),
            LogColorMode::Always => match self.level {
                LogLevel::Quiet => Some(DEFAULT_CALLBACK_PANIC_COLOR),
                _ => self.default_callback_ansi_color_code(),
            },
        };
        let mut line = String::new();
        if print_prefix {
            if let Some(timestamp) = self.timestamp {
                if self.level.permits_prefix_fields() && flags.contains(LogFlags::PRINT_DATETIME) {
                    if let Some(formatted) = timestamp
                        .format_default_callback_datetime_with_time_zone(
                            options.default_callback_time_zone(),
                        )
                    {
                        line.push_str(&formatted);
                        line.push(' ');
                    }
                } else if self.level.permits_prefix_fields() && flags.contains(LogFlags::PRINT_TIME)
                {
                    if let Some(formatted) = timestamp.format_default_callback_time_with_time_zone(
                        options.default_callback_time_zone(),
                    ) {
                        line.push_str(&formatted);
                        line.push(' ');
                    }
                }
            }
            if let Some(context) = context {
                let context_prefix = format!("[{} @ {}] ", context.item_name(), context.address());
                match options.color_mode() {
                    LogColorMode::Never => line.push_str(&context_prefix),
                    LogColorMode::Basic => {
                        line.push_str(&colorize(
                            DEFAULT_CALLBACK_BASIC_CONTEXT_COLOR,
                            &context_prefix,
                        ));
                    }
                    LogColorMode::Always => {
                        line.push_str(&colorize(DEFAULT_CALLBACK_CONTEXT_COLOR, &context_prefix));
                    }
                }
            }
            if self.level.permits_prefix_fields() && flags.contains(LogFlags::PRINT_LEVEL) {
                let level_prefix = format!("[{}] ", self.level.name());
                if let Some(color_code) = severity_color {
                    line.push_str(&colorize(color_code, &level_prefix));
                } else {
                    line.push_str(&level_prefix);
                }
            }
        }
        if let Some(color_code) = severity_color {
            line.push_str(&colorize(color_code, &self.message));
        } else {
            line.push_str(&self.message);
        }
        let next_print_prefix =
            matches!(self.message.as_bytes().last().copied(), Some(b'\n' | b'\r'));
        (line, next_print_prefix)
    }

    pub fn format_av_log_line_null_context(
        &self,
        flags: LogFlags,
        print_prefix: &mut bool,
        line_size: usize,
    ) -> AvResult<AvLogFormatLine> {
        self.format_av_log_line2_null_context(flags, print_prefix, line_size)
            .map(AvLogFormatLine::from)
    }

    pub fn format_av_log_line_context(
        &self,
        context: &AvLogContextPrefix,
        flags: LogFlags,
        print_prefix: &mut bool,
        line_size: usize,
    ) -> AvResult<AvLogFormatLine> {
        self.format_av_log_line2_context(context, flags, print_prefix, line_size)
            .map(AvLogFormatLine::from)
    }

    pub fn format_av_log_line2_null_context(
        &self,
        flags: LogFlags,
        print_prefix: &mut bool,
        line_size: usize,
    ) -> AvResult<AvLogFormatLine2> {
        self.format_av_log_line2_with_context(None, flags, print_prefix, line_size)
    }

    pub fn format_av_log_line2_context(
        &self,
        context: &AvLogContextPrefix,
        flags: LogFlags,
        print_prefix: &mut bool,
        line_size: usize,
    ) -> AvResult<AvLogFormatLine2> {
        self.format_av_log_line2_with_context(Some(context), flags, print_prefix, line_size)
    }

    fn format_av_log_line2_with_context(
        &self,
        context: Option<&AvLogContextPrefix>,
        flags: LogFlags,
        print_prefix: &mut bool,
        line_size: usize,
    ) -> AvResult<AvLogFormatLine2> {
        let mut full_line = String::new();
        if *print_prefix {
            if let Some(context) = context {
                full_line.push('[');
                full_line.push_str(context.item_name());
                full_line.push_str(" @ ");
                full_line.push_str(context.address());
                full_line.push_str("] ");
            }
            if self.level.permits_prefix_fields() && flags.contains(LogFlags::PRINT_LEVEL) {
                full_line.push('[');
                full_line.push_str(self.level.name());
                full_line.push_str("] ");
            }
        }
        full_line.push_str(&self.message);
        let full_bytes = full_line.into_bytes();
        let full_len = full_bytes.len();
        let copied_len = line_size.saturating_sub(1).min(full_len);
        let line = full_bytes[..copied_len].to_vec();
        *print_prefix = matches!(line.last().copied(), Some(b'\n' | b'\r'));
        let truncated = full_len >= line_size;

        Ok(AvLogFormatLine2::new(line, full_len, truncated))
    }

    fn format_plain_line_with_flags(&self, flags: LogFlags) -> String {
        if self.is_repetition_summary() {
            return self.message.clone();
        }

        let time_prefix = if !self.level.permits_prefix_fields() {
            String::new()
        } else if flags.contains(LogFlags::PRINT_DATETIME) {
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
        let prefix = if self.level.permits_prefix_fields() && flags.contains(LogFlags::PRINT_LEVEL)
        {
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

    fn default_callback_ansi_color_code(&self) -> Option<&'static str> {
        match self.level {
            LogLevel::Panic => Some(DEFAULT_CALLBACK_PANIC_COLOR),
            LogLevel::Fatal => Some(DEFAULT_CALLBACK_FATAL_COLOR),
            LogLevel::Error => Some(DEFAULT_CALLBACK_ERROR_COLOR),
            LogLevel::Warning => Some(DEFAULT_CALLBACK_WARNING_COLOR),
            LogLevel::Quiet
            | LogLevel::Info
            | LogLevel::Verbose
            | LogLevel::Debug
            | LogLevel::Trace => None,
        }
    }

    fn default_callback_basic_ansi_color_code(&self) -> Option<&'static str> {
        match self.level {
            LogLevel::Panic | LogLevel::Fatal | LogLevel::Quiet => {
                Some(DEFAULT_CALLBACK_BASIC_FATAL_COLOR)
            }
            LogLevel::Error => Some(DEFAULT_CALLBACK_BASIC_ERROR_COLOR),
            LogLevel::Warning => Some(DEFAULT_CALLBACK_BASIC_WARNING_COLOR),
            LogLevel::Info | LogLevel::Verbose | LogLevel::Debug | LogLevel::Trace => None,
        }
    }

    fn same_message_as(&self, other: &Self, flags: LogFlags) -> bool {
        self.kind == LogRecordKind::Message
            && other.kind == LogRecordKind::Message
            && self.level == other.level
            && self.raw_level == other.raw_level
            && self.target == other.target
            && self.message == other.message
            && (!flags.intersects(LogFlags::PRINT_TIME | LogFlags::PRINT_DATETIME)
                || self.timestamp == other.timestamp)
    }
}

impl core::ops::BitOr for LogFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self::from_bits_retain(self.bits | rhs.bits)
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
    level: i32,
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
        Self::new_with_raw_level(level.as_ffmpeg_value(), flags)
    }

    pub fn new_with_raw_level(raw_level: i32, flags: LogFlags) -> Self {
        Self {
            level: raw_level,
            flags,
            records: Vec::new(),
            repeated: None,
            callback: None,
        }
    }

    pub fn level(&self) -> LogLevel {
        LogLevel::from_ffmpeg_value(self.level).unwrap_or(LogLevel::Trace)
    }

    pub fn known_level(&self) -> Option<LogLevel> {
        LogLevel::from_ffmpeg_value(self.level)
    }

    pub fn raw_level(&self) -> i32 {
        self.level
    }

    pub fn set_level(&mut self, level: LogLevel) {
        self.set_raw_level(level.as_ffmpeg_value());
    }

    pub fn set_raw_level(&mut self, raw_level: i32) {
        self.level = raw_level;
    }

    pub fn flags(&self) -> LogFlags {
        self.flags
    }

    pub fn set_flags(&mut self, flags: LogFlags) {
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
        level.as_ffmpeg_value() <= self.level
    }

    pub fn enabled_raw(&self, raw_level: i32) -> bool {
        raw_level <= self.level
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
        if self.enabled_raw(record.raw_level()) {
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
        if self.enabled_raw(record.raw_level()) {
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

    pub fn log_custom_callback<F>(&mut self, record: LogRecord, callback: F)
    where
        F: FnOnce(&LogRecord),
    {
        self.records.push(record);
        let record_index = self.records.len() - 1;
        callback(&self.records[record_index]);
    }

    pub fn log_once(
        &mut self,
        state: &mut LogOnceState,
        record: LogRecord,
        subsequent_level: LogLevel,
    ) -> bool {
        let level = if state.has_logged() {
            subsequent_level
        } else {
            record.level()
        };
        let logged = self.log(record.with_level(level));
        state.mark_logged();
        logged
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

pub fn global_known_log_level() -> Option<LogLevel> {
    global_logger().lock().unwrap().known_level()
}

pub fn global_raw_log_level() -> i32 {
    global_logger().lock().unwrap().raw_level()
}

pub fn set_global_log_level(level: LogLevel) {
    global_logger().lock().unwrap().set_level(level);
}

pub fn set_global_raw_log_level(raw_level: i32) {
    global_logger().lock().unwrap().set_raw_level(raw_level);
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

pub fn global_log_once(
    state: &mut LogOnceState,
    record: LogRecord,
    subsequent_level: LogLevel,
) -> bool {
    global_logger()
        .lock()
        .unwrap()
        .log_once(state, record, subsequent_level)
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
    fn quiet_threshold_suppresses_ordinary_records_but_accepts_quiet_level() {
        let mut logger = Logger::new(LogLevel::Quiet);

        assert!(logger.log(LogRecord::new(LogLevel::Quiet, "ffmpeg", "kept")));
        assert!(!logger.log(LogRecord::new(LogLevel::Fatal, "ffmpeg", "ignored")));

        assert_eq!(logger.records().len(), 1);
        assert_eq!(logger.records()[0].message(), "kept");
    }

    #[test]
    fn raw_level_thresholds_match_ffmpeg_integer_filtering() {
        let mut logger = Logger::new_with_raw_level(23, LogFlags::PRINT_LEVEL);

        assert_eq!(logger.raw_level(), 23);
        assert_eq!(logger.known_level(), None);
        assert!(logger.enabled_raw(23));
        assert!(!logger.enabled_raw(24));
        assert!(logger.enabled(LogLevel::Error));
        assert!(!logger.enabled(LogLevel::Warning));
        assert!(logger.log(LogRecord::new(LogLevel::Error, "ffmpeg", "raw shown\n")));
        assert!(!logger.log(LogRecord::new(LogLevel::Warning, "ffmpeg", "raw hidden\n")));
        assert_eq!(logger.records().len(), 1);
        assert_eq!(
            logger.records()[0]
                .format_default_callback_line_null_context_with_flags(LogFlags::PRINT_LEVEL),
            "[error] raw shown\n"
        );

        assert!(logger.log(
            LogRecord::new(LogLevel::Warning, "ffmpeg", "raw record shown\n").with_raw_level(23)
        ));
        assert_eq!(logger.records().len(), 2);
        assert_eq!(logger.records()[1].level(), LogLevel::Warning);
        assert_eq!(logger.records()[1].raw_level(), 23);
        assert_eq!(logger.records()[1].known_level(), None);

        logger.set_raw_level(-1);
        assert_eq!(logger.raw_level(), -1);
        assert!(logger.enabled(LogLevel::Quiet));
        assert!(!logger.enabled(LogLevel::Panic));
        assert!(logger.log(LogRecord::new(LogLevel::Quiet, "ffmpeg", "quiet\n")));
        assert!(!logger.log(LogRecord::new(LogLevel::Panic, "ffmpeg", "panic hidden\n")));

        logger.set_level(LogLevel::Warning);
        assert_eq!(logger.raw_level(), LogLevel::Warning.as_ffmpeg_value());
        assert_eq!(logger.known_level(), Some(LogLevel::Warning));
        assert_eq!(logger.level(), LogLevel::Warning);
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
    fn log_flags_track_known_ffmpeg_bits_and_preserve_raw_state() {
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

        let retained = LogFlags::from_bits_retain(0x1234);
        assert_eq!(retained.bits(), 0x1234);
        assert!(retained.contains(LogFlags::PRINT_TIME));
        assert!(!retained.contains(LogFlags::PRINT_LEVEL));
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
        assert_eq!(timestamp.format_default_callback_time_utc(), "12:38:25.123");
        assert_eq!(
            timestamp.format_default_callback_datetime_utc(),
            "2024-01-01 12:38:25.123"
        );
        assert_eq!(
            timestamp.format_default_callback_time_with_offset_seconds(2 * 3_600),
            Some("14:38:25.123".to_string())
        );
        assert_eq!(
            timestamp.format_default_callback_datetime_with_offset_seconds(5 * 3_600 + 30 * 60),
            Some("2024-01-01 18:08:25.123".to_string())
        );
        assert_eq!(
            LogTimestamp::from_unix_micros(1_704_070_923_456_789)
                .format_default_callback_datetime_with_offset_seconds(-8 * 3_600),
            Some("2023-12-31 17:02:03.456".to_string())
        );

        let before_epoch = LogTimestamp::from_unix_micros(-1);
        assert_eq!(
            before_epoch.format_datetime_utc(),
            "1969-12-31 23:59:59.999999"
        );
        assert_eq!(
            before_epoch.format_default_callback_datetime_utc(),
            "1969-12-31 23:59:59.999"
        );
    }

    #[test]
    fn log_timestamps_format_posix_dst_default_callback_time_zone() {
        let pacific = LogDefaultCallbackTimeZone::posix_dst(
            -8 * 3_600,
            -7 * 3_600,
            PosixDstTransition::month_week_weekday(3, 2, 0).unwrap(),
            PosixDstTransition::month_week_weekday(11, 1, 0).unwrap(),
        );

        assert_eq!(PosixDstTransition::month_week_weekday(0, 2, 0), None);
        assert_eq!(PosixDstTransition::month_week_weekday(13, 2, 0), None);
        assert_eq!(PosixDstTransition::month_week_weekday(3, 0, 0), None);
        assert_eq!(PosixDstTransition::month_week_weekday(3, 6, 0), None);
        assert_eq!(PosixDstTransition::month_week_weekday(3, 2, 7), None);
        assert_eq!(
            PosixDstTransition::month_week_weekday_time(3, 2, 0, 86_400),
            None
        );

        assert_eq!(
            LogTimestamp::from_unix_micros(1_710_064_799_123_456)
                .format_default_callback_datetime_with_time_zone(pacific),
            Some("2024-03-10 01:59:59.123".to_string())
        );
        assert_eq!(
            LogTimestamp::from_unix_micros(1_710_064_800_123_456)
                .format_default_callback_datetime_with_time_zone(pacific),
            Some("2024-03-10 03:00:00.123".to_string())
        );
        assert_eq!(
            LogTimestamp::from_unix_micros(1_730_624_399_123_456)
                .format_default_callback_datetime_with_time_zone(pacific),
            Some("2024-11-03 01:59:59.123".to_string())
        );
        assert_eq!(
            LogTimestamp::from_unix_micros(1_730_624_400_123_456)
                .format_default_callback_datetime_with_time_zone(pacific),
            Some("2024-11-03 01:00:00.123".to_string())
        );
        assert_eq!(
            LogTimestamp::from_unix_micros(1_710_064_800_123_456)
                .format_default_callback_time_with_time_zone(pacific),
            Some("03:00:00.123".to_string())
        );

        let eastern_australia = LogDefaultCallbackTimeZone::posix_dst(
            10 * 3_600,
            11 * 3_600,
            PosixDstTransition::month_week_weekday(10, 1, 0).unwrap(),
            PosixDstTransition::month_week_weekday_time(4, 1, 0, 3 * 3_600).unwrap(),
        );
        assert_eq!(
            LogTimestamp::from_unix_micros(1_728_143_999_123_456)
                .format_default_callback_datetime_with_time_zone(eastern_australia),
            Some("2024-10-06 01:59:59.123".to_string())
        );
        assert_eq!(
            LogTimestamp::from_unix_micros(1_728_144_000_123_456)
                .format_default_callback_datetime_with_time_zone(eastern_australia),
            Some("2024-10-06 03:00:00.123".to_string())
        );
        assert_eq!(
            LogTimestamp::from_unix_micros(1_712_419_199_123_456)
                .format_default_callback_datetime_with_time_zone(eastern_australia),
            Some("2024-04-07 02:59:59.123".to_string())
        );
        assert_eq!(
            LogTimestamp::from_unix_micros(1_712_419_200_123_456)
                .format_default_callback_datetime_with_time_zone(eastern_australia),
            Some("2024-04-07 02:00:00.123".to_string())
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
    fn default_callback_formatting_uses_ffmpeg_timestamp_shape() {
        let timestamp = LogTimestamp::from_unix_micros(1_704_112_705_123_456);
        let record =
            LogRecord::new(LogLevel::Warning, "ignored", "plain\n").with_timestamp(timestamp);
        let context = AvLogContextPrefix::new("rustctx", "<ptr>");

        assert_eq!(
            record.format_default_callback_line_null_context_with_flags(LogFlags::PRINT_TIME),
            "12:38:25.123 plain\n"
        );
        assert_eq!(
            record.format_default_callback_line_null_context_with_flags(
                LogFlags::PRINT_TIME | LogFlags::PRINT_LEVEL
            ),
            "12:38:25.123 [warning] plain\n"
        );
        assert_eq!(
            record.format_default_callback_line_null_context_with_flags(
                LogFlags::PRINT_DATETIME | LogFlags::PRINT_LEVEL
            ),
            "2024-01-01 12:38:25.123 [warning] plain\n"
        );
        assert_eq!(
            record.format_default_callback_line_null_context_with_flags(
                LogFlags::PRINT_TIME | LogFlags::PRINT_DATETIME | LogFlags::PRINT_LEVEL
            ),
            "2024-01-01 12:38:25.123 [warning] plain\n"
        );
        assert_eq!(
            LogRecord::new(LogLevel::Warning, "ignored", "plain\n")
                .format_default_callback_line_null_context_with_flags(
                    LogFlags::PRINT_TIME | LogFlags::PRINT_LEVEL
                ),
            "[warning] plain\n"
        );
        assert_eq!(
            LogRecord::new(LogLevel::Warning, "ignored", "ctxmsg\n")
                .format_default_callback_line_context_with_flags(&context, LogFlags::empty()),
            "[rustctx @ <ptr>] ctxmsg\n"
        );
        assert_eq!(
            LogRecord::new(LogLevel::Warning, "ignored", "ctxmsg\n")
                .format_default_callback_line_context_with_flags(&context, LogFlags::PRINT_LEVEL),
            "[rustctx @ <ptr>] [warning] ctxmsg\n"
        );
        assert_eq!(
            LogRecord::new(LogLevel::Warning, "ignored", "ctxmsg\n")
                .with_timestamp(timestamp)
                .format_default_callback_line_context_with_flags(
                    &context,
                    LogFlags::PRINT_TIME | LogFlags::PRINT_LEVEL
                ),
            "12:38:25.123 [rustctx @ <ptr>] [warning] ctxmsg\n"
        );
        assert_eq!(
            LogRecord::new(LogLevel::Warning, "ignored", "ctxmsg\n")
                .with_timestamp(timestamp)
                .format_default_callback_line_context_with_flags(
                    &context,
                    LogFlags::PRINT_DATETIME | LogFlags::PRINT_LEVEL
                ),
            "2024-01-01 12:38:25.123 [rustctx @ <ptr>] [warning] ctxmsg\n"
        );

        let utc_plus_two = LogFormatOptions::new(LogFlags::PRINT_TIME)
            .with_default_callback_time_offset_seconds(2 * 3_600);
        assert_eq!(
            record.format_default_callback_line_null_context_with_options(utc_plus_two),
            "14:38:25.123 plain\n"
        );

        let utc_plus_five_thirty =
            LogFormatOptions::new(LogFlags::PRINT_DATETIME | LogFlags::PRINT_LEVEL)
                .with_default_callback_time_offset_seconds(5 * 3_600 + 30 * 60);
        assert_eq!(
            record.format_default_callback_line_null_context_with_options(utc_plus_five_thirty),
            "2024-01-01 18:08:25.123 [warning] plain\n"
        );

        let utc_minus_eight =
            LogFormatOptions::new(LogFlags::PRINT_DATETIME | LogFlags::PRINT_LEVEL)
                .with_default_callback_time_offset_seconds(-8 * 3_600);
        assert_eq!(
            LogRecord::new(LogLevel::Warning, "ignored", "local\n")
                .with_timestamp(LogTimestamp::from_unix_micros(1_704_070_923_456_789))
                .format_default_callback_line_null_context_with_options(utc_minus_eight),
            "2023-12-31 17:02:03.456 [warning] local\n"
        );

        let pacific = LogDefaultCallbackTimeZone::posix_dst(
            -8 * 3_600,
            -7 * 3_600,
            PosixDstTransition::month_week_weekday(3, 2, 0).unwrap(),
            PosixDstTransition::month_week_weekday(11, 1, 0).unwrap(),
        );
        let pacific_options =
            LogFormatOptions::new(LogFlags::PRINT_DATETIME | LogFlags::PRINT_LEVEL)
                .with_default_callback_time_zone(pacific);
        assert_eq!(
            LogRecord::new(LogLevel::Warning, "ignored", "dst\n")
                .with_timestamp(LogTimestamp::from_unix_micros(1_710_064_799_123_456))
                .format_default_callback_line_null_context_with_options(pacific_options),
            "2024-03-10 01:59:59.123 [warning] dst\n"
        );
        assert_eq!(
            LogRecord::new(LogLevel::Warning, "ignored", "dst\n")
                .with_timestamp(LogTimestamp::from_unix_micros(1_710_064_800_123_456))
                .format_default_callback_line_null_context_with_options(pacific_options),
            "2024-03-10 03:00:00.123 [warning] dst\n"
        );
        assert_eq!(
            LogRecord::new(LogLevel::Warning, "ignored", "dst\n")
                .with_timestamp(LogTimestamp::from_unix_micros(1_730_624_399_123_456))
                .format_default_callback_line_null_context_with_options(pacific_options),
            "2024-11-03 01:59:59.123 [warning] dst\n"
        );
        assert_eq!(
            LogRecord::new(LogLevel::Warning, "ignored", "dst\n")
                .with_timestamp(LogTimestamp::from_unix_micros(1_730_624_400_123_456))
                .format_default_callback_line_null_context_with_options(pacific_options),
            "2024-11-03 01:00:00.123 [warning] dst\n"
        );

        let eastern_australia = LogDefaultCallbackTimeZone::posix_dst(
            10 * 3_600,
            11 * 3_600,
            PosixDstTransition::month_week_weekday(10, 1, 0).unwrap(),
            PosixDstTransition::month_week_weekday_time(4, 1, 0, 3 * 3_600).unwrap(),
        );
        let eastern_australia_options =
            LogFormatOptions::new(LogFlags::PRINT_DATETIME | LogFlags::PRINT_LEVEL)
                .with_default_callback_time_zone(eastern_australia);
        for (timestamp, expected) in [
            (
                1_728_143_999_123_456,
                "2024-10-06 01:59:59.123 [warning] dst\n",
            ),
            (
                1_728_144_000_123_456,
                "2024-10-06 03:00:00.123 [warning] dst\n",
            ),
            (
                1_712_419_199_123_456,
                "2024-04-07 02:59:59.123 [warning] dst\n",
            ),
            (
                1_712_419_200_123_456,
                "2024-04-07 02:00:00.123 [warning] dst\n",
            ),
        ] {
            assert_eq!(
                LogRecord::new(LogLevel::Warning, "ignored", "dst\n")
                    .with_timestamp(LogTimestamp::from_unix_micros(timestamp))
                    .format_default_callback_line_null_context_with_options(
                        eastern_australia_options,
                    ),
                expected
            );
        }
        let quiet = LogRecord::new(LogLevel::Quiet, "ignored", "quiet\n").with_timestamp(timestamp);
        assert_eq!(
            quiet.format_default_callback_line_null_context_with_flags(
                LogFlags::PRINT_TIME | LogFlags::PRINT_LEVEL
            ),
            "quiet\n"
        );
        assert_eq!(
            quiet.format_default_callback_line_null_context_with_flags(
                LogFlags::PRINT_DATETIME | LogFlags::PRINT_LEVEL
            ),
            "quiet\n"
        );
        assert_eq!(
            quiet.format_default_callback_line_context_with_flags(
                &context,
                LogFlags::PRINT_DATETIME | LogFlags::PRINT_LEVEL
            ),
            "[rustctx @ <ptr>] quiet\n"
        );

        let force_color =
            LogFormatOptions::new(LogFlags::empty()).with_color_mode(LogColorMode::Always);
        assert_eq!(
            LogRecord::new(LogLevel::Warning, "ignored", "plain\n")
                .format_default_callback_line_null_context_with_options(force_color),
            "\x1b[48;5;0m\x1b[38;5;226mplain\n\x1b[0m"
        );
        assert_eq!(
            LogRecord::new(LogLevel::Error, "ignored", "plain\n")
                .format_default_callback_line_null_context_with_options(force_color),
            "\x1b[48;5;0m\x1b[38;5;196mplain\n\x1b[0m"
        );
        assert_eq!(
            LogRecord::new(LogLevel::Info, "ignored", "plain\n")
                .format_default_callback_line_null_context_with_options(force_color),
            "plain\n"
        );

        let force_color_level =
            LogFormatOptions::new(LogFlags::PRINT_LEVEL).with_color_mode(LogColorMode::Always);
        assert_eq!(
            LogRecord::new(LogLevel::Warning, "ignored", "plain\n")
                .format_default_callback_line_null_context_with_options(force_color_level),
            "\x1b[48;5;0m\x1b[38;5;226m[warning] \x1b[0m\x1b[48;5;0m\x1b[38;5;226mplain\n\x1b[0m"
        );
        assert_eq!(
            LogRecord::new(LogLevel::Fatal, "ignored", "plain\n")
                .format_default_callback_line_null_context_with_options(force_color_level),
            "\x1b[48;5;0m\x1b[38;5;208m[fatal] \x1b[0m\x1b[48;5;0m\x1b[38;5;208mplain\n\x1b[0m"
        );
        assert_eq!(
            LogRecord::new(LogLevel::Panic, "ignored", "plain\n")
                .format_default_callback_line_null_context_with_options(force_color_level),
            "\x1b[48;5;52m\x1b[38;5;196m[panic] \x1b[0m\x1b[48;5;52m\x1b[38;5;196mplain\n\x1b[0m"
        );
        assert_eq!(
            LogRecord::new(LogLevel::Warning, "ignored", "plain\n")
                .format_default_callback_line_context_with_options(&context, force_color_level),
            "\x1b[48;5;0m\x1b[38;5;250m[rustctx @ <ptr>] \x1b[0m\x1b[48;5;0m\x1b[38;5;226m[warning] \x1b[0m\x1b[48;5;0m\x1b[38;5;226mplain\n\x1b[0m"
        );
        let quiet_force_color = LogFormatOptions::new(LogFlags::PRINT_TIME | LogFlags::PRINT_LEVEL)
            .with_color_mode(LogColorMode::Always);
        assert_eq!(
            quiet.format_default_callback_line_null_context_with_options(quiet_force_color),
            "\x1b[48;5;52m\x1b[38;5;196mquiet\n\x1b[0m"
        );
        assert_eq!(
            quiet.format_default_callback_line_context_with_options(&context, quiet_force_color),
            "\x1b[48;5;0m\x1b[38;5;250m[rustctx @ <ptr>] \x1b[0m\x1b[48;5;52m\x1b[38;5;196mquiet\n\x1b[0m"
        );
    }

    #[test]
    fn default_callback_prefix_state_suppresses_prefix_until_newline() {
        let mut state = DefaultCallbackPrefixState::new();
        let flags = LogFlags::PRINT_LEVEL;

        assert_eq!(
            LogRecord::new(LogLevel::Warning, "ignored", "part")
                .format_default_callback_line_null_context_with_state(flags, &mut state),
            "[warning] part"
        );
        assert!(!state.print_prefix());

        assert_eq!(
            LogRecord::new(LogLevel::Warning, "ignored", "tail\n")
                .format_default_callback_line_null_context_with_state(flags, &mut state),
            "tail\n"
        );
        assert!(state.print_prefix());

        assert_eq!(
            LogRecord::new(LogLevel::Warning, "ignored", "progress\r")
                .format_default_callback_line_null_context_with_state(flags, &mut state),
            "[warning] progress\r"
        );
        assert!(state.print_prefix());

        assert_eq!(
            LogRecord::new(LogLevel::Warning, "ignored", "done\n")
                .format_default_callback_line_null_context_with_state(flags, &mut state),
            "[warning] done\n"
        );
        assert!(state.print_prefix());

        assert_eq!(
            LogRecord::new(LogLevel::Warning, "ignored", "next\n")
                .format_default_callback_line_null_context_with_state(flags, &mut state),
            "[warning] next\n"
        );
        assert!(state.print_prefix());

        let context = AvLogContextPrefix::new("rustctx", "<ptr>");
        let mut context_state = DefaultCallbackPrefixState::new();
        assert_eq!(
            LogRecord::new(LogLevel::Warning, "ignored", "part")
                .format_default_callback_line_context_with_state(
                    &context,
                    flags,
                    &mut context_state
                ),
            "[rustctx @ <ptr>] [warning] part"
        );
        assert!(!context_state.print_prefix());
        assert_eq!(
            LogRecord::new(LogLevel::Warning, "ignored", "tail\n")
                .format_default_callback_line_context_with_state(
                    &context,
                    flags,
                    &mut context_state
                ),
            "tail\n"
        );
        assert!(context_state.print_prefix());

        assert_eq!(
            LogRecord::new(LogLevel::Warning, "ignored", "progress\r")
                .format_default_callback_line_context_with_state(
                    &context,
                    flags,
                    &mut context_state
                ),
            "[rustctx @ <ptr>] [warning] progress\r"
        );
        assert!(context_state.print_prefix());
        assert_eq!(
            LogRecord::new(LogLevel::Warning, "ignored", "done\n")
                .format_default_callback_line_context_with_state(
                    &context,
                    flags,
                    &mut context_state
                ),
            "[rustctx @ <ptr>] [warning] done\n"
        );
        assert!(context_state.print_prefix());

        let mut resumed = DefaultCallbackPrefixState::from_print_prefix(false);
        assert_eq!(
            LogRecord::new(LogLevel::Error, "ignored", "done\n")
                .format_default_callback_line_null_context_with_state(flags, &mut resumed),
            "done\n"
        );
        resumed.reset();
        assert!(resumed.print_prefix());
    }

    #[test]
    fn default_callback_colored_prefix_state_suppresses_prefix_until_newline() {
        let options =
            LogFormatOptions::new(LogFlags::PRINT_LEVEL).with_color_mode(LogColorMode::Always);
        let mut state = DefaultCallbackPrefixState::new();

        assert_eq!(
            LogRecord::new(LogLevel::Warning, "ignored", "part")
                .format_default_callback_line_null_context_with_options_and_state(
                    options, &mut state,
                ),
            "\x1b[48;5;0m\x1b[38;5;226m[warning] \x1b[0m\x1b[48;5;0m\x1b[38;5;226mpart\x1b[0m"
        );
        assert!(!state.print_prefix());
        assert_eq!(
            LogRecord::new(LogLevel::Warning, "ignored", "tail\n")
                .format_default_callback_line_null_context_with_options_and_state(
                    options, &mut state,
                ),
            "\x1b[48;5;0m\x1b[38;5;226mtail\n\x1b[0m"
        );
        assert!(state.print_prefix());
        assert_eq!(
            LogRecord::new(LogLevel::Warning, "ignored", "progress\r")
                .format_default_callback_line_null_context_with_options_and_state(
                    options, &mut state,
                ),
            "\x1b[48;5;0m\x1b[38;5;226m[warning] \x1b[0m\x1b[48;5;0m\x1b[38;5;226mprogress\r\x1b[0m"
        );
        assert!(state.print_prefix());
        assert_eq!(
            LogRecord::new(LogLevel::Warning, "ignored", "done\n")
                .format_default_callback_line_null_context_with_options_and_state(
                    options, &mut state,
                ),
            "\x1b[48;5;0m\x1b[38;5;226m[warning] \x1b[0m\x1b[48;5;0m\x1b[38;5;226mdone\n\x1b[0m"
        );
        assert!(state.print_prefix());

        let context = AvLogContextPrefix::new("rustctx", "<ptr>");
        let mut context_state = DefaultCallbackPrefixState::new();
        assert_eq!(
            LogRecord::new(LogLevel::Warning, "ignored", "part")
                .format_default_callback_line_context_with_options_and_state(
                    &context,
                    options,
                    &mut context_state,
                ),
            "\x1b[48;5;0m\x1b[38;5;250m[rustctx @ <ptr>] \x1b[0m\x1b[48;5;0m\x1b[38;5;226m[warning] \x1b[0m\x1b[48;5;0m\x1b[38;5;226mpart\x1b[0m"
        );
        assert!(!context_state.print_prefix());
        assert_eq!(
            LogRecord::new(LogLevel::Warning, "ignored", "tail\n")
                .format_default_callback_line_context_with_options_and_state(
                    &context,
                    options,
                    &mut context_state,
                ),
            "\x1b[48;5;0m\x1b[38;5;226mtail\n\x1b[0m"
        );
        assert!(context_state.print_prefix());
        assert_eq!(
            LogRecord::new(LogLevel::Warning, "ignored", "progress\r")
                .format_default_callback_line_context_with_options_and_state(
                    &context,
                    options,
                    &mut context_state,
                ),
            "\x1b[48;5;0m\x1b[38;5;250m[rustctx @ <ptr>] \x1b[0m\x1b[48;5;0m\x1b[38;5;226m[warning] \x1b[0m\x1b[48;5;0m\x1b[38;5;226mprogress\r\x1b[0m"
        );
        assert!(context_state.print_prefix());
        assert_eq!(
            LogRecord::new(LogLevel::Warning, "ignored", "done\n")
                .format_default_callback_line_context_with_options_and_state(
                    &context,
                    options,
                    &mut context_state,
                ),
            "\x1b[48;5;0m\x1b[38;5;250m[rustctx @ <ptr>] \x1b[0m\x1b[48;5;0m\x1b[38;5;226m[warning] \x1b[0m\x1b[48;5;0m\x1b[38;5;226mdone\n\x1b[0m"
        );
        assert!(context_state.print_prefix());
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
    fn av_log_format_line2_null_context_matches_bounded_prefix_shape() {
        let mut prefix = true;
        let plain = LogRecord::new(LogLevel::Warning, "decoder", "plain")
            .format_av_log_line2_null_context(LogFlags::empty(), &mut prefix, 128)
            .unwrap();
        assert_eq!(plain.full_len(), 5);
        assert_eq!(plain.bytes(), b"plain");
        assert_eq!(plain.as_utf8(), Some("plain"));
        assert!(!plain.truncated());
        assert!(!prefix);

        prefix = true;
        let leveled = LogRecord::new(LogLevel::Warning, "decoder", "plain")
            .format_av_log_line2_null_context(LogFlags::PRINT_LEVEL, &mut prefix, 128)
            .unwrap();
        assert_eq!(leveled.full_len(), 15);
        assert_eq!(leveled.bytes(), b"[warning] plain");
        assert!(!leveled.truncated());
        assert!(!prefix);

        prefix = true;
        let quiet = LogRecord::new(LogLevel::Quiet, "decoder", "quiet")
            .format_av_log_line2_null_context(LogFlags::PRINT_LEVEL, &mut prefix, 128)
            .unwrap();
        assert_eq!(quiet.full_len(), 5);
        assert_eq!(quiet.bytes(), b"quiet");
        assert!(!quiet.truncated());
        assert!(!prefix);

        prefix = false;
        let quiet_no_prefix = LogRecord::new(LogLevel::Quiet, "decoder", "quiet")
            .format_av_log_line2_null_context(LogFlags::PRINT_LEVEL, &mut prefix, 128)
            .unwrap();
        assert_eq!(quiet_no_prefix.full_len(), 5);
        assert_eq!(quiet_no_prefix.bytes(), b"quiet");
        assert!(!quiet_no_prefix.truncated());
        assert!(!prefix);

        prefix = false;
        let no_prefix = LogRecord::new(LogLevel::Error, "demuxer", "after")
            .format_av_log_line2_null_context(LogFlags::PRINT_LEVEL, &mut prefix, 128)
            .unwrap();
        assert_eq!(no_prefix.full_len(), 5);
        assert_eq!(no_prefix.bytes(), b"after");
        assert!(!prefix);

        prefix = true;
        let newline = LogRecord::new(LogLevel::Info, "ffmpeg", "withnl\n")
            .format_av_log_line2_null_context(LogFlags::PRINT_LEVEL, &mut prefix, 128)
            .unwrap();
        assert_eq!(newline.full_len(), 14);
        assert_eq!(newline.bytes(), b"[info] withnl\n");
        assert_eq!(newline.line_lossy(), "[info] withnl\n");
        assert!(prefix);

        prefix = true;
        let carriage_return = LogRecord::new(LogLevel::Info, "ffmpeg", "withcr\r")
            .format_av_log_line2_null_context(LogFlags::PRINT_LEVEL, &mut prefix, 128)
            .unwrap();
        assert_eq!(carriage_return.full_len(), 14);
        assert_eq!(carriage_return.bytes(), b"[info] withcr\r");
        assert_eq!(carriage_return.line_lossy(), "[info] withcr\r");
        assert!(prefix);

        prefix = true;
        let small = LogRecord::new(LogLevel::Warning, "decoder", "plain")
            .format_av_log_line2_null_context(LogFlags::PRINT_LEVEL, &mut prefix, 8)
            .unwrap();
        assert_eq!(small.full_len(), 15);
        assert_eq!(small.bytes(), b"[warnin");
        assert!(small.truncated());
        assert!(!prefix);

        prefix = true;
        let null_zero = LogRecord::new(LogLevel::Warning, "decoder", "plain")
            .format_av_log_line2_null_context(LogFlags::PRINT_LEVEL, &mut prefix, 0)
            .unwrap();
        assert_eq!(null_zero.full_len(), 15);
        assert_eq!(null_zero.bytes(), b"");
        assert!(null_zero.truncated());
        assert!(!prefix);

        prefix = true;
        let time_ignored = LogRecord::new(LogLevel::Warning, "decoder", "plain")
            .format_av_log_line2_null_context(LogFlags::PRINT_TIME, &mut prefix, 128)
            .unwrap();
        assert_eq!(time_ignored.full_len(), 5);
        assert_eq!(time_ignored.bytes(), b"plain");
        assert!(!prefix);
    }

    #[test]
    fn av_log_format_line2_context_matches_bounded_prefix_shape() {
        let context = AvLogContextPrefix::new("rustctx", "<ptr>");
        assert_eq!(context.item_name(), "rustctx");
        assert_eq!(context.address(), "<ptr>");

        let mut prefix = true;
        let plain = LogRecord::new(LogLevel::Warning, "decoder", "ctxmsg")
            .format_av_log_line2_context(&context, LogFlags::empty(), &mut prefix, 128)
            .unwrap();
        assert_eq!(plain.full_len(), 24);
        assert_eq!(plain.bytes(), b"[rustctx @ <ptr>] ctxmsg");
        assert!(!plain.truncated());
        assert!(!prefix);

        prefix = true;
        let leveled = LogRecord::new(LogLevel::Warning, "decoder", "ctxmsg")
            .format_av_log_line2_context(&context, LogFlags::PRINT_LEVEL, &mut prefix, 128)
            .unwrap();
        assert_eq!(leveled.full_len(), 34);
        assert_eq!(leveled.bytes(), b"[rustctx @ <ptr>] [warning] ctxmsg");
        assert!(!leveled.truncated());
        assert!(!prefix);

        prefix = true;
        let quiet = LogRecord::new(LogLevel::Quiet, "decoder", "quiet")
            .format_av_log_line2_context(&context, LogFlags::PRINT_LEVEL, &mut prefix, 128)
            .unwrap();
        assert_eq!(quiet.full_len(), 23);
        assert_eq!(quiet.bytes(), b"[rustctx @ <ptr>] quiet");
        assert!(!quiet.truncated());
        assert!(!prefix);

        prefix = false;
        let no_prefix = LogRecord::new(LogLevel::Warning, "decoder", "nopfx")
            .format_av_log_line2_context(&context, LogFlags::PRINT_LEVEL, &mut prefix, 128)
            .unwrap();
        assert_eq!(no_prefix.full_len(), 5);
        assert_eq!(no_prefix.bytes(), b"nopfx");
        assert!(!prefix);

        prefix = true;
        let newline = LogRecord::new(LogLevel::Info, "ffmpeg", "withnl\n")
            .format_av_log_line2_context(&context, LogFlags::PRINT_LEVEL, &mut prefix, 128)
            .unwrap();
        assert_eq!(newline.full_len(), 32);
        assert_eq!(newline.bytes(), b"[rustctx @ <ptr>] [info] withnl\n");
        assert!(prefix);

        prefix = true;
        let carriage_return = LogRecord::new(LogLevel::Info, "ffmpeg", "withcr\r")
            .format_av_log_line2_context(&context, LogFlags::PRINT_LEVEL, &mut prefix, 128)
            .unwrap();
        assert_eq!(carriage_return.full_len(), 32);
        assert_eq!(
            carriage_return.bytes(),
            b"[rustctx @ <ptr>] [info] withcr\r"
        );
        assert!(prefix);

        prefix = true;
        let small = LogRecord::new(LogLevel::Warning, "decoder", "ctxmsg")
            .format_av_log_line2_context(&context, LogFlags::PRINT_LEVEL, &mut prefix, 12)
            .unwrap();
        assert_eq!(small.full_len(), 34);
        assert_eq!(small.bytes(), b"[rustctx @ ");
        assert!(small.truncated());
        assert!(!prefix);

        prefix = true;
        let size1_context = AvLogContextPrefix::new("rustctx", "0x123456789abc");
        let tiny = LogRecord::new(LogLevel::Warning, "decoder", "ctxmsg")
            .format_av_log_line2_context(&size1_context, LogFlags::PRINT_LEVEL, &mut prefix, 1)
            .unwrap();
        assert_eq!(tiny.full_len(), 43);
        assert_eq!(tiny.bytes(), b"");
        assert!(tiny.truncated());
        assert!(!prefix);

        prefix = true;
        let tiny = LogRecord::new(LogLevel::Warning, "decoder", "ctxmsg")
            .format_av_log_line2_context(&size1_context, LogFlags::PRINT_LEVEL, &mut prefix, 0)
            .unwrap();
        assert_eq!(tiny.full_len(), 43);
        assert_eq!(tiny.bytes(), b"");
        assert!(tiny.truncated());
        assert!(!prefix);

        prefix = true;
        let time_ignored = LogRecord::new(LogLevel::Warning, "decoder", "ctxmsg")
            .format_av_log_line2_context(&context, LogFlags::PRINT_DATETIME, &mut prefix, 128)
            .unwrap();
        assert_eq!(time_ignored.full_len(), 24);
        assert_eq!(time_ignored.bytes(), b"[rustctx @ <ptr>] ctxmsg");
        assert!(!prefix);
    }

    #[test]
    fn av_log_format_line_null_size_zero_suppresses_output_and_print_prefix() {
        let mut prefix = true;
        let line = LogRecord::new(LogLevel::Warning, "decoder", "plain")
            .format_av_log_line_null_context(LogFlags::PRINT_LEVEL, &mut prefix, 0)
            .unwrap();
        assert!(line.truncated());
        assert_eq!(line.bytes(), b"");
        assert_eq!(line.line_lossy(), "");
        assert!(!prefix);
    }

    #[test]
    fn av_log_format_line_matches_bounded_wrapper_shape() {
        let context = AvLogContextPrefix::new("rustctx", "<ptr>");

        let mut prefix = true;
        let leveled = LogRecord::new(LogLevel::Warning, "decoder", "plain")
            .format_av_log_line_null_context(LogFlags::PRINT_LEVEL, &mut prefix, 128)
            .unwrap();
        assert_eq!(leveled.bytes(), b"[warning] plain");
        assert_eq!(leveled.as_utf8(), Some("[warning] plain"));
        assert_eq!(leveled.line_lossy(), "[warning] plain");
        assert!(!leveled.truncated());
        assert!(!prefix);

        prefix = true;
        let quiet = LogRecord::new(LogLevel::Quiet, "decoder", "quiet")
            .format_av_log_line_null_context(LogFlags::PRINT_LEVEL, &mut prefix, 128)
            .unwrap();
        assert_eq!(quiet.bytes(), b"quiet");
        assert!(!quiet.truncated());
        assert!(!prefix);

        prefix = false;
        let no_prefix = LogRecord::new(LogLevel::Error, "demuxer", "after")
            .format_av_log_line_null_context(LogFlags::PRINT_LEVEL, &mut prefix, 128)
            .unwrap();
        assert_eq!(no_prefix.into_bytes(), b"after");
        assert!(!prefix);

        prefix = true;
        let newline = LogRecord::new(LogLevel::Info, "ffmpeg", "withnl\n")
            .format_av_log_line_null_context(LogFlags::PRINT_LEVEL, &mut prefix, 128)
            .unwrap();
        assert_eq!(newline.bytes(), b"[info] withnl\n");
        assert!(prefix);

        prefix = true;
        let carriage_return = LogRecord::new(LogLevel::Info, "ffmpeg", "withcr\r")
            .format_av_log_line_null_context(LogFlags::PRINT_LEVEL, &mut prefix, 128)
            .unwrap();
        assert_eq!(carriage_return.bytes(), b"[info] withcr\r");
        assert!(prefix);

        prefix = true;
        let small = LogRecord::new(LogLevel::Warning, "decoder", "plain")
            .format_av_log_line_null_context(LogFlags::PRINT_LEVEL, &mut prefix, 8)
            .unwrap();
        assert_eq!(small.bytes(), b"[warnin");
        assert!(small.truncated());
        assert!(!prefix);

        prefix = true;
        let size1 = LogRecord::new(LogLevel::Warning, "decoder", "plain")
            .format_av_log_line_null_context(LogFlags::PRINT_LEVEL, &mut prefix, 1)
            .unwrap();
        assert_eq!(size1.bytes(), b"");
        assert!(size1.truncated());
        assert!(!prefix);

        prefix = true;
        let context_line = LogRecord::new(LogLevel::Warning, "decoder", "ctxmsg")
            .format_av_log_line_context(&context, LogFlags::PRINT_LEVEL, &mut prefix, 128)
            .unwrap();
        assert_eq!(context_line.bytes(), b"[rustctx @ <ptr>] [warning] ctxmsg");
        assert!(!context_line.truncated());
        assert!(!prefix);

        prefix = true;
        let context_quiet = LogRecord::new(LogLevel::Quiet, "decoder", "quiet")
            .format_av_log_line_context(&context, LogFlags::PRINT_LEVEL, &mut prefix, 128)
            .unwrap();
        assert_eq!(context_quiet.bytes(), b"[rustctx @ <ptr>] quiet");
        assert!(!context_quiet.truncated());
        assert!(!prefix);

        prefix = true;
        let context_carriage_return = LogRecord::new(LogLevel::Info, "decoder", "withcr\r")
            .format_av_log_line_context(&context, LogFlags::PRINT_LEVEL, &mut prefix, 128)
            .unwrap();
        assert_eq!(
            context_carriage_return.bytes(),
            b"[rustctx @ <ptr>] [info] withcr\r"
        );
        assert!(prefix);

        prefix = false;
        let context_no_prefix = LogRecord::new(LogLevel::Warning, "decoder", "nopfx")
            .format_av_log_line_context(&context, LogFlags::PRINT_LEVEL, &mut prefix, 128)
            .unwrap();
        assert_eq!(context_no_prefix.bytes(), b"nopfx");
        assert!(!prefix);

        prefix = true;
        let time_ignored = LogRecord::new(LogLevel::Warning, "decoder", "plain")
            .format_av_log_line_null_context(LogFlags::PRINT_TIME, &mut prefix, 128)
            .unwrap();
        assert_eq!(time_ignored.bytes(), b"plain");
        assert!(!prefix);
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
    fn color_mode_treats_force_env_values_as_presence_only() {
        let is_present =
            |entries: &[(&str, &str)], name: &str| entries.iter().any(|(key, _value)| *key == name);

        for value in ["", "0"] {
            let entries = [(AV_LOG_FORCE_COLOR_ENV, value)];
            assert_eq!(
                LogColorMode::from_ffmpeg_env_vars(|name| is_present(&entries, name)),
                LogColorMode::Always
            );

            let entries = [
                (AV_LOG_FORCE_NOCOLOR_ENV, value),
                (AV_LOG_FORCE_COLOR_ENV, "1"),
            ];
            assert_eq!(
                LogColorMode::from_ffmpeg_env_vars_and_stderr(
                    |name| is_present(&entries, name),
                    true,
                ),
                LogColorMode::Never
            );
        }
    }

    #[test]
    fn color_mode_enables_color_for_terminal_stderr() {
        assert_eq!(
            LogColorMode::from_ffmpeg_env_vars_and_stderr(|_| false, true),
            LogColorMode::Always
        );
        assert_eq!(
            LogColorMode::from_ffmpeg_env_vars_and_stderr(|_| false, false),
            LogColorMode::Never
        );

        let options = LogFormatOptions::new(LogFlags::PRINT_LEVEL)
            .with_ffmpeg_env_color_vars_and_stderr(|_| false, true);
        assert_eq!(options.color_mode(), LogColorMode::Always);
        assert_eq!(
            LogRecord::new(LogLevel::Warning, "decoder", "damaged packet")
                .format_line_with_options(options),
            "\x1b[33m[warning] decoder: damaged packet\x1b[0m"
        );
    }

    #[test]
    fn color_mode_uses_term_for_terminal_palette_without_force_env() {
        assert_eq!(
            LogColorMode::from_ffmpeg_env_vars_stderr_and_term(
                |_| false,
                true,
                Some(OsStr::new("xterm-256color")),
            ),
            LogColorMode::Always
        );
        let term_256_options = LogFormatOptions::new(LogFlags::PRINT_LEVEL)
            .with_ffmpeg_env_color_vars_stderr_and_term(
                |_| false,
                true,
                Some(OsStr::new("xterm-256color")),
            );
        assert_eq!(term_256_options.color_mode(), LogColorMode::Always);
        assert_eq!(
            LogRecord::new(LogLevel::Error, "ignored", "plain\n")
                .format_default_callback_line_null_context_with_options(term_256_options),
            "\x1b[48;5;0m\x1b[38;5;196m[error] \x1b[0m\x1b[48;5;0m\x1b[38;5;196mplain\n\x1b[0m"
        );
        assert_eq!(
            LogRecord::new(LogLevel::Fatal, "ignored", "plain\n")
                .format_default_callback_line_null_context_with_options(term_256_options),
            "\x1b[48;5;0m\x1b[38;5;208m[fatal] \x1b[0m\x1b[48;5;0m\x1b[38;5;208mplain\n\x1b[0m"
        );
        assert_eq!(
            LogRecord::new(LogLevel::Panic, "ignored", "plain\n")
                .format_default_callback_line_null_context_with_options(term_256_options),
            "\x1b[48;5;52m\x1b[38;5;196m[panic] \x1b[0m\x1b[48;5;52m\x1b[38;5;196mplain\n\x1b[0m"
        );
        assert_eq!(
            LogColorMode::from_ffmpeg_env_vars_stderr_and_term(|_| false, true, None),
            LogColorMode::Never
        );
        assert_eq!(
            LogColorMode::from_ffmpeg_env_vars_stderr_and_term(
                |_| false,
                true,
                Some(OsStr::new("dumb")),
            ),
            LogColorMode::Basic
        );
        assert_eq!(
            LogColorMode::from_ffmpeg_env_vars_stderr_and_term(
                |_| false,
                true,
                Some(OsStr::new("")),
            ),
            LogColorMode::Basic
        );
        assert_eq!(
            LogColorMode::from_ffmpeg_env_vars_stderr_and_term(
                |name| name == AV_LOG_FORCE_COLOR_ENV,
                true,
                Some(OsStr::new("dumb")),
            ),
            LogColorMode::Always
        );
        assert_eq!(
            LogColorMode::from_ffmpeg_env_vars_stderr_and_term(
                |name| name == AV_LOG_FORCE_NOCOLOR_ENV,
                true,
                Some(OsStr::new("xterm-256color")),
            ),
            LogColorMode::Never
        );

        let options = LogFormatOptions::new(LogFlags::PRINT_LEVEL)
            .with_ffmpeg_env_color_vars_stderr_and_term(|_| false, true, Some(OsStr::new("dumb")));
        assert_eq!(options.color_mode(), LogColorMode::Basic);
        assert_eq!(
            LogRecord::new(LogLevel::Warning, "decoder", "damaged packet")
                .format_line_with_options(options),
            "\x1b[33m[warning] decoder: damaged packet\x1b[0m"
        );

        let context = AvLogContextPrefix::new("rustctx", "<ptr>");
        assert_eq!(
            LogRecord::new(LogLevel::Warning, "ignored", "plain\n")
                .format_default_callback_line_context_with_options(&context, options),
            "\x1b[0;39m[rustctx @ <ptr>] \x1b[0m\x1b[0;33m[warning] \x1b[0m\x1b[0;33mplain\n\x1b[0m"
        );
        assert_eq!(
            LogRecord::new(LogLevel::Error, "ignored", "plain\n")
                .format_default_callback_line_null_context_with_options(options),
            "\x1b[1;31m[error] \x1b[0m\x1b[1;31mplain\n\x1b[0m"
        );
        assert_eq!(
            LogRecord::new(LogLevel::Quiet, "ignored", "quiet\n")
                .format_default_callback_line_null_context_with_options(options),
            "\x1b[4;31mquiet\n\x1b[0m"
        );
        assert_eq!(
            LogRecord::new(LogLevel::Quiet, "ignored", "quiet\n")
                .format_default_callback_line_context_with_options(&context, options),
            "\x1b[0;39m[rustctx @ <ptr>] \x1b[0m\x1b[4;31mquiet\n\x1b[0m"
        );
        assert_eq!(
            LogRecord::new(LogLevel::Fatal, "ignored", "plain\n")
                .format_default_callback_line_null_context_with_options(options),
            "\x1b[4;31m[fatal] \x1b[0m\x1b[4;31mplain\n\x1b[0m"
        );
        assert_eq!(
            LogRecord::new(LogLevel::Panic, "ignored", "plain\n")
                .format_default_callback_line_null_context_with_options(options),
            "\x1b[4;31m[panic] \x1b[0m\x1b[4;31mplain\n\x1b[0m"
        );
        assert_eq!(
            LogRecord::new(LogLevel::Info, "ignored", "plain\n")
                .format_default_callback_line_null_context_with_options(
                    LogFormatOptions::new(LogFlags::empty()).with_color_mode(LogColorMode::Basic),
                ),
            "plain\n"
        );
    }

    #[test]
    fn color_mode_force_nocolor_env_wins_over_force_color() {
        let mut checked = Vec::new();
        let mode = LogColorMode::from_ffmpeg_env_vars_and_stderr(
            |name| {
                checked.push(name.to_owned());
                name == AV_LOG_FORCE_NOCOLOR_ENV || name == AV_LOG_FORCE_COLOR_ENV
            },
            true,
        );

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
    fn default_callback_color_state_caches_first_resolution() {
        let mut state = DefaultCallbackColorState::new();
        assert_eq!(state.cached_mode(), None);

        let first = LogFormatOptions::new(LogFlags::PRINT_LEVEL)
            .with_default_callback_color_state_and_resolver(&mut state, || LogColorMode::Never);
        assert_eq!(first.color_mode(), LogColorMode::Never);
        assert_eq!(state.cached_mode(), Some(LogColorMode::Never));

        let second = LogFormatOptions::new(LogFlags::PRINT_LEVEL)
            .with_default_callback_color_state_and_resolver(&mut state, || LogColorMode::Always);
        assert_eq!(second.color_mode(), LogColorMode::Never);
        assert_eq!(
            LogRecord::new(LogLevel::Warning, "ignored", "plain\n")
                .format_default_callback_line_null_context_with_options(second),
            "[warning] plain\n"
        );

        state.reset();
        let fresh = LogFormatOptions::new(LogFlags::PRINT_LEVEL)
            .with_default_callback_color_state_and_resolver(&mut state, || LogColorMode::Always);
        assert_eq!(fresh.color_mode(), LogColorMode::Always);
        assert_eq!(state.cached_mode(), Some(LogColorMode::Always));
        assert_eq!(
            LogRecord::new(LogLevel::Warning, "ignored", "plain\n")
                .format_default_callback_line_null_context_with_options(fresh),
            "\x1b[48;5;0m\x1b[38;5;226m[warning] \x1b[0m\x1b[48;5;0m\x1b[38;5;226mplain\n\x1b[0m"
        );

        let pre_resolved = DefaultCallbackColorState::from_resolved(LogColorMode::Always);
        assert_eq!(pre_resolved.cached_mode(), Some(LogColorMode::Always));
    }

    #[test]
    fn default_callback_color_state_stays_plain_for_redirected_stderr_without_force_env() {
        let mut state = DefaultCallbackColorState::new();
        let options = LogFormatOptions::new(LogFlags::PRINT_LEVEL)
            .with_default_callback_color_state_and_resolver(&mut state, || {
                LogColorMode::from_ffmpeg_env_vars_and_stderr(|_| false, false)
            });

        assert_eq!(options.color_mode(), LogColorMode::Never);
        assert_eq!(state.cached_mode(), Some(LogColorMode::Never));
        assert_eq!(
            LogRecord::new(LogLevel::Warning, "ignored", "plain\n")
                .format_default_callback_line_null_context_with_options(options),
            "[warning] plain\n"
        );

        let context = AvLogContextPrefix::new("rustctx", "<ptr>");
        assert_eq!(
            LogRecord::new(LogLevel::Warning, "ignored", "plain\n")
                .format_default_callback_line_context_with_options(&context, options),
            "[rustctx @ <ptr>] [warning] plain\n"
        );
    }

    #[test]
    fn default_callback_color_state_uses_terminal_stderr_without_force_env() {
        let mut state = DefaultCallbackColorState::new();
        let options = LogFormatOptions::new(LogFlags::PRINT_LEVEL)
            .with_default_callback_color_state_and_resolver(&mut state, || {
                LogColorMode::from_ffmpeg_env_vars_and_stderr(|_| false, true)
            });

        assert_eq!(options.color_mode(), LogColorMode::Always);
        assert_eq!(state.cached_mode(), Some(LogColorMode::Always));
        assert_eq!(
            LogRecord::new(LogLevel::Warning, "ignored", "plain\n")
                .format_default_callback_line_null_context_with_options(options),
            "\x1b[48;5;0m\x1b[38;5;226m[warning] \x1b[0m\x1b[48;5;0m\x1b[38;5;226mplain\n\x1b[0m"
        );

        let context = AvLogContextPrefix::new("rustctx", "<ptr>");
        assert_eq!(
            LogRecord::new(LogLevel::Warning, "ignored", "plain\n")
                .format_default_callback_line_context_with_options(&context, options),
            "\x1b[48;5;0m\x1b[38;5;250m[rustctx @ <ptr>] \x1b[0m\x1b[48;5;0m\x1b[38;5;226m[warning] \x1b[0m\x1b[48;5;0m\x1b[38;5;226mplain\n\x1b[0m"
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

        logger.set_flags(LogFlags::from_bits_retain(0xffff));
        assert_eq!(logger.flags().bits(), 0xffff);

        logger.set_flag(LogFlags::PRINT_LEVEL, false);
        assert_eq!(
            logger.flags().bits() & !LogFlags::PRINT_LEVEL.bits(),
            0xfffd
        );
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
    fn repeated_comparison_respects_target_identity_for_same_message() {
        let mut flags = LogFlags::PRINT_LEVEL;
        flags.insert(LogFlags::SKIP_REPEATED);
        let mut logger = Logger::new_with_flags(LogLevel::Warning, flags);

        assert!(logger.log(LogRecord::new(LogLevel::Warning, "ctx@one", "repeat")));
        assert!(logger.log(LogRecord::new(LogLevel::Warning, "ctx@two", "repeat")));
        assert!(logger.log(LogRecord::new(LogLevel::Warning, "ctx@one", "repeat")));

        assert_eq!(
            logger.formatted_records(),
            [
                "[warning] ctx@one: repeat".to_owned(),
                "[warning] ctx@two: repeat".to_owned(),
                "[warning] ctx@one: repeat".to_owned()
            ]
        );
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
    fn callback_receives_raw_record_fields_after_level_filtering() {
        let mut logger = Logger::new_with_flags(LogLevel::Warning, LogFlags::PRINT_LEVEL);
        let mut seen = Vec::new();

        assert!(!logger.log_with_callback(
            LogRecord::new(LogLevel::Info, "decoder", "hidden"),
            |record| seen.push((
                record.level(),
                record.target().to_owned(),
                record.message().to_owned(),
            ))
        ));
        assert!(seen.is_empty());

        assert!(logger.log_with_callback(
            LogRecord::new(LogLevel::Error, "", "raw:5\n"),
            |record| seen.push((
                record.level(),
                record.target().to_owned(),
                record.message().to_owned(),
            ))
        ));
        assert!(logger.log_with_callback(
            LogRecord::new(LogLevel::Warning, "rustctx", "ctx:3"),
            |record| seen.push((
                record.level(),
                record.target().to_owned(),
                record.message().to_owned(),
            ))
        ));

        assert_eq!(
            seen,
            [
                (LogLevel::Error, String::new(), "raw:5\n".to_owned()),
                (LogLevel::Warning, "rustctx".to_owned(), "ctx:3".to_owned(),),
            ]
        );
    }

    #[test]
    fn custom_callback_dispatch_ignores_level_filter_and_repeat_flags() {
        let mut flags = LogFlags::PRINT_LEVEL;
        flags.insert(LogFlags::SKIP_REPEATED);
        let mut logger = Logger::new_with_flags(LogLevel::Warning, flags);
        let mut seen = Vec::new();

        logger.log_custom_callback(LogRecord::new(LogLevel::Info, "", "hidden"), |record| {
            seen.push((
                record.level(),
                record.raw_level(),
                record.target().to_owned(),
                record.message().to_owned(),
            ));
        });
        logger.log_custom_callback(
            LogRecord::new(LogLevel::Warning, "", "rawlevel").with_raw_level(23),
            |record| {
                seen.push((
                    record.level(),
                    record.raw_level(),
                    record.target().to_owned(),
                    record.message().to_owned(),
                ));
            },
        );
        logger.log_custom_callback(
            LogRecord::new(LogLevel::Warning, "", "rawneg").with_raw_level(-1),
            |record| {
                seen.push((
                    record.level(),
                    record.raw_level(),
                    record.target().to_owned(),
                    record.message().to_owned(),
                ));
            },
        );
        logger.log_custom_callback(
            LogRecord::new(LogLevel::Warning, "", "mix:arg:7:Q:%").with_raw_level(57),
            |record| {
                seen.push((
                    record.level(),
                    record.raw_level(),
                    record.target().to_owned(),
                    record.message().to_owned(),
                ));
            },
        );
        logger.log_custom_callback(LogRecord::new(LogLevel::Error, "", "raw:5\n"), |record| {
            seen.push((
                record.level(),
                record.raw_level(),
                record.target().to_owned(),
                record.message().to_owned(),
            ));
        });
        for _ in 0..2 {
            logger.log_custom_callback(LogRecord::new(LogLevel::Warning, "", "repeat"), |record| {
                seen.push((
                    record.level(),
                    record.raw_level(),
                    record.target().to_owned(),
                    record.message().to_owned(),
                ));
            });
        }
        logger.log_custom_callback(
            LogRecord::new(LogLevel::Warning, "rustctx", "ctx:3"),
            |record| {
                seen.push((
                    record.level(),
                    record.raw_level(),
                    record.target().to_owned(),
                    record.message().to_owned(),
                ));
            },
        );
        for _ in 0..2 {
            logger.log_custom_callback(
                LogRecord::new(LogLevel::Warning, "rustctx", "ctxrepeat"),
                |record| {
                    seen.push((
                        record.level(),
                        record.raw_level(),
                        record.target().to_owned(),
                        record.message().to_owned(),
                    ));
                },
            );
        }
        logger.log_custom_callback(
            LogRecord::new(LogLevel::Quiet, "rustctx", "quietctx"),
            |record| {
                seen.push((
                    record.level(),
                    record.raw_level(),
                    record.target().to_owned(),
                    record.message().to_owned(),
                ));
            },
        );

        assert_eq!(
            seen,
            [
                (
                    LogLevel::Info,
                    LogLevel::Info.as_ffmpeg_value(),
                    String::new(),
                    "hidden".to_owned()
                ),
                (LogLevel::Warning, 23, String::new(), "rawlevel".to_owned()),
                (LogLevel::Warning, -1, String::new(), "rawneg".to_owned()),
                (
                    LogLevel::Warning,
                    57,
                    String::new(),
                    "mix:arg:7:Q:%".to_owned()
                ),
                (
                    LogLevel::Error,
                    LogLevel::Error.as_ffmpeg_value(),
                    String::new(),
                    "raw:5\n".to_owned()
                ),
                (
                    LogLevel::Warning,
                    LogLevel::Warning.as_ffmpeg_value(),
                    String::new(),
                    "repeat".to_owned()
                ),
                (
                    LogLevel::Warning,
                    LogLevel::Warning.as_ffmpeg_value(),
                    String::new(),
                    "repeat".to_owned()
                ),
                (
                    LogLevel::Warning,
                    LogLevel::Warning.as_ffmpeg_value(),
                    "rustctx".to_owned(),
                    "ctx:3".to_owned()
                ),
                (
                    LogLevel::Warning,
                    LogLevel::Warning.as_ffmpeg_value(),
                    "rustctx".to_owned(),
                    "ctxrepeat".to_owned()
                ),
                (
                    LogLevel::Warning,
                    LogLevel::Warning.as_ffmpeg_value(),
                    "rustctx".to_owned(),
                    "ctxrepeat".to_owned()
                ),
                (
                    LogLevel::Quiet,
                    LogLevel::Quiet.as_ffmpeg_value(),
                    "rustctx".to_owned(),
                    "quietctx".to_owned()
                ),
            ]
        );
        assert_eq!(logger.records().len(), 11);
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
    fn log_once_uses_initial_then_subsequent_level_and_marks_state() {
        let mut logger = Logger::new(LogLevel::Warning);
        let mut state = LogOnceState::new();

        assert_eq!(state.raw(), 0);
        assert!(!state.has_logged());
        assert!(logger.log_once(
            &mut state,
            LogRecord::new(LogLevel::Warning, "ffmpeg", "one-shot"),
            LogLevel::Debug,
        ));
        assert_eq!(state.raw(), 1);
        assert!(state.has_logged());
        assert_eq!(logger.records().len(), 1);
        assert_eq!(logger.records()[0].level(), LogLevel::Warning);

        assert!(!logger.log_once(
            &mut state,
            LogRecord::new(LogLevel::Warning, "ffmpeg", "one-shot"),
            LogLevel::Debug,
        ));
        assert_eq!(state.raw(), 1);
        assert_eq!(logger.records().len(), 1);

        let mut filtered_first = LogOnceState::new();
        assert!(!logger.log_once(
            &mut filtered_first,
            LogRecord::new(LogLevel::Info, "ffmpeg", "filtered first"),
            LogLevel::Error,
        ));
        assert_eq!(filtered_first.raw(), 1);
        assert!(logger.log_once(
            &mut filtered_first,
            LogRecord::new(LogLevel::Info, "ffmpeg", "second visible"),
            LogLevel::Error,
        ));
        assert_eq!(logger.records().len(), 2);
        assert_eq!(logger.records()[1].level(), LogLevel::Error);
        assert_eq!(logger.records()[1].message(), "second visible");

        let mut preseeded = LogOnceState::from_raw(7);
        assert!(preseeded.has_logged());
        assert!(logger.log_once(
            &mut preseeded,
            LogRecord::new(LogLevel::Info, "ffmpeg", "preseeded"),
            LogLevel::Error,
        ));
        assert_eq!(preseeded.raw(), 1);
        assert_eq!(logger.records()[2].level(), LogLevel::Error);
        assert_eq!(logger.records()[2].message(), "preseeded");
    }

    #[test]
    fn global_log_once_uses_shared_logger_state() {
        let _guard = GLOBAL_LOGGER_TEST_LOCK.lock().unwrap();
        reset_global_logger_for_tests(LogLevel::Warning, LogFlags::PRINT_LEVEL);
        let mut state = LogOnceState::new();

        assert!(global_log_once(
            &mut state,
            LogRecord::new(LogLevel::Warning, "ffmpeg", "once"),
            LogLevel::Debug,
        ));
        assert!(!global_log_once(
            &mut state,
            LogRecord::new(LogLevel::Warning, "ffmpeg", "once"),
            LogLevel::Debug,
        ));

        let records = take_global_log_records();
        assert_eq!(state.raw(), 1);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].level(), LogLevel::Warning);

        reset_global_logger_for_tests(LogLevel::Info, LogFlags::PRINT_LEVEL);
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
        assert_eq!(
            summary.format_default_callback_line_null_context_with_flags(LogFlags::PRINT_LEVEL),
            "    Last message repeated 3 times\n"
        );
    }

    #[test]
    fn default_callback_repeat_summary_stays_plain_when_colored() {
        let flags = LogFlags::SKIP_REPEATED | LogFlags::PRINT_LEVEL;
        let options = LogFormatOptions::new(flags).with_color_mode(LogColorMode::Always);
        let mut logger = Logger::new_with_flags(LogLevel::Trace, flags);
        let repeated = LogRecord::new(LogLevel::Warning, "ignored", "repeat\n");

        assert!(logger.log(repeated.clone()));
        assert!(logger.log(repeated.clone()));
        assert!(logger.log(repeated));
        assert!(logger.log(LogRecord::new(LogLevel::Error, "ignored", "next\n")));

        let lines: String = logger
            .records()
            .iter()
            .map(|record| record.format_default_callback_line_null_context_with_options(options))
            .collect();
        assert!(lines.contains("\x1b[48;5;0m\x1b[38;5;226m[warning] \x1b[0m"));
        assert!(lines.contains("    Last message repeated 2 times\n"));
        assert!(!lines.contains("\x1b[48;5;0m\x1b[38;5;226mLast message repeated"));
        assert!(!lines.contains("\x1b[48;5;0m\x1b[38;5;196mLast message repeated"));
        assert!(lines.contains("\x1b[48;5;0m\x1b[38;5;196m[error] \x1b[0m"));
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
