use std::{cmp, collections};

use serde::{Deserialize, Serialize};

use crate::{
    common,
    event::{self, EventTrait},
};

#[derive(Clone, Serialize, Deserialize)]
pub struct Graph {
    pub total_peers: usize,
    #[serde(with = "hash_map_hex_keys")]
    events: collections::HashMap<common::Hash, event::Event>,
}

/// Serde module for serializing HashMap<[u8; 32], V> with hex string keys
mod hash_map_hex_keys {
    use crate::common;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::HashMap;

    pub fn serialize<S, V>(map: &HashMap<common::Hash, V>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        V: Serialize,
    {
        let string_map: HashMap<String, &V> = map
            .iter()
            .map(|(k, v)| {
                let hex: String = k.iter().map(|b| format!("{:02x}", b)).collect();
                (hex, v)
            })
            .collect();
        string_map.serialize(serializer)
    }

    pub fn deserialize<'de, D, V>(deserializer: D) -> Result<HashMap<common::Hash, V>, D::Error>
    where
        D: Deserializer<'de>,
        V: Deserialize<'de>,
    {
        let string_map: HashMap<String, V> = HashMap::deserialize(deserializer)?;
        let mut result = HashMap::new();
        for (hex, value) in string_map {
            if hex.len() != 64 {
                return Err(serde::de::Error::custom(
                    "hash hex string must be 64 characters",
                ));
            }
            let mut hash = [0u8; common::HASH_SIZE];
            let mut chars = hex.chars();
            for byte in hash.iter_mut() {
                let a = chars
                    .next()
                    .ok_or_else(|| serde::de::Error::custom("unexpected end"))?;
                let b = chars
                    .next()
                    .ok_or_else(|| serde::de::Error::custom("unexpected end"))?;
                *byte = u8::from_str_radix(&format!("{}{}", a, b), 16)
                    .map_err(serde::de::Error::custom)?;
            }
            result.insert(hash, value);
        }
        Ok(result)
    }
}

impl Graph {
    pub fn new(id: u64, timestamp: u64, total_peers: usize) -> Graph {
        let initial_event = event::Event::Initial(event::Initial::new(id, timestamp));
        let mut events = collections::HashMap::new();
        events.insert(initial_event.hash(), initial_event);

        return Graph {
            total_peers,
            events,
        };
    }

    pub fn as_json(&self) -> String {
        serde_json::to_string(self).expect("failed to serialize graph to JSON")
    }

    pub fn from_json(json: &str) -> Graph {
        serde_json::from_str(json).expect("failed to deserialize graph from JSON")
    }

    pub fn is_supermajority(&self, count: usize) -> bool {
        3 * count > 2 * self.total_peers
    }

    pub fn get_event(&self, event_hash: &common::Hash) -> &event::Event {
        self.events
            .get(event_hash)
            .expect("event with the given hash not in graph")
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

    pub fn latest_event(&self, peer: u64) -> Option<&event::Event> {
        self.events
            .iter()
            .filter(|(hash, _)| {
                self.creator(hash) == peer
                    && self.events.values().all(|e| match e {
                        event::Event::Initial(_) => true,
                        event::Event::Default(d) => d.self_parent != **hash,
                    })
            })
            .map(|(_, event)| event)
            .next()
    }

    pub fn creator(&self, event_hash: &common::Hash) -> u64 {
        let mut current_hash = event_hash;
        loop {
            match self.get_event(current_hash) {
                event::Event::Initial(initial) => return initial.peer,
                event::Event::Default(default) => {
                    current_hash = &default.self_parent;
                }
            }
        }
    }

    /// x ≤ y
    pub fn is_ancestor(&self, x: &common::Hash, y: &common::Hash) -> bool {
        if x == y {
            true
        } else {
            match self.get_event(y) {
                event::Event::Initial(_) => false,
                event::Event::Default(default) => {
                    self.is_ancestor(x, &default.self_parent)
                        || self.is_ancestor(x, &default.other_parent)
                }
            }
        }
    }

    /// x < y
    pub fn is_strict_ancestor(&self, x: &common::Hash, y: &common::Hash) -> bool {
        x != y && self.is_ancestor(x, y)
    }

    /// x ⊑ y
    pub fn is_self_ancestor(&self, x: &common::Hash, y: &common::Hash) -> bool {
        let mut current_hash = y;
        while current_hash != x {
            match self.get_event(current_hash) {
                event::Event::Initial(_) => return false,
                event::Event::Default(default) => {
                    current_hash = &default.self_parent;
                }
            }
        }

        true
    }

    /// x ⊏ y
    pub fn is_strict_self_ancestor(&self, x: &common::Hash, y: &common::Hash) -> bool {
        x != y && self.is_self_ancestor(x, y)
    }

    pub fn is_fork(&self, x: &common::Hash, y: &common::Hash) -> bool {
        self.creator(x) == self.creator(y)
            && !self.is_self_ancestor(x, y)
            && !self.is_self_ancestor(y, x)
    }

    pub fn can_see_dishonesty(&self, event_hash: &common::Hash, peer: u64) -> bool {
        let peer_events = self
            .events
            .keys()
            .filter(|e| self.creator(e) == peer && self.is_ancestor(e, event_hash))
            .collect::<Vec<_>>();

        for i in 0..peer_events.len() {
            for j in 0..peer_events.len() {
                if i == j {
                    continue;
                }

                if self.is_fork(peer_events[i], peer_events[j]) {
                    return true;
                }
            }
        }

        return false;
    }

    /// x ⊴ y
    pub fn sees(&self, x: &common::Hash, y: &common::Hash) -> bool {
        self.is_ancestor(x, y) && !self.can_see_dishonesty(y, self.creator(x))
    }

    /// x ≪ y
    pub fn strongly_sees(&self, x: &common::Hash, y: &common::Hash) -> bool {
        let events = self
            .events
            .keys()
            .filter(|z| self.is_ancestor(z, y) && self.sees(x, z))
            .map(|z| self.creator(z))
            .collect::<collections::HashSet<_>>();

        return self.is_supermajority(events.len());
    }

    pub fn round(&self, event_hash: &common::Hash) -> u64 {
        let event = self.get_event(event_hash);

        match event {
            event::Event::Initial(_) => 0,
            event::Event::Default(default) => {
                let i = u64::max(
                    self.round(&default.self_parent),
                    self.round(&default.other_parent),
                );

                let base_round_event_peers = self
                    .events
                    .keys()
                    .filter(|e| self.strongly_sees(e, event_hash) && self.round(e) == i)
                    .map(|e| self.creator(e))
                    .collect::<collections::HashSet<_>>();

                if self.is_supermajority(base_round_event_peers.len()) {
                    i + 1
                } else {
                    i
                }
            }
        }
    }

    pub fn witnesses(&self, round: u64) -> Vec<common::Hash> {
        self.events
            .keys()
            .filter(|e| {
                self.round(e) == round
                    && match self.get_event(e) {
                        event::Event::Initial(_) => round == 0,
                        event::Event::Default(d) => self.round(&d.self_parent) != round,
                    }
            })
            .map(|e| e.clone())
            .collect::<Vec<_>>()
    }

    fn is_famous(&self, candidate: &common::Hash) -> bool {
        let mut round = self.round(&candidate) + 1;
        let mut previous_voters = self.witnesses(round);
        let mut previous_votes = previous_voters
            .iter()
            .map(|voter| self.is_strict_ancestor(&candidate, voter))
            .collect::<Vec<_>>();

        loop {
            round += 1;
            let voters = self.witnesses(round);
            if voters.len() == 0 {
                return false; // Should lead to coin round but I'm not implementing that
            }
            let mut votes = Vec::new();

            for voter in voters.iter() {
                let observed_votes = previous_voters
                    .iter()
                    .zip(previous_votes.iter())
                    .filter(|(previous_voter, _)| self.strongly_sees(previous_voter, voter))
                    .map(|(_, pv)| pv);

                let yes_count = observed_votes.clone().filter(|v| **v).count();
                let no_count = observed_votes.clone().filter(|v| !*v).count();

                if self.is_supermajority(yes_count) {
                    return true;
                }

                if self.is_supermajority(no_count) {
                    return false;
                }

                votes.push(yes_count >= no_count);
            }

            previous_voters = voters;
            previous_votes = votes;
        }
    }

    pub fn famous_witnesses(&self, round: u64) -> Vec<common::Hash> {
        self.witnesses(round)
            .into_iter()
            .filter(|w| self.is_famous(w))
            .collect::<Vec<_>>()
    }

    pub fn unique_famous_witnesses(&self, round: u64) -> Vec<common::Hash> {
        let mut map = collections::HashMap::new();
        for famous_witness in self.famous_witnesses(round) {
            let peer = self.creator(&famous_witness);

            if !map.contains_key(&peer) {
                map.insert(peer, Vec::new());
            }

            map.get_mut(&peer)
                .expect("i just inserted it")
                .push(famous_witness);
        }

        map.into_values()
            .map(|ufw_vec| {
                ufw_vec
                    .into_iter()
                    .min()
                    .expect("there should at least be one or the entry would not exist")
            })
            .collect::<Vec<_>>()
    }

    pub fn round_received(&self, event_hash: &common::Hash) -> Option<u64> {
        let mut round = self.round(event_hash);

        loop {
            let unique_famous_witnesses = self.unique_famous_witnesses(round);

            if unique_famous_witnesses.len() == 0 {
                return None;
            }

            if unique_famous_witnesses
                .iter()
                .all(|famous_witness| self.is_ancestor(event_hash, famous_witness))
            {
                return Some(round);
            }
            round += 1
        }
    }

    pub fn consensus_timestamp(&self, event_hash: &common::Hash) -> Option<u64> {
        let round = self.round_received(event_hash)?;

        let mut timestamps = self
            .unique_famous_witnesses(round)
            .into_iter()
            .map(|witness| {
                let mut current = witness;
                let mut last_with_x = None;

                loop {
                    if self.is_ancestor(event_hash, &current) {
                        last_with_x = Some(current);
                    }

                    match self.get_event(&current) {
                        event::Event::Initial(_) => {
                            let z = last_with_x.unwrap_or(current);
                            return self.get_event(&z).timestamp();
                        }
                        event::Event::Default(ev) => {
                            if !self.is_ancestor(event_hash, &current) && last_with_x.is_some() {
                                let z = last_with_x.unwrap();
                                return self.get_event(&z).timestamp();
                            }
                            current = ev.self_parent;
                        }
                    }
                }
            })
            .collect::<Vec<_>>();
        timestamps.sort();

        Some(
            *timestamps
                .get((timestamps.len() - 1) / 2)
                .expect("there should at least be one candidate timestamp"),
        )
    }

    pub fn consensus_ordering(&self, a: common::Hash, b: common::Hash) -> Option<cmp::Ordering> {
        let a_round = self.round_received(&a)?;
        let b_round = self.round_received(&b)?;
        let a_time = self.consensus_timestamp(&a)?;
        let b_time = self.consensus_timestamp(&b)?;

        Some(
            a_round
                .cmp(&b_round)
                .then(a_time.cmp(&b_time))
                .then(a.cmp(&b)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{self, EventTrait};
    use std::collections::HashMap;

    const ALICE: u64 = 1;
    const BOB: u64 = 2;
    const CATHY: u64 = 3;
    const DAVE: u64 = 4;

    /// Build the hashgraph from Figure 1 in the paper and
    /// return it together with a mapping from labels (A1, B3, ...)
    /// to their corresponding event hashes.
    fn build_figure1_graph() -> (Graph, HashMap<&'static str, common::Hash>) {
        let mut graph = Graph::new(ALICE, 0, 4);
        let mut labels: HashMap<&'static str, common::Hash> = HashMap::new();

        // Initial events
        let a1 = event::Event::Initial(event::Initial::new(ALICE, 0));
        let a1_hash = a1.hash();
        labels.insert("A1", a1_hash);

        let b1 = event::Event::Initial(event::Initial::new(BOB, 0));
        let b1_hash = b1.hash();
        graph.insert_event(b1);
        labels.insert("B1", b1_hash);

        let c1 = event::Event::Initial(event::Initial::new(CATHY, 0));
        let c1_hash = c1.hash();
        graph.insert_event(c1);
        labels.insert("C1", c1_hash);

        let d1 = event::Event::Initial(event::Initial::new(DAVE, 0));
        let d1_hash = d1.hash();
        graph.insert_event(d1);
        labels.insert("D1", d1_hash);

        // Non-initial events, following the description in the doc:
        //
        // Dave sent D1 to Cathy -> C2
        let c2 = event::Event::Default(event::Default::new(1, Vec::new(), c1_hash, d1_hash));
        let c2_hash = c2.hash();
        graph.insert_event(c2);
        labels.insert("C2", c2_hash);

        // Cathy sent C2 to Dave -> D2
        let d2 = event::Event::Default(event::Default::new(1, Vec::new(), d1_hash, c2_hash));
        let d2_hash = d2.hash();
        graph.insert_event(d2);
        labels.insert("D2", d2_hash);

        // Bob sent B1 to Alice -> A2
        let a2 = event::Event::Default(event::Default::new(1, Vec::new(), a1_hash, b1_hash));
        let a2_hash = a2.hash();
        graph.insert_event(a2);
        labels.insert("A2", a2_hash);

        // Bob sent B1 to Cathy -> C3
        let c3 = event::Event::Default(event::Default::new(2, Vec::new(), c2_hash, b1_hash));
        let c3_hash = c3.hash();
        graph.insert_event(c3);
        labels.insert("C3", c3_hash);

        // Alice sent A1 to Bob -> B2
        let b2 = event::Event::Default(event::Default::new(1, Vec::new(), b1_hash, a1_hash));
        let b2_hash = b2.hash();
        graph.insert_event(b2);
        labels.insert("B2", b2_hash);

        // Alice sent A2 to Bob -> B3
        let b3 = event::Event::Default(event::Default::new(2, Vec::new(), b2_hash, a2_hash));
        let b3_hash = b3.hash();
        graph.insert_event(b3);
        labels.insert("B3", b3_hash);

        // Cathy sent C3 to Bob -> B4
        let b4 = event::Event::Default(event::Default::new(3, Vec::new(), b3_hash, c3_hash));
        let b4_hash = b4.hash();
        graph.insert_event(b4);
        labels.insert("B4", b4_hash);

        // Dave sent D2 to Bob -> B5
        let b5 = event::Event::Default(event::Default::new(4, Vec::new(), b4_hash, d2_hash));
        let b5_hash = b5.hash();
        graph.insert_event(b5);
        labels.insert("B5", b5_hash);

        // Graph should now contain A1 (from new) plus the 11 events added above.
        assert_eq!(graph.events.len(), 12);

        (graph, labels)
    }

    #[test]
    fn graph_new_and_supermajority() {
        let graph = Graph::new(ALICE, 0, 4);

        // new() should create a single initial event
        assert_eq!(graph.events.len(), 1);

        // For 4 peers, supermajority means at least 3.
        assert!(!graph.is_supermajority(0));
        assert!(!graph.is_supermajority(1));
        assert!(!graph.is_supermajority(2));
        assert!(graph.is_supermajority(3));
        assert!(graph.is_supermajority(4));
    }

    #[test]
    fn creators_and_latest_events_match_figure1() {
        let (graph, labels) = build_figure1_graph();

        // Creator for each labeled event
        let expected_creators = [
            ("A1", ALICE),
            ("A2", ALICE),
            ("B1", BOB),
            ("B2", BOB),
            ("B3", BOB),
            ("B4", BOB),
            ("B5", BOB),
            ("C1", CATHY),
            ("C2", CATHY),
            ("C3", CATHY),
            ("D1", DAVE),
            ("D2", DAVE),
        ];

        for (label, peer) in expected_creators {
            let hash = labels.get(label).expect("missing event label");
            assert_eq!(graph.creator(hash), peer, "creator of {}", label);
        }

        // Latest events by peer correspond to the top of each column
        let latest_alice = graph.latest_event(ALICE).unwrap().hash();
        let latest_bob = graph.latest_event(BOB).unwrap().hash();
        let latest_cathy = graph.latest_event(CATHY).unwrap().hash();
        let latest_dave = graph.latest_event(DAVE).unwrap().hash();

        assert_eq!(latest_alice, *labels.get("A2").unwrap());
        assert_eq!(latest_bob, *labels.get("B5").unwrap());
        assert_eq!(latest_cathy, *labels.get("C3").unwrap());
        assert_eq!(latest_dave, *labels.get("D2").unwrap());
    }

    #[test]
    fn ancestor_and_self_ancestor_relations_follow_figure1() {
        let (graph, labels) = build_figure1_graph();

        let a1 = labels["A1"];
        let a2 = labels["A2"];
        let b1 = labels["B1"];
        let b2 = labels["B2"];
        let b5 = labels["B5"];
        let c1 = labels["C1"];
        let c3 = labels["C3"];
        let d1 = labels["D1"];
        let d2 = labels["D2"];

        // Self-ancestor chains on each peer
        assert!(graph.is_self_ancestor(&a1, &a2));
        assert!(graph.is_strict_self_ancestor(&a1, &a2));
        assert!(!graph.is_self_ancestor(&a2, &a1));

        assert!(graph.is_self_ancestor(&b1, &b5));
        assert!(graph.is_strict_self_ancestor(&b2, &b5));
        assert!(!graph.is_self_ancestor(&b5, &b1));

        assert!(graph.is_self_ancestor(&c1, &c3));
        assert!(graph.is_self_ancestor(&d1, &d2));

        // General ancestor relationships from the figure
        assert!(graph.is_ancestor(&b1, &b5));
        assert!(graph.is_strict_ancestor(&b1, &b5));
        assert!(graph.is_ancestor(&c1, &b5));
        assert!(graph.is_ancestor(&d1, &b5));

        // Negative cases: no cross-column ancestry where there is no path
        assert!(!graph.is_ancestor(&a2, &c1));
        assert!(!graph.is_ancestor(&c1, &a1));
        assert!(!graph.is_ancestor(&d2, &b1));
    }

    #[test]
    fn no_forks_and_no_seen_dishonesty_in_figure1_graph() {
        let (graph, labels) = build_figure1_graph();

        // In the example all peers are honest, so there should be no forks.
        let same_peer_pairs = [
            ("A1", "A2"),
            ("B1", "B2"),
            ("B3", "B4"),
            ("C1", "C3"),
            ("D1", "D2"),
        ];

        for (x_label, y_label) in same_peer_pairs {
            let x = labels[x_label];
            let y = labels[y_label];
            assert!(
                !graph.is_fork(&x, &y),
                "unexpected fork between {} and {}",
                x_label,
                y_label
            );
        }

        // Because there are no forks, no event should be able to see dishonesty.
        let peers = [ALICE, BOB, CATHY, DAVE];
        for (label, hash) in labels.iter() {
            for peer in peers {
                assert!(
                    !graph.can_see_dishonesty(hash, peer),
                    "event {} should not see dishonesty for peer {}",
                    label,
                    peer
                );
            }
        }
    }

    #[test]
    fn sees_and_strongly_sees_match_paper_examples() {
        let (graph, labels) = build_figure1_graph();

        let a1 = labels["A1"];
        let b1 = labels["B1"];
        let c1 = labels["C1"];
        let d1 = labels["D1"];
        let b4 = labels["B4"];
        let b5 = labels["B5"];
        let c2 = labels["C2"];
        let c3 = labels["C3"];
        let d2 = labels["D2"];

        // With no forks, "sees" is equivalent to "is_ancestor" in this example.
        assert_eq!(graph.sees(&b1, &b4), graph.is_ancestor(&b1, &b4));
        assert_eq!(graph.sees(&c1, &b5), graph.is_ancestor(&c1, &b5));

        // Paper examples for strongly seeing (x ≪ y: y strongly sees x).
        // B4 strongly sees B1 and D1, but not A1 or C1.
        assert!(graph.strongly_sees(&b1, &b4), "B4 should strongly see B1");
        assert!(graph.strongly_sees(&d1, &b4), "B4 should strongly see D1");
        assert!(
            !graph.strongly_sees(&a1, &b4),
            "B4 should not strongly see A1"
        );
        assert!(
            !graph.strongly_sees(&c1, &b4),
            "B4 should not strongly see C1"
        );

        // B5 strongly sees C1 using intermediaries B5, C2, D2.
        assert!(graph.strongly_sees(&c1, &b5), "B5 should strongly see C1");

        // Sanity check that the intermediaries mentioned in the paper are ancestors of B4/B5.
        assert!(graph.is_ancestor(&c2, &b5));
        assert!(graph.is_ancestor(&c3, &b4));
        assert!(graph.is_ancestor(&d2, &b5));
    }

    #[test]
    fn round_zero_for_initial_events() {
        let (graph, labels) = build_figure1_graph();

        // The paper states that all initial events are in round 0.
        // We only test initial events here to avoid relying on any
        // particular implementation strategy for later rounds.
        let initials = ["A1", "B1", "C1", "D1"];
        for label in initials {
            let hash = labels[label];
            assert_eq!(graph.round(&hash), 0, "round of {} should be 0", label);
        }
    }

    #[test]
    fn rounds_for_all_figure1_events_follow_paper_description() {
        let (graph, labels) = build_figure1_graph();

        // From the paper's description of Figure 1:
        // - All initial events (A1, B1, C1, D1) are in round 0.
        // - Every other event remains in round 0 except B5.
        // - B5 strongly sees B1, C1, and D1 on a supermajority of peers,
        //   so B5 advances to round 1.

        let round0_events = [
            "A1", "B1", "C1", "D1", // initial events
            "A2", "B2", "B3", "B4", "C2", "C3", "D2",
        ];

        for label in round0_events {
            let hash = labels[label];
            assert_eq!(
                graph.round(&hash),
                0,
                "event {} should remain in round 0 according to Figure 1",
                label
            );
        }

        let b5 = labels["B5"];
        assert_eq!(
            graph.round(&b5),
            1,
            "B5 should advance to round 1 according to Figure 1"
        );
    }

    #[test]
    fn witnesses_correspond_to_first_events_in_each_round() {
        let (graph, labels) = build_figure1_graph();

        // Round 0 witnesses are the initial events A1, B1, C1, D1.
        let mut expected_round0 = vec![labels["A1"], labels["B1"], labels["C1"], labels["D1"]];
        let mut actual_round0 = graph.witnesses(0);

        expected_round0.sort();
        actual_round0.sort();

        assert_eq!(
            actual_round0, expected_round0,
            "round 0 witnesses should be A1, B1, C1, and D1"
        );

        // From the paper's Figure 1, B5 is the only event that advances to round 1,
        // so it is the unique witness in that round.
        let mut expected_round1 = vec![labels["B5"]];
        let mut actual_round1 = graph.witnesses(1);

        expected_round1.sort();
        actual_round1.sort();

        assert_eq!(
            actual_round1, expected_round1,
            "round 1 should have B5 as its sole witness"
        );

        // There should be no witnesses beyond round 1 in this small example.
        assert!(
            graph.witnesses(2).is_empty(),
            "no events should inhabit rounds beyond 1 in Figure 1 graph"
        );
    }
}
