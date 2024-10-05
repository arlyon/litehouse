//! Litehouse: A home automation system using WebAssembly.
//!
//! This application serves as the core of the Litehouse home automation system, orchestrating
//! the execution of WebAssembly-based plugins for various home automation tasks.

#![feature(let_chains)]

use clap::Parser;
use cmd::Subcommand;
use miette::Result;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

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

    {
        #[cfg(feature = "console")]
        let console_layer = console_subscriber::spawn();

        let registry = tracing_subscriber::registry()
            .with(tracing_subscriber::fmt::layer().with_filter(EnvFilter::from_default_env()));

        #[cfg(feature = "console")]
        let registry = registry.with(console_layer);

        registry.init();
    }

    let opt = Opt::parse();

    if opt.version {
        println!("{}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    if let Some(command) = opt.command {
        return command.run().await;
    }

    Ok(())
}
