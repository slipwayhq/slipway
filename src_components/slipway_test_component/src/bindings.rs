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
            let vec4 = (e.into_bytes()).into_boxed_slice();
            let ptr4 = vec4.as_ptr().cast::<u8>();
            let len4 = vec4.len();
            ::core::mem::forget(vec4);
            *ptr2.add(8).cast::<usize>() = len4;
            *ptr2.add(4).cast::<*mut u8>() = ptr4.cast_mut();
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
    fn run(input: _rt::String) -> Result<_rt::String, _rt::String>;
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
    #[allow(unused_unsafe, clippy::all)]
    pub fn run(handle: &str, input: &str) -> _rt::String {
        unsafe {
            #[repr(align(4))]
            struct RetArea([::core::mem::MaybeUninit<u8>; 8]);
            let mut ret_area = RetArea([::core::mem::MaybeUninit::uninit(); 8]);
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
            let l3 = *ptr2.add(0).cast::<*mut u8>();
            let l4 = *ptr2.add(4).cast::<usize>();
            let len5 = l4;
            let bytes5 = _rt::Vec::from_raw_parts(l3.cast(), len5, len5);
            _rt::string_lift(bytes5)
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
    #[cfg(target_arch = "wasm32")]
    pub fn run_ctors_once() {
        wit_bindgen_rt::run_ctors_once();
    }
    pub unsafe fn cabi_dealloc(ptr: *mut u8, size: usize, align: usize) {
        if size == 0 {
            return;
        }
        let layout = alloc::Layout::from_size_align_unchecked(size, align);
        alloc::dealloc(ptr, layout);
    }
    extern crate alloc as alloc_crate;
    pub use alloc_crate::alloc;
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
pub static __WIT_BINDGEN_COMPONENT_TYPE: [u8; 417] = *b"\
\0asm\x0d\0\x01\0\0\x19\x16wit-component-encoding\x04\0\x07\x99\x02\x01A\x02\x01\
A\x09\x01B\x06\x01p}\x01r\x02\x06familys\x04data\0\x04\0\x0dresolved-font\x03\0\x01\
\x01k\x02\x01@\x01\x0afont-stacks\0\x03\x04\0\x0btry-resolve\x01\x04\x03\0\x04fo\
nt\x05\0\x01B\x02\x01@\x02\x06handles\x05inputs\0s\x04\0\x03run\x01\0\x03\0\x07c\
allout\x05\x01\x01B\x06\x01@\x01\x07messages\x01\0\x04\0\x05trace\x01\0\x04\0\x05\
debug\x01\0\x04\0\x04info\x01\0\x04\0\x04warn\x01\0\x04\0\x05error\x01\0\x03\0\x03\
log\x05\x02\x01j\x01s\x01s\x01@\x01\x05inputs\0\x03\x04\0\x03run\x01\x04\x04\0)s\
lipway:component/slipway-component@0.1.0\x04\0\x0b\x17\x01\0\x11slipway-componen\
t\x03\0\0\0G\x09producers\x01\x0cprocessed-by\x02\x0dwit-component\x070.220.0\x10\
wit-bindgen-rust\x060.35.0";
#[inline(never)]
#[doc(hidden)]
pub fn __link_custom_section_describing_imports() {
    wit_bindgen_rt::maybe_link_cabi_realloc();
}
