use gt_common::events::GameEvent;
use gt_common::types::Tick;

#[derive(Debug, Default)]
pub struct EventQueue {
    events: Vec<(Tick, GameEvent)>,
}

impl EventQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, tick: Tick, event: GameEvent) {
        self.events.push((tick, event));
    }

    pub fn drain(&mut self) -> Vec<(Tick, GameEvent)> {
        std::mem::take(&mut self.events)
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_queue_push_drain() {
        let mut queue = EventQueue::new();
        assert!(queue.is_empty());

        queue.push(
            1,
            GameEvent::MarketShiftOccurred {
                description: "test".to_string(),
            },
        );
        assert_eq!(queue.len(), 1);

        let events = queue.drain();
        assert_eq!(events.len(), 1);
        assert!(queue.is_empty());
    }
}
