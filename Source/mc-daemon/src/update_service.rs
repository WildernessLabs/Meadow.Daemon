use std::{thread::{sleep, self}, sync::{Mutex, Arc, mpsc::{self, Sender, Receiver}}};
use tokio::time;

use crate::{cloud_settings::CloudSettings, cloud_subscriber::CloudSubscriber, update_store::UpdateStore, update_descriptor::UpdateDescriptor};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateState {
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
    state: UpdateState,
    store: Arc<Mutex<UpdateStore>>,
    update_sender: Sender<UpdateDescriptor>,
    update_receiver: Receiver<UpdateDescriptor>,
    state_sender: Sender<UpdateState>,
    state_receiver: Receiver<UpdateState>
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
            state_receiver
        }
    }

    pub fn start(&mut self) {

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
                        self.state = UpdateState::Connecting;
                    }
                }
                UpdateState::Authenticating => {
                    // TODO
                }
                UpdateState::Connecting => {
                    let s = subscriber.clone();
                    let upd_snd = self.update_sender.clone();
                    let st_snd = self.state_sender.clone();

                    thread::spawn(move || {
                        s
                            .lock()
                            .unwrap()
                            .start(upd_snd, st_snd);
                    });

                    
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