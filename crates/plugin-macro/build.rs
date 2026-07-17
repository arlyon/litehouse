use std::{env, fs, io, path::Path};

fn main() -> io::Result<()> {
    let manifest = env::current_dir()?;
    let output = Path::new(&env::var("OUT_DIR").expect("Cargo sets OUT_DIR")).join("wit");
    if output.exists() {
        fs::remove_dir_all(&output)?;
    }
    copy_tree(&manifest.join("wit"), &output)?;

    let descriptor =
        manifest.join("../litehouse-contracts/packages/descriptor/1.0.0/wit/descriptor.wit");
    let descriptor_output = output.join("deps/descriptor.wit");
    fs::copy(&descriptor, descriptor_output)?;

    println!("cargo::rerun-if-changed=wit");
    println!("cargo::rerun-if-changed={}", descriptor.display());
    println!(
        "cargo::rustc-env=WIT_DIR={}",
        output.parent().expect("WIT output has a parent").display()
    );
    Ok(())
}

fn copy_tree(source: &Path, destination: &Path) -> io::Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let target = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}
