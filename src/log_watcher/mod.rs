#![allow(clippy::assigning_clones)]

use std::time::Duration;

use anyhow::Result;
use tokio::{sync::broadcast, task::JoinHandle, time::MissedTickBehavior};

use crate::parser::{utils::get_newest_log_path, LogParser};

pub struct LogWatcher {}

impl LogWatcher {
    #[must_use]
    pub fn new() -> Self {
        Self {}
    }

    pub async fn run_loop(self) -> Result<()> {
        let mut newest_log_path = None;
        let mut watch_handle: Option<JoinHandle<()>> = None;

        let mut interval = tokio::time::interval(Duration::from_secs(10));
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            interval.tick().await;

            let current_newest_log_path = get_newest_log_path().await.unwrap();

            if newest_log_path != current_newest_log_path {
                println!("new {current_newest_log_path:?}");
                newest_log_path = current_newest_log_path.clone();

                if let Some(old_handle) = watch_handle.take() {
                    old_handle.abort();
                }
                if let Some(log_path) = current_newest_log_path {
                    watch_handle = Some(tokio::spawn(async move {
                        let f = {
                            let log_path = log_path.clone();

                            async move {
                                let parser = LogParser::new(&log_path)?;
                                parser.read_loop().await?;
                                anyhow::Ok(())
                            }
                        };

                        if let Err(e) = f.await {
                            eprintln!("watch_handle: {log_path:?} {e:?}");
                        }
                    }));
                }
            }
        }
    }
}

impl Default for LogWatcher {
    fn default() -> Self {
        Self::new()
    }
}

pub async fn start_log_watcher(shutdown_send: broadcast::Sender<()>) -> Result<()> {
    let mut shutdown_recv = shutdown_send.subscribe();
    let watcher = LogWatcher::new();

    tokio::spawn(async move {
        tokio::select! {
            _ = shutdown_recv.recv() => {
                println!("start_log_watcher got shutdown");
            },
            result = watcher.run_loop() => {
                if let Err(e) = result {
                    eprintln!("start_log_watcher: {e:?}");
                }
            }
        }

        println!("start_log_watcher end");
        let _ = shutdown_send.send(());
    });

    Ok(())
}
