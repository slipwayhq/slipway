use std::{collections::HashMap, sync::Arc};

use crate::{
    errors::ComponentLoadError, utils::ExpectWith, App, Component, Schema, SlipwayReference,
};

pub(super) mod basic_components_loader;
mod prime_component_cache;

pub trait ComponentsLoader {
    fn load_components<'app>(
        &self,
        component_references: &[&'app SlipwayReference],
    ) -> Vec<Result<LoadedComponent<'app>, ComponentLoadError>>;
}

pub trait ComponentWasm {
    fn get(&self) -> Result<Arc<Vec<u8>>, ComponentLoadError>;
}

pub trait ComponentJson: Send + Sync {
    fn get(&self, file_name: &str) -> Result<Arc<serde_json::Value>, ComponentLoadError>;
}

pub struct LoadedComponent<'app> {
    pub reference: &'app SlipwayReference,
    pub definition: String,
    pub wasm: Arc<dyn ComponentWasm>,
    pub json: Arc<dyn ComponentJson>,
}

impl<'app> LoadedComponent<'app> {
    pub fn new(
        reference: &'app SlipwayReference,
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

    pub fn primed(app: &App, loader: &impl ComponentsLoader) -> Result<Self, ComponentLoadError> {
        prime_component_cache::prime_component_cache(app, loader)
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
