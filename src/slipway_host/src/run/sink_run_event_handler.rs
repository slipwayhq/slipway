use crate::run::RunEventHandler;

pub struct SinkRunEventHandler {}

impl SinkRunEventHandler {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for SinkRunEventHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl<'rig, 'cache> RunEventHandler<'rig, 'cache, ()> for SinkRunEventHandler {
    fn handle_component_run_start(
        &mut self,
        _event: crate::run::ComponentRunStartEvent<'rig>,
    ) -> Result<(), ()> {
        Ok(())
    }

    fn handle_component_run_end(
        &mut self,
        _event: crate::run::ComponentRunEndEvent<'rig>,
    ) -> Result<(), ()> {
        Ok(())
    }

    fn handle_state_changed<'state>(
        &mut self,
        _event: crate::run::StateChangeEvent<'rig, 'cache, 'state>,
    ) -> Result<(), ()> {
        Ok(())
    }
}
