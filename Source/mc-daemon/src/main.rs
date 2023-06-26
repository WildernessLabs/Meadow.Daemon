use std::{fs::read_to_string, sync::{Arc, Mutex}};
use mc_daemon::{cloud_settings::CloudSettings, update_service::UpdateService, rest_server, update_store::UpdateStore};

#[actix_web::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("starting daemon");

    let settings = CloudSettings::from_file("/etc/meadow.conf");    
    let machine_id = read_to_string("/etc/machine-id").unwrap();
    let update_store: Arc<Mutex<UpdateStore>> = Arc::new(Mutex::new(UpdateStore::new(settings.clone())));

    let mut update_service = UpdateService::new(settings, machine_id.clone(), update_store.clone());
    
//    update_service.start();

    tokio::spawn(async move {
        update_service.start();
    }); 

    let mut rest_server = rest_server::RestServer::new();

    match rest_server.start(update_store).await {
        Err(e) => { println!("Unable to start REST server: {}", e); },
        _ => { println!("daemon exited!"); }
    }
    
    Ok(())
}
