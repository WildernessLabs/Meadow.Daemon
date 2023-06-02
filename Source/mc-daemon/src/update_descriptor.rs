use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateDescriptor {
    #[serde(rename = "MpakID")]
    pub mpak_id: String,
    #[serde(rename = "MpakDownloadUrl")]
    pub mpak_download_url: String,
    #[serde(rename = "TargetDevices")]
    pub target_devices: Option<String>,
    #[serde(rename = "PublishedOn")]
    pub published_on: String,
    #[serde(rename = "UpdateType")]
    pub update_type: i32,
    #[serde(rename = "Version")]
    pub version: String,
    #[serde(rename = "DownloadSize")]
    pub download_size: u32,
    #[serde(rename = "Summary")]
    pub summary: Option<String>,
    #[serde(rename = "Detail")]
    pub detail: Option<String>,
    #[serde(rename = "Retrieved")]
    pub retrieved: bool,
    #[serde(rename = "Applied")]
    pub applied: bool,
    #[serde(rename = "DownloadHash")]
    pub download_hash: String
}

impl UpdateDescriptor {
    pub fn from_json(json: &str) -> UpdateDescriptor {
        let ud: UpdateDescriptor = serde_json::from_str(json).unwrap();
        ud
    }
}