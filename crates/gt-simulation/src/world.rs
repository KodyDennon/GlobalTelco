use std::collections::HashMap;

use gt_common::commands::Command;
use gt_common::types::{EntityId, GameSpeed, Tick, WorldConfig};

use crate::components::*;
use crate::events::EventQueue;
use crate::systems;

pub struct GameWorld {
    config: WorldConfig,
    tick: Tick,
    speed: GameSpeed,
    next_entity_id: EntityId,

    // Component storage — one HashMap per component type
    pub positions: HashMap<EntityId, Position>,
    pub ownerships: HashMap<EntityId, Ownership>,
    pub financials: HashMap<EntityId, Financial>,
    pub capacities: HashMap<EntityId, Capacity>,
    pub healths: HashMap<EntityId, Health>,
    pub constructions: HashMap<EntityId, Construction>,
    pub populations: HashMap<EntityId, Population>,
    pub demands: HashMap<EntityId, Demand>,
    pub workforces: HashMap<EntityId, Workforce>,
    pub ai_states: HashMap<EntityId, AiState>,
    pub policies: HashMap<EntityId, Policy>,
    pub corporations: HashMap<EntityId, Corporation>,

    pub event_queue: EventQueue,
}

impl GameWorld {
    pub fn new(config: WorldConfig) -> Self {
        Self {
            config,
            tick: 0,
            speed: GameSpeed::Normal,
            next_entity_id: 1,
            positions: HashMap::new(),
            ownerships: HashMap::new(),
            financials: HashMap::new(),
            capacities: HashMap::new(),
            healths: HashMap::new(),
            constructions: HashMap::new(),
            populations: HashMap::new(),
            demands: HashMap::new(),
            workforces: HashMap::new(),
            ai_states: HashMap::new(),
            policies: HashMap::new(),
            corporations: HashMap::new(),
            event_queue: EventQueue::new(),
        }
    }

    pub fn config(&self) -> &WorldConfig {
        &self.config
    }

    pub fn current_tick(&self) -> Tick {
        self.tick
    }

    pub fn speed(&self) -> GameSpeed {
        self.speed
    }

    pub fn entity_count(&self) -> usize {
        (self.next_entity_id - 1) as usize
    }

    pub fn allocate_entity(&mut self) -> EntityId {
        let id = self.next_entity_id;
        self.next_entity_id += 1;
        id
    }

    pub fn tick(&mut self) {
        if self.speed == GameSpeed::Paused {
            return;
        }
        self.tick += 1;
        systems::run_all_systems(self);
    }

    pub fn process_command(&mut self, command: Command) {
        match command {
            Command::SetSpeed(speed) => {
                self.speed = speed;
            }
            Command::TogglePause => {
                self.speed = if self.speed == GameSpeed::Paused {
                    GameSpeed::Normal
                } else {
                    GameSpeed::Paused
                };
            }
            // Other commands will be implemented as systems are fleshed out
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_world() {
        let world = GameWorld::new(WorldConfig::default());
        assert_eq!(world.current_tick(), 0);
        assert_eq!(world.entity_count(), 0);
    }

    #[test]
    fn test_tick_advances() {
        let mut world = GameWorld::new(WorldConfig::default());
        world.tick();
        assert_eq!(world.current_tick(), 1);
        world.tick();
        assert_eq!(world.current_tick(), 2);
    }

    #[test]
    fn test_pause_prevents_tick() {
        let mut world = GameWorld::new(WorldConfig::default());
        world.process_command(Command::SetSpeed(GameSpeed::Paused));
        world.tick();
        assert_eq!(world.current_tick(), 0);
    }

    #[test]
    fn test_toggle_pause() {
        let mut world = GameWorld::new(WorldConfig::default());
        world.process_command(Command::TogglePause);
        assert_eq!(world.speed(), GameSpeed::Paused);
        world.process_command(Command::TogglePause);
        assert_eq!(world.speed(), GameSpeed::Normal);
    }

    #[test]
    fn test_allocate_entity() {
        let mut world = GameWorld::new(WorldConfig::default());
        let e1 = world.allocate_entity();
        let e2 = world.allocate_entity();
        assert_eq!(e1, 1);
        assert_eq!(e2, 2);
        assert_eq!(world.entity_count(), 2);
    }

    #[test]
    fn test_construction_completes() {
        let mut world = GameWorld::new(WorldConfig::default());
        let entity = world.allocate_entity();
        world.constructions.insert(entity, Construction::new(0, 3));

        world.tick(); // tick 1
        world.tick(); // tick 2
        assert!(world.constructions.contains_key(&entity));

        world.tick(); // tick 3 — construction completes
        assert!(!world.constructions.contains_key(&entity));
    }

    #[test]
    fn test_revenue_adds_cash() {
        let mut world = GameWorld::new(WorldConfig::default());
        let corp = world.allocate_entity();
        world.financials.insert(corp, Financial {
            cash: 1000,
            revenue_per_tick: 100,
            cost_per_tick: 0,
            debt: 0,
        });

        world.tick();
        assert_eq!(world.financials[&corp].cash, 1100);
    }
}
