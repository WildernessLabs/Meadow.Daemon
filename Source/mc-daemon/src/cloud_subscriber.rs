
extern crate paho_mqtt as mqtt;

use std::{ process, thread, time::Duration};
use crate::{update_parser::UpdateParser, cloud_settings::CloudSettings};

const DFLT_CLIENT:&str = "mc_daemon";

pub struct CloudSubscriber {
    settings: CloudSettings,
    machine_id: String
}

impl CloudSubscriber {
    pub fn new(settings: CloudSettings, machine_id: String) -> CloudSubscriber {
        CloudSubscriber { settings, machine_id }
    }

    // Reconnect to the broker when connection is lost.
    fn try_reconnect(&self, cli: &mqtt::Client) -> bool
    {
        println!("Connection lost. Waiting to retry connection");
        for _ in 0..12 {
            thread::sleep(Duration::from_secs(self.settings.connect_retry_seconds));
            if cli.reconnect().is_ok() {
                println!("Successfully reconnected");
                return true;
            }
        }
        println!("Unable to reconnect after several attempts.");
        false
    }

    // Subscribes to multiple topics.
    fn subscribe_topics(&self, cli: &mqtt::Client, topics: &Vec<String>) {
        
        for topic in topics {
            // do macro substitution for '{ID}'
            let t = topic.replace("{ID}", &self.machine_id);

            println!("Subscribing to {}", topic);

            // QOS == 2 means deliver exactly once
            if let Err(e) = cli.subscribe(t.as_str(), 2) {
                println!("Error subscribing to {} {:?}", topic, e);
            }
        }
    }

    fn unsubscribe_topics(&self, cli: &mqtt::Client, topics: &Vec<String>) {
        for topic in topics {
            // do macro substitution for '{ID}'
            let t = topic.replace("{ID}", &self.machine_id);

            // QOS == 2 means deliver exactly once
            if let Err(e) = cli.unsubscribe(t.as_str()) {
                println!("Error subscribing to {} {:?}", topic, e);
            }
        }
    }

    pub fn start(&self) {            
        let host = format!("{}:{}", self.settings.update_server_address, self.settings.update_server_port);

        // Define the set of options for the create.
        // Use an ID for a persistent session.
        let create_opts = mqtt::CreateOptionsBuilder::new()
            .server_uri(host)
            .client_id(DFLT_CLIENT.to_string())
            .finalize();

        // Create a client.
        let client = mqtt::Client::new(create_opts).unwrap_or_else(|err| {
            println!("Error creating the client: {:?}", err);
            process::exit(1);
        });

        // Initialize the consumer before connecting.
        let receiver = client.start_consuming();

        // Define the set of options for the connection.
        let lwt = mqtt::MessageBuilder::new()
            .topic("test")
            .payload("Consumer lost connection")
            .finalize();

        let conn_opts = mqtt::ConnectOptionsBuilder::new()
            .keep_alive_interval(Duration::from_secs(20))
            .clean_session(false)
            .will_message(lwt)
            .finalize();

        println!("making MQTT connection:\n");

        // Connect and wait for it to complete or fail.
        if let Err(e) = client.connect(conn_opts) {
            println!("Unable to connect:\n\t{:?}", e);
            process::exit(1);
        }

        // Subscribe topics.
        self.subscribe_topics(&client, &self.settings.mqtt_topics);

        println!("Processing requests...");
        for msg in receiver.iter() {
            if let Some(msg) = msg {
                let update = UpdateParser::parse_message(msg.payload_str().as_ref());
                println!("{:?}", update);
            }
            else if !client.is_connected() {
                if self.try_reconnect(&client) {
                    println!("Resubscribe to topics...");
                    self.subscribe_topics(&client, &self.settings.mqtt_topics);
                } else {
                    break;
                }
            }
        }

        // If still connected, then disconnect now.
        if client.is_connected() {
            println!("Disconnecting");       
            self.unsubscribe_topics(&client, &self.settings.mqtt_topics); 
            client.disconnect(None).unwrap();
        }
        println!("Exiting");
            
    }
}