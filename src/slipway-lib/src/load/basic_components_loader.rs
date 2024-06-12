use crate::{errors::ComponentLoadError, SlipwayReference};

use super::{ComponentsLoader, LoadedComponent};

pub enum ComponentLoaderErrorBehavior {
    ErrorAlways,
    ErrorIfComponentNotLoaded,
}

#[derive(Default)]
pub struct BasicComponentsLoader {}

impl BasicComponentsLoader {
    pub fn new() -> Self {
        Self {}
    }
}

impl ComponentsLoader for BasicComponentsLoader {
    fn load_components<'app>(
        &self,
        component_references: &[&'app SlipwayReference],
    ) -> Vec<Result<LoadedComponent<'app>, ComponentLoadError>> {
        component_references
            .iter()
            .map(|r| load_component(r))
            .collect()
    }
}

fn load_component(
    component_reference: &SlipwayReference,
) -> Result<LoadedComponent, ComponentLoadError> {
    match component_reference {
        SlipwayReference::Local { path } => {
            let definition_string = std::fs::read_to_string(path).map_err(|e| {
                ComponentLoadError::DefinitionLoadFailed {
                    reference: component_reference.clone(),
                    error: e.to_string(),
                }
            })?;

            let wasm_path = path.with_extension("wasm");
            let wasm_bytes =
                std::fs::read(wasm_path).map_err(|e| ComponentLoadError::WasmLoadFailed {
                    reference: component_reference.clone(),
                    error: e.to_string(),
                })?;

            Ok(LoadedComponent::new(
                component_reference,
                definition_string,
                wasm_bytes,
            ))
        }
        _ => unimplemented!("Only local components are supported"),
    }
}
