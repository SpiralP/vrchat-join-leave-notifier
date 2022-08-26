use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tokio::fs;

pub async fn get_config_dir() -> Result<PathBuf> {
    let config_dir =
        create_dir_if_missing(dirs::config_dir().context("dirs::config_dir() None")?).await?;
    let config_dir = create_dir_if_missing(config_dir.join(env!("CARGO_PKG_NAME"))).await?;
    Ok(config_dir)
}

pub async fn get_data_dir() -> Result<PathBuf> {
    let data_dir =
        create_dir_if_missing(dirs::data_dir().context("dirs::data_dir() None")?).await?;
    let data_dir = create_dir_if_missing(data_dir.join(env!("CARGO_PKG_NAME"))).await?;
    Ok(data_dir)
}

pub async fn create_dir_if_missing<P: AsRef<Path>>(dir: P) -> Result<PathBuf> {
    let dir = dir.as_ref();

    if !dir.is_dir() {
        println!("creating new directory {:?}", dir);
        fs::create_dir(dir)
            .await
            .with_context(|| format!("creating {:?}", dir))?;
    }

    Ok(dir.to_path_buf())
}
