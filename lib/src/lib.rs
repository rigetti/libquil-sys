#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::{
    ffi::{CStr, CString},
    path::Path,
    sync::Once,
};

use bindings::{libquil_error, libquil_error_t, libquil_error_t_LIBQUIL_ERROR_SUCCESS};

pub mod quilc;
pub mod qvm;

#[allow(dead_code)]
pub(crate) mod bindings {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

static START: Once = Once::new();

/// Initializes libquil using it's core image. No-op after the first call.
pub(crate) fn init_libquil() {
    START.call_once(|| {
        let path = match std::env::var("LIBQUIL_CORE_PATH") {
            Ok(path) => path,
            Err(_) => "libquil.core".to_string(),
        };
        if !Path::new(&path).exists() {
            // TODO Make this an error rather than a panic
            panic!("Could not find libquil core file. Do you need to set LIBQUIL_CORE_PATH environment variable?");
        }
        let ptr = CString::new(path).unwrap().into_raw();

        unsafe {
            // The library built by maturin does link to libquil, but
            // the linker does not make the libquil symbols available
            // to the lisp image. To get around that, we load it here
            // with the `RTLD_GLOBAL` flag which makes symbols available
            // to the whole process.
            libloading::os::unix::Library::open(
                Some("libquil.so"),
                libloading::os::unix::RTLD_NOW | libloading::os::unix::RTLD_GLOBAL,
            )
            .unwrap();
            bindings::init(ptr);
        }
    })
}

pub(crate) fn handle_libquil_error(errno: libquil_error_t) -> Result<(), String> {
    if errno == libquil_error_t_LIBQUIL_ERROR_SUCCESS {
        return Ok(());
    }

    let mut error_str_ptr: *mut std::os::raw::c_char = std::ptr::null_mut();

    unsafe {
        let err = libquil_error.unwrap()(&mut error_str_ptr);
        if err != 0 {
            return Err("unknown error occurred".to_string());
        }
        let error_str = CStr::from_ptr(error_str_ptr).to_str().unwrap();
        Err(error_str.to_string())
    }
}
