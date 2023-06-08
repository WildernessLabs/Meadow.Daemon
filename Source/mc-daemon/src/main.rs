use std::{fs::read_to_string, thread::sleep, time::Duration};

mod cloud_subscriber;
mod update_parser;
mod update_descriptor;
mod cloud_settings;
mod update_service;

#[tokio::main]
async fn main() {
    println!("starting daemon");

    let settings = cloud_settings::CloudSettings::from_file("/etc/meadow.conf");

    let machine_id = read_to_string("/etc/machine-id").unwrap();

    let mut update_service = update_service::UpdateService::new(settings, machine_id.clone());
    update_service.start();

    loop {
        sleep(Duration::new(5, 0));
    }
}
