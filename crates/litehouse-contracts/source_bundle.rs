use anyhow::{Context, Result, bail};
use flate2::{Compression, GzBuilder};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

pub fn source_bundle(root: &Path, trusted_root: &Path) -> Result<Vec<u8>> {
    reject_symlinked_path(trusted_root, trusted_root)?;
    reject_symlinked_path(root, trusted_root)?;
    let canonical_trusted_root = trusted_root.canonicalize().with_context(|| {
        format!(
            "canonicalize trusted source root {}",
            trusted_root.display()
        )
    })?;
    let canonical_root = root
        .canonicalize()
        .with_context(|| format!("canonicalize source root {}", root.display()))?;
    if !canonical_root.starts_with(&canonical_trusted_root) {
        bail!(
            "source root escaped trusted package tree: {}",
            root.display()
        );
    }

    let mut files = Vec::new();
    collect_files(root, &canonical_root, &mut files)?;
    files.sort();

    let mut tar_bytes = Vec::new();
    {
        let mut archive = tar::Builder::new(&mut tar_bytes);
        for path in files {
            let relative = path
                .strip_prefix(root)
                .with_context(|| format!("source path escaped {}", root.display()))?;
            let canonical = path
                .canonicalize()
                .with_context(|| format!("canonicalize source file {}", path.display()))?;
            if !canonical.starts_with(&canonical_root) {
                bail!("source file escaped package root: {}", path.display());
            }
            let content = fs::read(&canonical)?;
            let mut header = tar::Header::new_gnu();
            header.set_size(content.len() as u64);
            header.set_mode(0o644);
            header.set_uid(0);
            header.set_gid(0);
            header.set_mtime(0);
            header.set_cksum();
            archive.append_data(&mut header, relative, content.as_slice())?;
        }
        archive.finish()?;
    }

    let mut gzip = GzBuilder::new()
        .mtime(0)
        .write(Vec::new(), Compression::best());
    gzip.write_all(&tar_bytes)?;
    Ok(gzip.finish()?)
}

fn reject_symlinked_path(path: &Path, trusted_root: &Path) -> Result<()> {
    let relative = path.strip_prefix(trusted_root).with_context(|| {
        format!(
            "source root {} is outside trusted tree {}",
            path.display(),
            trusted_root.display()
        )
    })?;
    let mut current = trusted_root.to_path_buf();
    reject_directory(&current)?;
    for component in relative.components() {
        current.push(component);
        reject_directory(&current)?;
    }
    Ok(())
}

fn reject_directory(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    let file_type = metadata.file_type();
    if file_type.is_symlink() {
        bail!("source bundles reject symlinked roots: {}", path.display());
    }
    if !file_type.is_dir() {
        bail!("source bundle root must be a directory: {}", path.display());
    }
    Ok(())
}

fn collect_files(root: &Path, canonical_root: &Path, output: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)?;
        let file_type = metadata.file_type();
        if file_type.is_symlink() {
            bail!("source bundles reject symlinks: {}", path.display());
        }
        if file_type.is_dir() {
            let canonical = path.canonicalize()?;
            if !canonical.starts_with(canonical_root) {
                bail!("source directory escaped package root: {}", path.display());
            }
            collect_files(&path, canonical_root, output)?;
        } else if file_type.is_file() {
            output.push(path);
        } else {
            bail!("source bundles require regular files: {}", path.display());
        }
    }
    Ok(())
}
