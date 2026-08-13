//! The event queue — the entire engine "loop".
//!
//! A binary min-heap ordered by (time, sequence number). The sequence number
//! makes ordering of simultaneous events deterministic: insertion order,
//! which is itself deterministic.

use crate::galaxy::StarId;
use crate::probe::ProbeId;
use crate::time::SimTime;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BinaryHeap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Event {
    /// A probe reaches its destination star.
    ProbeArrival { probe: ProbeId, star: StarId },
    /// Survey finished; decide whether to colonize or move on.
    SurveyComplete { probe: ProbeId, star: StarId },
    /// Autofactory bootstrapped; the system is now a colony.
    FactoryOnline { star: StarId },
    /// A colony finished manufacturing a new probe.
    ReplicaComplete { star: StarId },
    /// A probe was destroyed in transit (dust impact, systems failure).
    ProbeLost { probe: ProbeId, target: StarId },
    /// A civilization's interceptors reach and destroy one of our colonies.
    CivStrike { star: StarId, civ: (i32, i32) },
    /// Light from our probe has reached a civilization's homeworld; they
    /// now know we exist, and answer. Their reply still has to cross to
    /// Sol on its own.
    CivTransmission { civ: (i32, i32) },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Scheduled {
    at: SimTime,
    seq: u64,
    event: Event,
}

impl PartialEq for Scheduled {
    fn eq(&self, other: &Self) -> bool {
        self.at == other.at && self.seq == other.seq
    }
}
impl Eq for Scheduled {}
impl PartialOrd for Scheduled {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Scheduled {
    // Reversed: BinaryHeap is a max-heap, we want earliest first.
    fn cmp(&self, other: &Self) -> Ordering {
        (other.at, other.seq).cmp(&(self.at, self.seq))
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EventQueue {
    heap: BinaryHeap<Scheduled>,
    next_seq: u64,
}

impl EventQueue {
    pub fn schedule(&mut self, at: SimTime, event: Event) {
        let seq = self.next_seq;
        self.next_seq += 1;
        self.heap.push(Scheduled { at, seq, event });
    }

    pub fn peek_time(&self) -> Option<SimTime> {
        self.heap.peek().map(|s| s.at)
    }

    pub fn pop(&mut self) -> Option<(SimTime, Event)> {
        self.heap.pop().map(|s| (s.at, s.event))
    }

    pub fn len(&self) -> usize {
        self.heap.len()
    }

    pub fn is_empty(&self) -> bool {
        self.heap.is_empty()
    }
}
