use std::collections::{HashMap, HashSet};

use crate::{errors::RigError, parse::types::primitives::ComponentHandle};

/// When we pull dependencies out of the Rig JSON we end up with references to handles
/// in the rigging mriging to component handle values.
/// This function converts the component handle values back to references to handles in the rigging,
/// so that all the handles in the dependency_map are references with the same lifetime.
pub(super) fn map_dependencies_to_rig_handles(
    dependency_map: HashMap<&ComponentHandle, HashSet<ComponentHandle>>,
) -> Result<HashMap<&ComponentHandle, HashSet<&ComponentHandle>>, RigError> {
    let mut result: HashMap<&ComponentHandle, HashSet<&ComponentHandle>> = HashMap::new();
    for (&k, v) in dependency_map.iter() {
        let mut refs = HashSet::with_capacity(v.len());
        for d in v {
            let lookup_result = dependency_map.get_key_value(d);
            let kr = match lookup_result {
                Some((kr, _)) => kr,
                None => {
                    return Err(RigError::RigValidationFailed {
                        error: format!("dependency {:?} not found in rigging component keys", d),
                    })
                }
            };
            refs.insert(*kr);
        }

        result.insert(k, refs);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn it_should_convert_values_to_references() {
        let component1 = ComponentHandle::from_str("component1").unwrap();
        let component2 = ComponentHandle::from_str("component2").unwrap();
        let component3 = ComponentHandle::from_str("component3").unwrap();

        let mut dependency_map: HashMap<&ComponentHandle, HashSet<ComponentHandle>> =
            HashMap::new();
        dependency_map.insert(&component1, [component2.clone()].into_iter().collect());

        dependency_map.insert(
            &component2,
            [component3.clone(), component2.clone()]
                .into_iter()
                .collect(),
        );

        dependency_map.insert(&component3, HashSet::new());

        let result = map_dependencies_to_rig_handles(dependency_map).unwrap();

        let expected: HashMap<&ComponentHandle, HashSet<&ComponentHandle>> = [
            (&component1, [&component2].into_iter().collect()),
            (
                &component2,
                [&component3, &component2].into_iter().collect(),
            ),
            (&component3, HashSet::new()),
        ]
        .into_iter()
        .collect();

        assert_eq!(result, expected);
    }
}
