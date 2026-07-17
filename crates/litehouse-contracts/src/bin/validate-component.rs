use anyhow::{Context, Result, bail};
use litehouse_contracts::component::{ComponentKind, validate_component};
use std::{fs, path::PathBuf};

fn main() -> Result<()> {
    let mut arguments = std::env::args_os().skip(1);
    let kind = match arguments
        .next()
        .context("usage: validate-component <plugin|quantity-engine> <component.wasm>")?
        .to_str()
    {
        Some("plugin") => ComponentKind::Plugin,
        Some("quantity-engine") => ComponentKind::QuantityEngine,
        Some(other) => bail!("unknown component kind `{other}`"),
        None => bail!("component kind must be UTF-8"),
    };
    let path = arguments
        .next()
        .map(PathBuf::from)
        .context("missing component path")?;
    if arguments.next().is_some() {
        bail!("unexpected extra arguments");
    }
    let bytes = fs::read(&path).with_context(|| format!("read {}", path.display()))?;
    validate_component(&bytes, kind)
        .with_context(|| format!("runtime-profile validation failed for {}", path.display()))?;
    println!("component runtime profile valid: {}", path.display());
    Ok(())
}
