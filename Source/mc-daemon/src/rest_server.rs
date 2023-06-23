use std::time::{SystemTime, UNIX_EPOCH};
use actix_web::{App, Error, HttpResponse, HttpServer, web};
use serde::{Deserialize, Serialize};

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
   
    pub async fn start(&mut self) -> std::io::Result<()> {
        
        println!("Meadow daemon listening for REST calls on port {}", PORT);

        HttpServer::new(|| {
            App::new()
                .service(
                    web::scope("")
                        .route("/info", web::get().to(Self::get_info))
                )
                /*
                .service(
                    web::scope("/api")
                        .route("/files", web::post().to(Self::upload))
                )
                */
        })
            .bind(format!("0.0.0.0:{}", PORT))?
            .run()
            .await
    }

    async fn get_info() -> Result<HttpResponse, Error> {
        println!("REST GET INFO");

        Ok(HttpResponse::Ok().json(&ServiceInfo {
            service: "Wilderness Labs Meadow".to_string(),
            up_time: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            version: "1.0".to_string(),
            status: "Running".to_string()
        }))
    }
}