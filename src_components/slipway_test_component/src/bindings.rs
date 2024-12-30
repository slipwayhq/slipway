pub type ComponentError = slipway::component::types::ComponentError;
#[doc(hidden)]
#[allow(non_snake_case)]
pub unsafe fn _export_run_cabi<T: Guest>(arg0: *mut u8, arg1: usize) -> *mut u8 {
    #[cfg(target_arch = "wasm32")] _rt::run_ctors_once();
    let len0 = arg1;
    let bytes0 = _rt::Vec::from_raw_parts(arg0.cast(), len0, len0);
    let result1 = T::run(_rt::string_lift(bytes0));
    let ptr2 = _RET_AREA.0.as_mut_ptr().cast::<u8>();
    match result1 {
        Ok(e) => {
            *ptr2.add(0).cast::<u8>() = (0i32) as u8;
            let vec3 = (e.into_bytes()).into_boxed_slice();
            let ptr3 = vec3.as_ptr().cast::<u8>();
            let len3 = vec3.len();
            ::core::mem::forget(vec3);
            *ptr2.add(8).cast::<usize>() = len3;
            *ptr2.add(4).cast::<*mut u8>() = ptr3.cast_mut();
        }
        Err(e) => {
            *ptr2.add(0).cast::<u8>() = (1i32) as u8;
            let slipway::component::types::ComponentError { message: message4 } = e;
            let vec5 = (message4.into_bytes()).into_boxed_slice();
            let ptr5 = vec5.as_ptr().cast::<u8>();
            let len5 = vec5.len();
            ::core::mem::forget(vec5);
            *ptr2.add(8).cast::<usize>() = len5;
            *ptr2.add(4).cast::<*mut u8>() = ptr5.cast_mut();
        }
    };
    ptr2
}
#[doc(hidden)]
#[allow(non_snake_case)]
pub unsafe fn __post_return_run<T: Guest>(arg0: *mut u8) {
    let l0 = i32::from(*arg0.add(0).cast::<u8>());
    match l0 {
        0 => {
            let l1 = *arg0.add(4).cast::<*mut u8>();
            let l2 = *arg0.add(8).cast::<usize>();
            _rt::cabi_dealloc(l1, l2, 1);
        }
        _ => {
            let l3 = *arg0.add(4).cast::<*mut u8>();
            let l4 = *arg0.add(8).cast::<usize>();
            _rt::cabi_dealloc(l3, l4, 1);
        }
    }
}
pub trait Guest {
    fn run(input: _rt::String) -> Result<_rt::String, ComponentError>;
}
#[doc(hidden)]
macro_rules! __export_world_slipway_component_cabi {
    ($ty:ident with_types_in $($path_to_types:tt)*) => {
        const _ : () = { #[export_name = "run"] unsafe extern "C" fn export_run(arg0 : *
        mut u8, arg1 : usize,) -> * mut u8 { $($path_to_types)*:: _export_run_cabi::<$ty
        > (arg0, arg1) } #[export_name = "cabi_post_run"] unsafe extern "C" fn
        _post_return_run(arg0 : * mut u8,) { $($path_to_types)*:: __post_return_run::<$ty
        > (arg0) } };
    };
}
#[doc(hidden)]
pub(crate) use __export_world_slipway_component_cabi;
#[repr(align(4))]
struct _RetArea([::core::mem::MaybeUninit<u8>; 12]);
static mut _RET_AREA: _RetArea = _RetArea([::core::mem::MaybeUninit::uninit(); 12]);
#[allow(dead_code)]
pub mod slipway {
    #[allow(dead_code)]
    pub mod component {
        #[allow(dead_code, clippy::all)]
        pub mod types {
            #[used]
            #[doc(hidden)]
            static __FORCE_SECTION_REF: fn() = super::super::super::__link_custom_section_describing_imports;
            use super::super::super::_rt;
            #[derive(Clone)]
            pub struct ComponentError {
                pub message: _rt::String,
            }
            impl ::core::fmt::Debug for ComponentError {
                fn fmt(
                    &self,
                    f: &mut ::core::fmt::Formatter<'_>,
                ) -> ::core::fmt::Result {
                    f.debug_struct("ComponentError")
                        .field("message", &self.message)
                        .finish()
                }
            }
            impl ::core::fmt::Display for ComponentError {
                fn fmt(
                    &self,
                    f: &mut ::core::fmt::Formatter<'_>,
                ) -> ::core::fmt::Result {
                    write!(f, "{:?}", self)
                }
            }
            impl std::error::Error for ComponentError {}
        }
    }
}
#[allow(dead_code, clippy::all)]
pub mod font {
    #[used]
    #[doc(hidden)]
    static __FORCE_SECTION_REF: fn() = super::__link_custom_section_describing_imports;
    use super::_rt;
    #[derive(Clone)]
    pub struct ResolvedFont {
        pub family: _rt::String,
        pub data: _rt::Vec<u8>,
    }
    impl ::core::fmt::Debug for ResolvedFont {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            f.debug_struct("ResolvedFont")
                .field("family", &self.family)
                .field("data", &self.data)
                .finish()
        }
    }
    #[allow(unused_unsafe, clippy::all)]
    pub fn try_resolve(font_stack: &str) -> Option<ResolvedFont> {
        unsafe {
            #[repr(align(4))]
            struct RetArea([::core::mem::MaybeUninit<u8>; 20]);
            let mut ret_area = RetArea([::core::mem::MaybeUninit::uninit(); 20]);
            let vec0 = font_stack;
            let ptr0 = vec0.as_ptr().cast::<u8>();
            let len0 = vec0.len();
            let ptr1 = ret_area.0.as_mut_ptr().cast::<u8>();
            #[cfg(target_arch = "wasm32")]
            #[link(wasm_import_module = "font")]
            extern "C" {
                #[link_name = "try-resolve"]
                fn wit_import(_: *mut u8, _: usize, _: *mut u8);
            }
            #[cfg(not(target_arch = "wasm32"))]
            fn wit_import(_: *mut u8, _: usize, _: *mut u8) {
                unreachable!()
            }
            wit_import(ptr0.cast_mut(), len0, ptr1);
            let l2 = i32::from(*ptr1.add(0).cast::<u8>());
            match l2 {
                0 => None,
                1 => {
                    let e = {
                        let l3 = *ptr1.add(4).cast::<*mut u8>();
                        let l4 = *ptr1.add(8).cast::<usize>();
                        let len5 = l4;
                        let bytes5 = _rt::Vec::from_raw_parts(l3.cast(), len5, len5);
                        let l6 = *ptr1.add(12).cast::<*mut u8>();
                        let l7 = *ptr1.add(16).cast::<usize>();
                        let len8 = l7;
                        ResolvedFont {
                            family: _rt::string_lift(bytes5),
                            data: _rt::Vec::from_raw_parts(l6.cast(), len8, len8),
                        }
                    };
                    Some(e)
                }
                _ => _rt::invalid_enum_discriminant(),
            }
        }
    }
}
#[allow(dead_code, clippy::all)]
pub mod callout {
    #[used]
    #[doc(hidden)]
    static __FORCE_SECTION_REF: fn() = super::__link_custom_section_describing_imports;
    use super::_rt;
    pub type ComponentError = super::slipway::component::types::ComponentError;
    #[allow(unused_unsafe, clippy::all)]
    pub fn run(handle: &str, input: &str) -> Result<_rt::String, ComponentError> {
        unsafe {
            #[repr(align(4))]
            struct RetArea([::core::mem::MaybeUninit<u8>; 12]);
            let mut ret_area = RetArea([::core::mem::MaybeUninit::uninit(); 12]);
            let vec0 = handle;
            let ptr0 = vec0.as_ptr().cast::<u8>();
            let len0 = vec0.len();
            let vec1 = input;
            let ptr1 = vec1.as_ptr().cast::<u8>();
            let len1 = vec1.len();
            let ptr2 = ret_area.0.as_mut_ptr().cast::<u8>();
            #[cfg(target_arch = "wasm32")]
            #[link(wasm_import_module = "callout")]
            extern "C" {
                #[link_name = "run"]
                fn wit_import(_: *mut u8, _: usize, _: *mut u8, _: usize, _: *mut u8);
            }
            #[cfg(not(target_arch = "wasm32"))]
            fn wit_import(_: *mut u8, _: usize, _: *mut u8, _: usize, _: *mut u8) {
                unreachable!()
            }
            wit_import(ptr0.cast_mut(), len0, ptr1.cast_mut(), len1, ptr2);
            let l3 = i32::from(*ptr2.add(0).cast::<u8>());
            match l3 {
                0 => {
                    let e = {
                        let l4 = *ptr2.add(4).cast::<*mut u8>();
                        let l5 = *ptr2.add(8).cast::<usize>();
                        let len6 = l5;
                        let bytes6 = _rt::Vec::from_raw_parts(l4.cast(), len6, len6);
                        _rt::string_lift(bytes6)
                    };
                    Ok(e)
                }
                1 => {
                    let e = {
                        let l7 = *ptr2.add(4).cast::<*mut u8>();
                        let l8 = *ptr2.add(8).cast::<usize>();
                        let len9 = l8;
                        let bytes9 = _rt::Vec::from_raw_parts(l7.cast(), len9, len9);
                        super::slipway::component::types::ComponentError {
                            message: _rt::string_lift(bytes9),
                        }
                    };
                    Err(e)
                }
                _ => _rt::invalid_enum_discriminant(),
            }
        }
    }
    #[allow(unused_unsafe, clippy::all)]
    pub fn get_text(handle: &str, path: &str) -> Result<_rt::String, ComponentError> {
        unsafe {
            #[repr(align(4))]
            struct RetArea([::core::mem::MaybeUninit<u8>; 12]);
            let mut ret_area = RetArea([::core::mem::MaybeUninit::uninit(); 12]);
            let vec0 = handle;
            let ptr0 = vec0.as_ptr().cast::<u8>();
            let len0 = vec0.len();
            let vec1 = path;
            let ptr1 = vec1.as_ptr().cast::<u8>();
            let len1 = vec1.len();
            let ptr2 = ret_area.0.as_mut_ptr().cast::<u8>();
            #[cfg(target_arch = "wasm32")]
            #[link(wasm_import_module = "callout")]
            extern "C" {
                #[link_name = "get-text"]
                fn wit_import(_: *mut u8, _: usize, _: *mut u8, _: usize, _: *mut u8);
            }
            #[cfg(not(target_arch = "wasm32"))]
            fn wit_import(_: *mut u8, _: usize, _: *mut u8, _: usize, _: *mut u8) {
                unreachable!()
            }
            wit_import(ptr0.cast_mut(), len0, ptr1.cast_mut(), len1, ptr2);
            let l3 = i32::from(*ptr2.add(0).cast::<u8>());
            match l3 {
                0 => {
                    let e = {
                        let l4 = *ptr2.add(4).cast::<*mut u8>();
                        let l5 = *ptr2.add(8).cast::<usize>();
                        let len6 = l5;
                        let bytes6 = _rt::Vec::from_raw_parts(l4.cast(), len6, len6);
                        _rt::string_lift(bytes6)
                    };
                    Ok(e)
                }
                1 => {
                    let e = {
                        let l7 = *ptr2.add(4).cast::<*mut u8>();
                        let l8 = *ptr2.add(8).cast::<usize>();
                        let len9 = l8;
                        let bytes9 = _rt::Vec::from_raw_parts(l7.cast(), len9, len9);
                        super::slipway::component::types::ComponentError {
                            message: _rt::string_lift(bytes9),
                        }
                    };
                    Err(e)
                }
                _ => _rt::invalid_enum_discriminant(),
            }
        }
    }
    #[allow(unused_unsafe, clippy::all)]
    pub fn get_bin(handle: &str, path: &str) -> Result<_rt::Vec<u8>, ComponentError> {
        unsafe {
            #[repr(align(4))]
            struct RetArea([::core::mem::MaybeUninit<u8>; 12]);
            let mut ret_area = RetArea([::core::mem::MaybeUninit::uninit(); 12]);
            let vec0 = handle;
            let ptr0 = vec0.as_ptr().cast::<u8>();
            let len0 = vec0.len();
            let vec1 = path;
            let ptr1 = vec1.as_ptr().cast::<u8>();
            let len1 = vec1.len();
            let ptr2 = ret_area.0.as_mut_ptr().cast::<u8>();
            #[cfg(target_arch = "wasm32")]
            #[link(wasm_import_module = "callout")]
            extern "C" {
                #[link_name = "get-bin"]
                fn wit_import(_: *mut u8, _: usize, _: *mut u8, _: usize, _: *mut u8);
            }
            #[cfg(not(target_arch = "wasm32"))]
            fn wit_import(_: *mut u8, _: usize, _: *mut u8, _: usize, _: *mut u8) {
                unreachable!()
            }
            wit_import(ptr0.cast_mut(), len0, ptr1.cast_mut(), len1, ptr2);
            let l3 = i32::from(*ptr2.add(0).cast::<u8>());
            match l3 {
                0 => {
                    let e = {
                        let l4 = *ptr2.add(4).cast::<*mut u8>();
                        let l5 = *ptr2.add(8).cast::<usize>();
                        let len6 = l5;
                        _rt::Vec::from_raw_parts(l4.cast(), len6, len6)
                    };
                    Ok(e)
                }
                1 => {
                    let e = {
                        let l7 = *ptr2.add(4).cast::<*mut u8>();
                        let l8 = *ptr2.add(8).cast::<usize>();
                        let len9 = l8;
                        let bytes9 = _rt::Vec::from_raw_parts(l7.cast(), len9, len9);
                        super::slipway::component::types::ComponentError {
                            message: _rt::string_lift(bytes9),
                        }
                    };
                    Err(e)
                }
                _ => _rt::invalid_enum_discriminant(),
            }
        }
    }
}
#[allow(dead_code, clippy::all)]
pub mod log {
    #[used]
    #[doc(hidden)]
    static __FORCE_SECTION_REF: fn() = super::__link_custom_section_describing_imports;
    #[allow(unused_unsafe, clippy::all)]
    pub fn trace(message: &str) {
        unsafe {
            let vec0 = message;
            let ptr0 = vec0.as_ptr().cast::<u8>();
            let len0 = vec0.len();
            #[cfg(target_arch = "wasm32")]
            #[link(wasm_import_module = "log")]
            extern "C" {
                #[link_name = "trace"]
                fn wit_import(_: *mut u8, _: usize);
            }
            #[cfg(not(target_arch = "wasm32"))]
            fn wit_import(_: *mut u8, _: usize) {
                unreachable!()
            }
            wit_import(ptr0.cast_mut(), len0);
        }
    }
    #[allow(unused_unsafe, clippy::all)]
    pub fn debug(message: &str) {
        unsafe {
            let vec0 = message;
            let ptr0 = vec0.as_ptr().cast::<u8>();
            let len0 = vec0.len();
            #[cfg(target_arch = "wasm32")]
            #[link(wasm_import_module = "log")]
            extern "C" {
                #[link_name = "debug"]
                fn wit_import(_: *mut u8, _: usize);
            }
            #[cfg(not(target_arch = "wasm32"))]
            fn wit_import(_: *mut u8, _: usize) {
                unreachable!()
            }
            wit_import(ptr0.cast_mut(), len0);
        }
    }
    #[allow(unused_unsafe, clippy::all)]
    pub fn info(message: &str) {
        unsafe {
            let vec0 = message;
            let ptr0 = vec0.as_ptr().cast::<u8>();
            let len0 = vec0.len();
            #[cfg(target_arch = "wasm32")]
            #[link(wasm_import_module = "log")]
            extern "C" {
                #[link_name = "info"]
                fn wit_import(_: *mut u8, _: usize);
            }
            #[cfg(not(target_arch = "wasm32"))]
            fn wit_import(_: *mut u8, _: usize) {
                unreachable!()
            }
            wit_import(ptr0.cast_mut(), len0);
        }
    }
    #[allow(unused_unsafe, clippy::all)]
    pub fn warn(message: &str) {
        unsafe {
            let vec0 = message;
            let ptr0 = vec0.as_ptr().cast::<u8>();
            let len0 = vec0.len();
            #[cfg(target_arch = "wasm32")]
            #[link(wasm_import_module = "log")]
            extern "C" {
                #[link_name = "warn"]
                fn wit_import(_: *mut u8, _: usize);
            }
            #[cfg(not(target_arch = "wasm32"))]
            fn wit_import(_: *mut u8, _: usize) {
                unreachable!()
            }
            wit_import(ptr0.cast_mut(), len0);
        }
    }
    #[allow(unused_unsafe, clippy::all)]
    pub fn error(message: &str) {
        unsafe {
            let vec0 = message;
            let ptr0 = vec0.as_ptr().cast::<u8>();
            let len0 = vec0.len();
            #[cfg(target_arch = "wasm32")]
            #[link(wasm_import_module = "log")]
            extern "C" {
                #[link_name = "error"]
                fn wit_import(_: *mut u8, _: usize);
            }
            #[cfg(not(target_arch = "wasm32"))]
            fn wit_import(_: *mut u8, _: usize) {
                unreachable!()
            }
            wit_import(ptr0.cast_mut(), len0);
        }
    }
}
#[allow(dead_code, clippy::all)]
pub mod http {
    #[used]
    #[doc(hidden)]
    static __FORCE_SECTION_REF: fn() = super::__link_custom_section_describing_imports;
    use super::_rt;
    pub type Header = (_rt::String, _rt::String);
    #[derive(Clone)]
    pub struct RequestOptions {
        pub method: _rt::String,
        pub headers: _rt::Vec<Header>,
        /// Optional request body for POST, PUT, etc.
        pub body: Option<_rt::String>,
        /// Timeout in milliseconds.
        pub timeout_ms: Option<u32>,
    }
    impl ::core::fmt::Debug for RequestOptions {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            f.debug_struct("RequestOptions")
                .field("method", &self.method)
                .field("headers", &self.headers)
                .field("body", &self.body)
                .field("timeout-ms", &self.timeout_ms)
                .finish()
        }
    }
    #[derive(Clone)]
    pub struct BinResponse {
        /// HTTP status code (e.g., 200, 404, etc.).
        pub status: u16,
        /// Any response headers (e.g. Content-Type).
        pub headers: _rt::Vec<Header>,
        /// Response body as bytes.
        pub body: _rt::Vec<u8>,
    }
    impl ::core::fmt::Debug for BinResponse {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            f.debug_struct("BinResponse")
                .field("status", &self.status)
                .field("headers", &self.headers)
                .field("body", &self.body)
                .finish()
        }
    }
    #[derive(Clone)]
    pub struct RequestError {
        pub message: _rt::String,
        pub response: Option<BinResponse>,
    }
    impl ::core::fmt::Debug for RequestError {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            f.debug_struct("RequestError")
                .field("message", &self.message)
                .field("response", &self.response)
                .finish()
        }
    }
    impl ::core::fmt::Display for RequestError {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            write!(f, "{:?}", self)
        }
    }
    impl std::error::Error for RequestError {}
    #[derive(Clone)]
    pub struct TextResponse {
        /// HTTP status code (e.g., 200, 404, etc.).
        pub status: u16,
        /// Any response headers (e.g. Content-Type).
        pub headers: _rt::Vec<Header>,
        /// Response body as bytes.
        pub body: _rt::String,
    }
    impl ::core::fmt::Debug for TextResponse {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            f.debug_struct("TextResponse")
                .field("status", &self.status)
                .field("headers", &self.headers)
                .field("body", &self.body)
                .finish()
        }
    }
    #[allow(unused_unsafe, clippy::all)]
    pub fn request_text(
        uri: &str,
        opts: Option<&RequestOptions>,
    ) -> Result<TextResponse, RequestError> {
        unsafe {
            let mut cleanup_list = _rt::Vec::new();
            #[repr(align(4))]
            struct RetArea([::core::mem::MaybeUninit<u8>; 36]);
            let mut ret_area = RetArea([::core::mem::MaybeUninit::uninit(); 36]);
            let vec0 = uri;
            let ptr0 = vec0.as_ptr().cast::<u8>();
            let len0 = vec0.len();
            let (
                result10_0,
                result10_1,
                result10_2,
                result10_3,
                result10_4,
                result10_5,
                result10_6,
                result10_7,
                result10_8,
                result10_9,
            ) = match opts {
                Some(e) => {
                    let RequestOptions {
                        method: method1,
                        headers: headers1,
                        body: body1,
                        timeout_ms: timeout_ms1,
                    } = e;
                    let vec2 = method1;
                    let ptr2 = vec2.as_ptr().cast::<u8>();
                    let len2 = vec2.len();
                    let vec6 = headers1;
                    let len6 = vec6.len();
                    let layout6 = _rt::alloc::Layout::from_size_align_unchecked(
                        vec6.len() * 16,
                        4,
                    );
                    let result6 = if layout6.size() != 0 {
                        let ptr = _rt::alloc::alloc(layout6).cast::<u8>();
                        if ptr.is_null() {
                            _rt::alloc::handle_alloc_error(layout6);
                        }
                        ptr
                    } else {
                        ::core::ptr::null_mut()
                    };
                    for (i, e) in vec6.into_iter().enumerate() {
                        let base = result6.add(i * 16);
                        {
                            let (t3_0, t3_1) = e;
                            let vec4 = t3_0;
                            let ptr4 = vec4.as_ptr().cast::<u8>();
                            let len4 = vec4.len();
                            *base.add(4).cast::<usize>() = len4;
                            *base.add(0).cast::<*mut u8>() = ptr4.cast_mut();
                            let vec5 = t3_1;
                            let ptr5 = vec5.as_ptr().cast::<u8>();
                            let len5 = vec5.len();
                            *base.add(12).cast::<usize>() = len5;
                            *base.add(8).cast::<*mut u8>() = ptr5.cast_mut();
                        }
                    }
                    let (result8_0, result8_1, result8_2) = match body1 {
                        Some(e) => {
                            let vec7 = e;
                            let ptr7 = vec7.as_ptr().cast::<u8>();
                            let len7 = vec7.len();
                            (1i32, ptr7.cast_mut(), len7)
                        }
                        None => (0i32, ::core::ptr::null_mut(), 0usize),
                    };
                    let (result9_0, result9_1) = match timeout_ms1 {
                        Some(e) => (1i32, _rt::as_i32(e)),
                        None => (0i32, 0i32),
                    };
                    cleanup_list.extend_from_slice(&[(result6, layout6)]);
                    (
                        1i32,
                        ptr2.cast_mut(),
                        len2,
                        result6,
                        len6,
                        result8_0,
                        result8_1,
                        result8_2,
                        result9_0,
                        result9_1,
                    )
                }
                None => {
                    (
                        0i32,
                        ::core::ptr::null_mut(),
                        0usize,
                        ::core::ptr::null_mut(),
                        0usize,
                        0i32,
                        ::core::ptr::null_mut(),
                        0usize,
                        0i32,
                        0i32,
                    )
                }
            };
            let ptr11 = ret_area.0.as_mut_ptr().cast::<u8>();
            #[cfg(target_arch = "wasm32")]
            #[link(wasm_import_module = "http")]
            extern "C" {
                #[link_name = "request-text"]
                fn wit_import(
                    _: *mut u8,
                    _: usize,
                    _: i32,
                    _: *mut u8,
                    _: usize,
                    _: *mut u8,
                    _: usize,
                    _: i32,
                    _: *mut u8,
                    _: usize,
                    _: i32,
                    _: i32,
                    _: *mut u8,
                );
            }
            #[cfg(not(target_arch = "wasm32"))]
            fn wit_import(
                _: *mut u8,
                _: usize,
                _: i32,
                _: *mut u8,
                _: usize,
                _: *mut u8,
                _: usize,
                _: i32,
                _: *mut u8,
                _: usize,
                _: i32,
                _: i32,
                _: *mut u8,
            ) {
                unreachable!()
            }
            wit_import(
                ptr0.cast_mut(),
                len0,
                result10_0,
                result10_1,
                result10_2,
                result10_3,
                result10_4,
                result10_5,
                result10_6,
                result10_7,
                result10_8,
                result10_9,
                ptr11,
            );
            let l12 = i32::from(*ptr11.add(0).cast::<u8>());
            for (ptr, layout) in cleanup_list {
                if layout.size() != 0 {
                    _rt::alloc::dealloc(ptr.cast(), layout);
                }
            }
            match l12 {
                0 => {
                    let e = {
                        let l13 = i32::from(*ptr11.add(4).cast::<u16>());
                        let l14 = *ptr11.add(8).cast::<*mut u8>();
                        let l15 = *ptr11.add(12).cast::<usize>();
                        let base22 = l14;
                        let len22 = l15;
                        let mut result22 = _rt::Vec::with_capacity(len22);
                        for i in 0..len22 {
                            let base = base22.add(i * 16);
                            let e22 = {
                                let l16 = *base.add(0).cast::<*mut u8>();
                                let l17 = *base.add(4).cast::<usize>();
                                let len18 = l17;
                                let bytes18 = _rt::Vec::from_raw_parts(
                                    l16.cast(),
                                    len18,
                                    len18,
                                );
                                let l19 = *base.add(8).cast::<*mut u8>();
                                let l20 = *base.add(12).cast::<usize>();
                                let len21 = l20;
                                let bytes21 = _rt::Vec::from_raw_parts(
                                    l19.cast(),
                                    len21,
                                    len21,
                                );
                                (_rt::string_lift(bytes18), _rt::string_lift(bytes21))
                            };
                            result22.push(e22);
                        }
                        _rt::cabi_dealloc(base22, len22 * 16, 4);
                        let l23 = *ptr11.add(16).cast::<*mut u8>();
                        let l24 = *ptr11.add(20).cast::<usize>();
                        let len25 = l24;
                        let bytes25 = _rt::Vec::from_raw_parts(l23.cast(), len25, len25);
                        TextResponse {
                            status: l13 as u16,
                            headers: result22,
                            body: _rt::string_lift(bytes25),
                        }
                    };
                    Ok(e)
                }
                1 => {
                    let e = {
                        let l26 = *ptr11.add(4).cast::<*mut u8>();
                        let l27 = *ptr11.add(8).cast::<usize>();
                        let len28 = l27;
                        let bytes28 = _rt::Vec::from_raw_parts(l26.cast(), len28, len28);
                        let l29 = i32::from(*ptr11.add(12).cast::<u8>());
                        RequestError {
                            message: _rt::string_lift(bytes28),
                            response: match l29 {
                                0 => None,
                                1 => {
                                    let e = {
                                        let l30 = i32::from(*ptr11.add(16).cast::<u16>());
                                        let l31 = *ptr11.add(20).cast::<*mut u8>();
                                        let l32 = *ptr11.add(24).cast::<usize>();
                                        let base39 = l31;
                                        let len39 = l32;
                                        let mut result39 = _rt::Vec::with_capacity(len39);
                                        for i in 0..len39 {
                                            let base = base39.add(i * 16);
                                            let e39 = {
                                                let l33 = *base.add(0).cast::<*mut u8>();
                                                let l34 = *base.add(4).cast::<usize>();
                                                let len35 = l34;
                                                let bytes35 = _rt::Vec::from_raw_parts(
                                                    l33.cast(),
                                                    len35,
                                                    len35,
                                                );
                                                let l36 = *base.add(8).cast::<*mut u8>();
                                                let l37 = *base.add(12).cast::<usize>();
                                                let len38 = l37;
                                                let bytes38 = _rt::Vec::from_raw_parts(
                                                    l36.cast(),
                                                    len38,
                                                    len38,
                                                );
                                                (_rt::string_lift(bytes35), _rt::string_lift(bytes38))
                                            };
                                            result39.push(e39);
                                        }
                                        _rt::cabi_dealloc(base39, len39 * 16, 4);
                                        let l40 = *ptr11.add(28).cast::<*mut u8>();
                                        let l41 = *ptr11.add(32).cast::<usize>();
                                        let len42 = l41;
                                        BinResponse {
                                            status: l30 as u16,
                                            headers: result39,
                                            body: _rt::Vec::from_raw_parts(l40.cast(), len42, len42),
                                        }
                                    };
                                    Some(e)
                                }
                                _ => _rt::invalid_enum_discriminant(),
                            },
                        }
                    };
                    Err(e)
                }
                _ => _rt::invalid_enum_discriminant(),
            }
        }
    }
    #[allow(unused_unsafe, clippy::all)]
    pub fn request_bin(
        uri: &str,
        opts: Option<&RequestOptions>,
    ) -> Result<BinResponse, RequestError> {
        unsafe {
            let mut cleanup_list = _rt::Vec::new();
            #[repr(align(4))]
            struct RetArea([::core::mem::MaybeUninit<u8>; 36]);
            let mut ret_area = RetArea([::core::mem::MaybeUninit::uninit(); 36]);
            let vec0 = uri;
            let ptr0 = vec0.as_ptr().cast::<u8>();
            let len0 = vec0.len();
            let (
                result10_0,
                result10_1,
                result10_2,
                result10_3,
                result10_4,
                result10_5,
                result10_6,
                result10_7,
                result10_8,
                result10_9,
            ) = match opts {
                Some(e) => {
                    let RequestOptions {
                        method: method1,
                        headers: headers1,
                        body: body1,
                        timeout_ms: timeout_ms1,
                    } = e;
                    let vec2 = method1;
                    let ptr2 = vec2.as_ptr().cast::<u8>();
                    let len2 = vec2.len();
                    let vec6 = headers1;
                    let len6 = vec6.len();
                    let layout6 = _rt::alloc::Layout::from_size_align_unchecked(
                        vec6.len() * 16,
                        4,
                    );
                    let result6 = if layout6.size() != 0 {
                        let ptr = _rt::alloc::alloc(layout6).cast::<u8>();
                        if ptr.is_null() {
                            _rt::alloc::handle_alloc_error(layout6);
                        }
                        ptr
                    } else {
                        ::core::ptr::null_mut()
                    };
                    for (i, e) in vec6.into_iter().enumerate() {
                        let base = result6.add(i * 16);
                        {
                            let (t3_0, t3_1) = e;
                            let vec4 = t3_0;
                            let ptr4 = vec4.as_ptr().cast::<u8>();
                            let len4 = vec4.len();
                            *base.add(4).cast::<usize>() = len4;
                            *base.add(0).cast::<*mut u8>() = ptr4.cast_mut();
                            let vec5 = t3_1;
                            let ptr5 = vec5.as_ptr().cast::<u8>();
                            let len5 = vec5.len();
                            *base.add(12).cast::<usize>() = len5;
                            *base.add(8).cast::<*mut u8>() = ptr5.cast_mut();
                        }
                    }
                    let (result8_0, result8_1, result8_2) = match body1 {
                        Some(e) => {
                            let vec7 = e;
                            let ptr7 = vec7.as_ptr().cast::<u8>();
                            let len7 = vec7.len();
                            (1i32, ptr7.cast_mut(), len7)
                        }
                        None => (0i32, ::core::ptr::null_mut(), 0usize),
                    };
                    let (result9_0, result9_1) = match timeout_ms1 {
                        Some(e) => (1i32, _rt::as_i32(e)),
                        None => (0i32, 0i32),
                    };
                    cleanup_list.extend_from_slice(&[(result6, layout6)]);
                    (
                        1i32,
                        ptr2.cast_mut(),
                        len2,
                        result6,
                        len6,
                        result8_0,
                        result8_1,
                        result8_2,
                        result9_0,
                        result9_1,
                    )
                }
                None => {
                    (
                        0i32,
                        ::core::ptr::null_mut(),
                        0usize,
                        ::core::ptr::null_mut(),
                        0usize,
                        0i32,
                        ::core::ptr::null_mut(),
                        0usize,
                        0i32,
                        0i32,
                    )
                }
            };
            let ptr11 = ret_area.0.as_mut_ptr().cast::<u8>();
            #[cfg(target_arch = "wasm32")]
            #[link(wasm_import_module = "http")]
            extern "C" {
                #[link_name = "request-bin"]
                fn wit_import(
                    _: *mut u8,
                    _: usize,
                    _: i32,
                    _: *mut u8,
                    _: usize,
                    _: *mut u8,
                    _: usize,
                    _: i32,
                    _: *mut u8,
                    _: usize,
                    _: i32,
                    _: i32,
                    _: *mut u8,
                );
            }
            #[cfg(not(target_arch = "wasm32"))]
            fn wit_import(
                _: *mut u8,
                _: usize,
                _: i32,
                _: *mut u8,
                _: usize,
                _: *mut u8,
                _: usize,
                _: i32,
                _: *mut u8,
                _: usize,
                _: i32,
                _: i32,
                _: *mut u8,
            ) {
                unreachable!()
            }
            wit_import(
                ptr0.cast_mut(),
                len0,
                result10_0,
                result10_1,
                result10_2,
                result10_3,
                result10_4,
                result10_5,
                result10_6,
                result10_7,
                result10_8,
                result10_9,
                ptr11,
            );
            let l12 = i32::from(*ptr11.add(0).cast::<u8>());
            for (ptr, layout) in cleanup_list {
                if layout.size() != 0 {
                    _rt::alloc::dealloc(ptr.cast(), layout);
                }
            }
            match l12 {
                0 => {
                    let e = {
                        let l13 = i32::from(*ptr11.add(4).cast::<u16>());
                        let l14 = *ptr11.add(8).cast::<*mut u8>();
                        let l15 = *ptr11.add(12).cast::<usize>();
                        let base22 = l14;
                        let len22 = l15;
                        let mut result22 = _rt::Vec::with_capacity(len22);
                        for i in 0..len22 {
                            let base = base22.add(i * 16);
                            let e22 = {
                                let l16 = *base.add(0).cast::<*mut u8>();
                                let l17 = *base.add(4).cast::<usize>();
                                let len18 = l17;
                                let bytes18 = _rt::Vec::from_raw_parts(
                                    l16.cast(),
                                    len18,
                                    len18,
                                );
                                let l19 = *base.add(8).cast::<*mut u8>();
                                let l20 = *base.add(12).cast::<usize>();
                                let len21 = l20;
                                let bytes21 = _rt::Vec::from_raw_parts(
                                    l19.cast(),
                                    len21,
                                    len21,
                                );
                                (_rt::string_lift(bytes18), _rt::string_lift(bytes21))
                            };
                            result22.push(e22);
                        }
                        _rt::cabi_dealloc(base22, len22 * 16, 4);
                        let l23 = *ptr11.add(16).cast::<*mut u8>();
                        let l24 = *ptr11.add(20).cast::<usize>();
                        let len25 = l24;
                        BinResponse {
                            status: l13 as u16,
                            headers: result22,
                            body: _rt::Vec::from_raw_parts(l23.cast(), len25, len25),
                        }
                    };
                    Ok(e)
                }
                1 => {
                    let e = {
                        let l26 = *ptr11.add(4).cast::<*mut u8>();
                        let l27 = *ptr11.add(8).cast::<usize>();
                        let len28 = l27;
                        let bytes28 = _rt::Vec::from_raw_parts(l26.cast(), len28, len28);
                        let l29 = i32::from(*ptr11.add(12).cast::<u8>());
                        RequestError {
                            message: _rt::string_lift(bytes28),
                            response: match l29 {
                                0 => None,
                                1 => {
                                    let e = {
                                        let l30 = i32::from(*ptr11.add(16).cast::<u16>());
                                        let l31 = *ptr11.add(20).cast::<*mut u8>();
                                        let l32 = *ptr11.add(24).cast::<usize>();
                                        let base39 = l31;
                                        let len39 = l32;
                                        let mut result39 = _rt::Vec::with_capacity(len39);
                                        for i in 0..len39 {
                                            let base = base39.add(i * 16);
                                            let e39 = {
                                                let l33 = *base.add(0).cast::<*mut u8>();
                                                let l34 = *base.add(4).cast::<usize>();
                                                let len35 = l34;
                                                let bytes35 = _rt::Vec::from_raw_parts(
                                                    l33.cast(),
                                                    len35,
                                                    len35,
                                                );
                                                let l36 = *base.add(8).cast::<*mut u8>();
                                                let l37 = *base.add(12).cast::<usize>();
                                                let len38 = l37;
                                                let bytes38 = _rt::Vec::from_raw_parts(
                                                    l36.cast(),
                                                    len38,
                                                    len38,
                                                );
                                                (_rt::string_lift(bytes35), _rt::string_lift(bytes38))
                                            };
                                            result39.push(e39);
                                        }
                                        _rt::cabi_dealloc(base39, len39 * 16, 4);
                                        let l40 = *ptr11.add(28).cast::<*mut u8>();
                                        let l41 = *ptr11.add(32).cast::<usize>();
                                        let len42 = l41;
                                        BinResponse {
                                            status: l30 as u16,
                                            headers: result39,
                                            body: _rt::Vec::from_raw_parts(l40.cast(), len42, len42),
                                        }
                                    };
                                    Some(e)
                                }
                                _ => _rt::invalid_enum_discriminant(),
                            },
                        }
                    };
                    Err(e)
                }
                _ => _rt::invalid_enum_discriminant(),
            }
        }
    }
}
mod _rt {
    pub use alloc_crate::string::String;
    pub use alloc_crate::vec::Vec;
    pub unsafe fn string_lift(bytes: Vec<u8>) -> String {
        if cfg!(debug_assertions) {
            String::from_utf8(bytes).unwrap()
        } else {
            String::from_utf8_unchecked(bytes)
        }
    }
    pub unsafe fn invalid_enum_discriminant<T>() -> T {
        if cfg!(debug_assertions) {
            panic!("invalid enum discriminant")
        } else {
            core::hint::unreachable_unchecked()
        }
    }
    pub use alloc_crate::alloc;
    pub fn as_i32<T: AsI32>(t: T) -> i32 {
        t.as_i32()
    }
    pub trait AsI32 {
        fn as_i32(self) -> i32;
    }
    impl<'a, T: Copy + AsI32> AsI32 for &'a T {
        fn as_i32(self) -> i32 {
            (*self).as_i32()
        }
    }
    impl AsI32 for i32 {
        #[inline]
        fn as_i32(self) -> i32 {
            self as i32
        }
    }
    impl AsI32 for u32 {
        #[inline]
        fn as_i32(self) -> i32 {
            self as i32
        }
    }
    impl AsI32 for i16 {
        #[inline]
        fn as_i32(self) -> i32 {
            self as i32
        }
    }
    impl AsI32 for u16 {
        #[inline]
        fn as_i32(self) -> i32 {
            self as i32
        }
    }
    impl AsI32 for i8 {
        #[inline]
        fn as_i32(self) -> i32 {
            self as i32
        }
    }
    impl AsI32 for u8 {
        #[inline]
        fn as_i32(self) -> i32 {
            self as i32
        }
    }
    impl AsI32 for char {
        #[inline]
        fn as_i32(self) -> i32 {
            self as i32
        }
    }
    impl AsI32 for usize {
        #[inline]
        fn as_i32(self) -> i32 {
            self as i32
        }
    }
    pub unsafe fn cabi_dealloc(ptr: *mut u8, size: usize, align: usize) {
        if size == 0 {
            return;
        }
        let layout = alloc::Layout::from_size_align_unchecked(size, align);
        alloc::dealloc(ptr, layout);
    }
    #[cfg(target_arch = "wasm32")]
    pub fn run_ctors_once() {
        wit_bindgen_rt::run_ctors_once();
    }
    extern crate alloc as alloc_crate;
}
/// Generates `#[no_mangle]` functions to export the specified type as the
/// root implementation of all generated traits.
///
/// For more information see the documentation of `wit_bindgen::generate!`.
///
/// ```rust
/// # macro_rules! export{ ($($t:tt)*) => (); }
/// # trait Guest {}
/// struct MyType;
///
/// impl Guest for MyType {
///     // ...
/// }
///
/// export!(MyType);
/// ```
#[allow(unused_macros)]
#[doc(hidden)]
macro_rules! __export_slipway_component_impl {
    ($ty:ident) => {
        self::export!($ty with_types_in self);
    };
    ($ty:ident with_types_in $($path_to_types_root:tt)*) => {
        $($path_to_types_root)*:: __export_world_slipway_component_cabi!($ty
        with_types_in $($path_to_types_root)*);
    };
}
#[doc(inline)]
pub(crate) use __export_slipway_component_impl as export;
#[cfg(target_arch = "wasm32")]
#[link_section = "component-type:wit-bindgen:0.35.0:slipway:component@0.1.0:slipway-component:encoded world"]
#[doc(hidden)]
pub static __WIT_BINDGEN_COMPONENT_TYPE: [u8; 945] = *b"\
\0asm\x0d\0\x01\0\0\x19\x16wit-component-encoding\x04\0\x07\xa9\x06\x01A\x02\x01\
A\x0f\x01B\x02\x01r\x01\x07messages\x04\0\x0fcomponent-error\x03\0\0\x03\0\x1dsl\
ipway:component/types@0.1.0\x05\0\x02\x03\0\0\x0fcomponent-error\x03\0\x0fcompon\
ent-error\x03\0\x01\x01B\x06\x01p}\x01r\x02\x06familys\x04data\0\x04\0\x0dresolv\
ed-font\x03\0\x01\x01k\x02\x01@\x01\x0afont-stacks\0\x03\x04\0\x0btry-resolve\x01\
\x04\x03\0\x04font\x05\x03\x01B\x0b\x02\x03\x02\x01\x01\x04\0\x0fcomponent-error\
\x03\0\0\x01j\x01s\x01\x01\x01@\x02\x06handles\x05inputs\0\x02\x04\0\x03run\x01\x03\
\x01@\x02\x06handles\x04paths\0\x02\x04\0\x08get-text\x01\x04\x01p}\x01j\x01\x05\
\x01\x01\x01@\x02\x06handles\x04paths\0\x06\x04\0\x07get-bin\x01\x07\x03\0\x07ca\
llout\x05\x04\x01B\x06\x01@\x01\x07messages\x01\0\x04\0\x05trace\x01\0\x04\0\x05\
debug\x01\0\x04\0\x04info\x01\0\x04\0\x04warn\x01\0\x04\0\x05error\x01\0\x03\0\x03\
log\x05\x05\x01B\x16\x01o\x02ss\x04\0\x06header\x03\0\0\x01p\x01\x01ks\x01ky\x01\
r\x04\x06methods\x07headers\x02\x04body\x03\x0atimeout-ms\x04\x04\0\x0frequest-o\
ptions\x03\0\x05\x01p}\x01r\x03\x06status{\x07headers\x02\x04body\x07\x04\0\x0cb\
in-response\x03\0\x08\x01k\x09\x01r\x02\x07messages\x08response\x0a\x04\0\x0dreq\
uest-error\x03\0\x0b\x01r\x03\x06status{\x07headers\x02\x04bodys\x04\0\x0dtext-r\
esponse\x03\0\x0d\x01k\x06\x01j\x01\x0e\x01\x0c\x01@\x02\x03uris\x04opts\x0f\0\x10\
\x04\0\x0crequest-text\x01\x11\x01j\x01\x09\x01\x0c\x01@\x02\x03uris\x04opts\x0f\
\0\x12\x04\0\x0brequest-bin\x01\x13\x03\0\x04http\x05\x06\x01j\x01s\x01\x02\x01@\
\x01\x05inputs\0\x07\x04\0\x03run\x01\x08\x04\0)slipway:component/slipway-compon\
ent@0.1.0\x04\0\x0b\x17\x01\0\x11slipway-component\x03\0\0\0G\x09producers\x01\x0c\
processed-by\x02\x0dwit-component\x070.220.0\x10wit-bindgen-rust\x060.35.0";
#[inline(never)]
#[doc(hidden)]
pub fn __link_custom_section_describing_imports() {
    wit_bindgen_rt::maybe_link_cabi_realloc();
}
