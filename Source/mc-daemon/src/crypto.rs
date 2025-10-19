use std::{fs, path::Path, process::Command};
use anyhow::{Context, Result};

pub struct Crypto;

impl Crypto {
    pub fn get_private_key_pem() -> Result<String> {
        
        // for now, we'll hard-code to using the key from 
        let key_path = "/home/ctacke/.ssh";
        let priv_key_file = "id_rsa";

        let private_key_path = Path::new(&key_path).join(priv_key_file);
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
            let backup_path = Path::new(&key_path).join("id_rsa.bak");
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

    pub fn get_public_key_pem() -> Result<String> {

        // for now, we'll hard-code to using the key from
        let key_path = "/home/ctacke/.ssh";
        let pub_key_file = "id_rsa.pub";

        let pub_key_path = Path::new(&key_path).join(pub_key_file);
        if !pub_key_path.is_file() {
            anyhow::bail!("Public key file not found: {:?}", pub_key_path);
        }

        // read the key
        let mut pk_data = std::fs::read_to_string(&pub_key_path)
            .with_context(|| format!("Failed to read public key: {:?}", pub_key_path))?;

        // if it's not a PEM, convert the key to PEM format (this has to be done in-place)
        if !pk_data.starts_with("-----BEGIN RSA PUBLIC KEY-----") {
            // make a backup of the key file
            let backup_path = Path::new(&key_path).join("id_rsa.pub.bak");
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