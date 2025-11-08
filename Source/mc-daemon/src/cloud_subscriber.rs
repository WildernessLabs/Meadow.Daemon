
extern crate paho_mqtt as mqtt;

use std::{ process, thread, time::Duration, sync::mpsc::Sender};
use crate::{update_parser::UpdateParser, cloud_settings::CloudSettings, update_service::UpdateState, update_descriptor::UpdateDescriptor};

const DFLT_CLIENT:&str = "mc_daemon";


pub struct CloudSubscriber {
    settings: CloudSettings,
    machine_id: String,
    oid: String
}

impl CloudSubscriber {
    pub fn new(settings: CloudSettings, 
               machine_id: String,
               oid: String
                ) -> CloudSubscriber {
        CloudSubscriber { settings, machine_id, oid }
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
    fn subscribe_topics(&self, cli: &mqtt::Client, topics: &[String]) {
        
        for topic in topics {
            // do macro substitution
            let topic = topic
                .replace("{ID}", &self.machine_id)
                .replace("{OID}", &self.oid)
                .trim()
                .to_string();

            println!("Subscribing to '{}'", topic);

            match cli.subscribe(&topic, 0) {
                Ok(_) => {
                    println!("Successfully subscribed to topic: '{}'", topic);
                }
                Err(e) => {
                    println!("Error subscribing to topic: '{}' - {:?}", topic, e);
                }
            }
        }
    }

    fn unsubscribe_topics(&self, cli: &mqtt::Client, topics: &Vec<String>) {
        for topic in topics {
            // do macro substitution
            let t = topic
                .replace("{ID}", &self.machine_id)
                .replace("{OID}", &self.oid);

            println!("Unssubscribing from {}", t);

            if let Err(e) = cli.unsubscribe(t.as_str()) {
                println!("Error unsubscribing from {} {:?}", t, e);
            }
        }
    }

    pub fn start(&mut self, sender: Sender<UpdateDescriptor>, state_sender: Sender<UpdateState>, jwt: String, oid: String) { 
        let host = format!("{}:{}", self.settings.update_server_address, self.settings.update_server_port);
        self.oid = oid;

        // Define the set of options for the create.
        // Use an ID for a persistent session.
        let create_opts = mqtt::CreateOptionsBuilder::new()
            .server_uri(&host)
            .mqtt_version(mqtt::MQTT_VERSION_5)
            .client_id(DFLT_CLIENT.to_string())
            .finalize();

        // Create a client.
        let client = mqtt::Client::new(create_opts)
            .unwrap_or_else(|err| {
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

        let mut ssl_builder = mqtt::SslOptionsBuilder::new();
        if let Err(e) = ssl_builder.trust_store("/etc/ssl/ca-certificates.crt") {
            println!("WARNING: Failed to set SSL trust store: {}. Using defaults.", e);
        }
        let ssl_options = ssl_builder.finalize();

        let conn_opts = mqtt::ConnectOptionsBuilder::new()
            .keep_alive_interval(Duration::from_secs(20))
            .clean_session(false)
            .will_message(lwt)
            .user_name(self.machine_id.clone())
            .password(jwt)
            .ssl_options(ssl_options)
            .finalize();

        loop {
            println!("making MQTT connection to {}:\n", host);
            match client.connect(conn_opts.clone()) {
                Ok(response) => {
                    println!("MQTT connection succeeded! Response: {:?}", response);
                    break;
                },
                Err(e) => {
                    println!("MQTT connection failed: {}\n", e);
                    println!("Retrying in {} seconds...", self.settings.connect_retry_seconds);
                    thread::sleep(Duration::from_secs(self.settings.connect_retry_seconds));
                }
            }
        }

        println!("Sending Connected state to UpdateService...");
        if let Err(e) = state_sender.send(UpdateState::Connected) {
            println!("ERROR: Failed to send Connected state: {}", e);
        } else {
            println!("Connected state sent successfully");
        }

        // Subscribe to topics.
        self.subscribe_topics(&client, &self.settings.mqtt_topics);

        println!("Processing requests... waiting for MQTT messages");
        println!("Subscribed to topics: {:?}", self.settings.mqtt_topics);

        loop {
            match receiver.recv() {
                Ok(Some(msg)) => {
                    println!("\n>>> MQTT MESSAGE RECEIVED <<<");
                    println!("Topic: {}", msg.topic());
                    println!("Payload: {}", msg.payload_str());
                    println!("QoS: {:?}", msg.qos());

                    // Process the message here
                    match UpdateParser::parse_message(msg.payload_str().as_ref()) {
                        Ok(update) => {
                            println!("Successfully parsed update: {:?}", update);
                            // pass the descriptor back to the update service
                            if let Err(e) = sender.send(update) {
                                println!("ERROR: Failed to send update descriptor: {}", e);
                            } else {
                                println!("Update descriptor sent to UpdateService");
                            }
                        }
                        Err(e) => {
                            println!("ERROR: Failed to parse update message: {}", e);
                        }
                    }
                }
                Ok(None) => {
                    println!("Receiver returned None (timeout or spurious wakeup)");

                    if !client.is_connected() {
                        if self.try_reconnect(&client) {
                            println!("Resubscribe to topics...");
                            self.subscribe_topics(&client, &self.settings.mqtt_topics);
                        } else {
                            break;
                        }
                    }
                }
                Err(err) => {
                    println!("Error receiving message: {:?}", err);
                    break; // Optionally break out of the loop on error
                }
            }
        }

        for msg in receiver.iter() {
            if let Some(msg) = msg {
                println!("Received message: {:?}", msg);

            }
            else {
            }
        }

        // If still connected, then disconnect now.
        if client.is_connected() {
            println!("Disconnecting");
            self.unsubscribe_topics(&client, &self.settings.mqtt_topics);
            if let Err(e) = client.disconnect(None) {
                println!("ERROR: Failed to disconnect cleanly: {}", e);
            }
        }
        println!("Exiting");
            
    }
}