use std::io::Write;

use crate::{
    render_state::{to_view_model::RigExecutionStateViewModel, write_state},
    run::RunEventHandler,
    tracing_writer::{TraceOrWriter, TracingWriter},
};

pub struct TracingRunEventHandler {
    w: TracingWriter,
}

impl TracingRunEventHandler {
    pub fn new() -> Self {
        Self {
            w: TracingWriter::new(TraceOrWriter::Trace(tracing::Level::DEBUG)),
        }
    }

    pub fn new_for(level: TraceOrWriter) -> Self {
        Self {
            w: TracingWriter::new(level),
        }
    }

    pub fn writer(&mut self) -> &mut TracingWriter {
        &mut self.w
    }
}

impl Default for TracingRunEventHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl<'rig, 'cache> RunEventHandler<'rig, 'cache, std::io::Error> for TracingRunEventHandler {
    fn handle_component_run_start<'state>(
        &mut self,
        event: crate::run::ComponentRunStartEvent<'rig>,
    ) -> Result<(), std::io::Error> {
        writeln!(self.w, r#"Running "{}"..."#, event.component_handle)?;
        Ok(())
    }

    fn handle_component_run_end(
        &mut self,
        _event: crate::run::ComponentRunEndEvent<'rig>,
    ) -> Result<(), std::io::Error> {
        Ok(())
    }

    fn handle_state_changed<'state>(
        &mut self,
        event: crate::run::StateChangeEvent<'rig, 'cache, 'state>,
    ) -> Result<RigExecutionStateViewModel<'state>, std::io::Error> {
        if event.is_complete {
            writeln!(self.w, "No more components to run.")?;
            write_state::<_, std::io::Error>(&mut self.w, event.state)
        } else {
            write_state::<_, std::io::Error>(&mut self.w, event.state)
        }
    }
}
