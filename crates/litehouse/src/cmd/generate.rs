use std::{collections::HashMap, path::Path, sync::Arc};

use futures::{StreamExt, TryStreamExt};
use litehouse_config::{Import, LitehouseConfig, PluginConfig};
use litehouse_contracts::metadata::RuntimeMetadataV1;
use litehouse_plugin::serde_json::{self, Value};
use miette::{Context, IntoDiagnostic, Result, SourceSpan, miette};
use tokio::sync::broadcast::channel;
use tokio_stream::wrappers::ReadDirStream;

use crate::{
    runtime::{PluginRunnerFactory, epoch_ticks, set_up_engine},
    store::StoreStrategy,
};

pub async fn generate(wasm_path: &Path, cache: bool) -> Result<serde_json::Value> {
    let (engine, linker, cache) = set_up_engine(cache).await?;

    let (tx, _rx) = channel(1);

    let store = StoreStrategy::global(engine.clone(), PluginRunnerFactory::new(tx, vec![]));

    let linker = Arc::new(linker);
    let engine = Arc::new(engine);

    let dirs: Result<HashMap<_, _>> = ReadDirStream::new(
        tokio::fs::read_dir(wasm_path)
            .await
            .into_diagnostic()
            .wrap_err_with(|| format!("unable to read modules in `{}`", wasm_path.display()))?,
    )
    .map(|entry| {
        let entry = entry
            .into_diagnostic()
            .wrap_err("unable to read a plugin directory entry")?;
        let file_name = entry
            .file_name()
            .into_string()
            .map_err(|_| miette!("unable to parse plugin file name as UTF-8"))?;
        let plugin: Import = file_name.parse().wrap_err("invalid wasm plugin name")?;
        Ok((
            file_name,
            PluginConfig {
                plugin,
                instance_id: None,
                capabilities: Vec::new(),
                config: None,
            },
        ))
    })
    .try_collect()
    .await;

    let dirs = dirs?;

    let hosts = crate::runtime::instantiate_plugin_hosts(
        None, &engine, &store, &linker, &dirs, wasm_path, 10, 10,
    )
    .await?;
    if let Some(cache) = cache
        && let Err(e) = cache.drain().await
    {
        tracing::warn!("unable to save cache: {}", e)
    }

    if hosts.is_empty() {
        tracing::warn!("no plugins found in `{}`", wasm_path.display());
    }

    let jobs = hosts
        .into_iter()
        .map(|(a, instance, host, mut store, _)| async move {
            let mut store = store.enter().await;

            let indices =
                crate::runtime::bindings::exports::litehouse::plugin::plugin::GuestIndices::new(
                    &host.instance_pre(&mut store),
                )
                .map_err(|error| miette!("failed to load plugin export indices: {error:?}"))?;
            let guest = indices
                .load(&mut store, &host)
                .map_err(|error| miette!("failed to load plugin exports: {error:?}"))?;
            store.set_epoch_deadline(epoch_ticks(std::time::Duration::from_secs(10)));
            let metadata = tokio::time::timeout(
                std::time::Duration::from_secs(10),
                guest.call_get_metadata(&mut store),
            )
            .await
            .map_err(|_| miette!("plugin metadata call exceeded 10 seconds"))?;

            match metadata {
                Ok(metadata) => {
                    let metadata = RuntimeMetadataV1::decode_section(&metadata)
                        .into_diagnostic()
                        .wrap_err("plugin returned invalid bounded runtime metadata")?;
                    let identifier = metadata.package.name;
                    if instance.plugin.plugin != identifier {
                        tracing::error!(
                            "plugin identifier mismatch: {} != {}",
                            instance.plugin.plugin,
                            identifier
                        );
                        return Err(miette!("plugin identifier mismatch"));
                    };

                    let version = format!(
                        "{}.{}.{}",
                        metadata.package.version.major,
                        metadata.package.version.minor,
                        metadata.package.version.patch
                    )
                    .parse()
                    .into_diagnostic()?;
                    if let Some(version_exp) = &instance.plugin.version
                        && version_exp != &version
                    {
                        return Err(VersionMismatch {
                            file_exp: format!("{}@{}.wasm", identifier, version),
                            file_path: wasm_path.join(a).to_string_lossy().to_string(),
                            plugin: identifier,
                            source_code: format!("{} != {}", version, version_exp),
                            expected: (0, version.to_string().len()).into(),
                            actual: (version.to_string().len() + 4, version_exp.to_string().len())
                                .into(),
                        }
                        .into());
                    }

                    let config_schema = metadata
                        .config_schema
                        .map(|schema| {
                            serde_json::from_str(&schema)
                                .into_diagnostic()
                                .wrap_err("plugin returned malformed typed config schema")
                        })
                        .transpose()?;
                    Ok((
                        Import {
                            plugin: identifier,
                            version: Some(version),
                            registry: None,
                            sha: None,
                        },
                        config_schema,
                    ))
                }
                Err(error) => {
                    tracing::error!("failed to obtain plugin metadata: {error:?}");
                    Err(miette!("plugin metadata call failed"))
                }
            }
        });

    let schemas: Vec<(_, Option<serde_json::Value>)> = futures::future::try_join_all(jobs).await?;

    let config_schema = schemars::schema_for!(LitehouseConfig);
    let json = serde_json::to_value(&config_schema)
        .into_diagnostic()
        .wrap_err("unable to encode Litehouse config schema")?;

    inject_plugin_instance(json, schemas.into_iter())
}

#[derive(miette::Diagnostic, Debug, thiserror::Error)]
#[error("version mismatch for {plugin}")]
#[diagnostic(help(
    "rename the file at `{file_path}` to `{file_exp}` so it matches the version in the plugin"
))]
pub struct VersionMismatch {
    plugin: String,
    file_path: String,
    #[label("expected")]
    expected: SourceSpan,
    #[label("actual")]
    actual: SourceSpan,
    file_exp: String,
    #[source_code]
    source_code: String,
}

fn inject_plugin_instance(
    mut json: Value,
    plugins: impl Iterator<Item = (Import, Option<Value>)>,
) -> Result<serde_json::Value> {
    let definitions = json
        .get_mut("definitions")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| miette!("generated config schema has no definitions object"))?;
    let plugin_config = definitions
        .get_mut("PluginConfig")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| miette!("generated config schema has no PluginConfig definition"))?;
    let base = std::mem::take(plugin_config);

    let variants = plugins
        .map(|(import, schema)| {
            let mut config_base = base.clone();
            let properties = config_base
                .get_mut("properties")
                .and_then(Value::as_object_mut)
                .ok_or_else(|| miette!("PluginConfig schema has no properties object"))?;

            let plugin = properties
                .get_mut("plugin")
                .ok_or_else(|| miette!("PluginConfig schema has no plugin property"))?;
            *plugin =
                serde_json::Map::from_iter([("const".into(), import.to_string().into())]).into();

            let has_config = if let Some(mut schema) = schema {
                let object = schema
                    .as_object_mut()
                    .ok_or_else(|| miette!("plugin config schema must be an object"))?;
                object.remove("$schema");
                object.remove("title");
                let config = properties
                    .get_mut("config")
                    .ok_or_else(|| miette!("PluginConfig schema has no config property"))?;
                *config = schema;
                true
            } else {
                properties.remove("config");
                false
            };

            let required = config_base
                .get_mut("required")
                .and_then(Value::as_array_mut)
                .ok_or_else(|| miette!("PluginConfig schema has no required array"))?;
            match (
                required.iter().position(|value| value == "config"),
                has_config,
            ) {
                (Some(position), false) => {
                    required.remove(position);
                }
                (None, true) => required.push("config".into()),
                _ => {}
            }
            Ok(config_base)
        })
        .collect::<Result<Vec<_>>>()?;

    plugin_config.insert("oneOf".to_string(), variants.into());
    Ok(json)
}

#[cfg(test)]
mod tests {
    use super::inject_plugin_instance;
    use litehouse_config::{Import, LitehouseConfig};

    #[test]
    fn malformed_plugin_schema_returns_an_error() {
        let root = serde_json::to_value(schemars::schema_for!(LitehouseConfig)).unwrap();
        let plugin: Import = "noop".parse().unwrap();

        assert!(
            inject_plugin_instance(
                root,
                [(plugin, Some(serde_json::Value::String("invalid".into())))].into_iter(),
            )
            .is_err()
        );
    }
}
