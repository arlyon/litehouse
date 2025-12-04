//! The main commands for the Litehouse CLI.

mod auth;
mod generate;
mod packages;
mod run;
mod validate;

use std::path::PathBuf;

use auth::AuthCommand;
use jsonc_parser::CollectOptions;
use jsonschema::{error::ValidationErrorKind, paths::PathChunk};
use litehouse_config::{
    ConfigError, Import, ImportAddResult, LitehouseConfig, Manifest, ManifestAddResult,
    ManifestImport, ParseError, PluginConfig,
};
use litehouse_plugin::serde_json;
use litehouse_registry::Registry;
use miette::{Context, IntoDiagnostic, NamedSource, Result};
use tokio::sync::broadcast::channel;

use crate::{
    LogMessage,
    runtime::{PluginRunnerFactory, set_up_engine},
    store::StoreStrategy,
    util::resolve_span,
};

use self::validate::{FailedValidation, FailedValidations};

#[derive(Debug, serde::Serialize)]
struct FeedbackMessage {
    feedback: String,
    version: String,
    email: Option<String>,
    name: Option<String>,
}

/// Litehouse subcommands
#[derive(clap::Subcommand)]
pub enum Subcommand {
    /// Run the litehouse server
    Run {
        /// The path to look for wasm files in.
        #[clap(default_value = "./wasm", short, long)]
        wasm_path: PathBuf,
        /// Whether to enable the wasm cache
        #[clap(long)]
        no_cache: bool,
    },
    /// Inspect a plugin to see its metadata
    Inspect {
        #[clap(default_value = "./wasm", short, long)]
        wasm_path: PathBuf,
        /// The path to the wasm file to inspect
        plugin: Import,
        /// Whether to enable the wasm cache
        #[clap(long)]
        no_cache: bool,
    },
    /// Add a new plugin to the imports field in the settings file.
    Add {
        /// The package to add.
        package: Vec<litehouse_config::ManifestImport>,
        /// The path to look for wasm files in.
        #[clap(default_value = "./wasm", short, long)]
        wasm_path: PathBuf,
    },
    /// Generate a jsonschema for the config file, based on the
    /// plugins that are in your wasm path
    Generate {
        /// The path to look for wasm files in.
        #[clap(default_value = "./wasm", short, long)]
        wasm_path: PathBuf,
        /// Whether to enable the wasm cache
        #[clap(long)]
        no_cache: bool,
    },
    /// Initialize a new litehouse config in the current directory
    Init,
    /// Validate the config file, based on the jsonschema
    Validate {
        /// The path to look for wasm files in.
        #[clap(default_value = "./wasm", short, long)]
        wasm_path: PathBuf,
        /// Whether to enable the wasm cache
        #[clap(long)]
        no_cache: bool,
    },
    /// Publish a package to the registry. Run this in the root of your package.
    Publish {
        package: String,
        #[clap(long)]
        access_key: String,
        #[clap(long)]
        secret_key: String,
    },
    /// Fetch packages from the registry, based on the imports in your settings file.
    Fetch {
        #[clap(default_value = "wasm")]
        wasm_path: PathBuf,
    },
    /// Build a package and write the wasm file to the specified path.
    Build {
        package: String,
        #[clap(default_value = "wasm")]
        wasm_path: PathBuf,
        #[clap(long, default_value_t = false)]
        debug: bool,
        /// Do not perform the final packaging, leaving a regular wasm file.
        #[clap(long)]
        no_package: bool,
        /// Do not optimise the wasm file for size.
        #[clap(long)]
        no_optimise: bool,
    },
    /// Search for a package in the registry.
    Search {
        /// The plugin to search for.
        query: Option<String>,
    },
    /// Lock all the packages in your settings file, by setting their
    /// hashes (if they don't exist).
    Lock {
        #[clap(default_value = "wasm")]
        wasm_path: PathBuf,
    },
    /// Send any feedback! Note that this will be sent to the litehouse team
    /// with your git email and name (so that we can get in touch).
    Feedback { message: String },
    /// Authenticate with litehouse.arlyon.dev to upload plugins
    Auth {
        #[command(subcommand)]
        auth_command: AuthCommand,
    },
}

impl Subcommand {
    pub async fn run(self, logs_rx: tokio::sync::broadcast::Receiver<LogMessage>) -> Result<()> {
        let registry = Registry::build("default".to_string());

        match self {
            Subcommand::Init => {
                std::fs::create_dir_all("wasm")
                    .into_diagnostic()
                    .wrap_err("unable to create wasm directory")?;
                if std::fs::metadata("settings.json").is_ok() {
                    return Ok(());
                }
                let default = LitehouseConfig::default();
                let mut value = serde_json::to_value(default).expect("can't fail");
                value
                    .as_object_mut()
                    .expect("can't fail")
                    .insert("$schema".to_string(), "schema.json".into());
                let mut file = std::fs::File::create("settings.json")
                    .into_diagnostic()
                    .wrap_err("unable to create settings.json")?;
                serde_json::to_writer_pretty(&mut file, &value)
                    .into_diagnostic()
                    .wrap_err("unable to write settings.json")?;

                Ok(())
            }
            Subcommand::Add {
                package: packages,
                wasm_path,
            } => {
                // search the package, and add it to the settings' imports
                let mut config = LitehouseConfig::load()?;
                for package in packages {
                    match package {
                        ManifestImport::Import(package) => {
                            println!("adding package {}", package);
                            match config.add_import(package) {
                                ImportAddResult::Added(_) => {
                                    println!("package added");
                                }
                                ImportAddResult::Ignored(i) => {
                                    println!("package already added ({i})");
                                }
                                ImportAddResult::Replaced(r) => {
                                    println!("replaced {r}");
                                }
                            }
                        }
                        ManifestImport::Manifest(manifest) => {
                            println!("adding manifest");
                            let import = manifest.import.clone();
                            let to_replace = config
                                .add_manifest(manifest, false)
                                .filter_map(|result| match result {
                                    ManifestAddResult::Added(k) => {
                                        println!("- {k}: manifest added");
                                        None
                                    }
                                    ManifestAddResult::Ignored(k) => {
                                        println!("- {k}: identical manifest already exists");
                                        None
                                    }
                                    ManifestAddResult::WouldReplace(k, v) => {
                                        let ans = inquire::Confirm::new(&format!(
                                            " `{k}` already exists, replace it?"
                                        ))
                                        .with_default(false)
                                        .prompt();
                                        if let Ok(true) = ans {
                                            Some((k, v))
                                        } else {
                                            None
                                        }
                                    }
                                    ManifestAddResult::Replaced(k, _old) => {
                                        println!("- {k}: manifest replaced");
                                        None
                                    }
                                })
                                .fold(
                                    Manifest {
                                        import,
                                        config: Default::default(),
                                    },
                                    |mut acc, (k, v)| {
                                        acc.config.insert(k, v);
                                        acc
                                    },
                                );

                            config.add_manifest(to_replace, true).for_each(drop);
                        }
                    }
                }

                let cache_dir = litehouse_config::directories().map(|d| d.cache_dir().to_owned());

                println!("downloading");

                let _pass = packages::fetch(
                    &config,
                    &registry
                        .with_download(wasm_path, cache_dir)
                        .build()
                        .await
                        .wrap_err("can't fetch")?,
                )
                .await;

                config.save()?;

                Ok(())
            }
            Subcommand::Run {
                wasm_path,
                no_cache,
            } => run::run(&wasm_path, !no_cache, logs_rx)
                .await
                .wrap_err("unable to start litehouse"),
            Subcommand::Inspect {
                wasm_path,
                plugin,
                no_cache,
            } => {
                let (engine, linker, cache) = set_up_engine(!no_cache).await?;

                let dirs = [(
                    plugin.plugin.clone(),
                    PluginConfig {
                        config: None,
                        plugin,
                    },
                )]
                .into_iter()
                .collect();

                let (rx, _) = channel(1000);
                let factory = PluginRunnerFactory::new(rx, Default::default());
                let store = StoreStrategy::global(engine.clone(), factory);

                let hosts = crate::runtime::instantiate_plugin_hosts(
                    None, &engine, &store, &linker, &dirs, &wasm_path, 10, 10,
                )
                .await?;
                if let Some(cache) = cache
                    && let Err(e) = cache.drain().await
                {
                    tracing::warn!("unable to save cache: {}", e)
                }

                let jobs = hosts
                    .into_iter()
                    .map(|(a, _, host, mut store, _)| async move {
                        let mut store = store.enter().await;

                        let indices = crate::runtime::bindings::exports::litehouse::plugin::plugin::GuestIndices::new( &host.instance_pre(&mut store)).unwrap();
                        let guest = indices.load(&mut store, &host).unwrap();
                        let metadata = guest.call_get_metadata(&mut store).await;

                        match metadata {
                            Ok(meta) => {
                                println!("metadata for {}:\n", a);
                                println!(" - version: {}", meta.version);
                                if let Some(author) = meta.author
                                    && !author.is_empty()
                                {
                                    println!(" - author: {}", author);
                                }
                                if let Some(homepage) = meta.homepage
                                    && !homepage.is_empty()
                                {
                                    println!(" - homepage: {}", homepage);
                                }
                                if let Some(source) = meta.source
                                    && !source.is_empty()
                                {
                                    println!(" - source: {}", source);
                                }
                                if let Some(description) = meta.description
                                    && !description.is_empty()
                                {
                                    println!(" - description: {}", description);
                                }
                                println!(" - capabilities: {:?}", meta.capabilities);
                            }
                            Err(_) => {
                                tracing::error!("failed to generate schema: {:?}", metadata);
                                panic!();
                            }
                        }
                    });

                _ = futures::future::join_all(jobs).await;

                Ok(())
            }
            Subcommand::Generate {
                wasm_path,
                no_cache,
            } => {
                let schema = generate::generate(&wasm_path, !no_cache)
                    .await
                    .wrap_err("unable to generate schema")?;

                // write file
                let mut file = std::fs::File::create("schema.json")
                    .into_diagnostic()
                    .wrap_err("unable to create schema file")?;
                serde_json::to_writer_pretty(&mut file, &schema)
                    .into_diagnostic()
                    .wrap_err("unable to write schema file")?;

                println!("wrote jsonschema to schema.json");
                Ok(())
            }
            Subcommand::Validate {
                wasm_path,
                no_cache,
            } => {
                let schema = generate::generate(&wasm_path, !no_cache)
                    .await
                    .wrap_err("unable to generate schema")?;
                let schema = jsonschema::JSONSchema::compile(&schema).expect("can't fail");

                let data = std::fs::read_to_string("settings.json")
                    .into_diagnostic()
                    .wrap_err("could not read settings")?;

                let data2 = data.clone();

                let ast = jsonc_parser::parse_to_ast(
                    &data,
                    &CollectOptions {
                        comments: true,
                        tokens: true,
                    },
                    &Default::default(),
                )
                .map_err(|e| {
                    ConfigError::Parse(ParseError {
                        err_span: (e.range.start, e.range.end).into(),
                        src: NamedSource::new("settings.json", data.clone()),
                        error: e.message,
                    })
                })?;

                let config = ast.value.expect("has a value");

                schema
                    .validate(&config.clone().into())
                    .map_err(move |e| {
                        FailedValidations::new(
                            e.into_iter()
                                .map(|e| {
                                    let mut iter = e.instance_path.iter();
                                    let fst = iter.next();
                                    let snd = iter.next();
                                    let label = match (&e.kind, fst, snd) {
                                        (
                                            ValidationErrorKind::OneOfNotValid,
                                            Some(PathChunk::Property(p)),
                                            Some(PathChunk::Property(plugin)),
                                        ) if p.as_ref() == "plugins" => {
                                            format!("invalid plugin definition for `{}`", plugin)
                                        }
                                        _ => "invalid setting".to_string(),
                                    };

                                    FailedValidation {
                                        span: resolve_span(&config, &e.instance_path),
                                        message: e.to_string(),
                                        label,
                                        kind: e.kind,
                                        error: e.instance.into_owned(),
                                        instance_path: e.instance_path,
                                        schema_path: e.schema_path,
                                    }
                                })
                                .collect(),
                            data2,
                        )
                    })
                    .wrap_err("could not validate settings.json")
            }
            Subcommand::Publish {
                package,
                access_key,
                secret_key,
            } => {
                packages::publish(
                    &package,
                    &registry
                        .with_upload(access_key, secret_key)
                        .build()
                        .await
                        .wrap_err("can't download")?,
                )
                .await;
                Ok(())
            }
            Subcommand::Fetch { wasm_path } => {
                let cache_dir = litehouse_config::directories().map(|d| d.cache_dir().to_owned());
                let config = LitehouseConfig::load()?;

                let _pass = packages::fetch(
                    &config,
                    &registry
                        .with_download(wasm_path, cache_dir)
                        .build()
                        .await
                        .wrap_err("can't fetch")?,
                )
                .await;

                Ok(())
            }
            Subcommand::Build {
                wasm_path,
                package,
                debug,
                no_package,
                no_optimise,
            } => packages::build(&package, &wasm_path, debug, !no_optimise, no_package).await,
            Subcommand::Search { query } => {
                let prefix = query.map(|q| Import {
                    plugin: q,
                    registry: None,
                    version: None,
                    sha: None,
                });
                let registry = registry.build().await.wrap_err("can't search")?;
                let results = registry.list(prefix.as_ref()).await;
                for (import, _) in results {
                    println!("{}", import);
                }
                Ok(())
            }
            Subcommand::Lock { wasm_path } => {
                packages::lock(&wasm_path).await;
                Ok(())
            }
            Subcommand::Feedback { message: feedback } => {
                // create a message and send it using reqwest

                // get git email and name if possible using cmd
                let git_path = which::which("git").ok();
                let (email, name) = if let Some(git_path) = &git_path {
                    let email = std::process::Command::new(git_path)
                        .arg("config")
                        .arg("--get")
                        .arg("user.email")
                        .output()
                        .ok()
                        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string());
                    let name = std::process::Command::new(git_path)
                        .arg("config")
                        .arg("--get")
                        .arg("user.name")
                        .output()
                        .ok()
                        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string());
                    (email, name)
                } else {
                    (None, None)
                };

                let message = FeedbackMessage {
                    feedback: feedback.to_string(),
                    version: env!("CARGO_PKG_VERSION").to_string(),
                    email,
                    name,
                };

                tracing::debug!(?message, "sending feedback");

                let client = reqwest::Client::new();
                let res = client
                    .post("https://litehouse.arlyon.dev/api/feedback")
                    .json(&message)
                    .send()
                    .await
                    .into_diagnostic()
                    .wrap_err("unable to send feedback")?;

                if !res.status().is_success() {
                    let status = res.status();
                    let body = res.bytes().await;
                    tracing::error!(?body, "failed to send feedback");
                    return Err(miette::miette!("failed to send feedback ({})", status));
                }

                Ok(())
            }
            Subcommand::Auth { auth_command } => auth::do_auth(auth_command).await,
        }
    }
}
