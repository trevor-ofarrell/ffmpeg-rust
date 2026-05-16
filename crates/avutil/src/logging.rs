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

    pub fn log(&mut self, record: LogRecord) {
        if record.level() <= self.level && self.level != LogLevel::Quiet {
            self.records.push(record);
        }
    }

    pub fn records(&self) -> &[LogRecord] {
        &self.records
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logger_filters_by_level() {
        let mut logger = Logger::new(LogLevel::Warning);

        logger.log(LogRecord::new(LogLevel::Info, "ffmpeg", "ignored"));
        logger.log(LogRecord::new(LogLevel::Error, "ffmpeg", "kept"));

        assert_eq!(logger.records().len(), 1);
        assert_eq!(logger.records()[0].message(), "kept");
    }

    #[test]
    fn quiet_suppresses_all_records() {
        let mut logger = Logger::new(LogLevel::Quiet);

        logger.log(LogRecord::new(LogLevel::Fatal, "ffmpeg", "ignored"));

        assert!(logger.records().is_empty());
    }
}
