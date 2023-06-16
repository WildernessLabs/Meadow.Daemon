use std::{rc::Rc, collections::HashMap};

use crate::{cloud_settings::CloudSettings, update_descriptor::UpdateDescriptor};

pub struct UpdateStore {
    settings: CloudSettings,
    updates: HashMap<String, Rc<UpdateDescriptor>>
}

impl UpdateStore {
    pub fn new(settings: CloudSettings) -> UpdateStore {
        UpdateStore {
            settings,
            updates: HashMap::new()
        }
    }

    pub fn add(&mut self, descriptor: Rc<UpdateDescriptor>) {
        self.updates.insert(descriptor.mpak_id.clone(), descriptor);
    }

    pub fn len(&self) -> i32 {
        self.updates.len() as i32
    }

    pub fn get_message(&self, id: String) -> Option<&Rc<UpdateDescriptor>> {
        self.updates.get(&id)
    }
}