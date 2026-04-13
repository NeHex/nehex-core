use std::{
    collections::VecDeque,
    io::{self, Write},
    sync::{Arc, Mutex},
};

use once_cell::sync::Lazy;
use tracing_subscriber::fmt::MakeWriter;

const MAX_LOG_LINES: usize = 10_000;

static LOG_LINES: Lazy<Arc<Mutex<VecDeque<String>>>> =
    Lazy::new(|| Arc::new(Mutex::new(VecDeque::with_capacity(MAX_LOG_LINES))));

#[derive(Clone, Default)]
pub struct LogBufferMakeWriter;

#[derive(Default)]
pub struct LogBufferWriter {
    pending: Vec<u8>,
}

pub fn make_writer() -> LogBufferMakeWriter {
    LogBufferMakeWriter
}

pub fn list_recent_logs(limit: usize, keyword: Option<&str>) -> (usize, Vec<String>) {
    let safe_limit = limit.clamp(1, MAX_LOG_LINES);
    let keyword = keyword.unwrap_or_default().trim().to_lowercase();

    let guard = LOG_LINES
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let mut filtered = guard
        .iter()
        .filter(|line| {
            if keyword.is_empty() {
                return true;
            }
            line.to_lowercase().contains(&keyword)
        })
        .cloned()
        .collect::<Vec<String>>();

    let total = filtered.len();
    if total > safe_limit {
        let start = total - safe_limit;
        filtered = filtered.split_off(start);
    }

    (total, filtered)
}

impl<'a> MakeWriter<'a> for LogBufferMakeWriter {
    type Writer = LogBufferWriter;

    fn make_writer(&'a self) -> Self::Writer {
        LogBufferWriter::default()
    }
}

impl Write for LogBufferWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.pending.extend_from_slice(buf);
        self.drain_complete_lines();
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.drain_complete_lines();
        Ok(())
    }
}

impl Drop for LogBufferWriter {
    fn drop(&mut self) {
        self.drain_complete_lines();
        if self.pending.is_empty() {
            return;
        }
        let line = String::from_utf8_lossy(&self.pending).trim().to_string();
        if !line.is_empty() {
            push_line(line);
        }
        self.pending.clear();
    }
}

impl LogBufferWriter {
    fn drain_complete_lines(&mut self) {
        loop {
            let Some(pos) = self.pending.iter().position(|byte| *byte == b'\n') else {
                break;
            };

            let mut line_bytes = self.pending.drain(..=pos).collect::<Vec<u8>>();
            if line_bytes.last() == Some(&b'\n') {
                line_bytes.pop();
            }
            if line_bytes.last() == Some(&b'\r') {
                line_bytes.pop();
            }

            let line = String::from_utf8_lossy(&line_bytes).trim().to_string();
            if !line.is_empty() {
                push_line(line);
            }
        }
    }
}

fn push_line(line: String) {
    let mut guard = LOG_LINES
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if guard.len() >= MAX_LOG_LINES {
        guard.pop_front();
    }
    guard.push_back(line);
}
