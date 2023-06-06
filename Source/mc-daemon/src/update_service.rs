use std::{thread::{sleep}};
use tokio::time;

use crate::{cloud_settings::CloudSettings, cloud_subscriber::CloudSubscriber};

#[derive(Debug, Clone, Copy)]
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

    pub fn start(&mut self) {
        self.cloud_subscriber = Some(CloudSubscriber::new(self.settings.clone(), self.machine_id.clone()));

        tokio::spawn(async move {
            self.state_machine();
        });
    }
    
    async fn state_machine(&mut self) {
        let seconds = self.settings.connect_retry_seconds;
        let mut interval = time::interval(time::Duration::from_secs(seconds));
        interval.tick().await;

        // initialize()

        while ! self.stop_service {
            println!("service state: {:?}", self.state);

            match self.state {
                UpdateState::Disconnected => {
                    if self.settings.use_authentication {
                        self.state = UpdateState::Authenticating;
                    }
                    else {
                        self.state = UpdateState::Connecting;
                    }
                }
                UpdateState::Connecting => {
//                    m.cloud_subscriber.as_ref().unwrap().start();
                },
                _ => {
                    interval.tick().await;
                }
            }
        }
    }
}