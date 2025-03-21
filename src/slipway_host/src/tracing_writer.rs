use tracing::{debug, error, info, trace, warn};

pub struct TracingWriter {
    buffer: String,
    level: TraceOrWriter,
}

pub enum TraceOrWriter {
    Trace(tracing::Level),
    Writer(Box<dyn std::io::Write>),
}

impl TracingWriter {
    pub fn new(level: TraceOrWriter) -> Self {
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
                    trace_at_level(&mut self.level, line.trim_end_matches('\n'))?;
                }
            }
            Err(_) => {
                // Fallback for non-UTF8 data
                self.buffer.push_str(&format!("{:?}", buf));
                while let Some(idx) = self.buffer.find('\n') {
                    let line = self.buffer.drain(..=idx).collect::<String>();
                    trace_at_level(&mut self.level, line.trim_end_matches('\n'))?;
                }
            }
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if !self.buffer.is_empty() {
            trace_at_level(&mut self.level, &self.buffer)?;
            self.buffer.clear();
        }
        Ok(())
    }
}

fn trace_at_level(level: &mut TraceOrWriter, s: &str) -> std::io::Result<()> {
    match level {
        TraceOrWriter::Writer(w) => {
            writeln!(w, "{}", s)?;
        }
        TraceOrWriter::Trace(level) => match *level {
            tracing::Level::ERROR => error!("{}", s),
            tracing::Level::WARN => warn!("{}", s),
            tracing::Level::INFO => info!("{}", s),
            tracing::Level::DEBUG => debug!("{}", s),
            tracing::Level::TRACE => trace!("{}", s),
        },
    }

    Ok(())
}
