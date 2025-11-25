use std::collections;

use crate::{
    common,
    event::{self, EventTrait},
};

pub struct Graph {
    events: collections::HashMap<common::Hash, event::Event>,
}

impl Graph {
    pub fn new(id: u64, timestamp: u64) -> Graph {
        let initial_event = event::Event::Initial(event::Initial::new(id, timestamp));
        let mut events = collections::HashMap::new();
        events.insert(initial_event.hash(), initial_event);

        return Graph { events };
    }

    pub fn events_as_bytes(&self) -> Vec<u8> {
        self.events
            .values()
            .flat_map(|event| event.as_bytes())
            .collect()
    }

    pub fn insert_event(&mut self, event: event::Event) {
        self.events.insert(event.hash(), event);
    }

    pub fn update(&mut self, events: Vec<event::Event>) {
        events
            .into_iter()
            .for_each(|event| self.insert_event(event));
    }

    pub fn latest_event_by_peer(&self, peer: u64) -> Option<(&common::Hash, &event::Event)> {
        self.events
            .iter()
            .filter(|(hash, _)| self.get_event_peer(&hash) == peer)
            .max_by_key(|(_, event)| event.timestamp())
    }

    fn get_event_peer(&self, event_hash: &common::Hash) -> u64 {
        let mut current_hash = event_hash;
        loop {
            let event = self
                .events
                .get(current_hash)
                .expect("event with the given hash not in graph");
            match event {
                event::Event::Initial(initial) => return initial.peer,
                event::Event::Default(default) => {
                    current_hash = &default.self_parent;
                }
            }
        }
    }

    pub fn as_json(&self) -> String {
        let mut entries: Vec<String> = Vec::new();

        for (hash, event) in &self.events {
            let hash_hex = common::bytes_to_hex(hash);
            let event_json = match event {
                event::Event::Initial(initial) => {
                    format!(
                        r#"{{"kind":"initial","timestamp":{},"peer":{}}}"#,
                        initial.timestamp, initial.peer
                    )
                }
                event::Event::Default(default) => {
                    format!(
                        r#"{{"kind":"default","timestamp":{},"transactions":"{}","self_parent":"{}","other_parent":"{}"}}"#,
                        default.timestamp,
                        common::bytes_to_hex(&default.transactions),
                        common::bytes_to_hex(&default.self_parent),
                        common::bytes_to_hex(&default.other_parent)
                    )
                }
            };
            entries.push(format!(r#""{}": {}"#, hash_hex, event_json));
        }

        format!("{{{}}}", entries.join(","))
    }
}
