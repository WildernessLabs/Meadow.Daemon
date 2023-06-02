
mod cloud_subscriber;
mod update_parser;
mod update_descriptor;
mod cloud_settings;

fn main() {
    println!("starting daemon");

    let settings = cloud_settings::Settings::from_file("/etc/meadow.conf");

    let  subscriber = cloud_subscriber::CloudSubscriber::new(settings);

    subscriber.start();

    println!("exiting daemon");
}
