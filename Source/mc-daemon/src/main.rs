use std::{fs::read_to_string, sync::{Arc, Mutex}};
use mc_daemon::{cloud_settings::CloudSettings, update_service::UpdateService, rest_server, update_store::UpdateStore};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("starting daemon");

    let settings = CloudSettings::from_file("/etc/meadow.conf");

    // Try to read machine ID from standard locations
    let machine_id = read_to_string("/etc/machine-id")
        .or_else(|_| read_to_string("/var/lib/dbus/machine-id"))
        .unwrap_or_else(|e| {
            eprintln!("WARNING: Failed to read machine-id: {}. Using hostname.", e);
            std::env::var("HOSTNAME").unwrap_or_else(|_| "unknown-host".to_string())
        })
        .trim()
        .to_string();

    // Ensure meadow_root directory exists
    if !settings.meadow_root.exists() {
        println!("Creating meadow root directory: {:?}", settings.meadow_root);
        if let Err(e) = std::fs::create_dir_all(&settings.meadow_root) {
            eprintln!("ERROR: Failed to create meadow_root directory: {}", e);
            eprintln!("Please create the directory manually or set MEADOW_ROOT to a writable location.");
            return Err(e);
        }
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
        if let Err(e) = res {
            eprintln!("Task error: {}", e);
        }
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
