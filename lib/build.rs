use std::env;
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("Could not find error in any of the standard locations. Try setting C_INCLUDE_PATH or LIBQUIL_SRC_PATH")]
    HeaderNotFound,
    #[error("Could not read environment variable: {0}")]
    InvalidEnvvar(#[from] env::VarError),
}

fn get_header_path() -> Result<PathBuf, Error> {
    let mut paths = vec!["/usr/local/include/libquil", "/usr/include/libquil"];

    let libquil_src_path: Option<&'static str> = option_env!("LIBQUIL_SRC_PATH");
    if let Some(libquil_src_path) = libquil_src_path {
        paths.insert(0, libquil_src_path);
    }

    let c_include_path: Option<&'static str> = option_env!("C_INCLUDE_PATH");
    if let Some(c_include_path) = c_include_path {
        paths.insert(0, c_include_path);
    }

    for path in paths {
        let path = PathBuf::from(path).join("libquil.h");
        if path.exists() {
            return Ok(path);
        }
    }

    Err(Error::HeaderNotFound)
}

fn get_lib_search_paths() -> Vec<String> {
    let mut paths = vec!["/usr/local/lib".to_string(), "/usr/lib".to_string()];

    let libquil_src_path: Option<&'static str> = option_env!("LIBQUIL_SRC_PATH");
    if let Some(libquil_src_path) = libquil_src_path {
        paths.insert(0, libquil_src_path.to_string());
    }

    paths
}

fn main() -> Result<(), Error> {
    let libquil_header_path = get_header_path()?;

    for path in get_lib_search_paths() {
        println!("cargo:rustc-link-search={}", path);
    }

    println!("cargo:rustc-link-lib=quil");

    // Tell cargo to rerun if the libquil implementation has changed
    println!(
        "cargo:rustc-rerun-if-changed={}",
        libquil_header_path.clone().display()
    );

    // If this isn't set on MacOS, memory allocation errors occur when trying to initialize the
    // library
    if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-arg=-pagezero_size 0x100000");
    }

    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .header(libquil_header_path.to_string_lossy())
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Bindings should be generated");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    println!("Writing bindings to {}", out_path.display());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Should be able to write bindings to file.");

    Ok(())
}
