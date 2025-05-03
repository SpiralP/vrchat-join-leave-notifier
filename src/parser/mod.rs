pub mod utils;

use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{Context, Result};
use tokio::{
    fs::File,
    io::{AsyncBufReadExt, AsyncSeekExt, BufReader},
    time::MissedTickBehavior,
};

use crate::notifier::{debounced_notify, MessageEvent};

const ENTERING_WORLD_LOG: &str = "[Behaviour] Entering world";
const FINISHED_ENTERING_WORLD_LOG: &str = "[Behaviour] Finished entering world.";
const ON_LEFT_ROOM_LOG: &str = "[Behaviour] OnLeftRoom";
// TODO can we know if the game is closed if not getting this? yes we can! openvr close event!
const APPLICATION_QUIT_LOG_PREFIX: &str = "VRCApplication: OnApplicationQuit at ";

// [Behaviour] OnPlayerJoined SpiralP (usr_...)
const PLAYER_JOINED_LOG_PREFIX: &str = "[Behaviour] OnPlayerJoined ";
// [Behaviour] OnPlayerJoinComplete SpiralP
const PLAYER_JOIN_COMPLETE_LOG_PREFIX: &str = "[Behaviour] OnPlayerJoinComplete ";

// [Behaviour] OnPlayerLeft SpiralP (usr_...)
const PLAYER_LEFT_LOG_PREFIX: &str = "[Behaviour] OnPlayerLeft ";
const UNREGISTERING_LOG_PREFIX: &str = "[Behaviour] Unregistering ";

pub struct LogParser {
    log_path: PathBuf,
}

impl LogParser {
    pub fn new(log_path: &Path) -> Result<Self> {
        Ok(Self {
            log_path: log_path.to_owned(),
        })
    }

    pub async fn read_loop(mut self) -> Result<()> {
        let mut f = File::open(&self.log_path).await?;
        f.seek(std::io::SeekFrom::End(0)).await?;
        let f = BufReader::new(f);
        let mut lines = f.lines();

        let mut interval = tokio::time::interval(Duration::from_secs(1));
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            interval.tick().await;

            while let Some(line) = lines.next_line().await? {
                if line.is_empty() {
                    continue;
                }

                if let Err(_e) = self.handle_line(&line).await {
                    // eprintln!("{e:?}: {line:?}");
                }
            }
        }
    }

    async fn handle_line(&mut self, line: &str) -> Result<()> {
        // 2022.07.27 15:26:35 Log        -  [Behaviour] OnPlayerJoined SpiralP
        let mut parts = line.splitn(2, '-');
        let _info = parts.next().context("no first part")?;
        let message = parts.next().context("no second part")?.trim();

        if message == ENTERING_WORLD_LOG {
            self.handle_entering_world().await?;
            self.handle_world_state_change().await?;
        } else if message == FINISHED_ENTERING_WORLD_LOG {
            self.handle_world_state_change().await?;
        } else if message == ON_LEFT_ROOM_LOG {
            self.handle_left_room().await?;
            self.handle_world_state_change().await?;
        } else if message.starts_with(APPLICATION_QUIT_LOG_PREFIX) {
            self.handle_world_state_change().await?;
        } else if let Some(name_and_uid) = message.strip_prefix(PLAYER_JOINED_LOG_PREFIX) {
            let name = name_and_uid
                .split_once(" (usr_")
                .map(|(name, _)| name)
                .unwrap_or(name_and_uid);
            self.handle_player_join(name).await?;
        } else if let Some(name) = message.strip_prefix(PLAYER_JOIN_COMPLETE_LOG_PREFIX) {
            self.handle_player_join(name).await?;
        } else if let Some(name_and_uid) = message.strip_prefix(PLAYER_LEFT_LOG_PREFIX) {
            let name = name_and_uid
                .split_once(" (usr_")
                .map(|(name, _)| name)
                .unwrap_or(name_and_uid);
            self.handle_player_leave(name).await?;
        } else if let Some(name) = message.strip_prefix(UNREGISTERING_LOG_PREFIX) {
            self.handle_player_leave(name).await?;
        }

        Ok(())
    }

    async fn handle_entering_world(&mut self) -> Result<()> {
        println!("handle_entering_world");
        Ok(())
    }

    async fn handle_left_room(&mut self) -> Result<()> {
        println!("handle_left_room");
        Ok(())
    }

    async fn handle_world_state_change(&mut self) -> Result<()> {
        println!("handle_world_state_change");
        Ok(())
    }

    async fn handle_player_join(&mut self, name: &str) -> Result<()> {
        debounced_notify(MessageEvent::Join(name.to_owned())).await?;
        Ok(())
    }

    async fn handle_player_leave(&mut self, name: &str) -> Result<()> {
        debounced_notify(MessageEvent::Leave(name.to_owned())).await?;
        Ok(())
    }
}
