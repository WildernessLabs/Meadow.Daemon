use std::fs::{read_to_string};

#[derive(Clone)]
pub struct CloudSettings {
    pub enabled: bool,
    pub update_server_address: String,
    pub update_server_port: i32,
    pub use_authentication: bool,
    pub auth_server_address: Option<String>,
    pub auth_server_port: Option<i32>,
    pub mqtt_topics: Vec<String>,
    pub connect_retry_seconds: u64
}

impl CloudSettings {
    pub fn from_file(path: &str) -> CloudSettings {
        // set up defaults
        let mut settings = CloudSettings{
            enabled: true,
            update_server_address: "".to_string(),
            update_server_port: 883,
            use_authentication: false,
            auth_server_address: None,
            auth_server_port: None,
             mqtt_topics: vec!["ota".to_string(), "ota/{ID}/updates".to_string()],
             connect_retry_seconds: 15
        };

        let lines = CloudSettings::read_lines(path);
        for line in lines {

            let s = line
                .chars()
                .take_while(|&ch| ch != '#')
                .collect::<String>();
            
            if s.len() > 0 {
                let key = &s[..s.find(' ').unwrap()]
                    .to_lowercase();
                let val = &s[s.find(' ').unwrap()..]
                    .trim()
                    .to_string();

                match key.as_str() {
                    "enabled" => 
                    {
                        settings.enabled = val.to_lowercase() == "yes";
                    },
                    "update_server_address" =>
                    {
                        settings.update_server_address = val.into();
                    },
                    "update_server_port" =>
                    {
                        settings.update_server_port = val.parse::<i32>().unwrap();
                    },
                    "use_authentication" =>
                    {
                        settings.use_authentication = val.to_lowercase() == "yes";
                    },
                    "auth_server_address" =>
                    {
                        settings.auth_server_address = Some(val.into());
                    },
                    "auth_server_port" =>
                    {
                        settings.auth_server_port = Some(val.parse::<i32>().unwrap());
                    },
                    "mqtt_topics" =>
                    {
                        settings.mqtt_topics = val.split(';').map(String::from).collect();
                    },
                    "connect_retry_seconds" =>
                    {
                        settings.connect_retry_seconds = val.parse::<u64>().unwrap();
                    },
                    _ => 
                    {
                        println!("unknown setting '{}'", s);
                        // unknown setting
                    }

                }
            }
        }


        settings
    }

    fn read_lines(filename: &str) -> Vec<String> {
        read_to_string(filename) 
            .unwrap()  // panic on possible file-reading errors
            .lines()  // split the string into an iterator of string slices
            .map(String::from)  // make each slice into a string
            .collect()  // gather them together into a vector
    }
}