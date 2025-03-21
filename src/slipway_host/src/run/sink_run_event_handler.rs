use crate::{
    render_state::to_view_model::{self, RigExecutionStateViewModel},
    run::RunEventHandler,
};

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
    fn handle_component_run_start<'state>(
        &mut self,
        _event: crate::run::ComponentRunStartEvent<'rig, 'cache, 'state>,
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
        event: crate::run::StateChangeEvent<'rig, 'cache, 'state>,
    ) -> Result<RigExecutionStateViewModel<'state>, ()> {
        let view_model = to_view_model::to_view_model(event.state);
        Ok(view_model)
    }
}
