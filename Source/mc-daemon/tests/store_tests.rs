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