use std::{collections::HashMap, sync::Arc};

use crate::{errors::ComponentLoadError, utils::ExpectWith, App, Component, SlipwayReference};

pub(super) mod basic_components_loader;
mod prime_component_cache;

pub trait ComponentsLoader {
    fn load_components<'app>(
        &self,
        component_references: &[&'app SlipwayReference],
    ) -> Vec<Result<LoadedComponent<'app>, ComponentLoadError>>;
}

pub struct LoadedComponent<'app> {
    pub reference: &'app SlipwayReference,
    pub definition: String,
    pub wasm_bytes: Vec<u8>,
}

impl<'app> LoadedComponent<'app> {
    pub fn new(reference: &'app SlipwayReference, definition: String, wasm_bytes: Vec<u8>) -> Self {
        Self {
            reference,
            definition,
            wasm_bytes,
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
        definition: Component<jtd::Schema>,
        wasm_bytes: Vec<u8>,
    ) {
        self.components.insert(
            component_reference.clone(),
            PrimedComponent {
                definition: Arc::new(definition),
                wasm_bytes: Arc::new(wasm_bytes),
            },
        );
    }

    pub fn get_definition(
        &self,
        component_reference: &SlipwayReference,
    ) -> Arc<Component<jtd::Schema>> {
        self.get(component_reference).definition.clone()
    }

    pub fn get_wasm(&self, component_reference: &SlipwayReference) -> Arc<Vec<u8>> {
        self.get(component_reference).wasm_bytes.clone()
    }

    fn get(&self, component_reference: &SlipwayReference) -> &PrimedComponent {
        self.components
            .get(component_reference)
            .expect_with(|| format!("component {} not found in cache", component_reference))
    }
}

struct PrimedComponent {
    pub definition: Arc<Component<jtd::Schema>>,
    pub wasm_bytes: Arc<Vec<u8>>,
}
