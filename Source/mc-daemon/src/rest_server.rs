use std::{time::{SystemTime, UNIX_EPOCH}, sync::{Mutex, Arc}};
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
                )
                /*
                .service(
                    web::scope("/api")
                        .route("/updates", web::get().to(Self::get_updates2))
                )
                */
        })
            .bind(format!("0.0.0.0:{}", PORT))?
            .run()
            .await
    }

    async fn get_daemon_info() -> Result<HttpResponse, Error> {
        println!("REST GET DAEMON INFO");

        Ok(HttpResponse::Ok().json(&ServiceInfo {
            service: "Wilderness Labs Meadow".to_string(),
            up_time: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            version: "1.0".to_string(),
            status: "Running".to_string()
        }))
    }

    async fn get_updates2() 
        -> Result<HttpResponse, Error> { //actix_web::Result<impl Responder> {
        println!("REST GET UPDATE LIST");

        // open the store
        

        // retrieve update info

        Ok(HttpResponse::Ok().body("hello"))
    }

    async fn get_updates(
        store: web::Data<Arc<Mutex<UpdateStore>>>) 
        -> Result<HttpResponse, Error> { //actix_web::Result<impl Responder> {
        println!("REST GET UPDATE LIST");

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

        Ok(HttpResponse::Ok().json(result))
    }
   
}