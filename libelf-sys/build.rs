use std::{env, path::PathBuf};

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let binding = bindgen::builder().header("wrapper.h").generate().unwrap();
    binding.write_to_file(out_dir.join("bindings.rs")).unwrap();
}
