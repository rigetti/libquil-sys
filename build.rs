use std::env;
use std::path::PathBuf;

fn main() {
    // TODO These shouldn't be tied to my fs.
    // Tell cargo to look for shared libraries in the specified directory
    println!("cargo:rustc-link-search=/Users/mgsk/hackery/lisp/quilc/lib");

    println!("cargo:rustc-link-lib=quilc");

    // Tell cargo to invalidate the built crate whenever the wrapper changes
    println!("cargo:rerun-if-changed=/Users/mgsk/hackery/lisp/quilc/lib/libquilc.h");

    // TODO Condition this
    // macos fix
    println!("cargo:rustc-link-arg=-pagezero_size 0x100000");

    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .header("/Users/mgsk/hackery/lisp/quilc/lib/libquilc.h")
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    println!("Writing bindings to {}", out_path.to_string_lossy());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
