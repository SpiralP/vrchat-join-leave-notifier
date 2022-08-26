use std::{
    path::{Path, PathBuf},
    time::SystemTime,
};

use anyhow::{bail, Context, Result};
use tokio::fs;

async fn get_vrchat_dir() -> Result<PathBuf> {
    let vrchat_dir = {
        #[cfg(target_os = "windows")]
        {
            let mut cache_dir = dirs::cache_dir().context("cache_dir None")?;
            if !cache_dir.pop() {
                bail!("cache_dir.pop() false");
            }
            cache_dir.join("LocalLow\\VRChat\\VRChat")
        }

        #[cfg(target_os = "linux")]
        {
            let home_dir = dirs::home_dir().context("home_dir None")?;
            home_dir
                .join(
                    ".steam/steam/steamapps/compatdata/438100/pfx/drive_c/users/steamuser/AppData",
                )
                .join("LocalLow/VRChat/VRChat")
        }
    };

    if !fs::metadata(&vrchat_dir)
        .await
        .map(|metadata| metadata.is_dir())
        .unwrap_or(false)
    {
        bail!("vrchat dir doesn't exist");
    }

    Ok(vrchat_dir)
}

pub const LOG_FILE_PREFIX: &str = "output_log_";
pub const LOG_FILE_SUFFIX: &str = ".txt";

async fn is_log_file(path: &Path) -> bool {
    if !fs::metadata(path)
        .await
        .map(|metadata| metadata.is_file())
        .unwrap_or(false)
    {
        return false;
    }

    if let Some(file_name) = path.file_name() {
        let file_name = file_name.to_string_lossy();
        file_name.starts_with(LOG_FILE_PREFIX) && file_name.ends_with(LOG_FILE_SUFFIX)
    } else {
        false
    }
}

async fn get_log_paths_with_modified() -> Result<Vec<(PathBuf, SystemTime)>> {
    let vrchat_dir = get_vrchat_dir().await?;

    let mut log_paths_with_modified = Vec::new();

    let mut entries = fs::read_dir(vrchat_dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if !is_log_file(&path).await {
            continue;
        }

        if let Ok(modified) = entry
            .metadata()
            .await
            .and_then(|metadata| metadata.modified())
        {
            log_paths_with_modified.push((path, modified));
        }
    }

    Ok(log_paths_with_modified)
}

pub async fn get_newest_log_path() -> Result<Option<PathBuf>> {
    let mut paths = get_log_paths_with_modified().await?;
    paths.sort_by(|(_, a), (_, b)| b.cmp(a));
    let path = paths.into_iter().map(|(path, _)| path).next();

    Ok(path)
}
