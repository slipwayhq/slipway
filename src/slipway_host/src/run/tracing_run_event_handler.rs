use std::io::Write;

use crate::{render_state::write_state, run::RunEventHandler, tracing_writer::TracingWriter};

pub(super) struct TracingRunEventHandler {
    w: TracingWriter,
}

impl TracingRunEventHandler {
    pub fn new() -> Self {
        Self {
            w: TracingWriter::new(tracing::Level::DEBUG),
        }
    }
}

impl<'rig, 'cache> RunEventHandler<'rig, 'cache, std::io::Error> for TracingRunEventHandler {
    fn handle_component_run_start(
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
    ) -> Result<(), std::io::Error> {
        if event.is_complete {
            writeln!(self.w, "No more components to run.")?;
            write_state::<_, std::io::Error>(&mut self.w, event.state)?;
        } else {
            write_state::<_, std::io::Error>(&mut self.w, event.state)?;
        }

        Ok(())
    }
}
