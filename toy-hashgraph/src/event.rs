use sha2::Digest;

use crate::common;

pub trait EventTrait {
    fn variant_id(&self) -> u8;

    fn as_bytes(&self) -> Vec<u8>;

    fn from_bytes(bytes: &[u8]) -> (Self, usize)
    where
        Self: Sized;

    fn hash(&self) -> common::Hash {
        let hashed = sha2::Sha256::digest(self.as_bytes());
        assert_eq!(hashed.len(), common::HASH_SIZE);

        let mut buffer = [0; common::HASH_SIZE];
        buffer.copy_from_slice(hashed.as_slice());

        buffer
    }

    fn timestamp(&self) -> u64;
}

pub enum Event {
    Initial(Initial),
    Default(Default),
}

impl EventTrait for Event {
    fn variant_id(&self) -> u8 {
        match self {
            Event::Initial(initial) => initial.variant_id(),
            Event::Default(default) => default.variant_id(),
        }
    }

    fn as_bytes(&self) -> Vec<u8> {
        let mut buffer = Vec::new();
        buffer.push(self.variant_id());
        match self {
            Event::Initial(initial) => buffer.append(&mut initial.as_bytes()),
            Event::Default(default) => buffer.append(&mut default.as_bytes()),
        };
        buffer
    }

    fn from_bytes(bytes: &[u8]) -> (Self, usize) {
        let (&variant, payload) = bytes
            .split_first()
            .expect("event bytes must include variant id");
        let (event, consumed) = match variant {
            0 => {
                let (initial, len) = Initial::from_bytes(payload);
                (Event::Initial(initial), len)
            }
            1 => {
                let (default, len) = Default::from_bytes(payload);
                (Event::Default(default), len)
            }
            _ => panic!("unreachable event variant id: {}", variant),
        };
        (event, 1 + consumed)
    }

    fn timestamp(&self) -> u64 {
        match self {
            Event::Initial(initial) => initial.timestamp(),
            Event::Default(default) => default.timestamp(),
        }
    }
}

pub struct Initial {
    pub timestamp: u64,
    pub peer: u64,
}

impl Initial {
    pub fn new(peer: u64, timestamp: u64) -> Initial {
        Initial { timestamp, peer }
    }
}

impl Event {
    pub fn events_from_bytes(data: &[u8]) -> Vec<Event> {
        let mut cursor = data;
        let mut events = Vec::new();

        while !cursor.is_empty() {
            let (event, consumed) = Event::from_bytes(cursor);
            events.push(event);
            cursor = &cursor[consumed..];
        }

        events
    }
}

impl EventTrait for Initial {
    fn variant_id(&self) -> u8 {
        0
    }

    fn as_bytes(&self) -> Vec<u8> {
        let mut buffer = Vec::new();
        buffer.extend_from_slice(&self.timestamp.to_le_bytes());
        buffer.extend_from_slice(&self.peer.to_le_bytes());

        buffer
    }

    fn from_bytes(bytes: &[u8]) -> (Initial, usize) {
        assert!(
            bytes.len() >= 16,
            "initial event bytes must include timestamp and peer"
        );

        let timestamp =
            u64::from_le_bytes(bytes[0..8].try_into().expect("slice sized for timestamp"));
        let peer = u64::from_le_bytes(bytes[8..16].try_into().expect("slice sized for id"));

        (Initial { peer, timestamp }, 16)
    }

    fn timestamp(&self) -> u64 {
        self.timestamp
    }
}

pub struct Default {
    pub timestamp: u64,
    pub transactions: Vec<u8>,
    pub self_parent: common::Hash,
    pub other_parent: common::Hash,
}

impl Default {
    pub fn new(
        timestamp: u64,
        transactions: Vec<u8>,
        self_parent: common::Hash,
        other_parent: common::Hash,
    ) -> Default {
        return Default {
            timestamp,
            transactions,
            self_parent,
            other_parent,
        };
    }
}

impl EventTrait for Default {
    fn variant_id(&self) -> u8 {
        1
    }

    fn as_bytes(&self) -> Vec<u8> {
        let mut buffer = Vec::new();
        buffer.extend_from_slice(&self.timestamp.to_le_bytes());

        buffer.extend_from_slice(&(self.transactions.len() as u64).to_le_bytes());
        buffer.extend(self.transactions.iter());

        buffer.extend_from_slice(&self.self_parent);
        buffer.extend_from_slice(&self.other_parent);

        buffer
    }

    fn from_bytes(bytes: &[u8]) -> (Default, usize) {
        assert!(
            bytes.len() >= 16,
            "default event bytes must include timestamp and tx len"
        );

        let timestamp =
            u64::from_le_bytes(bytes[0..8].try_into().expect("slice sized for timestamp"));
        let tx_len = u64::from_le_bytes(bytes[8..16].try_into().expect("slice sized for tx len"));
        let tx_len_usize = usize::try_from(tx_len).expect("transaction length must fit into usize");

        let expected_len = 16 + tx_len_usize + (2 * common::HASH_SIZE);
        assert!(
            bytes.len() >= expected_len,
            "default event bytes truncated: expected {}, found {}",
            expected_len,
            bytes.len()
        );

        let tx_start = 16;
        let tx_end = tx_start + tx_len_usize;
        let parents_start = tx_end;
        let other_parent_start = parents_start + common::HASH_SIZE;

        let transactions = bytes[tx_start..tx_end].to_vec();

        let mut self_parent = [0u8; common::HASH_SIZE];
        self_parent.copy_from_slice(&bytes[parents_start..parents_start + common::HASH_SIZE]);

        let mut other_parent = [0u8; common::HASH_SIZE];
        other_parent
            .copy_from_slice(&bytes[other_parent_start..other_parent_start + common::HASH_SIZE]);

        (
            Default {
                timestamp,
                transactions,
                self_parent,
                other_parent,
            },
            expected_len,
        )
    }

    fn timestamp(&self) -> u64 {
        self.timestamp
    }
}
