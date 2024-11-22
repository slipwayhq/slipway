use std::sync::{Mutex, OnceLock};

use fontique::{
    Collection, CollectionOptions, GenericFamily, QueryFamily, QueryStatus, SourceCache,
    SourceCacheOptions,
};

static CONTEXT: OnceLock<Mutex<FontContext>> = OnceLock::new();

pub fn try_resolve(family: String) -> Option<Vec<u8>> {
    let names: Vec<String> = family.split(",").map(|s| s.trim().to_string()).collect();

    let context_mutex = get_context();
    let mut context = context_mutex
        .lock()
        .expect("should be able to acquire lock on font context");

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

    let mut result: Option<Vec<u8>> = None;
    query.matches_with(|font| {
        result = Some(Vec::from(font.blob.data()));
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
        let family = "Arial".to_string();
        let result = try_resolve(family);
        assert!(result.is_some(), "Arial font should be resolvable");
    }

    #[test]
    fn it_should_resolve_generic_font() {
        let family = "sans-serif".to_string();
        let result = try_resolve(family);
        assert!(result.is_some(), "Sans-serif font should be resolvable");
    }

    #[test]
    fn it_should_return_none_for_non_existent_font() {
        let family = "NonExistentFont".to_string();
        let result = try_resolve(family);
        assert!(result.is_none(), "NonExistentFont should not be resolvable");
    }

    #[test]
    fn test_try_resolve_with_fallbacks() {
        let family = "NonExistentFont, sans-serif".to_string();
        let result = try_resolve(family);
        assert!(result.is_some(), "Fallback should be resolved");
    }
}
