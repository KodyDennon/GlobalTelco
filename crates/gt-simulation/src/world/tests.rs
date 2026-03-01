use super::*;

#[test]
fn test_create_world() {
    let world = GameWorld::new(WorldConfig::default());
    assert_eq!(world.current_tick(), 0);
    assert!(world.entity_count() > 0);
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
fn test_world_has_regions_and_cities() {
    let world = GameWorld::new(WorldConfig {
        map_size: MapSize::Small,
        ..WorldConfig::default()
    });
    assert!(!world.regions.is_empty(), "World should have regions");
    assert!(!world.cities.is_empty(), "World should have cities");
    assert!(
        !world.land_parcels.is_empty(),
        "World should have land parcels"
    );
}

#[test]
fn test_world_has_corporations() {
    let config = WorldConfig {
        ai_corporations: 4,
        map_size: MapSize::Small,
        ..WorldConfig::default()
    };
    let world = GameWorld::new(config);
    assert_eq!(world.corporations.len(), 5); // 1 player + 4 AI
    assert!(world.player_corp_id().is_some());
}

#[test]
fn test_construction_completes() {
    let mut world = GameWorld::new(WorldConfig {
        map_size: MapSize::Small,
        ..WorldConfig::default()
    });
    let entity = world.allocate_entity();
    world.constructions.insert(entity, Construction::new(0, 3));

    world.tick(); // tick 1
    world.tick(); // tick 2
    assert!(world.constructions.contains_key(&entity));

    world.tick(); // tick 3 — construction completes
    assert!(!world.constructions.contains_key(&entity));
}

#[test]
fn test_cost_deducted_from_cash() {
    let mut world = GameWorld::new(WorldConfig {
        map_size: MapSize::Small,
        ..WorldConfig::default()
    });
    // Player corp gets created automatically — verify cost deduction works
    let corp_id = world.player_corp_id().unwrap();
    let initial_cash = world.financials[&corp_id].cash;

    // Set a known cost
    world.financials.get_mut(&corp_id).unwrap().cost_per_tick = 100;
    world.financials.get_mut(&corp_id).unwrap().revenue_per_tick = 0;

    world.tick();

    // Cash should decrease by cost_per_tick (cost system recalculates, so check decrease)
    assert!(
        world.financials[&corp_id].cash < initial_cash,
        "Cash should decrease after tick with costs"
    );
}

#[test]
fn test_take_loan() {
    let mut world = GameWorld::new(WorldConfig {
        map_size: MapSize::Small,
        ..WorldConfig::default()
    });
    let corp_id = world.player_corp_id().unwrap();
    let initial_cash = world.financials[&corp_id].cash;

    world.process_command(Command::TakeLoan {
        corporation: corp_id,
        amount: 1_000_000,
    });

    assert_eq!(world.financials[&corp_id].cash, initial_cash + 1_000_000);
    assert!(world.financials[&corp_id].debt > 0);
    assert!(!world.debt_instruments.is_empty());
}

#[test]
#[cfg(feature = "native-compression")]
fn test_save_load_binary_roundtrip() {
    let config = WorldConfig {
        map_size: MapSize::Small,
        ai_corporations: 1,
        ..WorldConfig::default()
    };
    let mut world = GameWorld::new(config);
    for _ in 0..10 {
        world.tick();
    }

    let binary = world.save_game_binary().expect("save should succeed");
    let loaded = GameWorld::load_game_binary(&binary).expect("load should succeed");

    assert_eq!(world.current_tick(), loaded.current_tick());
    assert_eq!(world.regions.len(), loaded.regions.len());
    assert_eq!(world.corporations.len(), loaded.corporations.len());
}

#[test]
#[cfg(feature = "native-compression")]
fn test_save_binary_corruption_detected() {
    let config = WorldConfig {
        map_size: MapSize::Small,
        ai_corporations: 1,
        ..WorldConfig::default()
    };
    let world = GameWorld::new(config);
    let mut binary = world.save_game_binary().expect("save should succeed");

    // Verify it loads fine unmodified
    assert!(GameWorld::load_game_binary(&binary).is_ok());

    // Corrupt a byte in the payload (after version + crc)
    if binary.len() > 10 {
        binary[10] ^= 0xFF;
    }

    let result = GameWorld::load_game_binary(&binary);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.contains("CRC32 mismatch") || err.contains("decompress"),
        "Expected corruption error, got: {}",
        err
    );
}

#[test]
#[cfg(feature = "native-compression")]
fn test_save_binary_empty_data() {
    let result = GameWorld::load_game_binary(&[]);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Empty save data"));
}

#[test]
#[cfg(feature = "native-compression")]
fn test_save_binary_unsupported_version() {
    let result = GameWorld::load_game_binary(&[99, 0, 0, 0, 0]);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Unsupported save version"));
}

#[test]
fn test_save_game_json_roundtrip() {
    let config = WorldConfig {
        map_size: MapSize::Small,
        ai_corporations: 1,
        ..WorldConfig::default()
    };
    let mut world = GameWorld::new(config);
    for _ in 0..10 {
        world.tick();
    }

    // This was previously broken due to tuple-keyed HashMaps
    let json = world.save_game().expect("JSON save should succeed");
    assert!(!json.is_empty());

    let loaded = GameWorld::load_game(&json).expect("JSON load should succeed");
    assert_eq!(world.current_tick(), loaded.current_tick());
    assert_eq!(world.regions.len(), loaded.regions.len());
    assert_eq!(world.corporations.len(), loaded.corporations.len());
}

#[test]
fn test_determinism() {
    let config = WorldConfig {
        seed: 42,
        map_size: MapSize::Small,
        ai_corporations: 2,
        ..WorldConfig::default()
    };

    let mut w1 = GameWorld::new(config.clone());
    let mut w2 = GameWorld::new(config);

    for _ in 0..50 {
        w1.tick();
        w2.tick();
    }

    assert_eq!(w1.current_tick(), w2.current_tick());
    assert_eq!(w1.regions.len(), w2.regions.len());
    assert_eq!(w1.cities.len(), w2.cities.len());

    // Check financial state matches
    for (&id, f1) in &w1.financials {
        if let Some(f2) = w2.financials.get(&id) {
            assert_eq!(f1.cash, f2.cash, "Cash mismatch for entity {}", id);
        }
    }
}
