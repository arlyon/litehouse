//! Litehouse: A home automation system using WebAssembly.
//!
//! This application serves as the core of the Litehouse home automation system, orchestrating
//! the execution of WebAssembly-based plugins for various home automation tasks.

use chrono::{DateTime, Utc};
use clap::Parser;
use cmd::Subcommand;
use miette::Result;
use tracing_subscriber::{EnvFilter, Layer, layer::SubscriberExt, util::SubscriberInitExt};

mod cache;
mod cmd;
mod runtime;
mod server;
mod store;
mod util;

/// CLI options for litehouse
#[derive(clap::Parser)]
#[command(arg_required_else_help(true))]
struct Opt {
    #[command(subcommand)]
    command: Option<Subcommand>,
    /// Print the current version of litehouse and exit
    #[clap(long)]
    version: bool,
}

#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

fn main() -> Result<()> {
    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();
    main_inner()
}

#[tokio::main]
async fn main_inner() -> Result<()> {
    miette::set_hook(Box::new(|_| {
        Box::new(
            miette::MietteHandlerOpts::new()
                .terminal_links(true)
                .context_lines(2)
                .tab_width(2)
                .build(),
        )
    }))
    .unwrap();

    let (logs_tx, logs_rx) = tokio::sync::broadcast::channel(1000);

    {
        #[cfg(feature = "console")]
        let console_layer = console_subscriber::spawn();

        let registry = tracing_subscriber::registry()
            .with(tracing_subscriber::fmt::layer().with_filter(EnvFilter::from_default_env()));

        #[cfg(feature = "console")]
        let registry = registry.with(console_layer);

        let registry = registry
            .with(QueueLayer { sender: logs_tx }.with_filter(EnvFilter::from_default_env()));

        registry.init();
    }

    let opt = Opt::parse();

    if opt.version {
        println!("{}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    if let Some(command) = opt.command {
        return command.run(logs_rx).await;
    }

    Ok(())
}

#[derive(Clone, serde::Serialize)]
pub struct LogMessage {
    source: String,
    level: String,
    message: String,
    timestamp: DateTime<Utc>,
}

impl LogMessage {
    fn new(level: tracing::Level, source: String, message: String) -> Self {
        Self {
            level: level.to_string(),
            source,
            message,
            timestamp: chrono::Utc::now(),
        }
    }
}

use tracing::{Event, Metadata, Subscriber, field::Visit};
use tracing_subscriber::registry::LookupSpan;

struct QueueLayer {
    sender: tokio::sync::broadcast::Sender<LogMessage>,
}

impl<S> Layer<S> for QueueLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn enabled(&self, _: &Metadata<'_>, _: tracing_subscriber::layer::Context<'_, S>) -> bool {
        true
    }

    fn on_event(&self, event: &Event<'_>, _: tracing_subscriber::layer::Context<'_, S>) {
        let mut string_visitor = MessageVisitor(None);
        event.record(&mut string_visitor);

        if let Some(message) = string_visitor.0 {
            let log_message = LogMessage::new(
                *event.metadata().level(),
                event.metadata().target().to_string(),
                message,
            );
            let _ = self.sender.send(log_message);
        }
    }
}

struct MessageVisitor(Option<String>);

impl Visit for MessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.0 = Some(format!("{:?}", value));
        }
    }
}
