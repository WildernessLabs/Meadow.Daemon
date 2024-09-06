use std::{thread::{sleep, self}, sync::{Mutex, Arc, mpsc::{self, Sender, Receiver}}, fs};
use oauth2::http::StatusCode;
use serde_json::json;
use serde::{Deserialize, Serialize};
use tokio::time;
use reqwest::Client;
use base64;
use base64::engine::general_purpose;
use base64::Engine;
use rsa::{RsaPrivateKey, pkcs1::DecodeRsaPrivateKey};

use crate::{cloud_settings::CloudSettings, cloud_subscriber::CloudSubscriber, update_store::UpdateStore, update_descriptor::UpdateDescriptor, crypto::Crypto};

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
    jwt: String
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
            jwt: String::new()
        }
    }

    //#[tokio::main] // this doesn't make it 'main' it just makes it synchonous (thanks for clarity, tokio!)
    async fn _authenticate(&mut self) -> bool {
        // connect to the cloud and get a JWT
        let device_id = fs::read_to_string("/var/lib/dbus/machine-id")
            .unwrap()
            .trim()
            .to_ascii_uppercase();

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
                        reqwest::StatusCode::OK => {
                            let json = response.text().await.unwrap();

                            let clr_result: Result<CloudLoginResponse, _> = serde_json::from_str(&json);
                            match clr_result {
                                Ok(clr) => {
                                    let encrypted_key_bytes = general_purpose::STANDARD.decode(clr.encrypted_key).unwrap();
                                    let private_key_pem = Crypto::get_private_key_pem();
                                    let private_key = RsaPrivateKey::from_pkcs1_pem(&private_key_pem).unwrap();
                                    let _decrypted_key = private_key.decrypt(rsa::Pkcs1v15Encrypt, &encrypted_key_bytes).unwrap();
                
                                    self.jwt = "foo".to_string();
                                    return true;
                                }
                                Err(e) => {
                                    // Print the JSON and the error message in case of failure
                                    eprintln!("Failed to parse JSON: {}\nOriginal JSON: {}", e, json);
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
                self.machine_id.clone()
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
                    }
                },
                UpdateState::Authenticated => {
                    let s = subscriber.clone();
                    let upd_snd = self.update_sender.clone();
                    let st_snd = self.state_sender.clone();
                    let jwt_copy = self.jwt.clone();

                    // this spawns a cloud MQTT listener/subscriber.
                    // when it connects, it will update the state to connected
                    thread::spawn(move || {
                        s
                            .lock()
                            .unwrap()
                            .start(upd_snd, st_snd, jwt_copy);
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

                            self.store
                                .lock()
                                .unwrap()
                                .add(Arc::new(d));
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