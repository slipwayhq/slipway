use std::{collections::HashMap, rc::Rc, sync::Arc};

use crate::{
    utils::ExpectWith, ComponentCache, ComponentFiles, ComponentHandle, ComponentInput,
    ComponentPermission, SlipwayReference, PERMISSIONS_FULL_TRUST_VEC,
};

use super::component_runner::ComponentRunner;

#[derive(Clone)]
pub struct ComponentExecutionData<'rig> {
    pub input: Rc<ComponentInput>,
    pub context: ComponentExecutionContext<'rig>,
}

#[derive(Clone)]
pub struct ComponentExecutionContext<'rig> {
    pub permission_chain: Arc<PermissionChain<'rig>>,
    pub component_runners: &'rig [Box<dyn ComponentRunner<'rig>>],
    pub files: Arc<dyn ComponentFiles>,
    pub callout_context: CalloutContext<'rig>,
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

#[derive(Clone)]
pub struct CalloutContext<'rig> {
    callout_handle_to_reference: HashMap<&'rig ComponentHandle, &'rig SlipwayReference>,
    pub component_cache: &'rig ComponentCache,
}

impl<'rig> CalloutContext<'rig> {
    pub fn new(
        callout_handle_to_reference: HashMap<&'rig ComponentHandle, &'rig SlipwayReference>,
        component_cache: &'rig ComponentCache,
    ) -> Self {
        Self {
            callout_handle_to_reference,
            component_cache,
        }
    }

    pub fn get_component_reference_for_handle(
        &self,
        handle: &ComponentHandle,
    ) -> &SlipwayReference {
        self.callout_handle_to_reference
            .get(handle)
            .expect_with(|| format!("Callout reference not found for handle {:?}", handle))
    }
}
