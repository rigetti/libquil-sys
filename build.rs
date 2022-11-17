use std::env;
use std::path::PathBuf;

fn main() {
    // Get the quilc library path from users environment
    // If unset, defaults to the standard directory for quicklisp local projects
    let quilc_library_path = PathBuf::from(
        env::var("QUILC_LIBRARY_PATH").unwrap_or(
            format!(
                "{}/quicklisp/local-projects/quilc/lib",
                env::var("HOME").expect("$HOME should be set")
            )
            .to_string(),
        ),
    );
    let quilc_library_path = quilc_library_path.to_string_lossy();

    println!("{}", quilc_library_path);
    // Tell cargo to look for shared libraries in the specified directory
    println!("cargo:rustc-link-search={}", quilc_library_path);

    println!("cargo:rustc-link-lib=quilc");

    // Tell cargo to invalidate the built crate whenever the wrapper changes
    println!("cargo:rerun-if-changed={}/libquilc.h", quilc_library_path);

    //
    if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-arg=-pagezero_size 0x100000");
    }

    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .header(format!("{}/libquilc.h", quilc_library_path))
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Bindings should be generated");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    println!("Writing bindings to {}", out_path.to_string_lossy());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Should be able to write bindings to file.");
}
