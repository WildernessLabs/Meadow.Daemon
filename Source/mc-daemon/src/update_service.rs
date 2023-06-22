use std::{thread::{sleep, self}, sync::{Mutex, Arc, mpsc}, rc::Rc};
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

pub struct UpdateStateMachine {
    settings: CloudSettings, 
    machine_id: String,
    state: Arc<Mutex<UpdateState>>,
    store: UpdateStore
}

impl UpdateStateMachine {

    pub fn new(settings: CloudSettings, machine_id: String) -> UpdateStateMachine {
        UpdateStateMachine{settings: settings.clone(), machine_id: machine_id, state: Arc::new(Mutex::new( UpdateState::Dead)), store: UpdateStore::new(settings.clone())}
    }

    pub fn start(&mut self) {        
        let (tx, rx) = mpsc::channel();

        let subscriber = Arc::new(
            CloudSubscriber::new(
                self.settings.clone(), 
                self.machine_id.clone(), 
                self.state.clone()
                ));
        
        sleep(time::Duration::from_secs(self.settings.connect_retry_seconds));

        // initialize()
        let mut last_state = *self.state.lock().unwrap();

        loop {
            let mut current_state = *self.state.lock().unwrap();

            if last_state != current_state {
                println!("service state: {:?}", current_state);
                last_state = current_state;
            }

            match current_state {
                UpdateState::Dead => {
                    *self.state.lock().unwrap() = UpdateState::Disconnected;
                },
                UpdateState::Disconnected => {
                    if self.settings.use_authentication {
                        *self.state.lock().unwrap() = UpdateState::Authenticating;
                    }
                    else {
                        *self.state.lock().unwrap() = UpdateState::Connecting;
                    }
                }
                UpdateState::Authenticating => {
                    // TODO
                }
                UpdateState::Connecting => {
                    let s = subscriber.clone();
                    let snd = tx.clone();

                    thread::spawn(move || {
                        s.start(snd);
                    });

                    
                },
                UpdateState::Connected => {
                    // look for any message from the subscriber
                    match rx.try_recv() {
                        Ok(d) => {
                            println!("{:?}", d);

                            self.store.add(Rc::new(d));
                        },
                        Err(e) => {
                            // no data
                        }
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

    pub fn start(&mut self) {

        // create copies for the thread closure
        let s = self.settings.clone();
        let id = self.machine_id.clone();

        let h = thread::spawn(move || {
            let mut sm = UpdateStateMachine::new(s, id);
            sm.start();
        });
        h.join().unwrap();
    }    
}