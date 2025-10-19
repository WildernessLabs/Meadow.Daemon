use serde::{Deserialize, Serialize};
use anyhow::{Context, Result};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UpdateDescriptor {
    #[serde(rename = "mpakId")]
    pub mpak_id: String,
    #[serde(rename = "mpakDownloadUrl")]
    pub mpak_download_url: String,
    #[serde(rename = "targetDevices")]
    pub target_devices: Option<Vec<String>>,
    #[serde(rename = "publishedOn")]
    pub published_on: String,
    #[serde(rename = "crc")]
    pub crc: String,
    #[serde(rename = "version")]
    pub version: Option<String>,
    #[serde(rename = "fileSize")]
    pub file_size: u32,
    #[serde(rename = "metadata")]
    pub metadata: Option<String>,
    #[serde(rename = "summary")]
    pub summary: Option<String>,
    #[serde(rename = "detail")]
    pub detail: Option<String>,
    #[serde(rename = "updateType")]
    pub update_type: Option<i32>,
    pub retrieved: Option<bool>,
    pub applied: Option<bool>,    
}

impl UpdateDescriptor {
    pub fn new(id: String) -> UpdateDescriptor {
        UpdateDescriptor { 
            mpak_id: id, 
            mpak_download_url: "http://foo.bar".to_string(),
            target_devices: None, 
            published_on: "1/1/1980".to_string(), 
            update_type: Some(1), 
            version: Some("0.999".to_string()), 
            file_size: 1234, 
            summary: None, 
            detail: None, 
            crc: "".to_string(),
            metadata: None,
            retrieved: None,
            applied: None
        }
    }

    pub fn from_json(json: &str) -> Result<UpdateDescriptor> {
        let ud: UpdateDescriptor = serde_json::from_str(json)
            .with_context(|| format!("Failed to parse UpdateDescriptor from JSON: {}",
                if json.len() > 100 { &json[..100] } else { json }))?;
        Ok(ud)
    }
}