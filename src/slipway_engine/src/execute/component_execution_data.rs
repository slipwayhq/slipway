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
    // pub component_handle: &'rig ComponentHandle,
    pub component_reference: &'rig SlipwayReference,
    pub component_definition: Arc<Component<Schema>>,
    pub component_cache: &'rig dyn ComponentCache,
    pub component_runners: &'runners [Box<dyn ComponentRunner>],
    pub call_chain: Arc<CallChain<'rig>>,
    pub files: Arc<dyn ComponentFiles>,
    pub callout_context: CalloutContext<'call, 'rig>,
}

impl ComponentExecutionContext<'_, '_, '_> {
    pub fn component_handle(&self) -> &ComponentHandle {
        self.call_chain.current_component_handle()
    }

    pub fn component_handle_trail(&self) -> String {
        self.call_chain.component_handle_trail()
    }
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
            .expect_with(|| format!(r#"Component reference not found for handle "{}""#, handle))
    }
}

#[derive(Clone)]
pub struct CallChain<'rig> {
    component_handle: Option<&'rig ComponentHandle>,
    permissions: Option<&'rig Vec<ComponentPermission>>,
    previous: Option<Arc<CallChain<'rig>>>,
}

const CHAIN_SEPARATOR: &str = " -> ";

impl<'rig> CallChain<'rig> {
    pub fn new(permissions: &'rig Vec<ComponentPermission>) -> CallChain<'rig> {
        CallChain {
            component_handle: None,
            permissions: Some(permissions),
            previous: None,
        }
    }

    pub fn new_child(
        component_handle: &'rig ComponentHandle,
        permissions: Option<&'rig Vec<ComponentPermission>>,
        previous: Arc<CallChain<'rig>>,
    ) -> CallChain<'rig> {
        CallChain {
            component_handle: Some(component_handle),
            permissions,
            previous: Some(previous),
        }
    }

    pub fn new_child_arc(
        component_handle: &'rig ComponentHandle,
        permissions: Option<&'rig Vec<ComponentPermission>>,
        previous: Arc<CallChain<'rig>>,
    ) -> Arc<CallChain<'rig>> {
        Arc::new(CallChain::new_child(
            component_handle,
            permissions,
            previous,
        ))
    }

    pub fn full_trust() -> CallChain<'rig> {
        CallChain {
            component_handle: None,
            permissions: Some(&PERMISSIONS_FULL_TRUST_VEC),
            previous: None,
        }
    }

    pub fn full_trust_arc() -> Arc<CallChain<'rig>> {
        Arc::new(CallChain {
            component_handle: None,
            permissions: Some(&PERMISSIONS_FULL_TRUST_VEC),
            previous: None,
        })
    }

    pub fn component_handle_trail(&self) -> String {
        let mut trail = format!("{}", self.current_component_handle());
        let mut maybe_current = &self.previous;

        while let Some(current) = maybe_current {
            if let Some(component_handle) = current.component_handle {
                trail.insert_str(0, &format!("{}{}", component_handle, CHAIN_SEPARATOR));
            }
            maybe_current = &current.previous;
        }

        trail
    }

    pub fn component_handle_trail_for(&self, handle: &ComponentHandle) -> String {
        let mut trail = self.component_handle_trail();
        let handle_string = format!("{}", handle);
        if trail.is_empty() {
            handle_string
        } else {
            trail.push_str(CHAIN_SEPARATOR);
            trail.push_str(&handle_string);
            trail
        }
    }

    pub fn current_component_handle(&self) -> &'rig ComponentHandle {
        self.component_handle
            .expect("No component handle in current call chain head")
    }
}
