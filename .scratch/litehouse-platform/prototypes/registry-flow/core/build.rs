fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=schema/registry.fbs");
    let output = std::env::var("OUT_DIR")?;
    flatc_rust::run(flatc_rust::Args {
        inputs: &[std::path::Path::new("schema/registry.fbs")],
        out_dir: std::path::Path::new(&output),
        extra: &["--gen-object-api"],
        ..Default::default()
    })?;
    Ok(())
}
