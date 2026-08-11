use std::env;
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("Could not find libquil.h in any of the standard locations. Try setting C_INCLUDE_PATH or LIBQUIL_SRC_PATH")]
    HeaderNotFound,
    #[error(
        "Found libquil.h at {0}, but no sbcl_librarian.h beside it. This crate requires a libquil \
         built against modern sbcl-librarian, which installs the runtime headers alongside \
         libquil.h; an older libquil (0.3.x or earlier) does not have them. Install a newer \
         libquil, or set LIBQUIL_SRC_PATH to a build that has one."
    )]
    RuntimeHeaderNotFound(String),
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

    // For installs that do not use /usr/local, where the headers and libraries live
    // in separate directories and LIBQUIL_SRC_PATH names only the former.
    let libquil_lib_path: Option<&'static str> = option_env!("LIBQUIL_LIB_PATH");
    if let Some(libquil_lib_path) = libquil_lib_path {
        paths.insert(0, libquil_lib_path.to_string());
    }

    let libquil_src_path: Option<&'static str> = option_env!("LIBQUIL_SRC_PATH");
    if let Some(libquil_src_path) = libquil_src_path {
        // libquil is a FASL library loaded into the libsbcl_librarian runtime, so
        // both must be found. A source tree keeps the runtime in a subdirectory;
        // an installed layout puts everything in one directory.
        paths.insert(0, format!("{libquil_src_path}/runtime"));
        paths.insert(0, libquil_src_path.to_string());
    }

    paths
}

/// Directories to search for headers. `libquil.h` includes `sbcl_librarian_err.h`,
/// and `get_error_message` is declared in `sbcl_librarian.h`, both of which ship
/// with the runtime.
fn get_include_paths(libquil_header_path: &std::path::Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(dir) = libquil_header_path.parent() {
        paths.push(dir.to_path_buf());
        paths.push(dir.join("runtime"));
    }
    paths.retain(|p| p.exists());
    paths
}

fn main() {
    // Cargo prints a build script's error with Debug, which would hide the
    // explanation these errors carry, so report it and exit rather than returning it.
    if let Err(error) = build() {
        eprintln!("\nerror: {error}\n");
        std::process::exit(1);
    }
}

fn build() -> Result<(), Error> {
    let libquil_header_path = get_header_path()?;

    for path in get_lib_search_paths() {
        println!("cargo:rustc-link-search={}", path);
    }

    println!("cargo:rustc-link-lib=quil");
    // The runtime that hosts libquil: it supplies the Lisp image, the error API
    // (get_error_message) and the handle API (lisp_release_handle).
    println!("cargo:rustc-link-lib=sbcl_librarian");

    // Tell cargo to rerun if the libquil implementation has changed
    println!(
        "cargo:rustc-rerun-if-changed={}",
        libquil_header_path.clone().display()
    );

    let include_paths = get_include_paths(&libquil_header_path);

    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .header(libquil_header_path.to_string_lossy())
        // ...and the runtime's header, which declares the error API that libquil's
        // functions report through.
        .header(
            include_paths
                .iter()
                .map(|dir| dir.join("sbcl_librarian.h"))
                .find(|path| path.exists())
                .ok_or_else(|| {
                    Error::RuntimeHeaderNotFound(libquil_header_path.display().to_string())
                })?
                .to_string_lossy(),
        )
        .clang_args(
            include_paths
                .iter()
                .map(|dir| format!("-I{}", dir.display())),
        )
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
