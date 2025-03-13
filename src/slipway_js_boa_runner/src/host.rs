use std::future::Future;

use boa_engine::{
    js_string,
    object::{
        builtins::{JsArrayBuffer, JsPromise},
        ObjectInitializer,
    },
    property::{Attribute, PropertyKey},
    Context, JsError, JsResult, JsValue, NativeFunction,
};
use serde::Deserialize;
use slipway_engine::{ComponentExecutionContext, RunComponentError};
use slipway_host::{
    fetch::{RequestError, RequestOptions},
    ComponentError,
};

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

        let mut object_initializer = ObjectInitializer::new(context);

        macro_rules! add_function {
            ($name:ident) => {{
                let f: Box<JsFunction> =
                    Box::new(move |this, args, ctx| host_static.$name(this, args, ctx));

                object_initializer.function(
                    NativeFunction::from_closure(f),
                    js_string!(stringify!($name)),
                    1,
                );
            }};
        }

        macro_rules! add_function_async {
            ($name:ident) => {{
                let f: Box<JsFunction> = Box::new(move |this, args, ctx| {
                    let future = host_static.$name(this, args, ctx);
                    // We know our future only holds references to data that lives longer than the Boa runtime,
                    // but Boa needs the data to be static, so again we transmute to satisfy the requirements.
                    let future = std::mem::transmute::<
                        std::pin::Pin<Box<dyn Future<Output = JsResult<JsValue>> + '_>>,
                        std::pin::Pin<Box<dyn Future<Output = JsResult<JsValue>> + 'static>>,
                    >(Box::pin(future));
                    Ok(JsPromise::from_future(future, ctx).into())
                });

                object_initializer.function(
                    NativeFunction::from_closure(f),
                    js_string!(stringify!($name)),
                    1,
                );
            }};
        }

        add_function!(log_trace);
        add_function!(log_debug);
        add_function!(log_info);
        add_function!(log_warn);
        add_function!(log_error);
        add_function_async!(font);
        add_function_async!(fetch_bin);
        add_function_async!(fetch_text);
        add_function_async!(run);
        add_function_async!(load_bin);
        add_function_async!(load_text);
        add_function!(env);
        add_function!(encode_bin);
        add_function!(decode_bin);

        object_initializer.build()
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

    pub fn log_trace(
        &self,
        _this: &JsValue,
        args: &[JsValue],
        context: &mut Context,
    ) -> JsResult<JsValue> {
        if !args.is_empty() {
            let message = get_string_arg(args, 0, context)?;
            ::slipway_host::log::log_trace(message);
        }
        Ok(JsValue::null())
    }

    pub fn log_debug(
        &self,
        _this: &JsValue,
        args: &[JsValue],
        context: &mut Context,
    ) -> JsResult<JsValue> {
        if !args.is_empty() {
            let message = get_string_arg(args, 0, context)?;
            ::slipway_host::log::log_debug(message);
        }
        Ok(JsValue::null())
    }

    pub fn log_info(
        &self,
        _this: &JsValue,
        args: &[JsValue],
        context: &mut Context,
    ) -> JsResult<JsValue> {
        if !args.is_empty() {
            let message = get_string_arg(args, 0, context)?;
            ::slipway_host::log::log_info(message);
        }
        Ok(JsValue::null())
    }

    pub fn log_warn(
        &self,
        _this: &JsValue,
        args: &[JsValue],
        context: &mut Context,
    ) -> JsResult<JsValue> {
        if !args.is_empty() {
            let message = get_string_arg(args, 0, context)?;
            ::slipway_host::log::log_warn(message);
        }
        Ok(JsValue::null())
    }

    pub fn log_error(
        &self,
        _this: &JsValue,
        args: &[JsValue],
        context: &mut Context,
    ) -> JsResult<JsValue> {
        if !args.is_empty() {
            let message = get_string_arg(args, 0, context)?;
            ::slipway_host::log::log_error(message);
        }
        Ok(JsValue::null())
    }

    pub fn font<'a>(
        &'a self,
        _this: &JsValue,
        args: &[JsValue],
        context: &'a mut Context,
    ) -> impl Future<Output = JsResult<JsValue>> + 'a + use<'a> {
        let font_stack = get_string_arg(args, 0, context);

        async move {
            let Ok(font_stack) = font_stack else {
                return Ok(JsValue::null());
            };

            let result = ::slipway_host::fonts::font(self.execution_context, font_stack).await;

            JsValue::from_json(
                &serde_json::to_value(&result).map_err(|e| {
                    js_error_from("Failed to serialize font".to_string(), e, context)
                })?,
                context,
            )
            .map_err(|e| {
                js_error_from(
                    "Failed to convert font result to JsValue".to_string(),
                    e,
                    context,
                )
            })
        }
    }

    pub fn fetch_bin<'a>(
        &'a self,
        _this: &JsValue,
        args: &[JsValue],
        context: &'a mut Context,
    ) -> impl Future<Output = JsResult<JsValue>> + 'a + use<'a> {
        let url_opts = get_url_and_request_options(args, context);

        async move {
            let (url, opts) = url_opts?;
            ::slipway_host::fetch::fetch_bin(self.execution_context, &url, opts)
                .await
                .map_err(|e| js_error_from_request_error(e, context))
                .and_then(|response| value_to_js_value(response, context))
        }
    }

    pub fn fetch_text<'a>(
        &'a self,
        _this: &JsValue,
        args: &[JsValue],
        context: &'a mut Context,
    ) -> impl Future<Output = JsResult<JsValue>> + 'a + use<'a> {
        let url_opts = get_url_and_request_options(args, context);

        async move {
            let (url, opts) = url_opts?;
            ::slipway_host::fetch::fetch_text(self.execution_context, &url, opts)
                .await
                .map_err(|e| js_error_from_request_error(e, context))
                .and_then(|response| value_to_js_value(response, context))
        }
    }

    pub fn run<'a>(
        &'a self,
        _this: &JsValue,
        args: &[JsValue],
        context: &'a mut Context,
    ) -> impl Future<Output = JsResult<JsValue>> + 'a + use<'a> {
        let handle = get_string_arg(args, 0, context);
        let input = if args.len() >= 2 {
            get_json_arg(args, 1, context)
        } else {
            Ok(serde_json::json!({}))
        };

        async move {
            let Ok(handle) = handle else {
                return Err(js_error(
                    "Expected the component handle to run.".to_string(),
                    context,
                ));
            };

            let input = input?;

            ::slipway_host::fetch::run_json(self.execution_context, handle, input)
                .await
                .map_err(|e| js_error_from_component_error(e, context))
                .and_then(|response| value_to_js_value(response, context))
        }
    }

    pub fn load_bin<'a>(
        &'a self,
        _this: &JsValue,
        args: &[JsValue],
        context: &'a mut Context,
    ) -> impl Future<Output = JsResult<JsValue>> + 'a + use<'a> {
        let handle_path = get_handle_and_path(args, context);

        async move {
            let (handle, path) = handle_path?;

            ::slipway_host::fetch::load_bin(self.execution_context, handle, path)
                .await
                .map_err(|e| js_error_from_component_error(e, context))
                .and_then(|response| value_to_js_value(response, context))
        }
    }

    pub fn load_text<'a>(
        &'a self,
        _this: &JsValue,
        args: &[JsValue],
        context: &'a mut Context,
    ) -> impl Future<Output = JsResult<JsValue>> + 'a + use<'a> {
        let handle_path = get_handle_and_path(args, context);

        async move {
            let (handle, path) = handle_path?;

            ::slipway_host::fetch::load_text(self.execution_context, handle, path)
                .await
                .map_err(|e| js_error_from_component_error(e, context))
                .and_then(|response| value_to_js_value(response, context))
        }
    }

    pub fn env(
        &self,
        _this: &JsValue,
        args: &[JsValue],
        context: &mut Context,
    ) -> JsResult<JsValue> {
        if !args.is_empty() {
            let key = get_string_arg(args, 0, context)?;
            let value = ::slipway_host::fetch::env(self.execution_context, &key);

            if let Some(value) = value {
                return Ok(JsValue::new(js_string!(value)));
            }
        }

        Ok(JsValue::null())
    }

    pub fn encode_bin(
        &self,
        _this: &JsValue,
        args: &[JsValue],
        context: &mut Context,
    ) -> JsResult<JsValue> {
        if args.is_empty() {
            return Err(js_error("Expected a u8 array.".to_string(), context));
        }

        let bin = get_bin_arg(args, 0, context)?;
        let text = ::slipway_host::bin::encode_bin(self.execution_context, bin);
        value_to_js_value(text, context)
    }

    pub fn decode_bin(
        &self,
        _this: &JsValue,
        args: &[JsValue],
        context: &mut Context,
    ) -> JsResult<JsValue> {
        if args.is_empty() {
            return Err(js_error("Expected a string.".to_string(), context));
        }

        let text = get_string_arg(args, 0, context)?;
        let bin = ::slipway_host::bin::decode_bin(self.execution_context, text)
            .map_err(|e| js_error_from_component_error(e, context))?;

        JsArrayBuffer::from_byte_block(bin, context).map(JsValue::from)
    }
}
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum BytesOrString {
    Bytes(Vec<u8>),
    String(String),
}

#[derive(Debug, Default, Deserialize)]
struct JsRequestOptions {
    #[serde(default)]
    pub method: Option<String>,

    #[serde(default)]
    pub headers: Option<Vec<(String, String)>>,

    #[serde(default)]
    pub body: Option<BytesOrString>,

    #[serde(default)]
    pub timeout_ms: Option<u32>,
}

impl From<JsRequestOptions> for RequestOptions {
    fn from(value: JsRequestOptions) -> Self {
        RequestOptions {
            method: value.method,
            headers: value.headers,
            body: value.body.map(|b| match b {
                BytesOrString::Bytes(bytes) => bytes,
                BytesOrString::String(string) => string.into_bytes(),
            }),
            timeout_ms: value.timeout_ms,
        }
    }
}

fn get_handle_and_path(
    args: &[JsValue],
    context: &mut Context,
) -> Result<(String, String), JsError> {
    if args.len() < 2 {
        return Err(js_error(
            "Expected a component handle and a path to a file.".to_string(),
            context,
        ));
    }

    let handle = get_string_arg(args, 0, context)?;
    let path = get_string_arg(args, 1, context)?;

    Ok((handle, path))
}

fn get_url_and_request_options(
    args: &[JsValue],
    context: &mut Context,
) -> Result<(String, Option<RequestOptions>), JsError> {
    if args.is_empty() {
        return Err(js_error("Expected a URL to fetch.".to_string(), context));
    }

    let url = get_string_arg(args, 0, context)?;

    let request_options = if args.len() >= 2 {
        Some(get_arg::<JsRequestOptions>(args, 1, context)?)
    } else {
        None
    };

    Ok((url, request_options.map(Into::into)))
}

fn get_string_arg(
    args: &[JsValue],
    index: usize,
    context: &mut Context,
) -> Result<String, JsError> {
    args.get(index)
        .ok_or_else(|| {
            js_error(
                format!("Expected a string argument at position {index}."),
                context,
            )
        })
        .and_then(|arg| {
            arg.to_string(context).map_err(|e| {
                js_error_from(
                    format!("Failed to convert argument at position {index} to string."),
                    e,
                    context,
                )
            })
        })
        .map(|js_string| js_string.to_std_string_lossy())
}

fn get_bin_arg(args: &[JsValue], index: usize, context: &mut Context) -> Result<Vec<u8>, JsError> {
    args.get(index)
        .ok_or_else(|| {
            js_error(
                format!("Expected a u8 array argument at position {index}."),
                context,
            )
        })
        .and_then(|value| value_to_bin_array(value, context))
}

fn get_json_arg(
    args: &[JsValue],
    index: usize,
    context: &mut Context,
) -> Result<serde_json::Value, JsError> {
    args.get(index)
        .ok_or_else(|| {
            js_error(
                format!("Expected an argument at position {index}."),
                context,
            )
        })
        .and_then(|arg| {
            arg.to_json(context).map_err(|e| {
                js_error_from(
                    format!("Failed to convert argument at position {index} to JSON."),
                    e,
                    context,
                )
            })
        })
}

fn get_arg<T>(args: &[JsValue], index: usize, context: &mut Context) -> Result<T, JsError>
where
    T: for<'de> serde::Deserialize<'de>,
{
    get_json_arg(args, index, context).and_then(|json| {
        serde_json::from_value::<T>(json).map_err(|e| {
            js_error_from(
                format!("Failed to deserialize argument at position {index}."),
                e,
                context,
            )
        })
    })
}

fn value_to_bin_array(value: &JsValue, context: &mut Context) -> Result<Vec<u8>, JsError> {
    if let Some(array) = value.as_object() {
        if array.is_array() {
            let length = array
                .get(PropertyKey::String(js_string!("length")), context)
                .map_err(|e| {
                    js_error_from("Failed to read length of array.".to_string(), e, context)
                })?
                .to_number(context)
                .map_err(|e| {
                    js_error_from(
                        "Failed to convert length of array to number.".to_string(),
                        e,
                        context,
                    )
                })? as usize;

            let mut result = Vec::with_capacity(length);
            for i in 0..length {
                let num = array
                    .get(i, context)
                    .map_err(|e| {
                        js_error_from(
                            format!("Failed to get array element at index {i}."),
                            e,
                            context,
                        )
                    })?
                    .to_number(context)
                    .map_err(|e| {
                        js_error_from(
                            format!("Failed to convert array element at index {i} to a u8."),
                            e,
                            context,
                        )
                    })? as u8;
                result.push(num);
            }

            return Ok(result);
        }
    } else if value.is_string() {
        let js_str = value.to_string(context)?;
        return Ok(js_str.to_std_string_escaped().into_bytes());
    }

    Err(js_error("Expected a u8 array.".to_string(), context))
}

fn value_to_js_value<T>(value: T, context: &mut Context) -> Result<JsValue, JsError>
where
    T: serde::Serialize,
{
    JsValue::from_json(
        &serde_json::to_value(value)
            .map_err(|e| js_error_from("Failed to serialize value".to_string(), e, context))?,
        context,
    )
}

fn js_error(message: String, context: &mut Context) -> JsError {
    js_error_from_component_error(ComponentError::for_error(message, None), context)
}

fn js_error_from(
    message: String,
    error: impl core::error::Error,
    context: &mut Context,
) -> JsError {
    js_error_from_component_error(
        ComponentError::for_error(message, Some(format!("{error}"))),
        context,
    )
}

fn js_error_from_request_error(error: RequestError, context: &mut Context) -> JsError {
    // We're using opaque errors here because I couldn't get native errors to flow
    // nicely through the JS layer, and because this way it is more consistent with
    // the WASM errors.
    JsError::from_opaque(
        value_to_js_value(error, context).expect("RequestError should be serializable"),
    )
}

fn js_error_from_component_error(error: ComponentError, context: &mut Context) -> JsError {
    JsError::from_opaque(
        value_to_js_value(error, context).expect("ComponentError should be serializable"),
    )
}
