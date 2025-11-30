use std::{collections::HashMap, path::Path, sync::Arc};

use futures::{StreamExt, TryStreamExt};
use litehouse_config::{Import, LitehouseConfig, PluginConfig};
use litehouse_plugin::serde_json::{self, Value};
use miette::{Context, IntoDiagnostic, Result, SourceSpan, miette};
use tokio::sync::broadcast::channel;
use tokio_stream::wrappers::ReadDirStream;

use crate::{
    runtime::{
        PluginRunnerFactory, bindings::exports::litehouse::plugin::plugin::Metadata, set_up_engine,
    },
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
    .map(|dir| {
        let file_name = dir
            .as_ref()
            .unwrap()
            .file_name()
            .into_string()
            .map_err(|_| miette!("unable to parse file name as string"))?;
        let plugin: Import = file_name.parse().wrap_err("invalid wasm plugin name")?;
        Ok((
            dir.unwrap().file_name().into_string().unwrap(),
            PluginConfig {
                config: None,
                plugin,
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

    let jobs = hosts.into_iter().map(|(a, instance, host, mut store, _)| {
        async move {
            let mut store = store.enter().await;

            let indices =
                crate::runtime::bindings::exports::litehouse::plugin::plugin::GuestIndices::new(
                    &host.instance_pre(&mut store),
                )
                .unwrap();
            let guest = indices.load(&mut store, &host).unwrap();
            let metadata = guest.call_get_metadata(&mut store);

            match metadata {
                Ok(Metadata {
                    config_schema,
                    identifier,
                    version,
                    ..
                }) => {
                    // check that version above and version here match

                    if instance.plugin.plugin != identifier {
                        tracing::error!(
                            "plugin identifier mismatch: {} != {}",
                            instance.plugin.plugin,
                            identifier
                        );
                        return Err(miette!("plugin identifier mismatch"));
                    };

                    let version = version.parse().into_diagnostic()?;
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

                    Ok((
                        Import {
                            plugin: identifier,
                            version: Some(version),
                            registry: None,
                            sha: None,
                        },
                        config_schema.and_then(|s| serde_json::from_str(&s).ok()),
                    ))
                }
                Err(_) => {
                    tracing::error!("failed to generate schema: {:?}", metadata);
                    panic!();
                }
            }
        }
    });

    let schemas: Vec<(_, Option<serde_json::Value>)> = futures::future::try_join_all(jobs).await?;

    let config_schema = schemars::schema_for!(LitehouseConfig);
    let json = serde_json::to_value(&config_schema).expect("can't fail");

    Ok(inject_plugin_instance(json, schemas.into_iter()))
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
) -> serde_json::Value {
    let definitions = json
        .get_mut("definitions")
        .expect("this is always present")
        .get_mut("PluginConfig")
        .expect("always exists")
        .as_object_mut()
        .expect("is always an object");

    let base = std::mem::take(definitions);

    definitions.insert(
        "oneOf".to_string(),
        plugins
            .map(|(import, schema)| {
                let mut config_base = base.clone();
                let properties = config_base
                    .get_mut("properties")
                    .expect("always exists")
                    .as_object_mut()
                    .expect("is always an object");

                *properties.get_mut("plugin").unwrap() =
                    serde_json::Map::from_iter([("const".into(), import.to_string().into())])
                        .into();

                let set = if let Some(mut schema) = schema {
                    let object = schema.as_object_mut().unwrap();
                    object.remove("$schema");
                    object.remove("title");
                    *properties.get_mut("config").unwrap() = schema;
                    true
                } else {
                    properties.remove("config");
                    false
                };

                let required = config_base
                    .get_mut("required")
                    .unwrap()
                    .as_array_mut()
                    .unwrap();

                match (required.iter().position(|s| s == "config"), set) {
                    (Some(pos), false) => {
                        required.remove(pos);
                    }
                    (None, true) => {
                        required.push("config".into());
                    }
                    _ => {}
                };

                config_base
            })
            .collect(),
    );

    json
}
