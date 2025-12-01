use serde::{Serialize, Deserialize};
use std::collections::{HashSet, HashMap};
use std::hash::Hash;

/// Last-Write-Wins Register
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LWWRegister<T> {
    value: T,
    timestamp: u64,
    node_id: String,
}

impl<T: Clone> LWWRegister<T> {
    pub fn new(value: T, timestamp: u64, node_id: String) -> Self {
        Self {
            value,
            timestamp,
            node_id,
        }
    }

    pub fn value(&self) -> &T {
        &self.value
    }

    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }

    pub fn merge(&mut self, other: LWWRegister<T>) {
        if other.timestamp > self.timestamp {
            *self = other;
        } else if other.timestamp == self.timestamp {
            // Tie-breaking using node_id for determinism
            if other.node_id > self.node_id {
                *self = other;
            }
        }
    }
}

/// Observed-Remove Set (Add-Wins)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ORSet<T: Eq + Hash> {
    // Element -> Set of unique add tags (timestamps/UUIDs)
    elements: HashMap<T, HashSet<String>>,
}

impl<T: Clone + Eq + Hash + Serialize + for<'a> Deserialize<'a>> ORSet<T> {
    pub fn new() -> Self {
        Self {
            elements: HashMap::new(),
        }
    }

    pub fn add(&mut self, element: T, tag: String) {
        self.elements.entry(element).or_default().insert(tag);
    }

    pub fn remove(&mut self, element: &T, tags: HashSet<String>) {
        if let Some(current_tags) = self.elements.get_mut(element) {
            for tag in tags {
                current_tags.remove(&tag);
            }
            if current_tags.is_empty() {
                self.elements.remove(element);
            }
        }
    }
    
    pub fn contains(&self, element: &T) -> bool {
        self.elements.contains_key(element)
    }

    pub fn elements(&self) -> Vec<T> {
        self.elements.keys().cloned().collect()
    }

    pub fn merge(&mut self, other: ORSet<T>) {
        for (elem, other_tags) in other.elements {
            let my_tags = self.elements.entry(elem).or_default();
            for tag in other_tags {
                my_tags.insert(tag);
            }
        }
    }
    
    // For OR-Set, true merge logic usually involves comparing add sets and remove sets if tracked separately.
    // But for a simple state-based OR-Set where we just exchange the full state of "elements and their tags":
    // The merge of two OR-Sets is the union of their (element, tag) pairs.
    // However, this implementation is strictly additive. To support removal, we need to know what was removed.
    // A proper state-based OR-Set usually keeps tombstones or we assume we are merging "Add" operations.
    // If we want to support "Remove", we typically use a Dot Kernel or similar.
    // For simplicity here, let's implement a standard "Add-Wins" set where we merge by taking the union of tags.
    // Removal is done by generating a "Remove" operation that lists the tags observed at the source.
    // But `merge` here takes another state. If we just union, we can never remove.
    // 
    // Let's stick to a simpler G-Set (Grow-only Set) behavior for `merge` if we don't have tombstones, 
    // OR implement the "observed" part: `remove` takes the tags it sees.
    // But `merge` between two replicas needs to know what was removed.
    // 
    // Actually, standard OR-Set merge:
    // E = E1 U E2
    // But this doesn't account for removal if we just sync states.
    // If we want state-based sync with removal, we need tombstones.
    // Let's add tombstones.
    
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ORSetWithTombstones<T: Eq + Hash> {
    add_set: HashMap<T, HashSet<u64>>, // Element -> Timestamps
    remove_set: HashMap<T, HashSet<u64>>, // Element -> Timestamps
}

impl<T: Clone + Eq + Hash + Serialize + for<'a> Deserialize<'a>> ORSetWithTombstones<T> {
    pub fn new() -> Self {
        Self {
            add_set: HashMap::new(),
            remove_set: HashMap::new(),
        }
    }

    pub fn add(&mut self, element: T, timestamp: u64) {
        self.add_set.entry(element).or_default().insert(timestamp);
    }

    pub fn remove(&mut self, element: T, timestamp: u64) {
        self.remove_set.entry(element).or_default().insert(timestamp);
    }

    pub fn contains(&self, element: &T) -> bool {
        if let Some(adds) = self.add_set.get(element) {
            if let Some(removes) = self.remove_set.get(element) {
                // Element is present if there is an add timestamp NOT in remove set
                adds.difference(removes).next().is_some()
            } else {
                !adds.is_empty()
            }
        } else {
            false
        }
    }

    pub fn merge(&mut self, other: ORSetWithTombstones<T>) {
        for (elem, timestamps) in other.add_set {
            let my_timestamps = self.add_set.entry(elem).or_default();
            my_timestamps.extend(timestamps);
        }
        for (elem, timestamps) in other.remove_set {
            let my_timestamps = self.remove_set.entry(elem).or_default();
            my_timestamps.extend(timestamps);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lww_register() {
        let mut r1 = LWWRegister::new(10, 100, "A".to_string());
        let r2 = LWWRegister::new(20, 200, "B".to_string());
        
        r1.merge(r2.clone());
        assert_eq!(*r1.value(), 20);
        
        let r3 = LWWRegister::new(30, 50, "C".to_string());
        r1.merge(r3);
        assert_eq!(*r1.value(), 20); // Older timestamp ignored
    }

    #[test]
    fn test_orset() {
        let mut s1 = ORSetWithTombstones::new();
        s1.add(String::from("A"), 100);
        s1.add(String::from("B"), 100);
        
        let mut s2 = ORSetWithTombstones::new();
        s2.remove(String::from("A"), 100);
        s2.add(String::from("C"), 200);
        
        s1.merge(s2);
        
        assert!(!s1.contains(&String::from("A"))); // Removed
        assert!(s1.contains(&String::from("B")));  // Kept
        assert!(s1.contains(&String::from("C")));  // Added
    }
}
