pub mod common;
pub mod event;
pub mod graph;

use std::collections::HashMap;

use ed25519_dalek::{self, ed25519::signature::SignerMut};

use crate::event::EventTrait;

pub struct Hashgraph {
    pub id: u64,
    pub graph: graph::Graph,
    pub pending_transactions: Vec<u8>,
    pub signer: ed25519_dalek::SigningKey,
    pub verifiers: HashMap<u64, ed25519_dalek::VerifyingKey>,
}

impl Hashgraph {
    pub fn new(
        id: u64,
        timestamp: u64,
        private_key: common::Key,
        public_keys: Vec<(u64, common::Key)>,
    ) -> Hashgraph {
        Hashgraph {
            id,
            graph: graph::Graph::new(id, timestamp, public_keys.len()),
            pending_transactions: Vec::new(),
            signer: ed25519_dalek::SigningKey::from_bytes(&private_key),
            verifiers: public_keys
                .into_iter()
                .map(|(peer, public_key)| {
                    (
                        peer,
                        ed25519_dalek::VerifyingKey::from_bytes(&public_key)
                            .expect("Public Key is not valid"),
                    )
                })
                .collect(),
        }
    }

    pub fn append_transaction(&mut self, transaction: &[u8]) {
        self.pending_transactions.extend_from_slice(transaction);
    }

    pub fn send(&mut self) -> Vec<u8> {
        let mut buffer = Vec::new();

        let data = self.graph.events_as_bytes();

        let signature = self.signer.try_sign(&data).unwrap().to_bytes();
        assert_eq!(signature.len(), common::SIGNATURE_SIZE);

        buffer.extend_from_slice(&self.id.to_le_bytes());
        buffer.extend_from_slice(&signature);
        buffer.extend_from_slice(&data);

        buffer
    }

    pub fn receive(&mut self, data: &[u8], timestamp: u64) {
        let sender_end = 8;
        let signature_end = 8 + common::SIGNATURE_SIZE;

        assert!(
            data.len() > signature_end,
            "a valid incoming payload should have a the sender_id, the signature and at least a byte of payload"
        );

        let sender = u64::from_le_bytes(
            data[..sender_end]
                .try_into()
                .expect("slice sized for sender"),
        );
        let signature = ed25519_dalek::Signature::from_bytes(
            data[sender_end..signature_end]
                .try_into()
                .expect("slice sized for digital signature"),
        );

        let data = data[signature_end..].to_vec();

        if self
            .verifiers
            .get(&sender)
            .expect("did not have public key for sender")
            .verify_strict(&data, &signature)
            .is_err()
        {
            panic!("digital signature failed")
        }

        let events = event::Event::events_from_bytes(&data);

        self.receive_inner(sender, events, timestamp);
    }

    fn receive_inner(&mut self, sender: u64, events: Vec<event::Event>, timestamp: u64) {
        self.graph.update(events);

        let mut transactions = Vec::new();
        transactions.append(&mut self.pending_transactions);

        let self_parent = self
            .graph
            .latest_event(self.id)
            .expect("at least the initial event should be present");
        let other_parent = self
            .graph
            .latest_event(sender)
            .expect("at least the initial event should be present");

        self.graph
            .insert_event(event::Event::Default(event::Default::new(
                timestamp,
                transactions,
                self_parent.hash(),
                other_parent.hash(),
            )));
    }

    pub fn as_json(&self) -> String {
        format!(
            r#"{{"id":{},"transactions":"{}","graph":{}}}"#,
            self.id,
            common::bytes_to_hex(&self.pending_transactions),
            self.graph.as_json()
        )
    }
}
