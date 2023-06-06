use std::{thread::{sleep, self}, hash::BuildHasher};
use tokio::time;

use crate::{cloud_settings::CloudSettings, cloud_subscriber::CloudSubscriber};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UpdateState {
    Dead,
    Disconnected,
    Authenticating,
    Connecting,
    Connected,
    Idle,
    UpdateAvailable,
    DownloadingFile,
    UpdateInProgress
}

struct UpdateStateMachine; 

impl UpdateStateMachine {
    pub async fn start(settings: CloudSettings, machine_id: String) {
        let mut state = UpdateState::Dead;
        sleep(time::Duration::from_secs(settings.connect_retry_seconds));

        // initialize()
        let mut last_state = state;

        loop {
            if last_state != state {
                println!("service state: {:?}", state);
                last_state = state;
            }

            match state {
                UpdateState::Dead => {
                    state = UpdateState::Disconnected;
                },
                UpdateState::Disconnected => {
                    if settings.use_authentication {
                        state = UpdateState::Authenticating;
                    }
                    else {
                        state = UpdateState::Connecting;
                    }
                }
                UpdateState::Authenticating => {
                    // TODO
                }
                UpdateState::Connecting => {
                    let subscriber = CloudSubscriber::new(settings.clone(), machine_id.clone());
                    thread::spawn(move || {
                        subscriber.start();
                    });
                    
                    state = UpdateState::Connected;
                },
                UpdateState::Connected => {
                    // TODO
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
            sleep(time::Duration::from_secs(1));
        }
    }
}

pub struct UpdateService {
    settings: CloudSettings,
    machine_id: String,
    stop_service: bool,
    state: UpdateState,
    cloud_subscriber: Option<CloudSubscriber>
}

impl UpdateService {
    pub fn new(settings: CloudSettings, machine_id: String) -> UpdateService {
        UpdateService {
            settings, 
            machine_id: machine_id, 
            stop_service: false, 
            state: UpdateState::Disconnected,
            cloud_subscriber: None}
    }

    pub async fn start(&mut self) {

        // create copies for the thread closure
        let s = self.settings.clone();
        let id = self.machine_id.clone();

        tokio::spawn(async {
            UpdateStateMachine::start(s, id).await;
        });
    }    
}