use std::fs::read_to_string;
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};

#[derive(Clone)]
pub struct CloudSettings {
    pub enabled: bool,
    pub meadow_root: PathBuf,
    pub rest_api_bind_address: String,
    pub update_server_address: String,
    pub update_server_port: i32,
    pub use_authentication: bool,
    pub auth_server_address: Option<String>,
    pub auth_server_port: Option<i32>,
    pub mqtt_topics: Vec<String>,
    pub connect_retry_seconds: u64,
    pub update_apply_timeout_seconds: u64,
    pub auth_max_retries: u32,
    pub ssh_key_path: PathBuf,
}

impl CloudSettings {
    fn get_default_ssh_key_path() -> PathBuf {
        // Try to get the current user's home directory
        if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home).join(".ssh").join("id_rsa")
        } else {
            // Fallback to a common default if HOME is not set
            PathBuf::from("/root/.ssh/id_rsa")
        }
    }

    pub fn default() -> CloudSettings {
        CloudSettings{
            enabled: true,
            meadow_root: PathBuf::from("/opt/meadow"),
            rest_api_bind_address: "127.0.0.1".to_string(),  // Localhost only for security
            update_server_address: "".to_string(),
            update_server_port: 883,
            use_authentication: true,
            auth_server_address: Some("https://www.meadowcloud.co".to_string()),
            auth_server_port: None,
            mqtt_topics: vec!["{OID}/ota/{ID}".to_string()],
            connect_retry_seconds: 15,
            update_apply_timeout_seconds: 300,  // 5 minutes
            auth_max_retries: 10,  // Max 10 authentication attempts before failing
            ssh_key_path: Self::get_default_ssh_key_path(),
        }
    }

    pub fn from_file(path: &str) -> CloudSettings {
        match Self::try_from_file(path) {
            Ok(settings) => settings,
            Err(e) => {
                println!("ERROR loading config from '{}': {}", path, e);
                println!("Using default settings");
                let mut settings = Self::default();
                Self::apply_env_overrides(&mut settings);
                settings
            }
        }
    }

    fn try_from_file(path: &str) -> Result<CloudSettings> {
        // set up defaults
        let mut settings = CloudSettings::default();

        if !Path::new(path).exists() {
            println!("WARNING: Config file '{}' does not exist", path);
            // Still apply environment variable overrides even if config file doesn't exist
            Self::apply_env_overrides(&mut settings);
            return Ok(settings);
        }

        let lines = Self::read_lines(path)
            .with_context(|| format!("Failed to read config file: {}", path))?;

        for line in lines {

            let s = line
                .chars()
                .take_while(|&ch| ch != '#')
                .collect::<String>();

            if s.len() > 0 {
                // Find the space separator
                let space_pos = match s.find(' ') {
                    Some(pos) => pos,
                    None => {
                        println!("WARNING: Skipping malformed config line (no space): '{}'", s);
                        continue;
                    }
                };

                let key = &s[..space_pos].to_lowercase();
                let val = &s[space_pos..].trim().to_string();

                match key.as_str() {
                    "enabled" =>
                    {
                        settings.enabled = val.to_lowercase() == "yes";
                    },
                    "meadow_root" =>
                    {
                        settings.meadow_root = PathBuf::from(val);
                    },
                    "rest_api_bind_address" =>
                    {
                        settings.rest_api_bind_address = val.into();
                    },
                    "update_server_address" =>
                    {
                        settings.update_server_address = val.into();
                    },
                    "update_server_port" =>
                    {
                        settings.update_server_port = val.parse::<i32>()
                            .unwrap_or_else(|e| {
                                println!("WARNING: Invalid port '{}': {}. Using default.", val, e);
                                CloudSettings::default().update_server_port
                            });
                    },
                    "use_authentication" =>
                    {
                        settings.use_authentication = val.to_lowercase() == "yes";
                    },
                    "auth_server_address" =>
                    {
                        settings.auth_server_address = Some(val.into());
                    },
                    "auth_server_port" =>
                    {
                        settings.auth_server_port = Some(val.parse::<i32>()
                            .unwrap_or_else(|e| {
                                println!("WARNING: Invalid auth port '{}': {}. Using default.", val, e);
                                443
                            }));
                    },
                    "mqtt_topics" =>
                    {
                        settings.mqtt_topics = val.split(';').map(String::from).collect();
                    },
                    "connect_retry_seconds" =>
                    {
                        settings.connect_retry_seconds = val.parse::<u64>()
                            .unwrap_or_else(|e| {
                                println!("WARNING: Invalid retry seconds '{}': {}. Using default.", val, e);
                                CloudSettings::default().connect_retry_seconds
                            });
                    },
                    "update_apply_timeout_seconds" =>
                    {
                        settings.update_apply_timeout_seconds = val.parse::<u64>()
                            .unwrap_or_else(|e| {
                                println!("WARNING: Invalid timeout '{}': {}. Using default.", val, e);
                                CloudSettings::default().update_apply_timeout_seconds
                            });
                    },
                    "auth_max_retries" =>
                    {
                        settings.auth_max_retries = val.parse::<u32>()
                            .unwrap_or_else(|e| {
                                println!("WARNING: Invalid auth_max_retries '{}': {}. Using default.", val, e);
                                CloudSettings::default().auth_max_retries
                            });
                    },
                    "ssh_key_path" =>
                    {
                        settings.ssh_key_path = PathBuf::from(val);
                    },
                    _ =>
                    {
                        println!("WARNING: unknown setting '{}'", s);
                        // unknown setting
                    }

                }
            }
        }

        // Apply environment variable overrides
        Self::apply_env_overrides(&mut settings);

        Ok(settings)
    }

    fn apply_env_overrides(settings: &mut CloudSettings) {
        // Check for MEADOW_ROOT environment variable
        if let Ok(meadow_root) = std::env::var("MEADOW_ROOT") {
            println!("Using MEADOW_ROOT from environment: {}", meadow_root);
            settings.meadow_root = PathBuf::from(meadow_root);
        }
    }

    fn read_lines(filename: &str) -> Result<Vec<String>> {
        let contents = read_to_string(filename)
            .with_context(|| format!("Failed to read file: {}", filename))?;

        Ok(contents
            .lines()
            .map(String::from)
            .collect())
    }
}