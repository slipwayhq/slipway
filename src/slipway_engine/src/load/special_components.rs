use async_trait::async_trait;

use crate::{
    ComponentFilesLoader, LoadedComponent, PrimedComponent, SlipwayReference,
    SpecialComponentReference,
};

use std::{str::FromStr, sync::Arc};

use super::{prime_component_cache::parse_component_with_json, ComponentFiles};

const SLIPWAY_PUBLISHER: &str = "slipwayhq";

pub fn load_special_component(reference: &SpecialComponentReference) -> LoadedComponent {
    let definition = get_special_definition(reference);

    LoadedComponent {
        reference: SlipwayReference::Special(reference.clone()),
        definition: serde_json::to_string(&definition)
            .expect("Special component definition should be serializable"),
        files: Arc::new(ComponentFiles::new(Box::new(NoFiles {
            reference: SlipwayReference::Special(reference.clone()),
        }))),
    }
}

pub fn prime_special_component(reference: &SpecialComponentReference) -> PrimedComponent {
    let full_reference = SlipwayReference::Special(reference.clone());
    let definition = get_special_definition(reference);
    let files = Arc::new(ComponentFiles::new(Box::new(NoFiles {
        reference: SlipwayReference::Special(reference.clone()),
    })));

    let definition = parse_component_with_json(&full_reference, definition, Arc::clone(&files))
        .expect("Special component should be valid");

    PrimedComponent {
        definition: Arc::new(definition),
        files,
    }
}

fn get_special_definition(
    reference: &SpecialComponentReference,
) -> crate::Component<serde_json::Value> {
    crate::Component::<serde_json::Value> {
        publisher: crate::Publisher::from_str(SLIPWAY_PUBLISHER)
            .expect("Slipway publisher should be valid"),
        name: crate::Name::from_str(&format!("{}", reference))
            .expect("Slipway special component name should be valid"),
        version: semver::Version::new(1, 0, 0),
        description: None,
        input: serde_json::json!({}),
        output: serde_json::json!({}),
        constants: None,
        rigging: None,
        callouts: None,
    }
}

struct NoFiles {
    reference: SlipwayReference,
}

#[async_trait(?Send)]
impl ComponentFilesLoader for NoFiles {
    fn get_component_reference(&self) -> &SlipwayReference {
        &self.reference
    }

    fn get_component_path(&self) -> &std::path::Path {
        std::path::Path::new(".")
    }

    async fn exists(&self, _file_name: &str) -> Result<bool, crate::errors::ComponentLoadError> {
        Ok(false)
    }

    async fn try_get_bin(
        &self,
        _file_name: &str,
    ) -> Result<Option<std::sync::Arc<Vec<u8>>>, crate::errors::ComponentLoadError> {
        Ok(None)
    }

    async fn try_get_text(
        &self,
        _file_name: &str,
    ) -> Result<Option<std::sync::Arc<String>>, crate::errors::ComponentLoadError> {
        Ok(None)
    }
}
