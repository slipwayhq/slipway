use tracing::{debug, error, info, trace, warn};

#[derive(Debug)]
pub struct TracingWriter {
    buffer: String,
    level: tracing::Level,
}

impl TracingWriter {
    pub fn new(level: tracing::Level) -> Self {
        TracingWriter {
            buffer: String::new(),
            level,
        }
    }
}

impl std::io::Write for TracingWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match std::str::from_utf8(buf) {
            Ok(s) => {
                self.buffer.push_str(s);
                while let Some(idx) = self.buffer.find('\n') {
                    let line = self.buffer.drain(..=idx).collect::<String>();
                    trace_at_level(self.level, line.trim_end_matches('\n'));
                }
            }
            Err(_) => {
                // Fallback for non-UTF8 data
                self.buffer.push_str(&format!("{:?}", buf));
                while let Some(idx) = self.buffer.find('\n') {
                    let line = self.buffer.drain(..=idx).collect::<String>();
                    trace_at_level(self.level, line.trim_end_matches('\n'));
                }
            }
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if !self.buffer.is_empty() {
            trace_at_level(self.level, &self.buffer);
            self.buffer.clear();
        }
        Ok(())
    }
}

fn trace_at_level(level: tracing::Level, s: &str) {
    match level {
        tracing::Level::ERROR => error!("{}", s),
        tracing::Level::WARN => warn!("{}", s),
        tracing::Level::INFO => info!("{}", s),
        tracing::Level::DEBUG => debug!("{}", s),
        tracing::Level::TRACE => trace!("{}", s),
    }
}
