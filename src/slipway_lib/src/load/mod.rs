use std::{collections::HashMap, sync::Arc};

use crate::{
    errors::ComponentLoadError, utils::ExpectWith, Component, Rig, Schema, SlipwayReference,
};

pub(super) mod basic_components_loader;
mod is_safe_path;
mod prime_component_cache;

const SLIPWAY_COMPONENT_FILE_NAME: &str = "slipway_component.json";
const SLIPWAY_COMPONENT_WASM_FILE_NAME: &str = "slipway_component.wasm";

pub trait ComponentsLoader {
    fn load_components(
        &self,
        component_references: &[SlipwayReference],
    ) -> Vec<Result<LoadedComponent, ComponentLoadError>>;
}

pub trait ComponentWasm {
    fn get(&self) -> Result<Arc<Vec<u8>>, ComponentLoadError>;
}

pub trait ComponentJson: Send + Sync {
    fn get(&self, file_name: &str) -> Result<Arc<serde_json::Value>, ComponentLoadError>;
}

pub struct LoadedComponent {
    pub reference: SlipwayReference,
    pub definition: String,
    pub wasm: Arc<dyn ComponentWasm>,
    pub json: Arc<dyn ComponentJson>,
}

impl LoadedComponent {
    pub fn new(
        reference: SlipwayReference,
        definition: String,
        wasm: Arc<dyn ComponentWasm>,
        json: Arc<dyn ComponentJson>,
    ) -> Self {
        Self {
            reference,
            definition,
            wasm,
            json,
        }
    }
}

pub struct ComponentCache {
    components: HashMap<SlipwayReference, PrimedComponent>,
}

impl ComponentCache {
    pub fn empty() -> Self {
        Self {
            components: HashMap::new(),
        }
    }

    pub fn primed(rig: &Rig, loader: &impl ComponentsLoader) -> Result<Self, ComponentLoadError> {
        prime_component_cache::prime_component_cache(rig, loader)
    }

    pub fn clear(&mut self) {
        self.components.clear();
    }

    pub fn add(
        &mut self,
        component_reference: &SlipwayReference,
        definition: Component<Schema>,
        wasm: Arc<dyn ComponentWasm>,
        json: Arc<dyn ComponentJson>,
    ) {
        self.components.insert(
            component_reference.clone(),
            PrimedComponent {
                definition: Arc::new(definition),
                wasm,
                json,
            },
        );
    }

    pub fn get_definition(&self, component_reference: &SlipwayReference) -> Arc<Component<Schema>> {
        self.get(component_reference).definition.clone()
    }

    pub fn get_wasm(
        &self,
        component_reference: &SlipwayReference,
    ) -> Result<Arc<Vec<u8>>, ComponentLoadError> {
        self.get(component_reference).wasm.get()
    }

    pub fn get_json(
        &self,
        component_reference: &SlipwayReference,
        file_name: &str,
    ) -> Result<Arc<serde_json::Value>, ComponentLoadError> {
        self.get(component_reference).json.get(file_name)
    }

    fn get(&self, component_reference: &SlipwayReference) -> &PrimedComponent {
        self.components
            .get(component_reference)
            .expect_with(|| format!("component {} not found in cache", component_reference))
    }
}

struct PrimedComponent {
    pub definition: Arc<Component<Schema>>,
    pub wasm: Arc<dyn ComponentWasm>,
    pub json: Arc<dyn ComponentJson>,
}
