use std::rc::Rc;

use mc_daemon::{update_store::UpdateStore, update_descriptor::UpdateDescriptor, cloud_settings::CloudSettings};

#[test]
fn insert_test() {
    let settings = CloudSettings::default();
    let mut store = UpdateStore::new(settings);
    let desc = UpdateDescriptor::new("ABCD".to_string());
    assert_eq!(0, store.len());
    store.add( Rc::new(desc));
    assert_eq!(1, store.len());
}

#[test]
fn insert_and_retrieve_test() {
    let settings = CloudSettings::default();
    let mut store = UpdateStore::new(settings);
    assert_eq!(0, store.len());
    store.add( Rc::new(UpdateDescriptor::new("Update1".to_string())));
    store.add( Rc::new(UpdateDescriptor::new("Update2".to_string())));
    store.add( Rc::new(UpdateDescriptor::new("Update3".to_string())));
    assert_eq!(3, store.len());

    let r = store.get_message("Update2".to_string());
    assert_eq!("Update2", r.unwrap().borrow().mpak_id);
}

#[test]
fn insert_and_clear_test() {
    let settings = CloudSettings::default();
    let mut store = UpdateStore::new(settings);
    assert_eq!(0, store.len());
    store.add( Rc::new(UpdateDescriptor::new("Update1".to_string())));
    store.add( Rc::new(UpdateDescriptor::new("Update3".to_string())));
    assert_eq!(2, store.len());

    store.clear();
    assert_eq!(0, store.len());
}