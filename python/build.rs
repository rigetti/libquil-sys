fn main() {
    eprintln!("python build");
    pyo3_build_config::add_extension_module_link_args();
    eprintln!("finished python build");
}
