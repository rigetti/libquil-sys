use std::env;

#[derive(Debug, thiserror::Error)]
enum Error {
    #[allow(dead_code)]
    #[error("Could not find error in any of the standard locations. Try setting C_INCLUDE_PATH or LIBQUIL_SRC_PATH")]
    HeaderNotFound,
    #[error("Could not read environment variable: {0}")]
    InvalidEnvvar(#[from] env::VarError),
}

fn split_lib_search_paths(paths: Vec<String>) -> Vec<String> {
    paths
        .into_iter()
        .flat_map(|p| p.split(':').map(Into::into).collect::<Vec<String>>())
        .collect()
}

fn get_lib_search_paths() -> Vec<String> {
    let mut paths = vec!["/usr/local/lib".to_string(), "/usr/lib".to_string()];

    let libquil_src_path: Option<&'static str> = option_env!("LIBQUIL_SRC_PATH");
    if let Some(libquil_src_path) = libquil_src_path {
        paths.insert(0, libquil_src_path.to_string());
    }

    let ld_library_path: Option<&'static str> = option_env!("LD_LIBRARY_PATH");
    if let Some(ld_library_path) = ld_library_path {
        paths.insert(0, ld_library_path.to_string());
    }

    split_lib_search_paths(paths)
}

#[cfg(feature = "codegen")]
mod codegen {
    use std::{env, path::PathBuf};

    fn get_header_path() -> Result<PathBuf, super::Error> {
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

        Err(super::Error::HeaderNotFound)
    }

    pub(crate) fn codegen() -> Result<(), super::Error> {
        use std::fs::OpenOptions;
        use std::io::Write;

        let libquil_header_path = get_header_path()?;

        // Tell cargo to rerun if the libquil implementation has changed
        println!(
            "cargo:rustc-rerun-if-changed={}",
            libquil_header_path.clone().display()
        );

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
        bindings
            .write_to_file(out_path.join("bindings.rs"))
            .expect("Should be able to write bindings to file.");

        let gen_code_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("gen");

        for entry in std::fs::read_dir(PathBuf::from(&out_path))
            .expect("OUT_DIR environment variable should point to a valid directory")
        {
            let src_path = entry.expect("OUT_DIR should contain files").path();
            if src_path.to_string_lossy().ends_with(".rs") {
                let dest = gen_code_dir.join(
                    src_path
                        .file_name()
                        .expect("path should include a valid file name"),
                );

                let mut file = OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(dest.clone())
                    .unwrap_or_else(|_| {
                        panic!("Should open file '{}' for writing", dest.to_string_lossy())
                    });
                writeln!(
                    file,
                    "{}",
                    std::fs::read_to_string(src_path).expect("Should read file contents")
                )
                .expect("Should write file contents");
            }
        }

        Ok(())
    }
}

fn main() -> Result<(), Error> {
    for path in get_lib_search_paths() {
        println!("cargo:rustc-link-search={}", path);
    }

    println!("cargo:rustc-link-lib=quil");

    // If this isn't set on MacOS, memory allocation errors occur when trying to initialize the
    // library
    if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-arg=-pagezero_size 0x100000");
    }

    #[cfg(feature = "codegen")]
    codegen::codegen()?;

    Ok(())
}
