#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::{ffi::CStr, str::Utf8Error, sync::Once};

use bindings::{get_error_message, lisp_err_t, lisp_err_t_LISP_ERR_SUCCESS};

pub mod quilc;
pub mod qvm;

#[allow(dead_code)]
pub(crate) mod bindings {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

static START: Once = Once::new();

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Unsupported Operating System: {0}")]
    UnsupportedOperatingSystem(String),
}

/// Prepares libquil for use. No-op after the first call.
pub(crate) fn init_libquil() -> Result<(), Error> {
    let library_name = match std::env::consts::OS {
        "linux" => Ok("libquil.so".to_string()),
        "macos" => Ok("libquil.dylib".to_string()),
        os => Err(Error::UnsupportedOperatingSystem(os.to_string())),
    }?;

    START.call_once(|| {
        unsafe {
            // The library built by maturin does link to libquil, but
            // the linker does not make the libquil symbols available
            // to the lisp image. To get around that, we load it here
            // with the `RTLD_GLOBAL` flag which makes symbols available
            // to the whole process.
            libloading::os::unix::Library::open(
                Some(library_name),
                libloading::os::unix::RTLD_NOW | libloading::os::unix::RTLD_GLOBAL,
            )
            .unwrap();
        }
    });

    Ok(())
}

pub(crate) fn handle_libquil_error(errno: lisp_err_t) -> Result<(), String> {
    if errno == lisp_err_t_LISP_ERR_SUCCESS {
        return Ok(());
    }

    let mut error_str_ptr: *mut std::os::raw::c_char = std::ptr::null_mut();

    unsafe {
        let err = get_error_message(&mut error_str_ptr);
        if err != lisp_err_t_LISP_ERR_SUCCESS {
            return Err("unknown error occurred".to_string());
        }
        let error_str = CStr::from_ptr(error_str_ptr).to_str().unwrap();
        Err(error_str.to_string())
    }
}

pub(crate) fn get_string_from_pointer_and_free(
    ptr: *mut std::os::raw::c_char,
) -> Result<String, Utf8Error> {
    unsafe {
        let s = CStr::from_ptr(ptr).to_str()?.to_string();
        libc::free(ptr as *mut _);
        Ok(s)
    }
}
