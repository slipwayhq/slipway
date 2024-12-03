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

impl<'call, 'rig, 'runners> ComponentExecutionData<'call, 'rig, 'runners> {
    pub fn get_component_definition(&self, handle: &ComponentHandle) -> Arc<Component<Schema>> {
        let component_reference = self
            .context
            .callout_context
            .get_component_reference_for_handle(handle);

        let component_cache = self.context.callout_context.component_cache;
        let primed_component = component_cache.get(component_reference);

        Arc::clone(&primed_component.definition)
    }

    pub fn get_component_reference(&self, handle: &ComponentHandle) -> &'rig SlipwayReference {
        self.context
            .callout_context
            .get_component_reference_for_handle(handle)
    }
}

#[derive(Clone)]
pub struct ComponentExecutionContext<'call, 'rig, 'runners> {
    pub permission_chain: Arc<PermissionChain<'rig>>,
    pub component_runners: &'runners [Box<dyn ComponentRunner<'rig>>],
    pub files: Arc<dyn ComponentFiles>,
    pub callout_context: CalloutContext<'call, 'rig>,
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
pub struct CalloutContext<'call, 'rig> {
    callout_handle_to_reference: HashMap<&'call ComponentHandle, &'rig SlipwayReference>,
    pub component_cache: &'rig ComponentCache,
}

impl<'call, 'rig> CalloutContext<'call, 'rig> {
    pub fn new(
        callout_handle_to_reference: HashMap<&'call ComponentHandle, &'rig SlipwayReference>,
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
    ) -> &'rig SlipwayReference {
        self.callout_handle_to_reference
            .get(handle)
            .expect_with(|| format!("Callout reference not found for handle {:?}", handle))
    }
}
