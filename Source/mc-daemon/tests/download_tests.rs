use mc_daemon::{update_store::UpdateStore, update_descriptor::UpdateDescriptor, cloud_settings::CloudSettings};

#[tokio::test]
async fn authentication_test() {
    let settings = CloudSettings::default();
    let mut store = UpdateStore::new(settings);

//    let success = store.authenticate_with_server().await;
    //let success = store.test_creds();
}
