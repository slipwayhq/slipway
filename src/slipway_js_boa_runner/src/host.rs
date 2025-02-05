use boa_engine::{
    js_string, object::ObjectInitializer, property::Attribute, Context, JsResult, JsValue,
    NativeFunction,
};
use slipway_engine::{ComponentExecutionContext, RunComponentError};

type JsFunction = dyn Fn(&JsValue, &[JsValue], &mut Context) -> JsResult<JsValue> + 'static;

pub(super) fn prepare_canopy_host<'call, 'rig, 'runners, 'host, 'context>(
    host: &'host SlipwayHost<'call, 'rig, 'runners>,
    context: &'context mut Context,
) -> Result<(), RunComponentError>
where
    'host: 'context,
{
    let host_object = unsafe {
        // We know that the host, and more specifically the execution context inside, will live
        // for the duration of the javascript execution, so we can safely transmute the reference
        // to a static lifetime to satisfy Boa's requirements.
        let host_static: &'static SlipwayHost<'_, '_, '_> = std::mem::transmute(host);

        let func: Box<JsFunction> =
            Box::new(move |this, args, ctx| host_static.font(this, args, ctx));

        ObjectInitializer::new(context)
            .function(NativeFunction::from_closure(func), js_string!("font"), 1)
            .build()
    };

    // Register "console" as a global property so that JS code can call it.
    context
        .register_global_property(
            js_string!("slipway_host"),
            host_object,
            Attribute::default(),
        )
        .map_err(|e| {
            RunComponentError::Other(format!("Failed to add slipway host to Boa context.\n{}", e))
        })?;

    Ok(())
}

#[derive(Clone, Copy)]
pub struct SlipwayHost<'call, 'rig, 'runners> {
    execution_context: &'call ComponentExecutionContext<'call, 'rig, 'runners>,
}

impl<'call, 'rig, 'runners> SlipwayHost<'call, 'rig, 'runners> {
    pub fn new(execution_context: &'call ComponentExecutionContext<'call, 'rig, 'runners>) -> Self {
        Self { execution_context }
    }

    pub fn font(
        &self,
        _this: &JsValue,
        args: &[JsValue],
        context: &mut Context,
    ) -> JsResult<JsValue> {
        if args.is_empty() {
            return Ok(JsValue::null());
        }

        let font_stack = args
            .first()
            .expect("Should have a first argument")
            .to_string(context)
            .expect("Should be able to convert argument to string")
            .to_std_string_lossy();

        let result = ::slipway_host::fonts::font(self.execution_context, font_stack);

        Ok(JsValue::from_json(
            &serde_json::to_value(&result).expect("Font should serialize"),
            context,
        )
        .expect("Should be able to convert font to JS value"))
    }
}
