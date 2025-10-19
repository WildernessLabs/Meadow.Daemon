use std::{error::Error, thread::{sleep, self}, sync::{Mutex, Arc, mpsc::{self, Sender, Receiver}}, fs};
use serde_json::{json, Value};
use serde::{Deserialize, Serialize};
use tokio::time;
use reqwest::Client;
use base64::engine::general_purpose;
use base64::Engine;
use rsa::{RsaPrivateKey, pkcs1::DecodeRsaPrivateKey};
use std::time::Duration;
use cbc::cipher::{KeyIvInit, BlockDecryptMut, generic_array::GenericArray, typenum::U16};

use crate::{cloud_settings::CloudSettings, cloud_subscriber::CloudSubscriber, update_store::UpdateStore, update_descriptor::UpdateDescriptor, crypto::Crypto};

type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;

#[derive(Serialize, Deserialize)]
struct CloudLoginResponse {
    #[serde(rename = "encryptedKey")]
    pub encrypted_key: String,
    #[serde(rename = "encryptedToken")]
    pub encrypted_token: String,
    pub iv: String
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateState {
    Dead,
    Disconnected,
    Authenticating,
    Authenticated,
    Connecting,
    Connected,
    Idle,
    UpdateAvailable,
    DownloadingFile,
    UpdateInProgress
}

pub struct UpdateService {
    settings: CloudSettings, 
    machine_id: String,
    state: UpdateState,
    store: Arc<Mutex<UpdateStore>>,
    update_sender: Sender<UpdateDescriptor>,
    update_receiver: Receiver<UpdateDescriptor>,
    state_sender: Sender<UpdateState>,
    state_receiver: Receiver<UpdateState>,
    jwt: String,
    oid: String,
    auth_fail_count: u32
}

impl UpdateService {

    pub fn new(settings: CloudSettings, machine_id: String, store: Arc<Mutex<UpdateStore>>) -> UpdateService {
        
        let (update_sender, update_receiver) = mpsc::channel();
        let (state_sender, state_receiver) = mpsc::channel();

        UpdateService {
            settings: settings.clone(), 
            machine_id: machine_id, 
            state: UpdateState::Dead, 
            store,
            update_sender,
            update_receiver,
            state_sender,
            state_receiver,
            jwt: String::new(),
            oid: String::new(),
            auth_fail_count: 0
        }
    }

    fn _extract_oid_from_jwt(&self, jwt: String) -> Result<String, Box<dyn Error>> {
        // Split the JWT by '.'
        let parts: Vec<&str> = jwt.split('.').collect();
        
        // Check if the JWT has at least 3 parts
        if parts.len() < 3 {
            return Err("invalid jwt segment length".into());
        }
        
        // Extract the second part (payload)
        let payload = parts[1];
        
        // Decode the base64 payload
        let decoded = general_purpose::STANDARD.decode(payload)?;
        
        // Convert the decoded bytes to a String
        let decoded_str = String::from_utf8(decoded)?;
        
        // Parse the string as JSON
        let json_value: Value = serde_json::from_str(&decoded_str)?;
        
        // Extract the "oid" field
        if let Some(oid) = json_value.get("oid") {
            if let Some(oid_str) = oid.as_str() {
                return Ok(oid_str.to_string());
            }
        }

        Err("oid not found".into())
    }

    fn _remove_pkcs7_padding(&self, mut data: Vec<u8>) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        if let Some(&padding_byte) = data.last() {
            let padding_length = padding_byte as usize;
    
            if padding_length == 0 || padding_length > data.len() {
                return Err("Invalid PKCS7 padding".into());
            }
    
            if data[data.len() - padding_length..].iter().all(|&b| b == padding_byte) {
                data.truncate(data.len() - padding_length);
                Ok(data)
            } else {
                Err("Invalid PKCS7 padding".into())
            }
        } else {
            Err("Failed to remove padding: Data is empty".into())
        }
    }
    
    //#[tokio::main] // this doesn't make it 'main' it just makes it synchonous (thanks for clarity, tokio!)
    async fn _authenticate(&mut self) -> bool {
        // connect to the cloud and get a JWT
        let device_id = match fs::read_to_string("/var/lib/dbus/machine-id") {
            Ok(id) => id.trim().to_ascii_uppercase(),
            Err(e) => {
                eprintln!("ERROR: Failed to read machine-id: {}", e);
                return false;
            }
        };

        let client = Client::new();
        let endpoint = format!("{}/api/devices/login", self.settings.auth_server_address
            .clone()
            .unwrap_or("https://www.meadowcloud.co".to_string()));
        let content = json!({
            "id": device_id
        });

        println!("Log in at {}", endpoint);

        match client.post(endpoint)
            .header("Content-Type", "application/json")
            .json(&content)
            .send()
            .await {
                Ok(response) => {

                    match response.status() {
                        reqwest::StatusCode::NOT_FOUND => {
                            eprintln!("ID not found.  This device needs to be (re)provisioned.");
                            // TODO: return a state that says "failed and don't retry"
                            return false;
                        }
                        reqwest::StatusCode::OK => {
                            let json = match response.text().await {
                                Ok(text) => text,
                                Err(e) => {
                                    eprintln!("ERROR: Failed to read response text: {}", e);
                                    return false;
                                }
                            };

                            let clr_result: Result<CloudLoginResponse, _> = serde_json::from_str(&json);
                            match clr_result {
                                Ok(clr) => {
                                    let encrypted_key_bytes = match general_purpose::STANDARD.decode(clr.encrypted_key) {
                                        Ok(bytes) => bytes,
                                        Err(e) => {
                                            eprintln!("ERROR: Failed to decode encrypted key: {}", e);
                                            return false;
                                        }
                                    };
                                    let private_key_pem = match Crypto::get_private_key_pem() {
                                        Ok(key) => key,
                                        Err(e) => {
                                            eprintln!("ERROR: Failed to get private key: {}", e);
                                            eprintln!("Authentication cannot proceed without SSH keys.");
                                            return false;
                                        }
                                    };
                                    let key_result = RsaPrivateKey::from_pkcs1_pem(&private_key_pem);
                                    match key_result {
                                        Ok(private_key) => {
                                            let _decrypted_key = match private_key.decrypt(rsa::Pkcs1v15Encrypt, &encrypted_key_bytes) {
                                                Ok(key) => key,
                                                Err(e) => {
                                                    eprintln!("ERROR: Failed to decrypt RSA key: {}", e);
                                                    return false;
                                                }
                                            };

                                            // Base64 decode the inputs
                                            let encrypted_token_bytes = match general_purpose::STANDARD.decode(clr.encrypted_token) {
                                                Ok(bytes) => bytes,
                                                Err(e) => {
                                                    eprintln!("ERROR: Failed to decode encrypted token: {}", e);
                                                    return false;
                                                }
                                            };
                                            let iv_bytes = match general_purpose::STANDARD.decode(clr.iv) {
                                                Ok(bytes) => bytes,
                                                Err(e) => {
                                                    eprintln!("ERROR: Failed to decode IV: {}", e);
                                                    return false;
                                                }
                                            };


                                            // Initialize the AES-256 CBC decryptor
                                            let mut decryptor = Aes256CbcDec::new(GenericArray::from_slice(&_decrypted_key), GenericArray::from_slice(&iv_bytes));

                                            // Split the encrypted data into blocks of 16 bytes
                                            let mut blocks: Vec<GenericArray<u8, U16>> = encrypted_token_bytes
                                            .chunks_exact(16)
                                            .map(|chunk| GenericArray::clone_from_slice(chunk))
                                            .collect();

                                            // Decrypt the blocks
                                            decryptor.decrypt_blocks_mut(&mut blocks);

                                            // Combine the decrypted blocks back into a single Vec<u8>
                                            let mut decrypted_buffer: Vec<u8> = Vec::with_capacity(encrypted_token_bytes.len());
                                            for block in blocks {
                                                decrypted_buffer.extend_from_slice(&block);
                                            }

                                            let decrypted_token_bytes = match self._remove_pkcs7_padding(decrypted_buffer) {
                                                Ok(bytes) => bytes,
                                                Err(e) => {
                                                    eprintln!("ERROR: Failed to remove PKCS7 padding: {}", e);
                                                    return false;
                                                }
                                            };

                                            // Convert decrypted bytes to a UTF-8 string
                                            self.jwt = match String::from_utf8(decrypted_token_bytes) {
                                                Ok(s) => s,
                                                Err(e) => {
                                                    eprintln!("ERROR: Failed to convert decrypted token to UTF-8: {}", e);
                                                    return false;
                                                }
                                            };
                                            self.oid = match self._extract_oid_from_jwt(self.jwt.clone()) {
                                                Ok(oid) => oid,
                                                Err(e) => {
                                                    eprintln!("ERROR: Failed to extract OID from JWT: {}", e);
                                                    return false;
                                                }
                                            };
                                            match self.store.lock() {
                                                Ok(mut store) => store.set_jwt(self.jwt.clone()),
                                                Err(e) => {
                                                    eprintln!("ERROR: Failed to lock store to set JWT: {}", e);
                                                    return false;
                                                }
                                            };

                                            return true;        
                                        }
                                        Err(e) => {
                                            eprintln!("Failed to decrypt private key: {}", e);
                                            return false;        
                                        }
                                    }
                                }
                                Err(e) => {
                                    // Print the JSON and the error message in case of failure
                                    eprintln!("Failed to parse JSON: {}\nOriginal JSON: {}", e, json);
                                    // TODO: return a state that says "failed and don't retry"
                                    return false;
                                }
                            }        
                        }                        
                        _=> {
                            eprintln!("Login call returned a: {}", response.status());
                            return false;
                        }

                    }
                },
                Err(e) => {
                    eprintln!("Failed to auth: {}", e);
                    return false;
                }
            }
    }

    pub async fn start(&mut self) {

        let subscriber = Arc::new(Mutex::new(
            CloudSubscriber::new(
                self.settings.clone(), 
                self.machine_id.to_ascii_uppercase().clone(),
                String::new()
                )));
        
//        sleep(time::Duration::from_secs(self.settings.connect_retry_seconds));

        // initialize()
        let mut last_state = self.state;

        loop {
            let current_state = self.state;

            if last_state != current_state {
                println!("service state: {:?}", current_state);
                last_state = current_state;
            }

            match current_state {
                UpdateState::Dead => {
                    self.state = UpdateState::Disconnected;
                },
                UpdateState::Disconnected => {
                    if self.settings.use_authentication {
                        self.state = UpdateState::Authenticating;
                    }
                    else {
                        self.state = UpdateState::Authenticated;
                    }
                }
                UpdateState::Authenticating => {
                    if self._authenticate().await {
                        self.state = UpdateState::Authenticated;
                        self.auth_fail_count = 0;
                    }
                    else {
                        // adaptively get slower with fails, to a max time of 1min
                        if self.auth_fail_count < 12 {
                            self.auth_fail_count = self.auth_fail_count + 1;
                        }
                        thread::sleep(Duration::from_secs(u64::from(self.auth_fail_count * 5)));
                    }
                },
                UpdateState::Authenticated => {
                    let s = subscriber.clone();
                    let upd_snd = self.update_sender.clone();
                    let st_snd = self.state_sender.clone();
                    let jwt_copy = self.jwt.clone();
                    let oid_copy = self.oid.clone();

                    // this spawns a cloud MQTT listener/subscriber.
                    // when it connects, it will update the state to connected
                    thread::spawn(move || {
                        match s.lock() {
                            Ok(mut subscriber) => {
                                subscriber.start(upd_snd, st_snd, jwt_copy, oid_copy);
                            },
                            Err(e) => {
                                eprintln!("ERROR: Failed to lock subscriber: {}", e);
                            }
                        }
                    });

                    self.state = UpdateState::Connecting;
                },
                UpdateState::Connecting => {
                    // just waiting for connected state
                },
                UpdateState::Connected => {
                    // look for any message from the subscriber
                    match self.update_receiver.try_recv() {
                        Ok(d) => {
                            println!("{:?}", d);

                            match self.store.lock() {
                                Ok(mut store) => {
                                    store.add(Arc::new(d));
                                },
                                Err(e) => {
                                    eprintln!("ERROR: Failed to lock store to add update: {}", e);
                                }
                            }
                        },
                        _ => { /* no data */ }
                    }
                },
                UpdateState::Idle => {
                    // TODO
                },
                UpdateState::DownloadingFile => {
                    // TODO
                },
                UpdateState::UpdateInProgress=> {
                    // TODO
                },
                _ => { } // nothing to do
            }

            // check to see if the cloud subscriber has a state change for us
            match self.state_receiver.try_recv() {
                Ok(new_state) => {
                    self.state = new_state;
                },
                _ => { /* NOP */ }
            }

            sleep(time::Duration::from_secs(1));
        }
    }
}