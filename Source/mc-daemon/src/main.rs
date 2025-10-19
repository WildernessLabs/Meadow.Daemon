use std::{fs::read_to_string, sync::{Arc, Mutex}};
use mc_daemon::{cloud_settings::CloudSettings, update_service::UpdateService, rest_server, update_store::UpdateStore};

#[actix_web::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("starting daemon");

    let settings = CloudSettings::from_file("/etc/meadow.conf");
    let machine_id = read_to_string("/etc/machine-id").unwrap();

    // Ensure meadow_root directory exists
    if !settings.meadow_root.exists() {
        println!("Creating meadow root directory: {:?}", settings.meadow_root);
        std::fs::create_dir_all(&settings.meadow_root).unwrap();
    }

    let update_store: Arc<Mutex<UpdateStore>> = Arc::new(Mutex::new(UpdateStore::new(settings.clone())));

    let mut update_service = UpdateService::new(settings, machine_id.clone(), update_store.clone());
    
    let mut rest_server = rest_server::RestServer::new();



    let mut set = tokio::task::JoinSet::new();

    set.spawn_local(async move { 
        update_service.start().await; 
    });
    set.spawn_local(async move { 
        let _ = rest_server.start(update_store).await; 
    });

    while let Some(res) = set.join_next().await {
        let _out = res?;
    }

    //    let us = std::thread::spawn(move || {
//        handle.spawn(async { update_service.start() } ) });

//    let rs = handle.spawn(rest_server.start(update_store).await);

//    tokio::join!(us, rs);

//    update_service.start();

/*
    tokio::spawn(async move {
        update_service.start();
    }); 


    match rest_server.start(update_store).await {
        Err(e) => { println!("Unable to start REST server: {}", e); },
        _ => { println!("daemon exited!"); }
    }
*/    
    Ok(())
}
