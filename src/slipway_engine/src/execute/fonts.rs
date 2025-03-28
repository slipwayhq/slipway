use std::path::Path;

use fontique::{Collection, CollectionOptions, SourceCache, SourceCacheOptions};
use tracing::debug;

#[derive(Default)]
pub struct FontContext {
    pub collection: Collection,
    pub source_cache: SourceCache,
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

    pub async fn new_with_path(font_path: &Path) -> Self {
        let mut collection = Collection::new(CollectionOptions {
            shared: true,
            system_fonts: true,
        });

        // Add all the fonts in the font_path directory to the collection.
        if font_path.exists() && font_path.is_dir() {
            if let Ok(entries) = font_path.read_dir() {
                for entry in entries.flatten() {
                    if let Ok(path) = entry.path().canonicalize() {
                        if path.is_file() {
                            let data = tokio::fs::read(&path).await.unwrap_or_default();
                            let result = collection.register_fonts(data);
                            debug!("Registered fonts from: {:?}", path);
                            for font in result {
                                for info in font.1 {
                                    debug!("Font info: {:?}", info);
                                }
                            }
                        }
                    }
                }
            }
        }

        let source_cache = SourceCache::new(SourceCacheOptions { shared: true });

        Self {
            collection,
            source_cache,
        }
    }

    pub fn spread(&mut self) -> (&mut Collection, &mut SourceCache) {
        (&mut self.collection, &mut self.source_cache)
    }
}
