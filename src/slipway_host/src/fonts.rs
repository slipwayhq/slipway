use std::sync::{Mutex, OnceLock};

use fontique::{
    Collection, CollectionOptions, FamilyId, GenericFamily, QueryFamily, QueryStatus, SourceCache,
    SourceCacheOptions,
};
use serde::Serialize;
use slipway_engine::ComponentExecutionContext;
use tracing::warn;

static CONTEXT: OnceLock<Mutex<FontContext>> = OnceLock::new();

// We can't use the Wasmtime/WIT generated ResolvedFont here, as this crate is host independent,
// so use our own struct.
#[derive(Debug, Serialize)]
pub struct ResolvedFont {
    pub family: String,
    pub data: Vec<u8>,
}

pub fn font(
    execution_context: &ComponentExecutionContext,
    font_stack: String,
) -> Option<ResolvedFont> {
    let families: Vec<String> = font_stack
        .split(",")
        .map(|s| s.trim().to_string())
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

    try_resolve_font_families(families)
}

fn try_resolve_font_families(families: Vec<String>) -> Option<ResolvedFont> {
    let context_mutex = get_context();
    let mut context = context_mutex
        .lock()
        .expect("should be able to acquire lock on font context");

    let result = try_resolve_with_context(&mut context, families);

    match result {
        None => None,
        Some(resolved) => {
            let font_info = context
                .collection
                .family(resolved.0)
                .expect("resolved font family should exist");

            Some(ResolvedFont {
                family: font_info.name().to_string(),
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

fn get_context() -> &'static Mutex<FontContext> {
    CONTEXT.get_or_init(|| Mutex::new(FontContext::new()))
}

struct FontContext {
    collection: Collection,
    source_cache: SourceCache,
}

impl FontContext {
    pub fn new() -> Self {
        Self {
            collection: Collection::new(CollectionOptions {
                shared: true,
                system_fonts: true,
            }),
            source_cache: SourceCache::new(SourceCacheOptions { shared: true }),
        }
    }

    pub fn spread(&mut self) -> (&mut Collection, &mut SourceCache) {
        (&mut self.collection, &mut self.source_cache)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_resolve_common_font() {
        let families = vec!["Arial".to_string()];
        let result = try_resolve_font_families(families);
        assert!(result.is_some(), "Arial font should be resolvable");
    }

    #[test]
    fn it_should_resolve_generic_font() {
        let families = vec!["sans-serif".to_string()];
        let result = try_resolve_font_families(families);
        assert!(result.is_some(), "Sans-serif font should be resolvable");
    }

    #[test]
    fn it_should_return_none_for_non_existent_font() {
        let families = vec!["NonExistentFont".to_string()];
        let result = try_resolve_font_families(families);
        assert!(result.is_none(), "NonExistentFont should not be resolvable");
    }

    #[test]
    fn test_try_resolve_with_fallbacks() {
        let families = vec!["NonExistentFont".to_string(), "sans-serif".to_string()];
        let result = try_resolve_font_families(families);
        assert!(result.is_some(), "Fallback should be resolved");
    }
}
