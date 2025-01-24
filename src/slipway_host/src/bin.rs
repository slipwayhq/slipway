use std::sync::Arc;

use base64::prelude::*;
use slipway_engine::{CallChain, ComponentExecutionContext};

use crate::ComponentError;

pub fn encode_bin(_execution_context: &ComponentExecutionContext, bin: Vec<u8>) -> String {
    encode_bin_inner(bin)
}

fn encode_bin_inner(bin: Vec<u8>) -> String {
    BASE64_STANDARD.encode(bin)
}

pub fn decode_bin(
    execution_context: &ComponentExecutionContext,
    text: String,
) -> Result<Vec<u8>, ComponentError> {
    decode_bin_inner(Arc::clone(&execution_context.call_chain), text)
}

fn decode_bin_inner(
    call_chain: Arc<CallChain<'_>>,
    text: String,
) -> Result<Vec<u8>, ComponentError> {
    BASE64_STANDARD.decode(text).map_err(|e| {
        ComponentError::for_error(
            format!(
                "Failed to decode from base64 for component \"{}\".",
                call_chain.component_handle_trail()
            ),
            Some(format!("{e}")),
        )
    })
}

#[cfg(test)]
mod tests {
    use slipway_engine::{utils::ch, ComponentHandle, Permissions};

    use super::*;

    static CH: std::sync::OnceLock<ComponentHandle> = std::sync::OnceLock::new();

    #[test]
    fn test_encode_decode_bin() {
        let handle = CH.get_or_init(|| ch("test"));
        let call_chain = Arc::new(CallChain::new_for_component(
            handle,
            Permissions::allow_all(),
        ));

        let bin = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let encoded = encode_bin_inner(bin.clone());
        let decoded = decode_bin_inner(call_chain, encoded).unwrap();
        assert_eq!(bin, decoded);
    }
}
