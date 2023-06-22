use std::{rc::Rc, collections::HashMap, cell::RefCell, borrow::Borrow, ops::Deref, path::{Path, PathBuf}, fs::{self, DirEntry, read_dir, OpenOptions}, str::FromStr, fmt::format, io::Write};

use crate::{cloud_settings::CloudSettings, update_descriptor::UpdateDescriptor};

pub struct UpdateStore {
    settings: CloudSettings,
    store_directory: PathBuf,
    updates: HashMap<String, Rc<RefCell<UpdateDescriptor>>>
}

impl UpdateStore {
    const STORE_ROOT_FOLDER:&str = "/home/ctacke/meadow/updates";
    const UPDATE_INFO_FILE_NAME: &str = "info.json";

    pub fn new(settings: CloudSettings) -> UpdateStore {
        let store = UpdateStore {
            settings,
            store_directory: PathBuf::from(Self::STORE_ROOT_FOLDER),
            updates: HashMap::new()
        };
        
        println!("{:?}", store.store_directory);

        if ! store.store_directory.exists() {
            fs::create_dir(&store.store_directory).unwrap();
        }
        else {
            // TODO: load all existing update descriptors
            for entry in fs::read_dir(&store.store_directory) {

            }
        }

        store
    }

    pub fn add(&mut self, descriptor: Rc<UpdateDescriptor>) {
        let rf = Rc::new( RefCell::new((*descriptor).clone()));
        let id = descriptor.deref().mpak_id.clone();
        self.updates.insert(id, rf);
        self.save_or_update(descriptor.deref());
    }

    pub fn len(&self) -> i32 {
        self.updates.len() as i32
    }

    pub fn get_message(&self, id: String) -> Option<&Rc<RefCell<UpdateDescriptor>>> {
        self.updates.get(&id)
    }

    pub fn clear(&mut self) {
        self.updates.clear();
    }

    pub fn set_retreived(&self, id: String) {
        let update = self.updates.get(&id);
         match update {
            Some(u) => {
                let mut d = u.borrow_mut();
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
                let mut d = u.borrow_mut();
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