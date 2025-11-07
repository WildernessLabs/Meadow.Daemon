use std::{fs, path::{Path, PathBuf}, process::Command};
use anyhow::{Context, Result};

pub struct Crypto;

impl Crypto {
    /// Get the private key in PEM format
    ///
    /// # Arguments
    /// * `key_path` - Optional path to the private key file. If None, uses ~/.ssh/id_rsa
    pub fn get_private_key_pem(key_path: Option<&Path>) -> Result<String> {
        // Determine the key path to use
        let private_key_path = if let Some(path) = key_path {
            path.to_path_buf()
        } else {
            // Default to current user's .ssh/id_rsa
            if let Ok(home) = std::env::var("HOME") {
                PathBuf::from(home).join(".ssh").join("id_rsa")
            } else {
                PathBuf::from("/root/.ssh/id_rsa")
            }
        };
        if !private_key_path.is_file() {
            anyhow::bail!("Private key file not found: {:?}", private_key_path);
        }

        // read the key
        let mut pk_data = std::fs::read_to_string(&private_key_path)
            .with_context(|| format!("Failed to read private key: {:?}", private_key_path))?;

        // if it's not a PEM, get the key in PEM format
        if !pk_data.starts_with("-----BEGIN RSA PRIVATE KEY-----") {
            println!("Private key is not in PEM format. Making a backup and converting...");
            // make a backup of the key file
            let backup_path = private_key_path.with_extension("rsa.bak");
            fs::copy(&private_key_path, &backup_path)
                .with_context(|| format!("Failed to backup private key to {:?}", backup_path))?;

            let key_path_str = private_key_path
                .to_str()
                .context("Private key path contains invalid UTF-8")?
                .to_string();

            let output = Command::new("ssh-keygen")
                .arg("-p")
                .arg("-m")
                .arg("pem")
                .arg("-N")
                .arg("")
                .arg("-f")
                .arg(&key_path_str)
                .output()
                .context("Failed to execute ssh-keygen for private key conversion")?;

            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            println!("STDOUT> {}", stdout);
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            println!("STDERR> {}", stderr);

            pk_data = std::fs::read_to_string(&private_key_path)
                .with_context(|| format!("Failed to read converted private key: {:?}", private_key_path))?;
        }

        Ok(pk_data)
    }

    /// Get the public key in PEM format
    ///
    /// # Arguments
    /// * `key_path` - Optional path to the private key file. The public key is derived by appending .pub
    ///                If None, uses ~/.ssh/id_rsa.pub
    pub fn get_public_key_pem(key_path: Option<&Path>) -> Result<String> {
        // Determine the public key path to use
        let pub_key_path = if let Some(path) = key_path {
            // Append .pub to the full private key path
            let mut pub_path = path.as_os_str().to_owned();
            pub_path.push(".pub");
            PathBuf::from(pub_path)
        } else {
            // Default to current user's .ssh/id_rsa.pub
            if let Ok(home) = std::env::var("HOME") {
                PathBuf::from(home).join(".ssh").join("id_rsa.pub")
            } else {
                PathBuf::from("/root/.ssh/id_rsa.pub")
            }
        };
        if !pub_key_path.is_file() {
            anyhow::bail!("Public key file not found: {:?}", pub_key_path);
        }

        // read the key
        let mut pk_data = std::fs::read_to_string(&pub_key_path)
            .with_context(|| format!("Failed to read public key: {:?}", pub_key_path))?;

        // if it's not a PEM, convert the key to PEM format (this has to be done in-place)
        if !pk_data.starts_with("-----BEGIN RSA PUBLIC KEY-----") {
            // make a backup of the key file
            let mut backup_path = pub_key_path.as_os_str().to_owned();
            backup_path.push(".bak");
            let backup_path = PathBuf::from(backup_path);
            fs::copy(&pub_key_path, &backup_path)
                .with_context(|| format!("Failed to backup public key to {:?}", backup_path))?;

            let key_path_str = pub_key_path
                .to_str()
                .context("Public key path contains invalid UTF-8")?
                .to_string();

            let output = Command::new("ssh-keygen")
                .arg("-e")
                .arg("-m")
                .arg("pem")
                .arg("-N")
                .arg("''")
                .arg("-f")
                .arg(&key_path_str)
                .output()
                .context("Failed to execute ssh-keygen for public key conversion")?;

            //let err = String::from_utf8_lossy(&output.stderr).to_string();
            pk_data = String::from_utf8_lossy(&output.stdout).to_string();
        }

        Ok(pk_data)
    }
}