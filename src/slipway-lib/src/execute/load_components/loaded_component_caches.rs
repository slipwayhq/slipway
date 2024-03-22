use std::{collections::HashMap, sync::Arc};

use tokio::{runtime::Runtime, task::JoinHandle};

use crate::{Component, SlipwayReference};

use super::{
    component_loaders::ComponentLoader,
    load_component::{load_component, LoadedComponent},
};

pub(crate) trait LoadedComponentCache {
    fn prime_cache_for(&mut self, component_reference: &SlipwayReference);

    fn get_definition(
        &mut self,
        component_reference: &SlipwayReference,
    ) -> &LoadedComponent<Component>;

    fn get_wasm(&mut self, component_reference: &SlipwayReference) -> &LoadedComponent<Vec<u8>>;
}

pub(crate) struct InMemoryComponentCache {
    definition: InMemoryComponentPartCache<Component>,
    wasm: InMemoryComponentPartCache<Vec<u8>>,
    runtime: Runtime,
}

impl InMemoryComponentCache {
    pub(crate) fn new(
        definition_loaders: Vec<Box<dyn ComponentLoader<Component>>>,
        wasm_loaders: Vec<Box<dyn ComponentLoader<Vec<u8>>>>,
    ) -> Self {
        Self {
            definition: InMemoryComponentPartCache::new(definition_loaders),
            wasm: InMemoryComponentPartCache::new(wasm_loaders),
            runtime: tokio::runtime::Builder::new_multi_thread()
                .worker_threads(1)
                .thread_name("download-pool")
                .enable_io()
                .build()
                .expect("Tokio runtime should be created"),
        }
    }
}

impl LoadedComponentCache for InMemoryComponentCache {
    fn prime_cache_for(&mut self, component_reference: &SlipwayReference) {
        self.definition
            .prime_cache_for(component_reference, &self.runtime);
        self.wasm
            .prime_cache_for(component_reference, &self.runtime);
    }

    fn get_definition(
        &mut self,
        component_reference: &SlipwayReference,
    ) -> &LoadedComponent<Component> {
        self.definition.get(&self.runtime, component_reference)
    }

    fn get_wasm(&mut self, component_reference: &SlipwayReference) -> &LoadedComponent<Vec<u8>> {
        self.wasm.get(&self.runtime, component_reference)
    }
}

struct InMemoryComponentPartCache<T> {
    cache: HashMap<SlipwayReference, LoadedComponent<T>>,
    future_cache: HashMap<SlipwayReference, JoinHandle<LoadedComponent<T>>>,
    loaders: Arc<Vec<Box<dyn ComponentLoader<T>>>>,
}

impl<T> InMemoryComponentPartCache<T>
where
    T: Send + Sync + 'static,
{
    pub fn new(loaders: Vec<Box<dyn ComponentLoader<T>>>) -> Self {
        Self {
            cache: HashMap::new(),
            future_cache: HashMap::new(),
            loaders: Arc::new(loaders),
        }
    }

    fn prime_cache_for(&mut self, component_reference: &SlipwayReference, runtime: &Runtime) {
        let future = load_component(component_reference.clone(), self.loaders.clone());
        let join_handle = runtime.spawn(future);
        self.future_cache
            .insert(component_reference.clone(), join_handle);
    }

    fn get(
        &mut self,
        runtime: &Runtime,
        component_reference: &SlipwayReference,
    ) -> &LoadedComponent<T> {
        if let Some(future) = self.future_cache.remove(component_reference) {
            let result = runtime.block_on(future).expect("Join should be successful");
            self.cache.insert(component_reference.clone(), result);
        }

        if let Some(result) = self.cache.get(component_reference) {
            return result;
        }

        panic!("Component not found in cache")
    }
}
