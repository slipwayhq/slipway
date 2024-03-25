use std::{collections::HashMap, sync::Arc};

use tokio::{runtime::Runtime, task::JoinHandle};

use crate::{Component, SlipwayReference};

use super::{
    super::{
        loaders::ComponentPartLoader,
        try_load_component_part::{try_load_component_part, LoadComponentResult},
    },
    LoadedComponentCache,
};

pub(crate) struct InMemoryComponentCache {
    definition: InMemoryComponentPartCache<Component>,
    wasm: InMemoryComponentPartCache<Vec<u8>>,
    runtime: Runtime,
}

impl InMemoryComponentCache {
    pub(crate) fn new(
        definition_loaders: Vec<Box<dyn ComponentPartLoader<Component>>>,
        wasm_loaders: Vec<Box<dyn ComponentPartLoader<Vec<u8>>>>,
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
    ) -> &LoadComponentResult<Component> {
        self.definition.get(&self.runtime, component_reference)
    }

    fn get_wasm(
        &mut self,
        component_reference: &SlipwayReference,
    ) -> &LoadComponentResult<Vec<u8>> {
        self.wasm.get(&self.runtime, component_reference)
    }
}

struct InMemoryComponentPartCache<T> {
    cache: HashMap<SlipwayReference, LoadComponentResult<T>>,
    future_cache: HashMap<SlipwayReference, JoinHandle<LoadComponentResult<T>>>,
    loaders: Arc<Vec<Box<dyn ComponentPartLoader<T>>>>,
}

impl<T> InMemoryComponentPartCache<T>
where
    T: Send + Sync + 'static,
{
    pub fn new(loaders: Vec<Box<dyn ComponentPartLoader<T>>>) -> Self {
        Self {
            cache: HashMap::new(),
            future_cache: HashMap::new(),
            loaders: Arc::new(loaders),
        }
    }

    fn prime_cache_for(&mut self, component_reference: &SlipwayReference, runtime: &Runtime) {
        let future = try_load_component_part(component_reference.clone(), self.loaders.clone());
        let join_handle = runtime.spawn(future);
        self.future_cache
            .insert(component_reference.clone(), join_handle);
    }

    fn get(
        &mut self,
        runtime: &Runtime,
        component_reference: &SlipwayReference,
    ) -> &LoadComponentResult<T> {
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
