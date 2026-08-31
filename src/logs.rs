use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::error::GnxError;

#[derive(Debug, Deserialize, Serialize)]
struct LogEntry {
    schema: u32,
    timestamp_unix_ms: u128,
    level: String,
    component: String,
    operation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<String>,
    message: String,
}

pub fn default_log_path() -> PathBuf {
    crate::config::data_root().join("logs").join("gnx.jsonl")
}

pub fn event(level: &str, component: &str, operation: &str, message: impl Into<String>) {
    append(LogEntry {
        schema: 1,
        timestamp_unix_ms: now_ms(),
        level: level.to_string(),
        component: component.to_string(),
        operation: operation.to_string(),
        code: None,
        message: message.into(),
    });
}

pub fn error(error: &GnxError) {
    append(LogEntry {
        schema: 1,
        timestamp_unix_ms: now_ms(),
        level: "error".to_string(),
        component: error.component.to_string(),
        operation: error.operation.to_string(),
        code: Some(error.code.to_string()),
        message: error.message.clone(),
    });
}

pub fn print_tail(limit: usize, json: bool) -> Result<(), GnxError> {
    let path = default_log_path();
    let file = match fs::File::open(&path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            println!("Aún no hay eventos. Ruta esperada: {}", path.display());
            return Ok(());
        }
        Err(error) => return Err(GnxError::io("logs_read", error.to_string())),
    };
    let lines: Vec<String> = BufReader::new(file)
        .lines()
        .collect::<Result<_, _>>()
        .map_err(|error| GnxError::io("logs_read", error.to_string()))?;
    let start = lines.len().saturating_sub(limit);
    for line in &lines[start..] {
        if json {
            println!("{line}");
            continue;
        }
        match serde_json::from_str::<LogEntry>(line) {
            Ok(entry) => println!(
                "[{}] {:<5} {}.{}{} — {}",
                entry.timestamp_unix_ms,
                entry.level.to_uppercase(),
                entry.component,
                entry.operation,
                entry
                    .code
                    .as_deref()
                    .map(|code| format!(" [{code}]"))
                    .unwrap_or_default(),
                entry.message
            ),
            Err(_) => println!("{line}"),
        }
    }
    Ok(())
}

fn append(entry: LogEntry) {
    let path = default_log_path();
    let Some(parent) = path.parent() else {
        return;
    };
    if fs::create_dir_all(parent).is_err() {
        return;
    }
    let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) else {
        return;
    };
    if serde_json::to_writer(&mut file, &entry).is_ok() {
        let _ = file.write_all(b"\n");
        let _ = file.flush();
    }
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_entry_is_single_line_json() {
        let entry = LogEntry {
            schema: 1,
            timestamp_unix_ms: 1,
            level: "info".to_string(),
            component: "test".to_string(),
            operation: "serialize".to_string(),
            code: None,
            message: "línea uno\nlínea dos".to_string(),
        };
        let encoded = serde_json::to_string(&entry).unwrap();
        assert!(!encoded.contains('\n'));
    }
}
