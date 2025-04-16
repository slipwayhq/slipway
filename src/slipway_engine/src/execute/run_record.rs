use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use crate::{
    CallChain, Callout, ChainItem, ComponentHandle, ComponentInput, ComponentRigging, Description,
    Permission, PermissionsChainLink, Rigging, SlipwayReference,
};

use crate::parse::types::Rig;

#[derive(Default, Clone)]
pub(super) struct RigRunRecord {
    records: Arc<Mutex<Vec<ComponentRunRecord>>>,
}

struct ComponentRunRecord {
    component_handle: ComponentHandle,
    component_reference: SlipwayReference,
    permissions: Vec<PermissionsOwned>,
    input: Arc<ComponentInput>,
    callouts: Option<HashMap<ComponentHandle, Callout>>,
}

impl RigRunRecord {
    pub fn new() -> Self {
        RigRunRecord {
            records: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn push_run_record(
        &self,
        component_reference: SlipwayReference,
        call_chain: Arc<CallChain>,
        input: Arc<ComponentInput>,
        callouts: Option<HashMap<ComponentHandle, Callout>>,
    ) {
        let component_handle = call_chain.unique_handle();
        let permissions = call_chain
            .permission_trail()
            .iter()
            .filter_map(|p| match &p.permissions {
                ChainItem::Some(permissions) => Some(PermissionsOwned::from(permissions)),
                ChainItem::Inherit => None,
            })
            .collect();

        self.records
            .lock()
            .expect("should be able to lock run_record")
            .push(ComponentRunRecord {
                component_handle,
                component_reference,
                permissions,
                input,
                callouts,
            });
    }

    pub fn run_record_as_rig(&self) -> Rig {
        let mut rigging: HashMap<ComponentHandle, ComponentRigging> = HashMap::new();

        for record in self
            .records
            .lock()
            .expect("should be able to lock run record")
            .iter()
        {
            rigging.insert(
                record.component_handle.clone(),
                ComponentRigging {
                    component: record.component_reference.clone(),
                    input: Some(record.input.value.clone()),
                    allow: None,
                    deny: None,
                    permissions_chain: Some(
                        record
                            .permissions
                            .iter()
                            .map(|p| PermissionsChainLink {
                                allow: p.allow.clone(),
                                deny: p.deny.clone(),
                            })
                            .collect(),
                    ),
                    callouts: record.callouts.clone(),
                },
            );
        }

        Rig {
            description: Some(
                Description::from_str("Automatically generated debug rig.")
                    .expect("description should be valid"),
            ),
            constants: None,
            rigging: Rigging {
                components: rigging,
            },
        }
    }
}

impl std::fmt::Debug for RigRunRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Record Count: {}",
            self.records
                .lock()
                .expect("should be able to lock run record")
                .len()
        )
    }
}

struct PermissionsOwned {
    pub allow: Vec<Permission>,
    pub deny: Vec<Permission>,
}

impl<'a> From<&crate::Permissions<'a>> for PermissionsOwned {
    fn from(permissions: &crate::Permissions<'a>) -> PermissionsOwned {
        PermissionsOwned {
            allow: permissions.allow.clone(),
            deny: permissions.deny.clone(),
        }
    }
}
