use cosmwasm_std::{Attribute, Event};

#[cfg(test)]
pub mod treasurechest;

pub fn get_events(ty: &str, events: &[Event]) -> Vec<Event> {
    events.iter().filter(|e| e.ty == ty).cloned().collect()
}

pub fn get_attribute(key: &str, attributes: &[Attribute]) -> Option<String> {
    attributes.iter().find(|a| a.key == key).map(|a| a.value.clone())
}
