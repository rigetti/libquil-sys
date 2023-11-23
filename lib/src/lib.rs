#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::{
    ffi::{CStr, CString},
    path::PathBuf,
    str::Utf8Error,
    sync::Once,
};

use bindings::{libquil_error, libquil_error_t, libquil_error_t_LIBQUIL_ERROR_SUCCESS};

pub mod quilc;
pub mod qvm;

#[allow(dead_code)]
pub(crate) mod bindings {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gen/bindings.rs"));
}

static START: Once = Once::new();

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Could not find libquil core file. Set the LIBQUIL_CORE_PATH environment variable.")]
    CoreFileNotFound,
    #[error("Unsupported Operating System: {0}")]
    UnsupportedOperatingSystem(String),
}

fn find_core_file() -> Result<String, Error> {
    let mut paths = vec!["/usr/local/lib/libquil.core", "/usr/lib/libquil.core"];

    let libquil_src_path: Option<&'static str> = option_env!("LIBQUIL_CORE_PATH");
    if let Some(libquil_src_path) = libquil_src_path {
        paths.insert(0, libquil_src_path);
    }

    for path in paths {
        if PathBuf::from(path).exists() {
            return Ok(path.to_string());
        }
    }

    Err(Error::CoreFileNotFound)
}

/// Initializes libquil using it's core image. No-op after the first call.
pub(crate) fn init_libquil() -> Result<(), Error> {
    let core_path = find_core_file()?;
    let library_name = match std::env::consts::OS {
        "linux" => Ok("libquil.so".to_string()),
        "macos" => Ok("libquil.dylib".to_string()),
        os => Err(Error::UnsupportedOperatingSystem(os.to_string())),
    }?;

    START.call_once(|| {
        let ptr = CString::new(core_path).unwrap().into_raw();

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
            bindings::init(ptr);
            let _ = CString::from_raw(ptr);
        }
    });

    Ok(())
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

pub(crate) fn get_string_from_pointer_and_free(ptr: *mut i8) -> Result<String, Utf8Error> {
    unsafe {
        let s = CStr::from_ptr(ptr).to_str()?.to_string();
        libc::free(ptr as *mut _);
        Ok(s)
    }
}
