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

    // Ensure meadow_temp directory exists
    if !settings.meadow_temp.exists() {
        println!("Creating meadow temp directory: {:?}", settings.meadow_temp);
        if let Err(e) = std::fs::create_dir_all(&settings.meadow_temp) {
            eprintln!("ERROR: Failed to create meadow_temp directory: {}", e);
            eprintln!("Please create the directory manually or set MEADOW_TEMP to a writable location.");
            return Err(e);
        }
    }

    // Ensure update_store_path directory exists
    if !settings.update_store_path.exists() {
        println!("Creating update store directory: {:?}", settings.update_store_path);
        if let Err(e) = std::fs::create_dir_all(&settings.update_store_path) {
            eprintln!("ERROR: Failed to create update_store_path directory: {}", e);
            eprintln!("Please create the directory manually or set UPDATE_STORE_PATH to a writable location.");
            return Err(e);
        }
    }

    // Ensure temp_extract_path directory exists
    if !settings.temp_extract_path.exists() {
        println!("Creating temp extract directory: {:?}", settings.temp_extract_path);
        if let Err(e) = std::fs::create_dir_all(&settings.temp_extract_path) {
            eprintln!("ERROR: Failed to create temp_extract_path directory: {}", e);
            eprintln!("Please create the directory manually or set TEMP_EXTRACT_PATH to a writable location.");
            return Err(e);
        }
    }

    // Ensure staging_path directory exists
    if !settings.staging_path.exists() {
        println!("Creating staging directory: {:?}", settings.staging_path);
        if let Err(e) = std::fs::create_dir_all(&settings.staging_path) {
            eprintln!("ERROR: Failed to create staging_path directory: {}", e);
            return Err(e);
        }
    }

    // Ensure rollback_path directory exists
    if !settings.rollback_path.exists() {
        println!("Creating rollback directory: {:?}", settings.rollback_path);
        if let Err(e) = std::fs::create_dir_all(&settings.rollback_path) {
            eprintln!("ERROR: Failed to create rollback_path directory: {}", e);
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
    match rest_server.start(update_store, settings.clone(), &settings.rest_api_bind_address).await {
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
