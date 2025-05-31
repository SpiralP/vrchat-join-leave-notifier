use std::collections::HashMap;

use serde::Serialize;

use super::APP_KEY;

#[derive(Debug, Serialize)]
pub struct VrManifest {
    source: VrManifestSource,
    applications: Vec<VrManifestApplication>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum VrManifestSource {
    Builtin,
}

#[derive(Debug, Serialize)]
pub struct VrManifestApplication {
    app_key: String,
    launch_type: VrManifestApplicationLaunchType,
    binary_path_windows: String,
    is_dashboard_overlay: bool,
    strings: HashMap<String, VrManifestApplicationString>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum VrManifestApplicationLaunchType {
    Binary,
}

#[derive(Debug, Serialize)]
pub struct VrManifestApplicationString {
    name: String,
    description: String,
}

impl VrManifest {
    #[must_use]
    pub fn new(binary_path_windows: &str) -> Self {
        let app_key = APP_KEY.into();
        let binary_path_windows = binary_path_windows.into();

        Self {
            source: VrManifestSource::Builtin,
            applications: vec![VrManifestApplication {
                app_key,
                binary_path_windows,
                is_dashboard_overlay: true,
                launch_type: VrManifestApplicationLaunchType::Binary,
                strings: HashMap::from([(
                    "en_us".into(),
                    VrManifestApplicationString {
                        name: "VRChat Join/Leave Notifier".into(),
                        description: "VRChat/Join Leave Notifier".into(),
                    },
                )]),
            }],
        }
    }
}
