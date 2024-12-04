use std::{collections::HashMap, rc::Rc, sync::Arc};

use crate::{
    utils::ExpectWith, Component, ComponentCache, ComponentFiles, ComponentHandle, ComponentInput,
    ComponentPermission, Schema, SlipwayReference, PERMISSIONS_FULL_TRUST_VEC,
};

use super::component_runner::ComponentRunner;

#[derive(Clone)]
pub struct ComponentExecutionData<'call, 'rig, 'runners> {
    pub input: Rc<ComponentInput>,
    pub context: ComponentExecutionContext<'call, 'rig, 'runners>,
}

#[derive(Clone)]
pub struct ComponentExecutionContext<'call, 'rig, 'runners> {
    pub component_handle: &'rig ComponentHandle,
    pub component_reference: &'rig SlipwayReference,
    pub component_definition: Arc<Component<Schema>>,
    pub component_cache: &'rig ComponentCache,
    pub component_runners: &'runners [Box<dyn ComponentRunner>],
    pub permission_chain: Arc<PermissionChain<'rig>>,
    pub files: Arc<dyn ComponentFiles>,
    pub callout_context: CalloutContext<'call, 'rig>,
}

#[derive(Clone)]
pub struct CalloutContext<'call, 'rig> {
    callout_handle_to_reference: HashMap<&'call ComponentHandle, &'rig SlipwayReference>,
}

impl<'call, 'rig> CalloutContext<'call, 'rig> {
    pub fn new(
        callout_handle_to_reference: HashMap<&'call ComponentHandle, &'rig SlipwayReference>,
    ) -> Self {
        Self {
            callout_handle_to_reference,
        }
    }

    pub fn get_component_reference_for_handle(
        &self,
        handle: &ComponentHandle,
    ) -> &'rig SlipwayReference {
        self.callout_handle_to_reference
            .get(handle)
            .expect_with(|| format!("Component reference not found for handle {:?}", handle))
    }
}

#[derive(Clone)]
pub struct PermissionChain<'rig> {
    current: &'rig Vec<ComponentPermission>,
    previous: Option<Arc<PermissionChain<'rig>>>,
}

impl<'rig> PermissionChain<'rig> {
    pub fn new(permissions: &'rig Vec<ComponentPermission>) -> PermissionChain<'rig> {
        PermissionChain {
            current: permissions,
            previous: None,
        }
    }

    pub fn new_child(
        permissions: &'rig Vec<ComponentPermission>,
        previous: Arc<PermissionChain<'rig>>,
    ) -> PermissionChain<'rig> {
        PermissionChain {
            current: permissions,
            previous: Some(previous),
        }
    }

    pub fn full_trust() -> PermissionChain<'rig> {
        PermissionChain {
            current: &PERMISSIONS_FULL_TRUST_VEC,
            previous: None,
        }
    }

    pub fn full_trust_arc() -> Arc<PermissionChain<'rig>> {
        Arc::new(PermissionChain {
            current: &PERMISSIONS_FULL_TRUST_VEC,
            previous: None,
        })
    }
}
