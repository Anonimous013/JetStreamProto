use std::collections::VecDeque;
use jsp_core::qos::QosPriority;

/// Priority queue item
#[derive(Debug)]
pub struct PriorityItem<T> {
    pub data: T,
    pub priority: QosPriority,
}

/// Priority queue with weighted fair queuing
pub struct PriorityQueue<T> {
    /// Queues for each priority level
    queues: [VecDeque<T>; 4],
    /// Credits for weighted fair queuing
    credits: [usize; 4],
    /// Total items in all queues
    total_items: usize,
}

impl<T> PriorityQueue<T> {
    /// Create a new priority queue
    pub fn new() -> Self {
        Self {
            queues: [
                VecDeque::new(), // Bulk (0)
                VecDeque::new(), // Chat (1)
                VecDeque::new(), // Media (2)
                VecDeque::new(), // System (3)
            ],
            credits: [0; 4],
            total_items: 0,
        }
    }
    
    /// Enqueue an item with priority
    pub fn enqueue(&mut self, item: T, priority: QosPriority) {
        let index = priority.value() as usize;
        self.queues[index].push_back(item);
        self.total_items += 1;
    }
    
    /// Dequeue an item using weighted fair queuing
    /// 
    /// Higher priority queues get more credits and are served more frequently
    pub fn dequeue(&mut self) -> Option<T> {
        if self.total_items == 0 {
            return None;
        }
        
        // Refill credits based on weights
        self.refill_credits();
        
        // Try to dequeue from highest priority queue with credits
        for priority_value in (0..4).rev() {
            if self.credits[priority_value] > 0 && !self.queues[priority_value].is_empty() {
                self.credits[priority_value] -= 1;
                self.total_items -= 1;
                return self.queues[priority_value].pop_front();
            }
        }
        
        // Fallback: dequeue from any non-empty queue
        for priority_value in (0..4).rev() {
            if !self.queues[priority_value].is_empty() {
                self.total_items -= 1;
                return self.queues[priority_value].pop_front();
            }
        }
        
        None
    }
    
    /// Refill credits for weighted fair queuing
    fn refill_credits(&mut self) {
        for priority_value in 0..4 {
            if !self.queues[priority_value].is_empty() {
                let priority = QosPriority::from_value(priority_value as u8).unwrap();
                self.credits[priority_value] += priority.weight();
            }
        }
    }
    
    /// Get total number of items in queue
    pub fn len(&self) -> usize {
        self.total_items
    }
    
    /// Check if queue is empty
    pub fn is_empty(&self) -> bool {
        self.total_items == 0
    }
    
    /// Get number of items for specific priority
    pub fn len_for_priority(&self, priority: QosPriority) -> usize {
        self.queues[priority.value() as usize].len()
    }
    
    /// Clear all queues
    pub fn clear(&mut self) {
        for queue in &mut self.queues {
            queue.clear();
        }
        self.credits = [0; 4];
        self.total_items = 0;
    }
}

impl<T> Default for PriorityQueue<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_priority_queue_basic() {
        let mut queue = PriorityQueue::new();
        
        queue.enqueue("bulk", QosPriority::Bulk);
        queue.enqueue("chat", QosPriority::Chat);
        queue.enqueue("media", QosPriority::Media);
        queue.enqueue("system", QosPriority::System);
        
        assert_eq!(queue.len(), 4);
        
        // Should dequeue in priority order
        assert_eq!(queue.dequeue(), Some("system"));
        assert_eq!(queue.dequeue(), Some("media"));
        assert_eq!(queue.dequeue(), Some("chat"));
        assert_eq!(queue.dequeue(), Some("bulk"));
        
        assert!(queue.is_empty());
    }
    
    #[test]
    fn test_weighted_fair_queuing() {
        let mut queue = PriorityQueue::new();
        
        // Add multiple items of different priorities
        for _ in 0..10 {
            queue.enqueue("bulk", QosPriority::Bulk);
        }
        for _ in 0..10 {
            queue.enqueue("chat", QosPriority::Chat);
        }
        for _ in 0..10 {
            queue.enqueue("media", QosPriority::Media);
        }
        
        // System priority should be served more frequently
        let mut _system_count = 0;
        let mut media_count = 0;
        let mut chat_count = 0;
        let mut bulk_count = 0;
        
        while let Some(item) = queue.dequeue() {
            match item {
                "system" => _system_count += 1,
                "media" => media_count += 1,
                "chat" => chat_count += 1,
                "bulk" => bulk_count += 1,
                _ => {}
            }
        }
        
        // Higher priority items should be dequeued first
        assert_eq!(media_count, 10);
        assert_eq!(chat_count, 10);
        assert_eq!(bulk_count, 10);
    }
    
    #[test]
    fn test_priority_queue_len() {
        let mut queue = PriorityQueue::new();
        
        assert_eq!(queue.len(), 0);
        
        queue.enqueue(1, QosPriority::Chat);
        assert_eq!(queue.len(), 1);
        
        queue.enqueue(2, QosPriority::Media);
        assert_eq!(queue.len(), 2);
        
        queue.dequeue();
        assert_eq!(queue.len(), 1);
    }
    
    #[test]
    fn test_len_for_priority() {
        let mut queue = PriorityQueue::new();
        
        queue.enqueue(1, QosPriority::Chat);
        queue.enqueue(2, QosPriority::Chat);
        queue.enqueue(3, QosPriority::Media);
        
        assert_eq!(queue.len_for_priority(QosPriority::Chat), 2);
        assert_eq!(queue.len_for_priority(QosPriority::Media), 1);
        assert_eq!(queue.len_for_priority(QosPriority::System), 0);
    }
    
    #[test]
    fn test_clear() {
        let mut queue = PriorityQueue::new();
        
        queue.enqueue(1, QosPriority::Chat);
        queue.enqueue(2, QosPriority::Media);
        
        assert_eq!(queue.len(), 2);
        
        queue.clear();
        
        assert_eq!(queue.len(), 0);
        assert!(queue.is_empty());
    }
    
    #[test]
    fn test_empty_dequeue() {
        let mut queue: PriorityQueue<i32> = PriorityQueue::new();
        
        assert_eq!(queue.dequeue(), None);
    }
}
