use std::sync::{Mutex, Arc};
use std::{collections::HashMap, ops::Deref};
use std::path::{Path, PathBuf};
use std::fs::{self, OpenOptions};
use std::io::{Write};

use crate::{cloud_settings::CloudSettings, update_descriptor::UpdateDescriptor};

pub struct UpdateStore {
    _settings: CloudSettings,
    store_directory: PathBuf,
    updates: HashMap<String, Arc<Mutex<UpdateDescriptor>>>
}

impl UpdateStore {
    const STORE_ROOT_FOLDER:&str = "/home/ctacke/meadow/updates";
    const UPDATE_INFO_FILE_NAME: &str = "info.json";

    pub fn new(settings: CloudSettings) -> UpdateStore {
        let store = UpdateStore {
            _settings : settings,
            store_directory: PathBuf::from(Self::STORE_ROOT_FOLDER),
            updates: HashMap::new()
        };
        
        println!("Update data will be stored in '{:?}'", store.store_directory);

        if ! store.store_directory.exists() {
            fs::create_dir(&store.store_directory).unwrap();
        }
        else {
            // TODO: load all existing update descriptors
            /*
            for entry in fs::read_dir(&store.store_directory) {

            }
            */
        }

        store
    }

    pub fn get_all_messages(&self) -> Vec<Arc<Mutex<UpdateDescriptor>>> {
        self.updates.values().cloned().collect::<Vec<Arc<Mutex<UpdateDescriptor>>>>()        
    }

    pub fn add(&mut self, descriptor: Arc<UpdateDescriptor>) {
        let rf = Arc::new( Mutex::new((*descriptor).clone()));
        let id = descriptor.deref().mpak_id.clone();
        self.updates.insert(id, rf);
        self.save_or_update(descriptor.deref());
    }

    pub fn len(&self) -> i32 {
        self.updates.len() as i32
    }

    pub fn get_message(&self, id: String) -> Option<&Arc<Mutex<UpdateDescriptor>>> {
        self.updates.get(&id)
    }

    pub fn clear(&mut self) {
        self.updates.clear();
    }

    pub fn set_retreived(&self, id: String) {
        let update = self.updates.get(&id);
         match update {
            Some(u) => {
                let mut d = u.lock().unwrap();
                d.retrieved = true;

                // update file
                self.save_or_update(d.deref());
            },
            None => {}
         }
    }

    pub fn set_applied(&self, id: String) {
        let update = self.updates.get(&id);
         match update {
            Some(u) => {
                let mut d = u.lock().unwrap();
                d.applied = true;

                // update file
                self.save_or_update(d.deref());
            },
            None => {}
         }
    }

    fn save_or_update(&self, descriptor: &UpdateDescriptor) {
        println!("{:?}", descriptor);

        // todo: make sure subdir exists
        let mut path = Path::new(Self::STORE_ROOT_FOLDER).join(&descriptor.mpak_id);
        if ! path.exists() {
            fs::create_dir(&path).unwrap();
        }

        // todo: serialize
        let json = serde_json::to_string_pretty(&descriptor).unwrap();

        // todo: erase any existing file
        path.push(&Self::UPDATE_INFO_FILE_NAME);

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(path)
            .unwrap();

        // todo: save
        file.write_all(json.as_bytes()).unwrap();

    }
}