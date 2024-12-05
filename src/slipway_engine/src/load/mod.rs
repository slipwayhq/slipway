use core::panic;
use std::{collections::HashMap, default, path::Path, sync::Arc};

use crate::{
    errors::{ComponentLoadError, ComponentLoadErrorInner},
    utils::ExpectWith,
    Component, Rig, Schema, SlipwayReference,
};

pub(super) mod basic_components_loader;
mod is_safe_path;
mod prime_component_cache;
pub(super) mod special_components;

const SLIPWAY_COMPONENT_FILE_NAME: &str = "slipway_component.json";

pub trait ComponentsLoader {
    fn load_components(
        &self,
        component_references: &[SlipwayReference],
    ) -> Vec<Result<LoadedComponent, ComponentLoadError>>;
}

// We return Arcs here so that the implementors can cache files in memory if they want to.
// This was originally the case with the WebAssembly files, but currently we don't do any caching.
pub trait ComponentFiles: Send + Sync {
    fn get_component_reference(&self) -> &SlipwayReference;
    fn get_component_path(&self) -> &Path;
    fn exists(&self, file_name: &str) -> Result<bool, ComponentLoadError>;
    fn try_get_bin(&self, file_name: &str) -> Result<Option<Arc<Vec<u8>>>, ComponentLoadError>;
    fn try_get_text(&self, file_name: &str) -> Result<Option<Arc<String>>, ComponentLoadError>;

    fn try_get_json(
        &self,
        file_name: &str,
    ) -> Result<Option<Arc<serde_json::Value>>, ComponentLoadError> {
        let buffer = self.try_get_bin(file_name)?;

        match buffer {
            None => Ok(None),
            Some(buffer) => {
                let slice = buffer.as_slice();
                let json = serde_json::from_slice(slice).map_err(|e| {
                    ComponentLoadError::new(
                        self.get_component_reference(),
                        ComponentLoadErrorInner::FileJsonParseFailed {
                            path: format!(
                                "{}{}{}",
                                self.get_component_path().to_string_lossy(),
                                self.get_component_file_separator(),
                                file_name
                            ),
                            error: Arc::new(e),
                        },
                    )
                })?;
                Ok(Some(Arc::new(json)))
            }
        }
    }

    fn get_json(&self, file_name: &str) -> Result<Arc<serde_json::Value>, ComponentLoadError> {
        self.try_get_json(file_name)?
            .ok_or_else(|| self.get_file_not_found_error(file_name))
    }

    fn get_bin(&self, file_name: &str) -> Result<Arc<Vec<u8>>, ComponentLoadError> {
        self.try_get_bin(file_name)?
            .ok_or_else(|| self.get_file_not_found_error(file_name))
    }

    fn get_text(&self, file_name: &str) -> Result<Arc<String>, ComponentLoadError> {
        self.try_get_text(file_name)?
            .ok_or_else(|| self.get_file_not_found_error(file_name))
    }

    fn get_component_file_separator(&self) -> &str {
        "/"
    }

    fn get_file_not_found_error(&self, file_name: &str) -> ComponentLoadError {
        ComponentLoadError::new(
            self.get_component_reference(),
            ComponentLoadErrorInner::FileLoadFailed {
                path: format!(
                    "{}{}{}",
                    self.get_component_path().to_string_lossy(),
                    self.get_component_file_separator(),
                    file_name
                ),
                error: format!("Component does not contain the file \"{}\"", file_name),
            },
        )
    }
}
pub struct LoadedComponent {
    pub reference: SlipwayReference,
    pub definition: String,
    pub files: Arc<dyn ComponentFiles>,
}

impl LoadedComponent {
    pub fn new(
        reference: SlipwayReference,
        definition: String,
        files: Arc<dyn ComponentFiles>,
    ) -> Self {
        Self {
            reference,
            definition,
            files,
        }
    }
}

pub trait ComponentCache: Sync + Send {
    fn clear(&mut self);

    fn add(
        &mut self,
        component_reference: &SlipwayReference,
        definition: Component<Schema>,
        files: Arc<dyn ComponentFiles>,
    );

    fn try_get(&self, component_reference: &SlipwayReference) -> Option<&PrimedComponent>;

    fn get(&self, component_reference: &SlipwayReference) -> &PrimedComponent;
}

pub struct PrimedComponent {
    pub definition: Arc<Component<Schema>>,
    pub files: Arc<dyn ComponentFiles>,
}

pub struct BasicComponentCache {
    components: HashMap<SlipwayReference, PrimedComponent>,
}

impl BasicComponentCache {
    pub fn empty() -> Self {
        Self {
            components: HashMap::new(),
        }
    }

    pub fn primed(rig: &Rig, loader: &impl ComponentsLoader) -> Result<Self, ComponentLoadError> {
        prime_component_cache::prime_component_cache(rig, loader)
    }

    pub fn for_primed(components: HashMap<SlipwayReference, PrimedComponent>) -> Self {
        Self { components }
    }
}

impl default::Default for BasicComponentCache {
    fn default() -> Self {
        Self::empty()
    }
}

impl ComponentCache for BasicComponentCache {
    fn clear(&mut self) {
        self.components.clear();
    }

    fn add(
        &mut self,
        component_reference: &SlipwayReference,
        definition: Component<Schema>,
        files: Arc<dyn ComponentFiles>,
    ) {
        self.components.insert(
            component_reference.clone(),
            PrimedComponent {
                definition: Arc::new(definition),
                files,
            },
        );
    }

    fn try_get(&self, component_reference: &SlipwayReference) -> Option<&PrimedComponent> {
        self.components.get(component_reference)
    }

    fn get(&self, component_reference: &SlipwayReference) -> &PrimedComponent {
        self.components
            .get(component_reference)
            .expect_with(|| format!("component \"{}\" not found in cache", component_reference))
    }
}

pub struct MultiComponentCache<'a> {
    caches: Vec<&'a dyn ComponentCache>,
}

impl<'a> MultiComponentCache<'a> {
    pub fn new(caches: Vec<&'a dyn ComponentCache>) -> Self {
        Self { caches }
    }
}

impl ComponentCache for MultiComponentCache<'_> {
    fn clear(&mut self) {
        panic!("Cannot clear a MultiComponentCache");
    }

    fn add(
        &mut self,
        _component_reference: &SlipwayReference,
        _definition: Component<Schema>,
        _files: Arc<dyn ComponentFiles>,
    ) {
        panic!("Cannot add to a MultiComponentCache");
    }

    fn try_get(&self, component_reference: &SlipwayReference) -> Option<&PrimedComponent> {
        for cache in self.caches.iter() {
            if let Some(component) = cache.try_get(component_reference) {
                return Some(component);
            }
        }

        None
    }

    fn get(&self, component_reference: &SlipwayReference) -> &PrimedComponent {
        for cache in self.caches.iter() {
            if let Some(component) = cache.try_get(component_reference) {
                return component;
            }
        }

        panic!(
            "component \"{}\" not found in any cache",
            component_reference
        );
    }
}
