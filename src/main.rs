#![warn(clippy::pedantic)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::missing_errors_doc)]

pub mod audio;
pub mod config;
pub mod log_watcher;
pub mod notifier;
pub mod parser;
pub mod vr;

use std::env::args;

use anyhow::Result;
use tokio::{signal, sync::broadcast};

use crate::{
    audio::start_audio,
    log_watcher::start_log_watcher,
    vr::{runtime::start_runtime, setup::setup_vr},
};

#[tokio::main]
async fn main() -> Result<()> {
    let arg = args().nth(1).unwrap_or_default();

    if arg == "install" {
        setup_vr().await?;

        println!("OK");
        return Ok(());
    }

    let (shutdown_send, mut shutdown_recv) = broadcast::channel(8);

    start_log_watcher(shutdown_send.clone()).await?;

    start_runtime(shutdown_send.clone()).await?;

    start_audio(shutdown_send.clone()).await?;

    tokio::select! {
        _ = signal::ctrl_c() => {
            println!("ctrl-c");
            let _ = shutdown_send.send(());
        },
        _ = shutdown_recv.recv() => {
            println!("main got shutdown");
        },
    }

    drop(shutdown_send);

    // wait for everyone to drop their senders
    while shutdown_recv.recv().await.is_ok() {
        println!("shutdown_recv");
    }

    println!("main end");

    Ok(())
}
