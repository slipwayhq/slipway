use std::{
    cell::RefCell,
    collections::HashMap,
    path::{Path, PathBuf},
    rc::Rc,
    sync::Arc,
};

use boa_engine::{
    Context, JsError, JsNativeError, JsResult, JsString, JsValue, Module, Source,
    job::NativeAsyncJob,
    js_string,
    module::{ModuleLoader, Referrer, resolve_module_specifier},
};
use boa_gc::GcRefCell;
use slipway_engine::ComponentFiles;
use tracing::{debug, error};

/// A component module loader that loads modules from the current component.
///
/// Code is based on the built in SimpleModuleLoader and
/// https://github.com/boa-dev/boa/blob/main/examples/src/bin/module_fetch_async.rs
pub struct ComponentModuleLoader {
    files: Arc<ComponentFiles>,
    module_map: Rc<ModuleMap>,
}

struct ModuleMap {
    inner: GcRefCell<HashMap<PathBuf, Module>>,
}

impl ModuleMap {
    fn new() -> Self {
        Self {
            inner: GcRefCell::default(),
        }
    }

    /// Inserts a new module onto the module map.
    #[inline]
    pub fn insert(&self, path: PathBuf, module: Module) {
        self.inner.borrow_mut().insert(path, module);
    }

    /// Gets a module from its original path.
    #[inline]
    pub fn get(&self, path: &Path) -> Option<Module> {
        self.inner.borrow().get(path).cloned()
    }
}

impl ComponentModuleLoader {
    /// Creates a new `ComponentModuleLoader` from a root module path.
    pub fn new(files: Arc<ComponentFiles>) -> JsResult<Self> {
        Ok(Self {
            files,
            module_map: Rc::new(ModuleMap::new()),
        })
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

        let short_path = specifier.to_std_string_escaped();
        let files = Arc::clone(&self.files);
        let module_map = Rc::clone(&self.module_map);

        context.enqueue_job(
            NativeAsyncJob::with_realm(
                move |context| {
                    Box::pin(async move {
                        let result = load_module(
                            referrer, specifier, context, short_path, files, module_map,
                        )
                        .await;

                        finish_load(result, &mut context.borrow_mut());

                        Ok(JsValue::undefined())
                    })
                },
                context.realm().clone(),
            )
            .into(),
        );
    }

    fn register_module(&self, specifier: JsString, module: Module) {
        let path = PathBuf::from(specifier.to_std_string_escaped());

        self.module_map.insert(path, module);
    }

    fn get_module(&self, specifier: JsString) -> Option<Module> {
        let path = specifier.to_std_string_escaped();

        self.module_map.get(Path::new(&path))
    }
}

async fn load_module(
    referrer: Referrer,
    specifier: JsString,
    context: &RefCell<&mut Context>,
    short_path: String,
    files: Arc<ComponentFiles>,
    module_map: Rc<ModuleMap>,
) -> JsResult<Module> {
    let path =
        resolve_module_specifier(None, &specifier, referrer.path(), &mut context.borrow_mut())?;

    if let Some(module) = module_map.get(&path) {
        return Ok(module);
    }

    let file_text = files
        .get_text(&path.to_string_lossy())
        .await
        .map_err(|err| {
            error!("Failed to read file `{}`\n{}", short_path, err);
            JsNativeError::typ()
                .with_message(format!("could not read file `{short_path}`"))
                .with_cause(JsError::from_opaque(js_string!(err.to_string()).into()))
        })?;

    let source = Source::from_bytes(&*file_text).with_path(&path);
    let module = Module::parse(source, None, &mut context.borrow_mut()).map_err(|err| {
        JsNativeError::syntax()
            .with_message(format!("could not parse module `{short_path}`"))
            .with_cause(err)
    })?;
    module_map.insert(path, module.clone());

    Ok(module)
}
