use std::{str::FromStr, sync::Arc};

use crate::{SlipwayReference, SpecialComponentReference};

use super::{ComponentFiles, LoadedComponent};

const SLIPWAY_PUBLISHER: &str = "slipway";

pub fn load_special_component(reference: &SpecialComponentReference) -> LoadedComponent {
    let definition = get_special_definition(reference);

    LoadedComponent {
        reference: SlipwayReference::Special(reference.clone()),
        definition: serde_json::to_string(&definition)
            .expect("Special component definition should be serializable"),
        files: Arc::new(NoFiles {
            reference: SlipwayReference::Special(reference.clone()),
        }),
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

impl ComponentFiles for NoFiles {
    fn get_component_reference(&self) -> &SlipwayReference {
        &self.reference
    }

    fn get_component_path(&self) -> &std::path::Path {
        std::path::Path::new(".")
    }

    fn exists(&self, _file_name: &str) -> Result<bool, crate::errors::ComponentLoadError> {
        Ok(false)
    }

    fn try_get_bin(
        &self,
        _file_name: &str,
    ) -> Result<Option<std::sync::Arc<Vec<u8>>>, crate::errors::ComponentLoadError> {
        Ok(None)
    }

    fn try_get_text(
        &self,
        _file_name: &str,
    ) -> Result<Option<std::sync::Arc<String>>, crate::errors::ComponentLoadError> {
        Ok(None)
    }
}
