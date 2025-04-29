use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use boa_engine::{
    Context, JsError, JsNativeError, JsResult, JsString, Module, Source, js_string,
    module::{ModuleLoader, Referrer, resolve_module_specifier},
};
use boa_gc::GcRefCell;
use slipway_engine::ComponentFiles;
use tracing::{debug, error};

/// A component module loader that loads modules from the current component.
///
/// Code is based on the built in SimpleModuleLoader.
pub struct ComponentModuleLoader {
    files: Arc<ComponentFiles>,
    module_map: GcRefCell<HashMap<PathBuf, Module>>,
}

impl ComponentModuleLoader {
    /// Creates a new `ComponentModuleLoader` from a root module path.
    pub fn new(files: Arc<ComponentFiles>) -> JsResult<Self> {
        Ok(Self {
            files,
            module_map: GcRefCell::default(),
        })
    }

    /// Inserts a new module onto the module map.
    #[inline]
    pub fn insert(&self, path: PathBuf, module: Module) {
        self.module_map.borrow_mut().insert(path, module);
    }

    /// Gets a module from its original path.
    #[inline]
    pub fn get(&self, path: &Path) -> Option<Module> {
        self.module_map.borrow().get(path).cloned()
    }
}

impl ModuleLoader for ComponentModuleLoader {
    fn load_imported_module(
        &self,
        referrer: Referrer,
        specifier: JsString,
        finish_load: Box<dyn FnOnce(JsResult<Module>, &mut Context)>,
        context: &mut Context,
    ) {
        debug!(
            "Loading Javascript module {:?} from {:?}",
            specifier,
            referrer.path().expect("Referrer path should exist"),
        );
        let result = (|| {
            let short_path = specifier.to_std_string_escaped();

            // let referrer_path = referrer.path().unwrap_or(Path::new("/"));
            let path = resolve_module_specifier(None, &specifier, referrer.path(), context)?;
            if let Some(module) = self.get(&path) {
                return Ok(module);
            }

            // IMPROVEMENT: Do this asynchronously, or pre-load all module files.
            let file_text = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    // Call your async function here
                    self.files.get_text(&path.to_string_lossy()).await
                })
            })
            .map_err(|err| {
                error!("Failed to read file `{}`\n{}", short_path, err);
                JsNativeError::typ()
                    .with_message(format!("could not read file `{short_path}`"))
                    .with_cause(JsError::from_opaque(js_string!(err.to_string()).into()))
            })?;
            // let file_text = self.files.try_get_text(&path).await.map_err(|err| {
            //     JsNativeError::typ()
            //         .with_message(format!("could not read file `{short_path}`"))
            //         .with_cause(JsError::from_opaque(js_string!(err.to_string()).into()))
            // })?;

            let source = Source::from_bytes(&*file_text).with_path(&path);
            let module = Module::parse(source, None, context).map_err(|err| {
                JsNativeError::syntax()
                    .with_message(format!("could not parse module `{short_path}`"))
                    .with_cause(err)
            })?;
            self.insert(path, module.clone());
            Ok(module)
        })();

        finish_load(result, context);
    }

    fn register_module(&self, specifier: JsString, module: Module) {
        let path = PathBuf::from(specifier.to_std_string_escaped());

        self.insert(path, module);
    }

    fn get_module(&self, specifier: JsString) -> Option<Module> {
        let path = specifier.to_std_string_escaped();

        self.get(Path::new(&path))
    }
}
