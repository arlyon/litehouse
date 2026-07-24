use std::{env, error::Error, io, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=registry.fbs");

    let out_dir = env::var_os("OUT_DIR")
        .map(PathBuf::from)
        .ok_or_else(|| io::Error::other("Cargo did not set OUT_DIR"))?;
    let schema = PathBuf::from("registry.fbs");
    flatc_rust::run(flatc_rust::Args {
        inputs: &[&schema],
        out_dir: &out_dir,
        ..Default::default()
    })?;
    Ok(())
}
