use std::sync::Arc;

use crate::ComponentPermission;

use super::{CallChain, ChainItem};

/// Ensure that the check passes at every level of the call chain.
#[must_use]
pub fn ensure_permissions<F>(call_chain: Arc<CallChain<'_>>, check: F) -> bool
where
    F: Fn(&[ComponentPermission]) -> bool,
{
    let mut is_inheriting = true;
    let mut maybe_current = Some(call_chain);

    while let Some(current) = maybe_current {
        let permissions = current.permissions;
        match permissions {
            ChainItem::Some(permissions) => {
                is_inheriting = false;
                if !check(permissions) {
                    return false;
                }
            }
            ChainItem::Inherit => {
                is_inheriting = true;
            }
        }

        maybe_current = current.previous.as_ref().map(Arc::clone);
    }

    // If we are inheriting permissions and reach the end of the chain, check against empty permissions.
    if is_inheriting && !check(&[]) {
        return false;
    }

    true
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_ensure_permissions_one_level() {
        let permissions = vec![ComponentPermission::FullTrust];

        let call_chain = Arc::new(CallChain {
            component_handle: None,
            permissions: ChainItem::Some(&permissions),
            previous: None,
        });

        assert!(ensure_permissions(call_chain.clone(), |p| p
            .contains(&ComponentPermission::FullTrust)));

        assert!(!ensure_permissions(call_chain.clone(), |p| p
            .contains(&ComponentPermission::FetchAny)));
    }

    #[test]
    fn test_ensure_permissions_inherit() {
        let permissions1 = vec![
            ComponentPermission::FullTrust,
            ComponentPermission::FetchAny,
        ];
        let permissions2 = vec![ComponentPermission::FetchAny];

        let call_chain = Arc::new(CallChain {
            component_handle: None,
            permissions: ChainItem::Some(&permissions1),
            previous: Some(Arc::new(CallChain {
                component_handle: None,
                permissions: ChainItem::Inherit,
                previous: Some(Arc::new(CallChain {
                    component_handle: None,
                    permissions: ChainItem::Some(&permissions2),
                    previous: None,
                })),
            })),
        });

        assert!(ensure_permissions(call_chain.clone(), |p| p
            .contains(&ComponentPermission::FetchAny)));

        assert!(!ensure_permissions(call_chain.clone(), |p| p
            .contains(&ComponentPermission::FullTrust)));
    }
    #[test]
    fn test_ensure_permissions_inherit_empty() {
        let permissions = vec![ComponentPermission::FullTrust];

        let call_chain = Arc::new(CallChain {
            component_handle: None,
            permissions: ChainItem::Some(&permissions),
            previous: Some(Arc::new(CallChain {
                component_handle: None,
                permissions: ChainItem::Inherit, // Inherits from nothing.
                previous: None,
            })),
        });

        // Fails because the final inherit implicitly inherits from an empty permission set.
        assert!(!ensure_permissions(call_chain.clone(), |p| p
            .contains(&ComponentPermission::FullTrust)));
    }
}
