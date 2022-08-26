use anyhow::{ensure, Context, Result};

use crate::{config::setup_vr::setup_vr_files, vr::APP_KEY};

pub async fn setup_vr() -> Result<()> {
    let manifest_path = setup_vr_files().await?;

    let context = unsafe { openvr::init(openvr::ApplicationType::Utility)? };
    let applications = context.applications()?;

    applications
        .add_application_manifest(&manifest_path, false)
        .context("add_application_manifest")?;

    ensure!(
        applications.is_application_installed(APP_KEY),
        "didn't install"
    );

    // println!("{:#?}", context.get_runtime_path());
    // println!("{:#?}", applications.get_application_count());

    // for i in 0..applications.get_application_count() {
    //     let key = applications.get_application_key_by_index(i).unwrap();
    //     let is_application_installed = applications.is_application_installed(&key);

    //     let auto_launch = applications.get_application_auto_launch(&key);
    //     println!("{key:?} {is_application_installed} {auto_launch}");
    // }

    applications
        .set_application_auto_launch(APP_KEY, true)
        .context("set_application_auto_launch")?;

    Ok(())
}
