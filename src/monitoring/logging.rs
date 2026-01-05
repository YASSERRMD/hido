//! Structured logging for HIDO.
//!
//! Provides log levels, structured entries, and tracing integration.

use crate::core::{now, Timestamp};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;

/// Log level.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LogLevel {
    /// Trace level (most verbose)
    Trace = 0,
    /// Debug level
    Debug = 1,
    /// Info level
    Info = 2,
    /// Warning level
    Warn = 3,
    /// Error level
    Error = 4,
    /// Fatal level
    Fatal = 5,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Trace => write!(f, "TRACE"),
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warn => write!(f, "WARN"),
            LogLevel::Error => write!(f, "ERROR"),
            LogLevel::Fatal => write!(f, "FATAL"),
        }
    }
}

/// A structured log entry.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogEntry {
    /// Timestamp
    pub timestamp: Timestamp,
    /// Log level
    pub level: LogLevel,
    /// Message
    pub message: String,
    /// Target (module path)
    pub target: String,
    /// Structured fields
    pub fields: HashMap<String, serde_json::Value>,
    /// Span context (for tracing)
    pub span_id: Option<String>,
    /// Trace ID
    pub trace_id: Option<String>,
}

impl LogEntry {
    /// Create a new log entry.
    pub fn new(level: LogLevel, message: &str) -> Self {
        Self {
            timestamp: now(),
            level,
            message: message.to_string(),
            target: String::new(),
            fields: HashMap::new(),
            span_id: None,
            trace_id: None,
        }
    }

    /// Set target.
    pub fn with_target(mut self, target: &str) -> Self {
        self.target = target.to_string();
        self
    }

    /// Add a field.
    pub fn with_field(mut self, key: &str, value: impl Serialize) -> Self {
        if let Ok(v) = serde_json::to_value(value) {
            self.fields.insert(key.to_string(), v);
        }
        self
    }

    /// Set trace context.
    pub fn with_trace(mut self, trace_id: &str, span_id: &str) -> Self {
        self.trace_id = Some(trace_id.to_string());
        self.span_id = Some(span_id.to_string());
        self
    }

    /// Format as JSON.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// Format as text.
    pub fn to_text(&self) -> String {
        let fields_str = if self.fields.is_empty() {
            String::new()
        } else {
            format!(" {:?}", self.fields)
        };

        format!(
            "{} {} [{}] {}{}",
            self.timestamp,
            self.level,
            self.target,
            self.message,
            fields_str
        )
    }
}

/// Log output format.
#[derive(Clone, Debug)]
pub enum LogFormat {
    /// Plain text
    Text,
    /// JSON
    Json,
}

/// Logger configuration.
#[derive(Clone, Debug)]
pub struct LoggerConfig {
    /// Minimum log level
    pub level: LogLevel,
    /// Output format
    pub format: LogFormat,
    /// Include timestamps
    pub timestamps: bool,
    /// Include trace IDs
    pub include_traces: bool,
}

impl Default for LoggerConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            format: LogFormat::Text,
            timestamps: true,
            include_traces: true,
        }
    }
}

/// Logger for the application.
pub struct Logger {
    /// Configuration
    config: LoggerConfig,
    /// Log buffer (for testing/inspection)
    buffer: RwLock<Vec<LogEntry>>,
    /// Maximum buffer size
    max_buffer: usize,
}

impl Logger {
    /// Create a new logger.
    pub fn new(config: LoggerConfig) -> Self {
        Self {
            config,
            buffer: RwLock::new(Vec::new()),
            max_buffer: 1000,
        }
    }

    /// Create with default config.
    pub fn default_logger() -> Self {
        Self::new(LoggerConfig::default())
    }

    /// Log an entry.
    pub fn log(&self, entry: LogEntry) {
        if entry.level < self.config.level {
            return;
        }

        // Add to buffer
        {
            let mut buffer = self.buffer.write().unwrap();
            if buffer.len() >= self.max_buffer {
                buffer.remove(0);
            }
            buffer.push(entry.clone());
        }

        // Output (in real impl would use tracing)
        let output = match self.config.format {
            LogFormat::Text => entry.to_text(),
            LogFormat::Json => entry.to_json(),
        };

        // Print to appropriate output
        match entry.level {
            LogLevel::Error | LogLevel::Fatal => eprintln!("{}", output),
            _ => {} // In production, would print or send to collector
        }
    }

    /// Log at trace level.
    pub fn trace(&self, message: &str) {
        self.log(LogEntry::new(LogLevel::Trace, message));
    }

    /// Log at debug level.
    pub fn debug(&self, message: &str) {
        self.log(LogEntry::new(LogLevel::Debug, message));
    }

    /// Log at info level.
    pub fn info(&self, message: &str) {
        self.log(LogEntry::new(LogLevel::Info, message));
    }

    /// Log at warn level.
    pub fn warn(&self, message: &str) {
        self.log(LogEntry::new(LogLevel::Warn, message));
    }

    /// Log at error level.
    pub fn error(&self, message: &str) {
        self.log(LogEntry::new(LogLevel::Error, message));
    }

    /// Get buffered logs.
    pub fn get_logs(&self) -> Vec<LogEntry> {
        self.buffer.read().unwrap().clone()
    }

    /// Get logs at or above a level.
    pub fn get_logs_at_level(&self, min_level: LogLevel) -> Vec<LogEntry> {
        self.buffer
            .read()
            .unwrap()
            .iter()
            .filter(|e| e.level >= min_level)
            .cloned()
            .collect()
    }

    /// Clear the buffer.
    pub fn clear(&self) {
        self.buffer.write().unwrap().clear();
    }

    /// Set log level.
    pub fn set_level(&mut self, level: LogLevel) {
        self.config.level = level;
    }
}

impl Default for Logger {
    fn default() -> Self {
        Self::default_logger()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_entry() {
        let entry = LogEntry::new(LogLevel::Info, "Test message")
            .with_target("test::module")
            .with_field("user_id", "123");

        assert_eq!(entry.level, LogLevel::Info);
        assert_eq!(entry.message, "Test message");
        assert_eq!(entry.target, "test::module");
        assert!(entry.fields.contains_key("user_id"));
    }

    #[test]
    fn test_log_format() {
        let entry = LogEntry::new(LogLevel::Info, "Test");

        let json = entry.to_json();
        assert!(json.contains("Info"));

        let text = entry.to_text();
        assert!(text.contains("INFO"));
    }

    #[test]
    fn test_logger() {
        let logger = Logger::default_logger();
        logger.info("Info message");
        logger.warn("Warning message");
        logger.error("Error message");

        let logs = logger.get_logs();
        assert_eq!(logs.len(), 3);
    }

    #[test]
    fn test_log_level_filtering() {
        let mut config = LoggerConfig::default();
        config.level = LogLevel::Warn;
        let logger = Logger::new(config);

        logger.info("Should be filtered");
        logger.warn("Should appear");
        logger.error("Should appear");

        let logs = logger.get_logs();
        assert_eq!(logs.len(), 2);
    }

    #[test]
    fn test_get_logs_at_level() {
        let logger = Logger::default_logger();
        logger.info("Info");
        logger.warn("Warn");
        logger.error("Error");

        let errors = logger.get_logs_at_level(LogLevel::Error);
        assert_eq!(errors.len(), 1);

        let warnings = logger.get_logs_at_level(LogLevel::Warn);
        assert_eq!(warnings.len(), 2);
    }

    #[test]
    fn test_log_level_ordering() {
        assert!(LogLevel::Trace < LogLevel::Debug);
        assert!(LogLevel::Debug < LogLevel::Info);
        assert!(LogLevel::Info < LogLevel::Warn);
        assert!(LogLevel::Warn < LogLevel::Error);
        assert!(LogLevel::Error < LogLevel::Fatal);
    }
}
