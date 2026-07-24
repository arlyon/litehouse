use std::{
    collections::{BTreeMap, BTreeSet},
    net::SocketAddr,
    path::Path,
    sync::{Arc, Mutex},
    time::Duration,
};

use litehouse_config::{Capability, LitehouseConfig, PluginConfig};
use litehouse_contracts::{
    component::{ComponentKind, validate_component},
    config::environment_variable_name,
};
use litehouse_management::{
    BUNDLED_PROVIDERS, BundledProvider, ManagedProvider, ManagementService, McpAdapter, Principal,
    WorkflowExecutor, WorkflowRepository, WorkflowRuntime,
};
use litehouse_mcp_server::{McpAdapterFactory, McpServerConfig};
use litehouse_provider_host::{HttpOrigin, ProviderConfig, start_provider};
use litehouse_service_fabric::{Generation, InstanceId, ServiceFabric};
use miette::{IntoDiagnostic, Result, miette};
use serde_json::{Map, Value};
use wasmtime::component::Component;

use crate::runtime::set_up_engine;

const PROVIDER_STARTUP_TIMEOUT: Duration = Duration::from_secs(30);
const PROVIDER_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_COMPONENT_BYTES: u64 = 64 * 1024 * 1024;
const MAX_SECRET_BYTES: usize = 16 * 1024;

pub(crate) async fn run(
    component_path: &Path,
    cache: bool,
    bind_addr: SocketAddr,
    allowed_hosts: Vec<String>,
    allowed_origins: Vec<String>,
) -> Result<()> {
    let config = LitehouseConfig::load()?;
    validate_phase1_settings(&config)?;
    let workflows = Arc::new(
        WorkflowRepository::open_with_authoring_root(&config.workflows, ".")
            .map_err(|error| miette!("unable to load workflows: {error}"))?,
    );
    let loaded_workflows = workflows
        .snapshot()
        .map_err(|error| miette!("unable to capture workflow snapshot: {error}"))?;

    let (engine, _legacy_linker, _cache) = set_up_engine(cache).await?;
    let fabric = Arc::new(Mutex::new(ServiceFabric::default()));
    let mut components: BTreeMap<BundledProvider, Arc<Component>> = BTreeMap::new();
    let mut handles = Vec::new();
    let mut managed = Vec::new();

    for (instance_name, instance) in &config.plugins {
        let kind = bundled_provider(instance)?;
        let component = if let Some(component) = components.get(&kind) {
            component.clone()
        } else {
            let component = Arc::new(load_component(&engine, component_path, kind)?);
            components.insert(kind, component.clone());
            component
        };
        let (allowed_tcp_addresses, allowed_http_origins) =
            provider_capabilities(&instance.capabilities)?;
        let handle = start_provider(
            &engine,
            &component,
            ProviderConfig {
                instance: InstanceId::new(instance_name)
                    .map_err(|_| miette!("invalid provider instance name: {instance_name}"))?,
                generation: Generation::new(1)
                    .map_err(|_| miette!("invalid initial provider generation"))?,
                config_json: prepared_provider_config(kind, instance)?,
                allowed_tcp_addresses,
                allowed_http_origins,
                startup_timeout: PROVIDER_STARTUP_TIMEOUT,
            },
            fabric.clone(),
        )
        .await
        .map_err(|error| miette!("unable to start provider {instance_name}: {error}"))?;
        managed.push(ManagedProvider::new(kind, handle.clone()));
        handles.push(handle);
    }

    let management = ManagementService::from_bundled(managed)
        .map_err(|error| miette!("unable to build management service: {error}"))?;
    let principal = Principal::proving("phase1-local-mcp")
        .map_err(|_| miette!("unable to create the Phase 1 MCP principal"))?;
    let workflow_runtime = Arc::new(WorkflowRuntime::new());
    let mut workflow_executor = WorkflowExecutor::activate(
        management.clone(),
        principal.clone(),
        loaded_workflows,
        workflow_runtime.clone(),
    )
    .await
    .map_err(|error| miette!("unable to activate workflows: {error}"))?;
    let adapter = McpAdapter::new(management, principal)
        .with_workflows(workflows)
        .with_workflow_runtime(workflow_runtime);
    let factory: McpAdapterFactory = Arc::new(move || Ok(adapter.clone()));
    let mut server_config = McpServerConfig::new(bind_addr, allowed_origins);
    if !allowed_hosts.is_empty() {
        server_config.allowed_hosts = allowed_hosts;
    }
    let server = litehouse_mcp_server::start(server_config, factory)
        .await
        .map_err(|error| miette!("unable to start MCP server: {error}"))?;
    server.set_ready(true);
    tracing::info!(address = %server.local_addr(), "Litehouse Phase 1 daemon is ready");

    let run_result = tokio::select! {
        result = shutdown_signal() => result,
        result = workflow_executor.run() => {
            result.map_err(|error| miette!("workflow executor stopped: {error}"))
        }
    };
    server.set_ready(false);
    server
        .shutdown()
        .await
        .map_err(|error| miette!("unable to stop MCP server: {error}"))?;
    for handle in handles.iter().rev() {
        handle
            .shutdown(PROVIDER_SHUTDOWN_TIMEOUT)
            .await
            .map_err(|error| miette!("unable to stop provider: {error}"))?;
    }
    run_result
}

fn validate_phase1_settings(config: &LitehouseConfig) -> Result<()> {
    if !config.registries.is_empty()
        || !config.imports.is_empty()
        || !config.capabilities.is_empty()
        || config.broker.is_some()
    {
        return Err(miette!(
            "Phase 1 rejects registries, imports, global capabilities, and broker settings"
        ));
    }
    for instance in config.plugins.values() {
        bundled_provider(instance)?;
        provider_capabilities(&instance.capabilities)?;
    }
    Ok(())
}

fn bundled_provider(instance: &PluginConfig) -> Result<BundledProvider> {
    if instance.plugin.registry.is_some() || instance.plugin.sha.is_some() {
        return Err(miette!("Phase 1 providers must use bundled components"));
    }
    let kind = BUNDLED_PROVIDERS
        .iter()
        .copied()
        .find(|kind| kind.package() == instance.plugin.plugin)
        .ok_or_else(|| miette!("provider is not in the Phase 1 bundle"))?;
    if let Some(version) = &instance.plugin.version
        && version.to_string() != kind.version()
    {
        return Err(miette!(
            "configured provider version does not match the bundled version"
        ));
    }
    Ok(kind)
}

fn load_component(
    engine: &wasmtime::Engine,
    root: &Path,
    kind: BundledProvider,
) -> Result<Component> {
    let path = root.join(kind.component_file_name());
    let metadata = std::fs::metadata(&path)
        .into_diagnostic()
        .map_err(|error| miette!("unable to inspect {}: {error}", path.display()))?;
    if !metadata.is_file() || metadata.len() > MAX_COMPONENT_BYTES {
        return Err(miette!("invalid bundled component: {}", path.display()));
    }
    let bytes = std::fs::read(&path)
        .into_diagnostic()
        .map_err(|error| miette!("unable to read {}: {error}", path.display()))?;
    validate_component(&bytes, ComponentKind::Provider)
        .map_err(|error| miette!("invalid provider component {}: {error}", path.display()))?;
    Component::new(engine, bytes)
        .map_err(|error| miette!("unable to compile {}: {error}", path.display()))
}

fn provider_capabilities(
    capabilities: &[Capability],
) -> Result<(BTreeSet<SocketAddr>, BTreeSet<HttpOrigin>)> {
    let mut tcp = BTreeSet::new();
    let mut http = BTreeSet::new();
    for capability in capabilities {
        match capability {
            Capability::TcpClient(address) => {
                tcp.insert(*address);
            }
            Capability::HttpClient(value) => {
                let url = url::Url::parse(value)
                    .map_err(|_| miette!("HTTP client capability must be an exact origin"))?;
                if !url.username().is_empty()
                    || url.password().is_some()
                    || !matches!(url.path(), "" | "/")
                    || url.query().is_some()
                    || url.fragment().is_some()
                {
                    return Err(miette!("HTTP client capability must be an exact origin"));
                }
                let host = url
                    .host_str()
                    .ok_or_else(|| miette!("HTTP client capability requires a host"))?;
                let port = url
                    .port_or_known_default()
                    .ok_or_else(|| miette!("HTTP client capability requires an HTTP(S) port"))?;
                http.insert(
                    HttpOrigin::new(url.scheme(), host, port)
                        .map_err(|error| miette!("{error}"))?,
                );
            }
            Capability::HttpServer(_) => {
                return Err(miette!(
                    "provider instances cannot receive HTTP server capabilities"
                ));
            }
        }
    }
    Ok((tcp, http))
}

fn prepared_provider_config(kind: BundledProvider, instance: &PluginConfig) -> Result<String> {
    let mut config = match instance
        .config
        .clone()
        .unwrap_or_else(|| Value::Object(Map::new()))
    {
        Value::Object(config) => config,
        _ => return Err(miette!("provider configuration must be a JSON object")),
    };
    for (field, token) in protected_fields(kind) {
        if config.get(*field).is_some_and(|value| !value.is_null()) {
            return Err(miette!(
                "protected provider values must not be stored in settings.json"
            ));
        }
        let instance_id = instance
            .instance_id
            .as_deref()
            .ok_or_else(|| miette!("provider instance_id is required for protected values"))?;
        let variable = environment_variable_name(instance_id, token)
            .map_err(|error| miette!("invalid protected provider field: {error}"))?;
        let value = std::env::var(&variable).map_err(|_| {
            miette!("required protected environment variable is missing: {variable}")
        })?;
        if value.is_empty() || value.len() > MAX_SECRET_BYTES {
            return Err(miette!(
                "protected environment variable has an invalid length"
            ));
        }
        config.insert((*field).to_owned(), Value::String(value));
    }
    serde_json::to_string(&Value::Object(config))
        .into_diagnostic()
        .map_err(|error| miette!("unable to encode provider configuration: {error}"))
}

fn protected_fields(kind: BundledProvider) -> &'static [(&'static str, &'static str)] {
    match kind {
        BundledProvider::Esphome => &[("noise_key", "NOISE_KEY")],
        BundledProvider::Tasmota => &[("WEB_PASSWORD", "WEB_PASSWORD")],
        BundledProvider::Ntfy => &[("ACCESS_TOKEN", "ACCESS_TOKEN")],
        BundledProvider::OpenMeteo => &[],
    }
}

#[cfg(unix)]
async fn shutdown_signal() -> Result<()> {
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .into_diagnostic()?;
    tokio::select! {
        result = tokio::signal::ctrl_c() => result.into_diagnostic(),
        _ = terminate.recv() => Ok(()),
    }
}

#[cfg(not(unix))]
async fn shutdown_signal() -> Result<()> {
    tokio::signal::ctrl_c().await.into_diagnostic()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn bundle_accepts_exactly_the_four_providers() {
        for kind in BUNDLED_PROVIDERS {
            let instance = PluginConfig {
                plugin: litehouse_config::Import::from_str(&format!(
                    "{}@{}",
                    kind.package(),
                    kind.version()
                ))
                .unwrap(),
                instance_id: None,
                capabilities: Vec::new(),
                config: None,
            };
            assert_eq!(bundled_provider(&instance).unwrap(), kind);
        }
    }

    #[test]
    fn provider_http_authority_must_be_an_exact_origin() {
        assert!(
            provider_capabilities(&[Capability::HttpClient("https://api.open-meteo.com".into())])
                .is_ok()
        );
        assert!(
            provider_capabilities(&[Capability::HttpClient(
                "https://api.open-meteo.com/v1/forecast".into()
            )])
            .is_err()
        );
        assert!(provider_capabilities(&[Capability::HttpServer(8080)]).is_err());
    }
}
