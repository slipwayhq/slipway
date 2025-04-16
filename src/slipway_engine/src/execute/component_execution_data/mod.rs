use std::{collections::HashMap, str::FromStr, sync::Arc};

use crate::{
    Callout, Component, ComponentCache, ComponentFiles, ComponentHandle, ComponentInput,
    PERMISSIONS_ALL_VEC, PERMISSIONS_NONE_VEC, Permission, RigSessionOptions, Schema,
    SlipwayReference, errors::RigError,
};

use super::component_runner::ComponentRunner;

pub(crate) mod permissions;

#[derive(Clone)]
pub struct ComponentExecutionData<'call, 'rig, 'runners> {
    pub input: Arc<ComponentInput>,
    pub context: ComponentExecutionContext<'call, 'rig, 'runners>,
}

#[derive(Clone)]
pub struct ComponentExecutionContext<'call, 'rig, 'runners> {
    pub component_reference: &'rig SlipwayReference,
    pub component_definition: Arc<Component<Schema>>,
    pub component_cache: &'rig dyn ComponentCache,
    pub component_runners: &'runners [Box<dyn ComponentRunner>],
    pub call_chain: Arc<CallChain<'rig>>,
    pub files: Arc<ComponentFiles>,
    pub callout_context: CalloutContext<'call, 'rig>,
    pub rig_session_options: &'rig RigSessionOptions,
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
    callout_handle_to_callout: HashMap<&'call ComponentHandle, &'rig Callout>,
}

impl<'call, 'rig> CalloutContext<'call, 'rig> {
    pub fn new(callout_handle_to_callout: HashMap<&'call ComponentHandle, &'rig Callout>) -> Self {
        Self {
            callout_handle_to_callout,
        }
    }

    pub fn get_component_callout_for_handle(
        &self,
        handle: &ComponentHandle,
    ) -> Result<&'rig Callout, RigError> {
        self.callout_handle_to_callout
            .get(handle)
            .copied()
            .ok_or_else(|| RigError::ComponentNotFound {
                handle: handle.clone(),
            })
    }
}

#[derive(Copy, Clone, Debug)]
pub enum ChainItem<T> {
    Some(T),
    Inherit,
}

#[derive(Clone, Debug)]
pub struct CallChain<'rig> {
    component_handle: Option<&'rig ComponentHandle>,
    permissions: ChainItem<Permissions<'rig>>,
    previous: Option<Arc<CallChain<'rig>>>,
}

#[derive(Clone, Debug)]
pub struct Permissions<'rig> {
    pub allow: &'rig Vec<Permission>,
    pub deny: &'rig Vec<Permission>,
}

impl<'rig> Permissions<'rig> {
    pub fn empty() -> Permissions<'static> {
        Permissions {
            allow: &PERMISSIONS_NONE_VEC,
            deny: &PERMISSIONS_NONE_VEC,
        }
    }

    pub fn allow_all() -> Permissions<'rig> {
        Permissions {
            allow: &PERMISSIONS_ALL_VEC,
            deny: &PERMISSIONS_NONE_VEC,
        }
    }

    pub fn new(allow: &'rig Vec<Permission>, deny: &'rig Vec<Permission>) -> Permissions<'rig> {
        Permissions { allow, deny }
    }

    pub fn allow(allow: &'rig Vec<Permission>) -> Permissions<'rig> {
        Permissions {
            allow,
            deny: &PERMISSIONS_NONE_VEC,
        }
    }

    pub fn deny(deny: &'rig Vec<Permission>) -> Permissions<'rig> {
        Permissions {
            allow: &PERMISSIONS_NONE_VEC,
            deny,
        }
    }
}

const CHAIN_SEPARATOR: &str = " -> ";
const HANDLE_SEPARATOR: &str = "_then_";

#[derive(Clone, Debug)]
pub struct CallChainLink<'rig> {
    pub handle: Option<&'rig ComponentHandle>,
    pub permissions: ChainItem<Permissions<'rig>>,
}

impl<'rig> CallChain<'rig> {
    pub fn new(permissions: Permissions<'rig>) -> CallChain<'rig> {
        CallChain {
            component_handle: None,
            permissions: ChainItem::Some(permissions),
            previous: None,
        }
    }

    pub fn new_for_component(
        component_handle: &'rig ComponentHandle,
        permissions: Permissions<'rig>,
    ) -> CallChain<'rig> {
        CallChain {
            component_handle: Some(component_handle),
            permissions: ChainItem::Some(permissions),
            previous: None,
        }
    }

    pub fn new_child(
        component_handle: &'rig ComponentHandle,
        permissions: ChainItem<Permissions<'rig>>,
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
        permissions: ChainItem<Permissions<'rig>>,
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
            permissions: ChainItem::Some(Permissions {
                allow: &PERMISSIONS_ALL_VEC,
                deny: &PERMISSIONS_NONE_VEC,
            }),
            previous: None,
        }
    }

    pub fn full_trust_arc() -> Arc<CallChain<'rig>> {
        Arc::new(CallChain {
            component_handle: None,
            permissions: ChainItem::Some(Permissions {
                allow: &PERMISSIONS_ALL_VEC,
                deny: &PERMISSIONS_NONE_VEC,
            }),
            previous: None,
        })
    }

    fn custom_component_handle_trail(&self, separator: &str) -> String {
        let mut trail = format!("{}", self.current_component_handle());
        let mut maybe_current = &self.previous;

        while let Some(current) = maybe_current {
            if let Some(component_handle) = current.component_handle {
                trail.insert_str(0, &format!("{}{}", component_handle, separator));
            }
            maybe_current = &current.previous;
        }

        trail
    }

    pub fn component_handle_trail(&self) -> String {
        self.custom_component_handle_trail(CHAIN_SEPARATOR)
    }

    pub fn rig_or_component_handle_trail_error_prefix(&self) -> String {
        if self.component_handle.is_none() && self.previous.is_none() {
            "Rig".to_string()
        } else {
            format!("Component \"{}\"", self.component_handle_trail())
        }
    }

    pub fn permission_trail(&self) -> Vec<CallChainLink<'rig>> {
        let mut result = vec![CallChainLink {
            handle: self.component_handle,
            permissions: self.permissions.clone(),
        }];
        let mut maybe_current = &self.previous;

        while let Some(current) = maybe_current {
            result.push(CallChainLink {
                handle: current.component_handle,
                permissions: current.permissions.clone(),
            });
            maybe_current = &current.previous;
        }

        result
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

    /// Creates a unique handle for the given handle, based on the current call chain.
    pub fn unique_handle(&self) -> ComponentHandle {
        let trail = self.custom_component_handle_trail(HANDLE_SEPARATOR);
        if trail.is_empty() {
            panic!("Call chain should not be empty");
        } else {
            ComponentHandle::from_str(&trail).expect("Handle should be valid")
        }
    }

    pub fn current_component_handle(&self) -> &'rig ComponentHandle {
        match self.component_handle {
            Some(handle) => handle,
            None => match &self.previous {
                Some(_) => INHERITED_HANDLE.get_or_init(|| {
                    ComponentHandle::from_str(INHERITED_HANDLE_STR)
                        .expect("Inherited handle placeholder should be a valid handle")
                }),
                None => ROOT_HANDLE.get_or_init(|| {
                    ComponentHandle::from_str(ROOT_HANDLE_STR).expect("Root handle should be valid")
                }),
            },
        }
    }
}

static INHERITED_HANDLE: std::sync::OnceLock<ComponentHandle> = std::sync::OnceLock::new();
static ROOT_HANDLE: std::sync::OnceLock<ComponentHandle> = std::sync::OnceLock::new();

const INHERITED_HANDLE_STR: &str = "__inherited";
const ROOT_HANDLE_STR: &str = "__root";
