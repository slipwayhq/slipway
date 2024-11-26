use std::io::Write;

use anyhow::Context;
use slipway_lib::{
    parse_rig, BasicComponentsLoader, ComponentCache, ComponentHandle, Immutable, Instruction,
    RigExecutionState, RigSession,
};
use slipway_wasmtime::run_component_wasm;

use crate::render_state::{write_state, write_state_with_outputs};

pub(super) fn run_rig<W: Write>(w: &mut W, input: std::path::PathBuf) -> anyhow::Result<()> {
    writeln!(w, "Launching {}", input.display())?;
    writeln!(w)?;

    let file_contents = std::fs::read_to_string(input.clone())
        .with_context(|| format!("Failed to read component from {}", input.display()))?;
    let rig = parse_rig(&file_contents)?;

    let component_cache = ComponentCache::primed(&rig, &BasicComponentsLoader::default())?;
    let session = RigSession::new(rig, component_cache);
    let state = session.initialize()?;

    run(w, &session, state)?;

    Ok(())
}

fn run<'rig, W: Write>(
    w: &mut W,
    _session: &'rig RigSession,
    mut state: Immutable<RigExecutionState<'rig>>,
) -> anyhow::Result<()> {
    loop {
        let ready_components: Vec<&ComponentHandle> = state
            .component_states
            .iter()
            .filter_map(|(&handle, component_state)| {
                if component_state.execution_input.is_some() && component_state.output().is_none() {
                    Some(handle)
                } else {
                    None
                }
            })
            .collect();

        if ready_components.is_empty() {
            writeln!(w, "No more components to run.")?;
            writeln!(w)?;
            write_state_with_outputs(
                w,
                &state,
                crate::render_state::PrintComponentOutputsType::LeafComponents,
            )?;

            break;
        }

        write_state(w, &state)?;

        for handle in ready_components {
            writeln!(w, r#"Running "{}"..."#, handle)?;

            let execution_data = state.get_component_execution_data(handle)?;

            let result = run_component_wasm(execution_data, handle)?;

            writeln!(w)?;

            state = state.step(Instruction::SetOutput {
                handle: handle.clone(),
                value: result.output,
                metadata: result.metadata,
            })?;
        }
    }

    Ok(())
}
