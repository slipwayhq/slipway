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
            let slipway::component::types::ComponentError {
                message: message4,
                inner: inner4,
            } = e;
            let vec5 = (message4.into_bytes()).into_boxed_slice();
            let ptr5 = vec5.as_ptr().cast::<u8>();
            let len5 = vec5.len();
            ::core::mem::forget(vec5);
            *ptr2.add(8).cast::<usize>() = len5;
            *ptr2.add(4).cast::<*mut u8>() = ptr5.cast_mut();
            let vec7 = inner4;
            let len7 = vec7.len();
            let layout7 = _rt::alloc::Layout::from_size_align_unchecked(
                vec7.len() * 8,
                4,
            );
            let result7 = if layout7.size() != 0 {
                let ptr = _rt::alloc::alloc(layout7).cast::<u8>();
                if ptr.is_null() {
                    _rt::alloc::handle_alloc_error(layout7);
                }
                ptr
            } else {
                ::core::ptr::null_mut()
            };
            for (i, e) in vec7.into_iter().enumerate() {
                let base = result7.add(i * 8);
                {
                    let vec6 = (e.into_bytes()).into_boxed_slice();
                    let ptr6 = vec6.as_ptr().cast::<u8>();
                    let len6 = vec6.len();
                    ::core::mem::forget(vec6);
                    *base.add(4).cast::<usize>() = len6;
                    *base.add(0).cast::<*mut u8>() = ptr6.cast_mut();
                }
            }
            *ptr2.add(16).cast::<usize>() = len7;
            *ptr2.add(12).cast::<*mut u8>() = result7;
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
            let l5 = *arg0.add(12).cast::<*mut u8>();
            let l6 = *arg0.add(16).cast::<usize>();
            let base9 = l5;
            let len9 = l6;
            for i in 0..len9 {
                let base = base9.add(i * 8);
                {
                    let l7 = *base.add(0).cast::<*mut u8>();
                    let l8 = *base.add(4).cast::<usize>();
                    _rt::cabi_dealloc(l7, l8, 1);
                }
            }
            _rt::cabi_dealloc(base9, len9 * 8, 4);
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
struct _RetArea([::core::mem::MaybeUninit<u8>; 20]);
static mut _RET_AREA: _RetArea = _RetArea([::core::mem::MaybeUninit::uninit(); 20]);
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
                pub inner: _rt::Vec<_rt::String>,
            }
            impl ::core::fmt::Debug for ComponentError {
                fn fmt(
                    &self,
                    f: &mut ::core::fmt::Formatter<'_>,
                ) -> ::core::fmt::Result {
                    f.debug_struct("ComponentError")
                        .field("message", &self.message)
                        .field("inner", &self.inner)
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
pub mod slipway_host {
    #[used]
    #[doc(hidden)]
    static __FORCE_SECTION_REF: fn() = super::__link_custom_section_describing_imports;
    use super::_rt;
    pub type ComponentError = super::slipway::component::types::ComponentError;
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
    pub type Header = (_rt::String, _rt::String);
    #[derive(Clone)]
    pub struct RequestOptions {
        pub method: Option<_rt::String>,
        pub body: Option<_rt::Vec<u8>>,
        pub headers: Option<_rt::Vec<Header>>,
        pub timeout_ms: Option<u32>,
    }
    impl ::core::fmt::Debug for RequestOptions {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            f.debug_struct("RequestOptions")
                .field("method", &self.method)
                .field("body", &self.body)
                .field("headers", &self.headers)
                .field("timeout-ms", &self.timeout_ms)
                .finish()
        }
    }
    #[derive(Clone)]
    pub struct BinResponse {
        pub status: u16,
        pub headers: _rt::Vec<Header>,
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
    pub struct TextResponse {
        pub status: u16,
        pub headers: _rt::Vec<Header>,
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
    #[derive(Clone)]
    pub struct RequestError {
        pub message: _rt::String,
        pub inner: _rt::Vec<_rt::String>,
        pub response: Option<BinResponse>,
    }
    impl ::core::fmt::Debug for RequestError {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            f.debug_struct("RequestError")
                .field("message", &self.message)
                .field("inner", &self.inner)
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
    #[allow(unused_unsafe, clippy::all)]
    pub fn try_resolve_font(font_stack: &str) -> Option<ResolvedFont> {
        unsafe {
            #[repr(align(4))]
            struct RetArea([::core::mem::MaybeUninit<u8>; 20]);
            let mut ret_area = RetArea([::core::mem::MaybeUninit::uninit(); 20]);
            let vec0 = font_stack;
            let ptr0 = vec0.as_ptr().cast::<u8>();
            let len0 = vec0.len();
            let ptr1 = ret_area.0.as_mut_ptr().cast::<u8>();
            #[cfg(target_arch = "wasm32")]
            #[link(wasm_import_module = "slipway-host")]
            extern "C" {
                #[link_name = "try-resolve-font"]
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
    #[allow(unused_unsafe, clippy::all)]
    pub fn log_trace(message: &str) {
        unsafe {
            let vec0 = message;
            let ptr0 = vec0.as_ptr().cast::<u8>();
            let len0 = vec0.len();
            #[cfg(target_arch = "wasm32")]
            #[link(wasm_import_module = "slipway-host")]
            extern "C" {
                #[link_name = "log-trace"]
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
    pub fn log_debug(message: &str) {
        unsafe {
            let vec0 = message;
            let ptr0 = vec0.as_ptr().cast::<u8>();
            let len0 = vec0.len();
            #[cfg(target_arch = "wasm32")]
            #[link(wasm_import_module = "slipway-host")]
            extern "C" {
                #[link_name = "log-debug"]
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
    pub fn log_info(message: &str) {
        unsafe {
            let vec0 = message;
            let ptr0 = vec0.as_ptr().cast::<u8>();
            let len0 = vec0.len();
            #[cfg(target_arch = "wasm32")]
            #[link(wasm_import_module = "slipway-host")]
            extern "C" {
                #[link_name = "log-info"]
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
    pub fn log_warn(message: &str) {
        unsafe {
            let vec0 = message;
            let ptr0 = vec0.as_ptr().cast::<u8>();
            let len0 = vec0.len();
            #[cfg(target_arch = "wasm32")]
            #[link(wasm_import_module = "slipway-host")]
            extern "C" {
                #[link_name = "log-warn"]
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
    pub fn log_error(message: &str) {
        unsafe {
            let vec0 = message;
            let ptr0 = vec0.as_ptr().cast::<u8>();
            let len0 = vec0.len();
            #[cfg(target_arch = "wasm32")]
            #[link(wasm_import_module = "slipway-host")]
            extern "C" {
                #[link_name = "log-error"]
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
    pub fn fetch_bin(
        url: &str,
        options: Option<&RequestOptions>,
    ) -> Result<BinResponse, RequestError> {
        unsafe {
            let mut cleanup_list = _rt::Vec::new();
            #[repr(align(4))]
            struct RetArea([::core::mem::MaybeUninit<u8>; 44]);
            let mut ret_area = RetArea([::core::mem::MaybeUninit::uninit(); 44]);
            let vec0 = url;
            let ptr0 = vec0.as_ptr().cast::<u8>();
            let len0 = vec0.len();
            let (
                result12_0,
                result12_1,
                result12_2,
                result12_3,
                result12_4,
                result12_5,
                result12_6,
                result12_7,
                result12_8,
                result12_9,
                result12_10,
                result12_11,
            ) = match options {
                Some(e) => {
                    let RequestOptions {
                        method: method1,
                        body: body1,
                        headers: headers1,
                        timeout_ms: timeout_ms1,
                    } = e;
                    let (result3_0, result3_1, result3_2) = match method1 {
                        Some(e) => {
                            let vec2 = e;
                            let ptr2 = vec2.as_ptr().cast::<u8>();
                            let len2 = vec2.len();
                            (1i32, ptr2.cast_mut(), len2)
                        }
                        None => (0i32, ::core::ptr::null_mut(), 0usize),
                    };
                    let (result5_0, result5_1, result5_2) = match body1 {
                        Some(e) => {
                            let vec4 = e;
                            let ptr4 = vec4.as_ptr().cast::<u8>();
                            let len4 = vec4.len();
                            (1i32, ptr4.cast_mut(), len4)
                        }
                        None => (0i32, ::core::ptr::null_mut(), 0usize),
                    };
                    let (result10_0, result10_1, result10_2) = match headers1 {
                        Some(e) => {
                            let vec9 = e;
                            let len9 = vec9.len();
                            let layout9 = _rt::alloc::Layout::from_size_align_unchecked(
                                vec9.len() * 16,
                                4,
                            );
                            let result9 = if layout9.size() != 0 {
                                let ptr = _rt::alloc::alloc(layout9).cast::<u8>();
                                if ptr.is_null() {
                                    _rt::alloc::handle_alloc_error(layout9);
                                }
                                ptr
                            } else {
                                ::core::ptr::null_mut()
                            };
                            for (i, e) in vec9.into_iter().enumerate() {
                                let base = result9.add(i * 16);
                                {
                                    let (t6_0, t6_1) = e;
                                    let vec7 = t6_0;
                                    let ptr7 = vec7.as_ptr().cast::<u8>();
                                    let len7 = vec7.len();
                                    *base.add(4).cast::<usize>() = len7;
                                    *base.add(0).cast::<*mut u8>() = ptr7.cast_mut();
                                    let vec8 = t6_1;
                                    let ptr8 = vec8.as_ptr().cast::<u8>();
                                    let len8 = vec8.len();
                                    *base.add(12).cast::<usize>() = len8;
                                    *base.add(8).cast::<*mut u8>() = ptr8.cast_mut();
                                }
                            }
                            cleanup_list.extend_from_slice(&[(result9, layout9)]);
                            (1i32, result9, len9)
                        }
                        None => (0i32, ::core::ptr::null_mut(), 0usize),
                    };
                    let (result11_0, result11_1) = match timeout_ms1 {
                        Some(e) => (1i32, _rt::as_i32(e)),
                        None => (0i32, 0i32),
                    };
                    (
                        1i32,
                        result3_0,
                        result3_1,
                        result3_2,
                        result5_0,
                        result5_1,
                        result5_2,
                        result10_0,
                        result10_1,
                        result10_2,
                        result11_0,
                        result11_1,
                    )
                }
                None => {
                    (
                        0i32,
                        0i32,
                        ::core::ptr::null_mut(),
                        0usize,
                        0i32,
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
            let ptr13 = ret_area.0.as_mut_ptr().cast::<u8>();
            #[cfg(target_arch = "wasm32")]
            #[link(wasm_import_module = "slipway-host")]
            extern "C" {
                #[link_name = "fetch-bin"]
                fn wit_import(
                    _: *mut u8,
                    _: usize,
                    _: i32,
                    _: i32,
                    _: *mut u8,
                    _: usize,
                    _: i32,
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
                _: i32,
                _: *mut u8,
                _: usize,
                _: i32,
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
                result12_0,
                result12_1,
                result12_2,
                result12_3,
                result12_4,
                result12_5,
                result12_6,
                result12_7,
                result12_8,
                result12_9,
                result12_10,
                result12_11,
                ptr13,
            );
            let l14 = i32::from(*ptr13.add(0).cast::<u8>());
            for (ptr, layout) in cleanup_list {
                if layout.size() != 0 {
                    _rt::alloc::dealloc(ptr.cast(), layout);
                }
            }
            match l14 {
                0 => {
                    let e = {
                        let l15 = i32::from(*ptr13.add(4).cast::<u16>());
                        let l16 = *ptr13.add(8).cast::<*mut u8>();
                        let l17 = *ptr13.add(12).cast::<usize>();
                        let base24 = l16;
                        let len24 = l17;
                        let mut result24 = _rt::Vec::with_capacity(len24);
                        for i in 0..len24 {
                            let base = base24.add(i * 16);
                            let e24 = {
                                let l18 = *base.add(0).cast::<*mut u8>();
                                let l19 = *base.add(4).cast::<usize>();
                                let len20 = l19;
                                let bytes20 = _rt::Vec::from_raw_parts(
                                    l18.cast(),
                                    len20,
                                    len20,
                                );
                                let l21 = *base.add(8).cast::<*mut u8>();
                                let l22 = *base.add(12).cast::<usize>();
                                let len23 = l22;
                                let bytes23 = _rt::Vec::from_raw_parts(
                                    l21.cast(),
                                    len23,
                                    len23,
                                );
                                (_rt::string_lift(bytes20), _rt::string_lift(bytes23))
                            };
                            result24.push(e24);
                        }
                        _rt::cabi_dealloc(base24, len24 * 16, 4);
                        let l25 = *ptr13.add(16).cast::<*mut u8>();
                        let l26 = *ptr13.add(20).cast::<usize>();
                        let len27 = l26;
                        BinResponse {
                            status: l15 as u16,
                            headers: result24,
                            body: _rt::Vec::from_raw_parts(l25.cast(), len27, len27),
                        }
                    };
                    Ok(e)
                }
                1 => {
                    let e = {
                        let l28 = *ptr13.add(4).cast::<*mut u8>();
                        let l29 = *ptr13.add(8).cast::<usize>();
                        let len30 = l29;
                        let bytes30 = _rt::Vec::from_raw_parts(l28.cast(), len30, len30);
                        let l31 = *ptr13.add(12).cast::<*mut u8>();
                        let l32 = *ptr13.add(16).cast::<usize>();
                        let base36 = l31;
                        let len36 = l32;
                        let mut result36 = _rt::Vec::with_capacity(len36);
                        for i in 0..len36 {
                            let base = base36.add(i * 8);
                            let e36 = {
                                let l33 = *base.add(0).cast::<*mut u8>();
                                let l34 = *base.add(4).cast::<usize>();
                                let len35 = l34;
                                let bytes35 = _rt::Vec::from_raw_parts(
                                    l33.cast(),
                                    len35,
                                    len35,
                                );
                                _rt::string_lift(bytes35)
                            };
                            result36.push(e36);
                        }
                        _rt::cabi_dealloc(base36, len36 * 8, 4);
                        let l37 = i32::from(*ptr13.add(20).cast::<u8>());
                        RequestError {
                            message: _rt::string_lift(bytes30),
                            inner: result36,
                            response: match l37 {
                                0 => None,
                                1 => {
                                    let e = {
                                        let l38 = i32::from(*ptr13.add(24).cast::<u16>());
                                        let l39 = *ptr13.add(28).cast::<*mut u8>();
                                        let l40 = *ptr13.add(32).cast::<usize>();
                                        let base47 = l39;
                                        let len47 = l40;
                                        let mut result47 = _rt::Vec::with_capacity(len47);
                                        for i in 0..len47 {
                                            let base = base47.add(i * 16);
                                            let e47 = {
                                                let l41 = *base.add(0).cast::<*mut u8>();
                                                let l42 = *base.add(4).cast::<usize>();
                                                let len43 = l42;
                                                let bytes43 = _rt::Vec::from_raw_parts(
                                                    l41.cast(),
                                                    len43,
                                                    len43,
                                                );
                                                let l44 = *base.add(8).cast::<*mut u8>();
                                                let l45 = *base.add(12).cast::<usize>();
                                                let len46 = l45;
                                                let bytes46 = _rt::Vec::from_raw_parts(
                                                    l44.cast(),
                                                    len46,
                                                    len46,
                                                );
                                                (_rt::string_lift(bytes43), _rt::string_lift(bytes46))
                                            };
                                            result47.push(e47);
                                        }
                                        _rt::cabi_dealloc(base47, len47 * 16, 4);
                                        let l48 = *ptr13.add(36).cast::<*mut u8>();
                                        let l49 = *ptr13.add(40).cast::<usize>();
                                        let len50 = l49;
                                        BinResponse {
                                            status: l38 as u16,
                                            headers: result47,
                                            body: _rt::Vec::from_raw_parts(l48.cast(), len50, len50),
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
    pub fn fetch_text(
        url: &str,
        options: Option<&RequestOptions>,
    ) -> Result<TextResponse, RequestError> {
        unsafe {
            let mut cleanup_list = _rt::Vec::new();
            #[repr(align(4))]
            struct RetArea([::core::mem::MaybeUninit<u8>; 44]);
            let mut ret_area = RetArea([::core::mem::MaybeUninit::uninit(); 44]);
            let vec0 = url;
            let ptr0 = vec0.as_ptr().cast::<u8>();
            let len0 = vec0.len();
            let (
                result12_0,
                result12_1,
                result12_2,
                result12_3,
                result12_4,
                result12_5,
                result12_6,
                result12_7,
                result12_8,
                result12_9,
                result12_10,
                result12_11,
            ) = match options {
                Some(e) => {
                    let RequestOptions {
                        method: method1,
                        body: body1,
                        headers: headers1,
                        timeout_ms: timeout_ms1,
                    } = e;
                    let (result3_0, result3_1, result3_2) = match method1 {
                        Some(e) => {
                            let vec2 = e;
                            let ptr2 = vec2.as_ptr().cast::<u8>();
                            let len2 = vec2.len();
                            (1i32, ptr2.cast_mut(), len2)
                        }
                        None => (0i32, ::core::ptr::null_mut(), 0usize),
                    };
                    let (result5_0, result5_1, result5_2) = match body1 {
                        Some(e) => {
                            let vec4 = e;
                            let ptr4 = vec4.as_ptr().cast::<u8>();
                            let len4 = vec4.len();
                            (1i32, ptr4.cast_mut(), len4)
                        }
                        None => (0i32, ::core::ptr::null_mut(), 0usize),
                    };
                    let (result10_0, result10_1, result10_2) = match headers1 {
                        Some(e) => {
                            let vec9 = e;
                            let len9 = vec9.len();
                            let layout9 = _rt::alloc::Layout::from_size_align_unchecked(
                                vec9.len() * 16,
                                4,
                            );
                            let result9 = if layout9.size() != 0 {
                                let ptr = _rt::alloc::alloc(layout9).cast::<u8>();
                                if ptr.is_null() {
                                    _rt::alloc::handle_alloc_error(layout9);
                                }
                                ptr
                            } else {
                                ::core::ptr::null_mut()
                            };
                            for (i, e) in vec9.into_iter().enumerate() {
                                let base = result9.add(i * 16);
                                {
                                    let (t6_0, t6_1) = e;
                                    let vec7 = t6_0;
                                    let ptr7 = vec7.as_ptr().cast::<u8>();
                                    let len7 = vec7.len();
                                    *base.add(4).cast::<usize>() = len7;
                                    *base.add(0).cast::<*mut u8>() = ptr7.cast_mut();
                                    let vec8 = t6_1;
                                    let ptr8 = vec8.as_ptr().cast::<u8>();
                                    let len8 = vec8.len();
                                    *base.add(12).cast::<usize>() = len8;
                                    *base.add(8).cast::<*mut u8>() = ptr8.cast_mut();
                                }
                            }
                            cleanup_list.extend_from_slice(&[(result9, layout9)]);
                            (1i32, result9, len9)
                        }
                        None => (0i32, ::core::ptr::null_mut(), 0usize),
                    };
                    let (result11_0, result11_1) = match timeout_ms1 {
                        Some(e) => (1i32, _rt::as_i32(e)),
                        None => (0i32, 0i32),
                    };
                    (
                        1i32,
                        result3_0,
                        result3_1,
                        result3_2,
                        result5_0,
                        result5_1,
                        result5_2,
                        result10_0,
                        result10_1,
                        result10_2,
                        result11_0,
                        result11_1,
                    )
                }
                None => {
                    (
                        0i32,
                        0i32,
                        ::core::ptr::null_mut(),
                        0usize,
                        0i32,
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
            let ptr13 = ret_area.0.as_mut_ptr().cast::<u8>();
            #[cfg(target_arch = "wasm32")]
            #[link(wasm_import_module = "slipway-host")]
            extern "C" {
                #[link_name = "fetch-text"]
                fn wit_import(
                    _: *mut u8,
                    _: usize,
                    _: i32,
                    _: i32,
                    _: *mut u8,
                    _: usize,
                    _: i32,
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
                _: i32,
                _: *mut u8,
                _: usize,
                _: i32,
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
                result12_0,
                result12_1,
                result12_2,
                result12_3,
                result12_4,
                result12_5,
                result12_6,
                result12_7,
                result12_8,
                result12_9,
                result12_10,
                result12_11,
                ptr13,
            );
            let l14 = i32::from(*ptr13.add(0).cast::<u8>());
            for (ptr, layout) in cleanup_list {
                if layout.size() != 0 {
                    _rt::alloc::dealloc(ptr.cast(), layout);
                }
            }
            match l14 {
                0 => {
                    let e = {
                        let l15 = i32::from(*ptr13.add(4).cast::<u16>());
                        let l16 = *ptr13.add(8).cast::<*mut u8>();
                        let l17 = *ptr13.add(12).cast::<usize>();
                        let base24 = l16;
                        let len24 = l17;
                        let mut result24 = _rt::Vec::with_capacity(len24);
                        for i in 0..len24 {
                            let base = base24.add(i * 16);
                            let e24 = {
                                let l18 = *base.add(0).cast::<*mut u8>();
                                let l19 = *base.add(4).cast::<usize>();
                                let len20 = l19;
                                let bytes20 = _rt::Vec::from_raw_parts(
                                    l18.cast(),
                                    len20,
                                    len20,
                                );
                                let l21 = *base.add(8).cast::<*mut u8>();
                                let l22 = *base.add(12).cast::<usize>();
                                let len23 = l22;
                                let bytes23 = _rt::Vec::from_raw_parts(
                                    l21.cast(),
                                    len23,
                                    len23,
                                );
                                (_rt::string_lift(bytes20), _rt::string_lift(bytes23))
                            };
                            result24.push(e24);
                        }
                        _rt::cabi_dealloc(base24, len24 * 16, 4);
                        let l25 = *ptr13.add(16).cast::<*mut u8>();
                        let l26 = *ptr13.add(20).cast::<usize>();
                        let len27 = l26;
                        let bytes27 = _rt::Vec::from_raw_parts(l25.cast(), len27, len27);
                        TextResponse {
                            status: l15 as u16,
                            headers: result24,
                            body: _rt::string_lift(bytes27),
                        }
                    };
                    Ok(e)
                }
                1 => {
                    let e = {
                        let l28 = *ptr13.add(4).cast::<*mut u8>();
                        let l29 = *ptr13.add(8).cast::<usize>();
                        let len30 = l29;
                        let bytes30 = _rt::Vec::from_raw_parts(l28.cast(), len30, len30);
                        let l31 = *ptr13.add(12).cast::<*mut u8>();
                        let l32 = *ptr13.add(16).cast::<usize>();
                        let base36 = l31;
                        let len36 = l32;
                        let mut result36 = _rt::Vec::with_capacity(len36);
                        for i in 0..len36 {
                            let base = base36.add(i * 8);
                            let e36 = {
                                let l33 = *base.add(0).cast::<*mut u8>();
                                let l34 = *base.add(4).cast::<usize>();
                                let len35 = l34;
                                let bytes35 = _rt::Vec::from_raw_parts(
                                    l33.cast(),
                                    len35,
                                    len35,
                                );
                                _rt::string_lift(bytes35)
                            };
                            result36.push(e36);
                        }
                        _rt::cabi_dealloc(base36, len36 * 8, 4);
                        let l37 = i32::from(*ptr13.add(20).cast::<u8>());
                        RequestError {
                            message: _rt::string_lift(bytes30),
                            inner: result36,
                            response: match l37 {
                                0 => None,
                                1 => {
                                    let e = {
                                        let l38 = i32::from(*ptr13.add(24).cast::<u16>());
                                        let l39 = *ptr13.add(28).cast::<*mut u8>();
                                        let l40 = *ptr13.add(32).cast::<usize>();
                                        let base47 = l39;
                                        let len47 = l40;
                                        let mut result47 = _rt::Vec::with_capacity(len47);
                                        for i in 0..len47 {
                                            let base = base47.add(i * 16);
                                            let e47 = {
                                                let l41 = *base.add(0).cast::<*mut u8>();
                                                let l42 = *base.add(4).cast::<usize>();
                                                let len43 = l42;
                                                let bytes43 = _rt::Vec::from_raw_parts(
                                                    l41.cast(),
                                                    len43,
                                                    len43,
                                                );
                                                let l44 = *base.add(8).cast::<*mut u8>();
                                                let l45 = *base.add(12).cast::<usize>();
                                                let len46 = l45;
                                                let bytes46 = _rt::Vec::from_raw_parts(
                                                    l44.cast(),
                                                    len46,
                                                    len46,
                                                );
                                                (_rt::string_lift(bytes43), _rt::string_lift(bytes46))
                                            };
                                            result47.push(e47);
                                        }
                                        _rt::cabi_dealloc(base47, len47 * 16, 4);
                                        let l48 = *ptr13.add(36).cast::<*mut u8>();
                                        let l49 = *ptr13.add(40).cast::<usize>();
                                        let len50 = l49;
                                        BinResponse {
                                            status: l38 as u16,
                                            headers: result47,
                                            body: _rt::Vec::from_raw_parts(l48.cast(), len50, len50),
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
    pub fn run(handle: &str, input: &str) -> Result<_rt::String, ComponentError> {
        unsafe {
            #[repr(align(4))]
            struct RetArea([::core::mem::MaybeUninit<u8>; 20]);
            let mut ret_area = RetArea([::core::mem::MaybeUninit::uninit(); 20]);
            let vec0 = handle;
            let ptr0 = vec0.as_ptr().cast::<u8>();
            let len0 = vec0.len();
            let vec1 = input;
            let ptr1 = vec1.as_ptr().cast::<u8>();
            let len1 = vec1.len();
            let ptr2 = ret_area.0.as_mut_ptr().cast::<u8>();
            #[cfg(target_arch = "wasm32")]
            #[link(wasm_import_module = "slipway-host")]
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
                        let l10 = *ptr2.add(12).cast::<*mut u8>();
                        let l11 = *ptr2.add(16).cast::<usize>();
                        let base15 = l10;
                        let len15 = l11;
                        let mut result15 = _rt::Vec::with_capacity(len15);
                        for i in 0..len15 {
                            let base = base15.add(i * 8);
                            let e15 = {
                                let l12 = *base.add(0).cast::<*mut u8>();
                                let l13 = *base.add(4).cast::<usize>();
                                let len14 = l13;
                                let bytes14 = _rt::Vec::from_raw_parts(
                                    l12.cast(),
                                    len14,
                                    len14,
                                );
                                _rt::string_lift(bytes14)
                            };
                            result15.push(e15);
                        }
                        _rt::cabi_dealloc(base15, len15 * 8, 4);
                        super::slipway::component::types::ComponentError {
                            message: _rt::string_lift(bytes9),
                            inner: result15,
                        }
                    };
                    Err(e)
                }
                _ => _rt::invalid_enum_discriminant(),
            }
        }
    }
    #[allow(unused_unsafe, clippy::all)]
    pub fn load_bin(handle: &str, path: &str) -> Result<_rt::Vec<u8>, ComponentError> {
        unsafe {
            #[repr(align(4))]
            struct RetArea([::core::mem::MaybeUninit<u8>; 20]);
            let mut ret_area = RetArea([::core::mem::MaybeUninit::uninit(); 20]);
            let vec0 = handle;
            let ptr0 = vec0.as_ptr().cast::<u8>();
            let len0 = vec0.len();
            let vec1 = path;
            let ptr1 = vec1.as_ptr().cast::<u8>();
            let len1 = vec1.len();
            let ptr2 = ret_area.0.as_mut_ptr().cast::<u8>();
            #[cfg(target_arch = "wasm32")]
            #[link(wasm_import_module = "slipway-host")]
            extern "C" {
                #[link_name = "load-bin"]
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
                        let l10 = *ptr2.add(12).cast::<*mut u8>();
                        let l11 = *ptr2.add(16).cast::<usize>();
                        let base15 = l10;
                        let len15 = l11;
                        let mut result15 = _rt::Vec::with_capacity(len15);
                        for i in 0..len15 {
                            let base = base15.add(i * 8);
                            let e15 = {
                                let l12 = *base.add(0).cast::<*mut u8>();
                                let l13 = *base.add(4).cast::<usize>();
                                let len14 = l13;
                                let bytes14 = _rt::Vec::from_raw_parts(
                                    l12.cast(),
                                    len14,
                                    len14,
                                );
                                _rt::string_lift(bytes14)
                            };
                            result15.push(e15);
                        }
                        _rt::cabi_dealloc(base15, len15 * 8, 4);
                        super::slipway::component::types::ComponentError {
                            message: _rt::string_lift(bytes9),
                            inner: result15,
                        }
                    };
                    Err(e)
                }
                _ => _rt::invalid_enum_discriminant(),
            }
        }
    }
    #[allow(unused_unsafe, clippy::all)]
    pub fn load_text(handle: &str, path: &str) -> Result<_rt::String, ComponentError> {
        unsafe {
            #[repr(align(4))]
            struct RetArea([::core::mem::MaybeUninit<u8>; 20]);
            let mut ret_area = RetArea([::core::mem::MaybeUninit::uninit(); 20]);
            let vec0 = handle;
            let ptr0 = vec0.as_ptr().cast::<u8>();
            let len0 = vec0.len();
            let vec1 = path;
            let ptr1 = vec1.as_ptr().cast::<u8>();
            let len1 = vec1.len();
            let ptr2 = ret_area.0.as_mut_ptr().cast::<u8>();
            #[cfg(target_arch = "wasm32")]
            #[link(wasm_import_module = "slipway-host")]
            extern "C" {
                #[link_name = "load-text"]
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
                        let l10 = *ptr2.add(12).cast::<*mut u8>();
                        let l11 = *ptr2.add(16).cast::<usize>();
                        let base15 = l10;
                        let len15 = l11;
                        let mut result15 = _rt::Vec::with_capacity(len15);
                        for i in 0..len15 {
                            let base = base15.add(i * 8);
                            let e15 = {
                                let l12 = *base.add(0).cast::<*mut u8>();
                                let l13 = *base.add(4).cast::<usize>();
                                let len14 = l13;
                                let bytes14 = _rt::Vec::from_raw_parts(
                                    l12.cast(),
                                    len14,
                                    len14,
                                );
                                _rt::string_lift(bytes14)
                            };
                            result15.push(e15);
                        }
                        _rt::cabi_dealloc(base15, len15 * 8, 4);
                        super::slipway::component::types::ComponentError {
                            message: _rt::string_lift(bytes9),
                            inner: result15,
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
pub static __WIT_BINDGEN_COMPONENT_TYPE: [u8; 964] = *b"\
\0asm\x0d\0\x01\0\0\x19\x16wit-component-encoding\x04\0\x07\xbc\x06\x01A\x02\x01\
A\x09\x01B\x03\x01ps\x01r\x02\x07messages\x05inner\0\x04\0\x0fcomponent-error\x03\
\0\x01\x03\0\x1dslipway:component/types@0.1.0\x05\0\x02\x03\0\0\x0fcomponent-err\
or\x03\0\x0fcomponent-error\x03\0\x01\x01B.\x02\x03\x02\x01\x01\x04\0\x0fcompone\
nt-error\x03\0\0\x01p}\x01r\x02\x06familys\x04data\x02\x04\0\x0dresolved-font\x03\
\0\x03\x01o\x02ss\x04\0\x06header\x03\0\x05\x01ks\x01k\x02\x01p\x06\x01k\x09\x01\
ky\x01r\x04\x06method\x07\x04body\x08\x07headers\x0a\x0atimeout-ms\x0b\x04\0\x0f\
request-options\x03\0\x0c\x01r\x03\x06status{\x07headers\x09\x04body\x02\x04\0\x0c\
bin-response\x03\0\x0e\x01r\x03\x06status{\x07headers\x09\x04bodys\x04\0\x0dtext\
-response\x03\0\x10\x01ps\x01k\x0f\x01r\x03\x07messages\x05inner\x12\x08response\
\x13\x04\0\x0drequest-error\x03\0\x14\x01k\x04\x01@\x01\x0afont-stacks\0\x16\x04\
\0\x10try-resolve-font\x01\x17\x01@\x01\x07messages\x01\0\x04\0\x09log-trace\x01\
\x18\x04\0\x09log-debug\x01\x18\x04\0\x08log-info\x01\x18\x04\0\x08log-warn\x01\x18\
\x04\0\x09log-error\x01\x18\x01k\x0d\x01j\x01\x0f\x01\x15\x01@\x02\x03urls\x07op\
tions\x19\0\x1a\x04\0\x09fetch-bin\x01\x1b\x01j\x01\x11\x01\x15\x01@\x02\x03urls\
\x07options\x19\0\x1c\x04\0\x0afetch-text\x01\x1d\x01j\x01s\x01\x01\x01@\x02\x06\
handles\x05inputs\0\x1e\x04\0\x03run\x01\x1f\x01j\x01\x02\x01\x01\x01@\x02\x06ha\
ndles\x04paths\0\x20\x04\0\x08load-bin\x01!\x01@\x02\x06handles\x04paths\0\x1e\x04\
\0\x09load-text\x01\"\x03\0\x0cslipway-host\x05\x03\x01j\x01s\x01\x02\x01@\x01\x05\
inputs\0\x04\x04\0\x03run\x01\x05\x04\0)slipway:component/slipway-component@0.1.\
0\x04\0\x0b\x17\x01\0\x11slipway-component\x03\0\0\0G\x09producers\x01\x0cproces\
sed-by\x02\x0dwit-component\x070.220.0\x10wit-bindgen-rust\x060.35.0";
#[inline(never)]
#[doc(hidden)]
pub fn __link_custom_section_describing_imports() {
    wit_bindgen_rt::maybe_link_cabi_realloc();
}
