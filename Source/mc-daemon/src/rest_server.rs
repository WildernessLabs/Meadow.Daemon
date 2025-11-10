use std::{time::{SystemTime, UNIX_EPOCH}, sync::{Mutex, Arc}, fs::{self}, path::PathBuf};
use actix_web::{App, Error, HttpResponse, HttpServer, web, Responder};
use serde::{Deserialize, Serialize};

use crate::{crypto::Crypto, update_descriptor::UpdateDescriptor, update_store::UpdateStore};

const PORT: &str = "5000";

/*
#[derive(Serialize, Deserialize)]
struct File {
    name: String,
    up_time: u64,
    err: String,
}
*/

#[derive(Serialize, Deserialize)]
struct DeviceInfo {
    serial_number: String,
    device_name: String,
    platform: String,
    os_version: String,
    os_release: String,
    os_name: String,
    machine: String
}

#[derive(Serialize, Deserialize)]
struct ServiceInfo {
    service: String,
    up_time: u64,
    version: String,
    status: String,
    device_info: DeviceInfo,
    public_key: String
}

#[derive(Serialize, Deserialize)]
struct UpdateAction {
    action: String,
    pid: Option<i32>,
    app_dir: Option<String>,
    command: Option<String>
}

#[derive(Serialize, Deserialize)]
struct ApplyAction {
    pid: Option<i32>,
    app_dir: Option<String>,
    command: Option<String>
}

#[derive(Serialize, Deserialize)]
struct FileInfo {
    name: String,
    #[serde(rename = "isDirectory")]
    is_directory: bool,
    size: u64,
}

#[derive(Serialize, Deserialize)]
struct FileListResponse {
    path: String,
    files: Vec<FileInfo>,
}

pub struct RestServer;

fn trim_newline(s: &mut String) {
    if s.ends_with('\n') {
        s.pop();
        if s.ends_with('\r') {
            s.pop();
        }
    }
}

fn validate_and_resolve_path(
    root: &PathBuf,
    requested_path: Option<&str>
) -> Result<PathBuf, String> {
    // Use root if no path specified
    let target = match requested_path {
        Some(p) if !p.is_empty() => root.join(p),
        _ => root.clone(),
    };

    // Canonicalize both paths
    let canonical_root = root.canonicalize()
        .map_err(|e| format!("Invalid root: {}", e))?;
    let canonical_target = target.canonicalize()
        .map_err(|e| format!("Path not found: {}", e))?;

    // Security check: ensure target is within root
    if !canonical_target.starts_with(&canonical_root) {
        return Err("Path traversal attempt detected".to_string());
    }

    Ok(canonical_target)
}

impl ServiceInfo {
    pub fn new() -> ServiceInfo {

        let mut sn = match fs::read_to_string("/var/lib/dbus/machine-id") {
            Ok(id) => id.to_uppercase(),
            Err(e) => {
                eprintln!("WARNING: Failed to read machine-id: {}. Using 'UNKNOWN'.", e);
                "UNKNOWN".to_string()
            }
        };
        trim_newline(&mut sn);
        let info = uname::uname().expect("CRITICAL: Failed to get system info via uname. This should never fail on a Linux system.");

        // todo: should this be in a separate service, maybe?

        ServiceInfo {
            service: "Wilderness Labs Meadow.Daemon".to_string(),
            up_time: SystemTime::now().duration_since(UNIX_EPOCH)
                .unwrap_or_else(|e| {
                    eprintln!("WARNING: Failed to get system time: {}. Using 0.", e);
                    std::time::Duration::from_secs(0)
                })
                .as_secs(),
            version: "1.0".to_string(), // TODO: actually get this number
            status: "Running".to_string(),
            device_info: DeviceInfo 
            { 
                serial_number: sn,
                platform: "Meadow.Linux".to_string(), // TODO: pull from lscpu?
                device_name: info.nodename,
                os_version: info.version,
                os_release: info.release,
                os_name: info.sysname,
                machine: info.machine
            },
            public_key: Crypto::get_public_key_pem(None).unwrap_or_else(|e| {
                eprintln!("WARNING: Failed to get public key: {}. Using placeholder.", e);
                eprintln!("  Looked in ~/.ssh/id_rsa.pub (or /root/.ssh/id_rsa.pub)");
                eprintln!("  You can configure the SSH key path in /etc/meadow.conf using the 'ssh_key_path' setting.");
                "[No Public Key]".to_string()
            })
        }
    }
}

impl RestServer {
    pub fn new() -> RestServer {
        RestServer { }
    }
   
    pub async fn start(&mut self, store: Arc<Mutex<UpdateStore>>, settings: crate::cloud_settings::CloudSettings, bind_address: &str) -> std::io::Result<()> {

        println!("Meadow daemon listening for REST calls on {}:{}", bind_address, PORT);

        HttpServer::new(move || {
            App::new()
                .app_data(web::Data::new(store.clone()))
                .app_data(web::Data::new(settings.clone()))
                .service(
                    web::scope("api")
                        .route("/info", web::get().to(Self::get_daemon_info))
                        .route("/updates", web::get().to(Self::get_updates))
                        .route("/updates/{id}", web::put().to(Self::update_action))
                        .route("/updates", web::delete().to(Self::clear_update_store))
                        .route("/apply", web::put().to(Self::apply_extracted))
                        .route("/files", web::get().to(Self::list_files))
                        .route("/files/{path:.*}", web::get().to(Self::list_files))
                )
        })
            .bind(format!("{}:{}", bind_address, PORT))?
            .run()
            .await
    }

    async fn clear_update_store(
        store: web::Data<Arc<Mutex<UpdateStore>>>)
        -> Result<HttpResponse, Error> {

        println!("REST CLEAR UPDATE STORE");

        match store.lock() {
            Ok(mut s) => {
                s.clear();
                Ok(HttpResponse::Ok().finish())
            },
            Err(e) => {
                eprintln!("ERROR: Failed to lock store: {}", e);
                Ok(HttpResponse::InternalServerError().body("Failed to lock store"))
            }
        }
    }

    async fn get_daemon_info() -> Result<HttpResponse, Error> {
        Ok(HttpResponse::Ok().json(&ServiceInfo::new()))
    }

    async fn update_action(
        store: web::Data<Arc<Mutex<UpdateStore>>>,
        data: web::Json<UpdateAction>, id: web::Path<String>) 
        -> impl Responder {

        println!("REST PUT UPDATE");
        
        match data.action.as_str() {
            "download" => {
                println!("Download MPAK for {}", id);
                match store.lock() {
                    Ok(s) => {
                        match s.retrieve_update(&id).await {
                            Ok(_result) => {
                                HttpResponse::Ok().finish()
                            },
                            Err(msg) => {
                                println!("Error sending MPAK for {}: {}", id, msg);
                                HttpResponse::NotFound().body(msg)
                            }
                        }
                    },
                    Err(e) => {
                        eprintln!("ERROR: Failed to lock store: {}", e);
                        HttpResponse::InternalServerError().body("Failed to lock store")
                    }
                }
            },
            "apply" => {
                println!("Apply update {}", id);
                let pid = data.pid.unwrap_or(0);
                let  app_path;

                match &data.app_dir {
                    None => {
                        match fs::read_link(format!("/proc/{}/exe", pid)) {
                            Ok(path) => {
                                app_path = path;
                            },
                            Err(_) => {
                                let msg = format!("Caller sent in an invalid PID {}", pid);
                                println!("{}", msg);
                                return HttpResponse::NotFound().body(msg);
                            }
                        }
                    },
                    Some(p) => {
                        // TODO verify the provided path is valid?
                        app_path = PathBuf::from(p);
                    }
                }

                if pid != 0 {
                    // note: this will launch a thread to wait and apply
                    match store.lock() {
                        Ok(s) => {
                            match s.apply_update(&id, &app_path, pid, &data.command).await {
                                Ok(_result) => {
                                    return HttpResponse::Ok().finish();
                                },
                                Err(msg) => {
                                    return HttpResponse::NotFound().body(msg);
                                }
                            }
                        },
                        Err(e) => {
                            eprintln!("ERROR: Failed to lock store: {}", e);
                            return HttpResponse::InternalServerError().body("Failed to lock store");
                        }
                    }
                }
                else {
                    // TODO: should we support non-pid apply calls?
                    let msg = format!("Caller did not provide a valid PID");
                    println!("{}", msg);
                    return HttpResponse::BadRequest().body(msg);
                }
            },
            _ => {
                println!("Unknown action request: {}", data.action);
                HttpResponse::NotFound().finish()
            }
        }
    }
    
    async fn get_updates(
        store: web::Data<Arc<Mutex<UpdateStore>>>)
        -> Result<HttpResponse, Error> { //actix_web::Result<impl Responder> {

        // open the store
        let updates = match store.lock() {
            Ok(s) => s.get_all_messages(),
            Err(e) => {
                eprintln!("ERROR: Failed to lock store: {}", e);
                return Ok(HttpResponse::InternalServerError().body("Failed to lock store"));
            }
        };

        let mut result: Vec<UpdateDescriptor> = Vec::with_capacity(updates.len());

        for i in 0..updates.len() {
            match updates[i].lock() {
                Ok(update) => result.push(update.clone()),
                Err(e) => {
                    eprintln!("WARNING: Failed to lock update descriptor {}: {}", i, e);
                    // Continue with other updates
                }
            }
        }


        // retrieve update info
        println!("  sending {} results...", result.len());

        Ok(HttpResponse::Ok().json(result))
    }

    async fn apply_extracted(
        store: web::Data<Arc<Mutex<UpdateStore>>>,
        data: web::Json<ApplyAction>)
        -> impl Responder {

        println!("REST APPLY EXTRACTED UPDATE");

        let pid = match data.pid {
            Some(p) if p > 0 => p,
            _ => {
                let msg = "PID is required and must be greater than 0";
                println!("ERROR: {}", msg);
                return HttpResponse::BadRequest().body(msg);
            }
        };

        let app_path = match &data.app_dir {
            None => {
                // Try to get app path from /proc/{pid}/exe
                match fs::read_link(format!("/proc/{}/exe", pid)) {
                    Ok(path) => path,
                    Err(e) => {
                        let msg = format!("Failed to determine application path from PID {}: {}", pid, e);
                        println!("ERROR: {}", msg);
                        return HttpResponse::BadRequest().body(msg);
                    }
                }
            },
            Some(p) => PathBuf::from(p)
        };

        println!("Applying update to: {:?}", app_path);
        println!("Waiting for PID: {}", pid);
        if let Some(ref cmd) = data.command {
            println!("Restart command: {}", cmd);
        }

        match store.lock() {
            Ok(s) => {
                match s.apply_extracted_update(&app_path, pid, &data.command).await {
                    Ok(_) => {
                        println!("Apply operation started successfully");
                        HttpResponse::Ok().body("Update apply started")
                    },
                    Err(msg) => {
                        println!("ERROR: Failed to apply update: {}", msg);
                        HttpResponse::InternalServerError().body(msg)
                    }
                }
            },
            Err(e) => {
                eprintln!("ERROR: Failed to lock store: {}", e);
                HttpResponse::InternalServerError().body("Failed to lock store")
            }
        }
    }

    async fn list_files(
        settings: web::Data<crate::cloud_settings::CloudSettings>,
        path: web::Path<String>,
    ) -> Result<HttpResponse, Error> {
        let requested_path = path.into_inner();
        let requested_path = if requested_path.is_empty() {
            None
        } else {
            Some(requested_path.as_str())
        };

        // Validate and resolve path
        let target_path = match validate_and_resolve_path(&settings.meadow_root, requested_path) {
            Ok(p) => p,
            Err(e) => return Ok(HttpResponse::BadRequest().body(e)),
        };

        // Check if path is a directory
        if !target_path.is_dir() {
            return Ok(HttpResponse::BadRequest().body("Path is not a directory"));
        }

        // Read directory entries
        let entries = match fs::read_dir(&target_path) {
            Ok(e) => e,
            Err(e) => return Ok(HttpResponse::InternalServerError()
                .body(format!("Failed to read directory: {}", e))),
        };

        // Collect file information
        let mut files: Vec<FileInfo> = Vec::new();
        for entry in entries {
            if let Ok(entry) = entry {
                if let Ok(metadata) = entry.metadata() {
                    files.push(FileInfo {
                        name: entry.file_name().to_string_lossy().to_string(),
                        is_directory: metadata.is_dir(),
                        size: if metadata.is_file() { metadata.len() } else { 0 },
                    });
                }
            }
        }

        // Sort: directories first, then alphabetically
        files.sort_by(|a, b| {
            match (a.is_directory, b.is_directory) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            }
        });

        // Build response
        let response = FileListResponse {
            path: requested_path.unwrap_or("").to_string(),
            files,
        };

        Ok(HttpResponse::Ok().json(response))
    }

}