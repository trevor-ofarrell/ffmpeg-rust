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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogRecord {
    level: LogLevel,
    target: String,
    message: String,
}

impl LogRecord {
    pub fn new(level: LogLevel, target: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            level,
            target: target.into(),
            message: message.into(),
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

    pub fn format_line(&self) -> String {
        if self.target.is_empty() {
            format!("[{}] {}", self.level.name(), self.message)
        } else {
            format!("[{}] {}: {}", self.level.name(), self.target, self.message)
        }
    }
}

#[derive(Debug, Clone)]
pub struct Logger {
    level: LogLevel,
    records: Vec<LogRecord>,
}

impl Default for Logger {
    fn default() -> Self {
        Self::new(LogLevel::Info)
    }
}

impl Logger {
    pub fn new(level: LogLevel) -> Self {
        Self {
            level,
            records: Vec::new(),
        }
    }

    pub fn level(&self) -> LogLevel {
        self.level
    }

    pub fn set_level(&mut self, level: LogLevel) {
        self.level = level;
    }

    pub fn enabled(&self, level: LogLevel) -> bool {
        self.level != LogLevel::Quiet && level != LogLevel::Quiet && level <= self.level
    }

    pub fn log(&mut self, record: LogRecord) -> bool {
        if self.enabled(record.level()) {
            self.records.push(record);
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
            self.records.push(record);
            let record_index = self.records.len() - 1;
            callback(&self.records[record_index]);
            true
        } else {
            false
        }
    }

    pub fn records(&self) -> &[LogRecord] {
        &self.records
    }

    pub fn clear(&mut self) {
        self.records.clear();
    }

    pub fn take_records(&mut self) -> Vec<LogRecord> {
        core::mem::take(&mut self.records)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
