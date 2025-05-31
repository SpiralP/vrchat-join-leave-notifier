use std::{env::current_exe, path::PathBuf};

use anyhow::{Context, Result};
use tokio::{
    fs::{self, File},
    io::AsyncWriteExt,
};

use super::utils::get_data_dir;
use crate::vr::manifest::VrManifest;

/// returns `manifest_path`
pub async fn setup_vr_files() -> Result<PathBuf> {
    let data_dir = get_data_dir().await?;
    let manifest_path = data_dir.join("manifest.vrmanifest");

    let exe_path = copy_exe_to_data_dir().await?;
    let exe_name = exe_path.file_name().context("file_name None")?;

    let manifest = VrManifest::new(&exe_name.to_string_lossy());
    let bytes = serde_json::to_vec_pretty(&manifest)?;

    {
        let mut f = File::create(&manifest_path).await?;
        f.write_all(&bytes).await?;
    }

    Ok(manifest_path)
}

async fn copy_exe_to_data_dir() -> Result<PathBuf> {
    let data_dir = get_data_dir().await?;

    let current_exe_path = current_exe()?;
    let exe_path = data_dir.join(current_exe_path.file_name().context("file_name None")?);
    fs::copy(&current_exe_path, &exe_path).await?;

    Ok(exe_path)
}
