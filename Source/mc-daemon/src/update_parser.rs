use crate::update_descriptor::UpdateDescriptor;
use anyhow::Result;

pub struct UpdateParser;

impl UpdateParser {
    pub fn parse_message(message_json: &str) -> Result<UpdateDescriptor> {
        println!("parsing: {}", message_json);
        UpdateDescriptor::from_json(message_json)
    }
}