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

    println!("Creating update store...");
    let update_store: Arc<Mutex<UpdateStore>> = Arc::new(Mutex::new(UpdateStore::new(settings.clone())));

    println!("Creating update service...");
    let mut update_service = UpdateService::new(settings.clone(), machine_id.clone(), update_store.clone());

    println!("Spawning UpdateService in background thread...");
    std::thread::spawn(move || {
        println!("UpdateService thread started!");
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime for UpdateService");

        let local = tokio::task::LocalSet::new();
        local.block_on(&rt, async move {
            println!("UpdateService task starting...");
            update_service.start().await;
            println!("UpdateService task ended!");
        });
    });

    println!("Creating REST server...");
    let mut rest_server = rest_server::RestServer::new();

    println!("Starting REST server in main thread...");
    match rest_server.start(update_store, &settings.rest_api_bind_address).await {
        Ok(_) => {
            println!("REST server stopped");
            Ok(())
        },
        Err(e) => {
            eprintln!("ERROR: REST server failed: {}", e);
            Err(e)
        }
    }
}
