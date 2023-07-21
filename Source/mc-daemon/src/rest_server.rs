use std::{time::{SystemTime, UNIX_EPOCH}, sync::{Mutex, Arc}, fs::{self}, path::PathBuf};
use actix_web::{App, Error, HttpResponse, HttpServer, web, Responder};
use serde::{Deserialize, Serialize};

use crate::{update_store::UpdateStore, update_descriptor::UpdateDescriptor};

const PORT: &str = "5000";

#[derive(Serialize, Deserialize)]
struct File {
    name: String,
    up_time: u64,
    err: String,
}

#[derive(Serialize, Deserialize)]
struct ServiceInfo {
    service: String,
    up_time: u64,
    version: String,
    status: String
}

#[derive(Serialize, Deserialize)]
struct UpdateAction {
    action: String,
    pid: i32,
    app_dir: Option<String>
}

pub struct RestServer;

impl RestServer {
    pub fn new() -> RestServer {
        RestServer { }
    }
   
    pub async fn start(&mut self, store: Arc<Mutex<UpdateStore>>) -> std::io::Result<()> {
        
        println!("Meadow daemon listening for REST calls on port {}", PORT);

        HttpServer::new(move || {
            App::new()
                .app_data(web::Data::new(store.clone()))
                .service(
                    web::scope("api")
                        .route("/info", web::get().to(Self::get_daemon_info))
                        .route("/updates", web::get().to(Self::get_updates))
                        .route("/updates/{id}", web::put().to(Self::update_action))
                        .route("/updates", web::delete().to(Self::clear_update_store))
                )
        })
            .bind(format!("0.0.0.0:{}", PORT))?
            .run()
            .await
    }

    async fn clear_update_store(
        store: web::Data<Arc<Mutex<UpdateStore>>>)
        -> Result<HttpResponse, Error> {

        println!("REST CLEAR UPDATE STORE");

        store
            .lock()
            .unwrap()
            .clear();

            Ok(HttpResponse::Ok().finish())
    }

    async fn get_daemon_info() -> Result<HttpResponse, Error> {
        Ok(HttpResponse::Ok().json(&ServiceInfo {
            service: "Wilderness Labs Meadow".to_string(),
            up_time: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            version: "1.0".to_string(),
            status: "Running".to_string()
        }))
    }

    async fn update_action(
        store: web::Data<Arc<Mutex<UpdateStore>>>,
        data: web::Json<UpdateAction>, id: web::Path<String>) 
        -> impl Responder {

        println!("REST PUT UPDATE");
        
        match data.action.as_str() {
            "download" => {
                println!("Download MPAK for {}", id);
                match store
                    .lock()
                    .unwrap()
                    .retrieve_update(&id)
                    .await {
                        Ok(_result) => {
                            HttpResponse::Ok().finish()
                        },
                        Err(msg) => {
                            println!("Error sending MPAK for {}: {}", id, msg);                         
                            HttpResponse::NotFound().body(msg) 
                        }
                    }
            },
            "apply" => {
                println!("Apply update {}", id);
                let pid = data.pid;
                let  app_path;

                match &data.app_dir {
                    None => {
                        match fs::read_link(format!("/proc/{}/exe", pid)) {
                            Ok(path) => {
                                app_path = path;
                            },
                            Err(_) => {
                                let msg = format!("Caller sent in an invalid PID {}", data.pid);
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
                    match store
                        .lock()
                        .unwrap()
                        .apply_update(&id, &app_path, pid)
                        .await {
                        Ok(_result) => {
                            return HttpResponse::Ok().finish();
                        },
                        Err(msg) => {
                            return HttpResponse::NotFound().body(msg);
                        }
                    }
                }
                else {
                    // TODO: should we support non-pid apply calls?
                    let msg = format!("Caller did not provide a PID");
                    println!("{}", msg);
                    return HttpResponse::BadRequest().body(msg) ;
                    /*
                    match store
                        .lock()
                        .unwrap()
                        .extract_update(&id, "/home/ctacke/upd/".to_string())
                        .await {
                            Ok(_result) => {
                                HttpResponse::Ok().finish()
                            },
                            Err(msg) => {
                                HttpResponse::NotFound().body(msg) 
                            }
                        }
                    */
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
        let updates = store
            .lock()
            .unwrap()
            .get_all_messages();

        let mut result: Vec<UpdateDescriptor> = Vec::with_capacity(updates.len());

        for i in 0..updates.len() {
            result.push(updates[i]
                .lock()
                .unwrap()
                .clone()
            );
        }


        // retrieve update info
        println!("  sending {} results...", updates.len());

        Ok(HttpResponse::Ok().json(result))
    }
   
}