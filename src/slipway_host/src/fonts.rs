use fontique::{FamilyId, GenericFamily, QueryFamily, QueryStatus};
use serde::Serialize;
use slipway_engine::{ComponentExecutionContext, FontContext};
use tracing::{debug, warn};

// We can't use the Wasmtime/WIT generated ResolvedFont here, as this crate is host independent,
// so use our own struct.
#[derive(Debug, Serialize)]
pub struct ResolvedFont {
    pub family: String,
    pub data: Vec<u8>,
}

// Async in case we add support resolving from, e.g. Google Fonts in the future.
pub async fn font(
    execution_context: &ComponentExecutionContext<'_, '_, '_>,
    font_stack: String,
) -> Option<ResolvedFont> {
    let families: Vec<String> = font_stack
        .split(",")
        .map(|s| s.trim().trim_matches('"').to_string())
        .filter(|s| {
            if let Err(e) = crate::permissions::ensure_can_query_font(s, execution_context) {
                warn!(
                    "Removing font family \"{}\" from stack due to permissions: {}",
                    s, e.message
                );
                return false;
            }

            true
        })
        .collect();

    let context_mutex = execution_context.rig_session_options.font_context();
    let mut context = context_mutex.lock().await;

    try_resolve_font_families(&mut context, families)
}

fn try_resolve_font_families(
    context: &mut FontContext,
    families: Vec<String>,
) -> Option<ResolvedFont> {
    let result = try_resolve_with_context(context, families);

    match result {
        None => None,
        Some(resolved) => {
            let font_info = context
                .collection
                .family(resolved.0)
                .expect("resolved font family should exist");

            let font_name = font_info.name();
            debug!("Found font: {font_name}");

            Some(ResolvedFont {
                family: font_name.to_string(),
                data: resolved.1,
            })
        }
    }
}

fn try_resolve_with_context(
    context: &mut FontContext,
    names: Vec<String>,
) -> Option<(FamilyId, Vec<u8>)> {
    let (collection, source_cache) = context.spread();

    let mut query = collection.query(source_cache);
    let mut families = Vec::new();
    for name in names.iter() {
        match GenericFamily::parse(name) {
            Some(family) => {
                families.push(QueryFamily::Generic(family));
            }
            None => {
                families.push(QueryFamily::Named(name));
            }
        }
    }

    query.set_families(families);

    let mut result: Option<(FamilyId, Vec<u8>)> = None;
    query.matches_with(|font| {
        result = Some((font.family.0, Vec::from(font.blob.data())));
        QueryStatus::Stop
    });

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_resolve_common_font() {
        let mut context = FontContext::new();
        let families = vec!["Arial".to_string(), "DejaVu Sans".to_string()];
        let result = try_resolve_font_families(&mut context, families);
        assert!(result.is_some(), "Common font should be resolvable");
    }

    #[test]
    fn it_should_resolve_generic_font() {
        let mut context = FontContext::new();
        let families = vec!["sans-serif".to_string()];
        let result = try_resolve_font_families(&mut context, families);
        assert!(result.is_some(), "Sans-serif font should be resolvable");
    }

    #[test]
    fn it_should_return_none_for_non_existent_font() {
        let mut context = FontContext::new();
        let families = vec!["NonExistentFont".to_string()];
        let result = try_resolve_font_families(&mut context, families);
        assert!(result.is_none(), "NonExistentFont should not be resolvable");
    }

    #[test]
    fn test_try_resolve_with_fallbacks() {
        let mut context = FontContext::new();
        let families = vec!["NonExistentFont".to_string(), "sans-serif".to_string()];
        let result = try_resolve_font_families(&mut context, families);
        assert!(result.is_some(), "Fallback should be resolved");
    }
}
