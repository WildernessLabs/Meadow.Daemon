use crate::update_descriptor::{ UpdateDescriptor};

pub struct UpdateParser;

impl UpdateParser {
    pub fn parse_message(message_json: &str) -> UpdateDescriptor {
        println!("parsing: {}", message_json);
        let desc = UpdateDescriptor::from_json(message_json);
        desc
    }    
}