use std::{fs, path::Path, process::Command};

pub struct Crypto;

impl Crypto {
    pub fn get_private_key_pem() -> String {
        
        // for now, we'll hard-code to using the key from 
        let key_path = "/home/ctacke/.ssh";
        let priv_key_file = "id_rsa";

        let private_key_path = Path::new(&key_path).join(priv_key_file);
        if !private_key_path.is_file() {
            return "[No Key Found]".to_string();
        }
        
        // read the key
        let mut pk_data =std::fs::read_to_string(&private_key_path)
            .expect("Unable to open private key file");

        // if it's not a PEM, get the key in PEM format

        if !pk_data.starts_with("-----BEGIN RSA PRIVATE KEY-----") {
            println!("Private key is not in PEM format. Making a backup and converting...");
            // make a backup of the key file
            let backup_path = Path::new(&key_path).join("id_rsa.bak");
            fs::copy(private_key_path.clone(), backup_path).unwrap();

            let output = Command::new("ssh-keygen")
                .arg("-p")
                .arg("-m")
                .arg("pem")
                .arg("-N")
                .arg("")
                .arg("-f")
                .arg(private_key_path.clone()
                    .into_os_string()
                    .into_string()
                    .unwrap())
                .output()
                .expect("failed to execute ssh-keygen");
            
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            println!("STDOUT> {}", stdout);
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            println!("STDERR> {}", stderr);

            pk_data =std::fs::read_to_string(&private_key_path)
                .expect("Unable to open private key file");
        }
        
        pk_data
    }

    pub fn get_public_key_pem() -> String {
        
        // for now, we'll hard-code to using the key from 
        let key_path = "/home/ctacke/.ssh";
        let pub_key_file = "id_rsa.pub";

        let pub_key_path = Path::new(&key_path).join(pub_key_file);
        if !pub_key_path.is_file() {
            return "[No Key Found]".to_string();
        }
        
        // read the key
        let mut pk_data =std::fs::read_to_string(&pub_key_path)
            .expect("Unable to open public key file");

        // if it's not a PEM, convert the key to PEM format (this has to be done in-place)        
        if !pk_data.starts_with("-----BEGIN RSA PUBLIC KEY-----") {
            // make a backup of the key file
            let backup_path = Path::new(&key_path).join("id_rsa.pub.bak");
            fs::copy(pub_key_path.clone(), backup_path).unwrap();

            let output = Command::new("ssh-keygen")
                .arg("-e")
                .arg("-m")
                .arg("pem")
                .arg("-N")
                .arg("''")
                .arg("-f")
                .arg(pub_key_path
                    .into_os_string()
                    .into_string()
                    .unwrap())
                .output()
                .expect("failed to execute ssh-keygen");

            //let err = String::from_utf8_lossy(&output.stderr).to_string();
            pk_data = String::from_utf8_lossy(&output.stdout).to_string();
        }
        
        pk_data
    }
}