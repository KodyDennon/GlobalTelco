//! Determinism verification tests for the simulation engine.
//!
//! These tests verify that the simulation produces identical observable state
//! from identical inputs (same seed, same commands, same tick count).
//!
//! KNOWN ISSUE: Rust's HashMap uses RandomState with per-instance random seeds,
//! so `bincode::serialize(world)` produces different byte sequences for logically
//! identical worlds. This does NOT affect gameplay determinism (same inputs →
//! same observable state), but it means binary snapshots cannot be compared
//! byte-for-byte across process boundaries. For cross-platform WASM/native
//! parity checks, we use canonical hashing of sorted state instead.

use sha2::{Digest, Sha256};

use gt_common::commands::Command;
use gt_common::types::*;
use gt_simulation::world::GameWorld;

// ── Helpers ──────────────────────────────────────────────────────────────────

fn make_config(seed: u64, ai_corps: u32) -> WorldConfig {
    WorldConfig {
        seed,
        map_size: MapSize::Small,
        ai_corporations: ai_corps,
        starting_era: Era::Modern,
        difficulty: DifficultyPreset::Normal,
        ..WorldConfig::default()
    }
}

/// Extract observable state from a world for deterministic comparison.
/// This avoids HashMap iteration order issues by sorting all keys.
struct WorldSnapshot {
    tick: u64,
    corporation_count: usize,
    region_count: usize,
    city_count: usize,
    node_count: usize,
    edge_count: usize,
    parcel_count: usize,
    /// Sorted (corp_id, cash, revenue, cost, debt, node_count)
    corp_financials: Vec<(u64, i64, i64, i64, i64, usize)>,
    /// Sorted (node_id, node_type_name, lon, lat, owner)
    node_positions: Vec<(u64, String, i64, i64, u64)>,
    /// Total population across all regions
    total_population: u64,
}

impl WorldSnapshot {
    fn from_world(w: &GameWorld) -> Self {
        let mut corp_financials: Vec<_> = w
            .financials
            .iter()
            .map(|(&id, fin)| {
                let node_count = w
                    .corp_infra_nodes
                    .get(&id)
                    .map(|n| n.len())
                    .unwrap_or(0);
                (
                    id,
                    fin.cash as i64,
                    fin.revenue_per_tick as i64,
                    fin.cost_per_tick as i64,
                    fin.debt as i64,
                    node_count,
                )
            })
            .collect();
        corp_financials.sort_by_key(|c| c.0);

        // Discretize f64 positions to avoid float comparison issues
        let mut node_positions: Vec<_> = w
            .infra_nodes
            .iter()
            .map(|(&id, node)| {
                let pos = w.positions.get(&id);
                let (lon, lat) = pos.map(|p| (p.x, p.y)).unwrap_or((0.0, 0.0));
                let owner = w
                    .ownerships
                    .get(&id)
                    .map(|o| o.owner)
                    .unwrap_or(0);
                (
                    id,
                    format!("{:?}", node.node_type),
                    (lon * 1000.0) as i64,
                    (lat * 1000.0) as i64,
                    owner,
                )
            })
            .collect();
        node_positions.sort_by_key(|n| n.0);

        let total_population: u64 = w
            .populations
            .values()
            .map(|p| p.count as u64)
            .sum();

        WorldSnapshot {
            tick: w.current_tick(),
            corporation_count: w.corporations.len(),
            region_count: w.regions.len(),
            city_count: w.cities.len(),
            node_count: w.infra_nodes.len(),
            edge_count: w.infra_edges.len(),
            parcel_count: w.land_parcels.len(),
            corp_financials,
            node_positions,
            total_population,
        }
    }

    /// Compute a deterministic hash of the observable state.
    fn hash(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(self.tick.to_le_bytes());
        hasher.update(self.corporation_count.to_le_bytes());
        hasher.update(self.region_count.to_le_bytes());
        hasher.update(self.city_count.to_le_bytes());
        hasher.update(self.node_count.to_le_bytes());
        hasher.update(self.edge_count.to_le_bytes());
        hasher.update(self.parcel_count.to_le_bytes());
        hasher.update(self.total_population.to_le_bytes());
        for (id, cash, rev, cost, debt, nc) in &self.corp_financials {
            hasher.update(id.to_le_bytes());
            hasher.update(cash.to_le_bytes());
            hasher.update(rev.to_le_bytes());
            hasher.update(cost.to_le_bytes());
            hasher.update(debt.to_le_bytes());
            hasher.update(nc.to_le_bytes());
        }
        for (id, nt, lon, lat, owner) in &self.node_positions {
            hasher.update(id.to_le_bytes());
            hasher.update(nt.as_bytes());
            hasher.update(lon.to_le_bytes());
            hasher.update(lat.to_le_bytes());
            hasher.update(owner.to_le_bytes());
        }
        hasher.finalize().into()
    }
}

/// Run a world for N ticks and return its observable snapshot.
fn run_and_snapshot(config: &WorldConfig, ticks: u64) -> WorldSnapshot {
    let mut world = GameWorld::new(config.clone());
    for _ in 0..ticks {
        world.tick();
    }
    WorldSnapshot::from_world(&world)
}

/// Run a world for N ticks, applying commands at specific ticks.
fn run_with_commands_snapshot(
    config: &WorldConfig,
    commands: &[(u64, Command)],
    total_ticks: u64,
) -> WorldSnapshot {
    let mut world = GameWorld::new(config.clone());
    let mut cmd_idx = 0;
    for t in 0..total_ticks {
        while cmd_idx < commands.len() && commands[cmd_idx].0 == t {
            world.process_command(commands[cmd_idx].1.clone());
            cmd_idx += 1;
        }
        world.tick();
    }
    WorldSnapshot::from_world(&world)
}

// ── Core Determinism Tests ──────────────────────────────────────────────────

#[test]
fn determinism_same_seed_same_ticks() {
    let config = make_config(42, 4);
    let snap_a = run_and_snapshot(&config, 200);
    let snap_b = run_and_snapshot(&config, 200);

    assert_eq!(snap_a.tick, snap_b.tick);
    assert_eq!(snap_a.corporation_count, snap_b.corporation_count);
    assert_eq!(snap_a.node_count, snap_b.node_count);
    assert_eq!(snap_a.edge_count, snap_b.edge_count);
    assert_eq!(snap_a.total_population, snap_b.total_population);
    assert_eq!(snap_a.corp_financials, snap_b.corp_financials);
    assert_eq!(snap_a.node_positions, snap_b.node_positions);
    assert_eq!(snap_a.hash(), snap_b.hash(), "State hash mismatch at 200 ticks");
}

#[test]
fn determinism_same_seed_500_ticks() {
    let config = make_config(42, 4);
    let snap_a = run_and_snapshot(&config, 500);
    let snap_b = run_and_snapshot(&config, 500);

    assert_eq!(snap_a.hash(), snap_b.hash(), "500-tick determinism check failed");
}

#[test]
fn determinism_different_seeds_differ() {
    let snap_a = run_and_snapshot(&make_config(42, 4), 200);
    let snap_b = run_and_snapshot(&make_config(123, 4), 200);

    assert_ne!(
        snap_a.hash(),
        snap_b.hash(),
        "Different seeds must produce different states"
    );
}

#[test]
fn determinism_zero_ai() {
    let config = make_config(99, 0);
    let snap_a = run_and_snapshot(&config, 300);
    let snap_b = run_and_snapshot(&config, 300);

    assert_eq!(snap_a.hash(), snap_b.hash(), "Zero-AI determinism failed");
}

#[test]
fn determinism_8_ai_heavy() {
    let config = make_config(7777, 8);
    let snap_a = run_and_snapshot(&config, 100);
    let snap_b = run_and_snapshot(&config, 100);

    assert_eq!(
        snap_a.hash(),
        snap_b.hash(),
        "8-AI 100-tick determinism failed"
    );
}

// ── Determinism With Player Commands ────────────────────────────────────────

#[test]
fn determinism_with_build_commands() {
    let config = make_config(42, 2);

    let commands = vec![
        (
            10,
            Command::BuildNode {
                node_type: NodeType::CellTower,
                lon: 10.0,
                lat: 50.0,
            },
        ),
        (
            20,
            Command::BuildNode {
                node_type: NodeType::CellTower,
                lon: 11.0,
                lat: 51.0,
            },
        ),
        (
            50,
            Command::BuildNode {
                node_type: NodeType::DataCenter,
                lon: 12.0,
                lat: 52.0,
            },
        ),
    ];

    let snap_a = run_with_commands_snapshot(&config, &commands, 100);
    let snap_b = run_with_commands_snapshot(&config, &commands, 100);

    assert_eq!(
        snap_a.hash(),
        snap_b.hash(),
        "Build command determinism failed"
    );
}

#[test]
fn determinism_with_financial_commands() {
    let config = make_config(42, 2);

    let commands = vec![
        (
            5,
            Command::TakeLoan {
                corporation: 0,
                amount: 500_000,
            },
        ),
        (
            30,
            Command::SetBudget {
                corporation: 0,
                category: "maintenance".to_string(),
                amount: 100_000,
            },
        ),
    ];

    let snap_a = run_with_commands_snapshot(&config, &commands, 100);
    let snap_b = run_with_commands_snapshot(&config, &commands, 100);

    assert_eq!(
        snap_a.hash(),
        snap_b.hash(),
        "Financial command determinism failed"
    );
}

#[test]
fn determinism_with_game_control_commands() {
    let config = make_config(42, 2);

    let commands = vec![
        (10, Command::SetSpeed(GameSpeed::Fast)),
        (50, Command::SetSpeed(GameSpeed::Normal)),
    ];

    let snap_a = run_with_commands_snapshot(&config, &commands, 100);
    let snap_b = run_with_commands_snapshot(&config, &commands, 100);

    assert_eq!(
        snap_a.hash(),
        snap_b.hash(),
        "Game control command determinism failed"
    );
}

// ── Save/Load Round-Trip Determinism ────────────────────────────────────────

#[test]
fn determinism_save_load_roundtrip() {
    let config = make_config(42, 4);
    let mut world = GameWorld::new(config);

    // Run 100 ticks
    for _ in 0..100 {
        world.tick();
    }

    // Save
    let saved = world.save_game_binary().expect("save failed");

    // Load
    let mut loaded = GameWorld::load_game_binary(&saved).expect("load failed");

    // Run both for 100 more ticks
    for _ in 0..100 {
        world.tick();
        loaded.tick();
    }

    // Compare observable state
    let snap_original = WorldSnapshot::from_world(&world);
    let snap_loaded = WorldSnapshot::from_world(&loaded);

    assert_eq!(
        snap_original.hash(),
        snap_loaded.hash(),
        "Save/load round-trip determinism failed: world state diverged after reload"
    );
}

#[test]
fn determinism_json_roundtrip() {
    let config = make_config(42, 2);
    let mut world = GameWorld::new(config);

    for _ in 0..50 {
        world.tick();
    }

    // Save as JSON, reload, continue
    let json = world.save_game().expect("json save failed");
    let mut loaded = GameWorld::load_game(&json).expect("json load failed");

    for _ in 0..50 {
        world.tick();
        loaded.tick();
    }

    let snap_a = WorldSnapshot::from_world(&world);
    let snap_b = WorldSnapshot::from_world(&loaded);

    assert_eq!(
        snap_a.hash(),
        snap_b.hash(),
        "JSON round-trip determinism failed"
    );
}

// ── World Generation Determinism ────────────────────────────────────────────

#[test]
fn determinism_world_generation() {
    let config = make_config(42, 4);
    let world_a = GameWorld::new(config.clone());
    let world_b = GameWorld::new(config);

    // Compare entity counts
    assert_eq!(world_a.corporations.len(), world_b.corporations.len());
    assert_eq!(world_a.regions.len(), world_b.regions.len());
    assert_eq!(world_a.cities.len(), world_b.cities.len());
    assert_eq!(world_a.land_parcels.len(), world_b.land_parcels.len());
    assert_eq!(world_a.infra_nodes.len(), world_b.infra_nodes.len());
    assert_eq!(world_a.infra_edges.len(), world_b.infra_edges.len());

    // Compare via observable snapshot hash
    let snap_a = WorldSnapshot::from_world(&world_a);
    let snap_b = WorldSnapshot::from_world(&world_b);
    assert_eq!(snap_a.hash(), snap_b.hash(), "World generation is not deterministic");
}

#[test]
fn determinism_world_generation_different_map_sizes() {
    for map_size in [MapSize::Small, MapSize::Medium, MapSize::Large] {
        let config = WorldConfig {
            seed: 42,
            map_size,
            ai_corporations: 2,
            starting_era: Era::Modern,
            ..WorldConfig::default()
        };
        let snap_a = WorldSnapshot::from_world(&GameWorld::new(config.clone()));
        let snap_b = WorldSnapshot::from_world(&GameWorld::new(config));
        assert_eq!(
            snap_a.hash(),
            snap_b.hash(),
            "Map size {:?} generation non-deterministic",
            map_size
        );
    }
}

#[test]
fn determinism_world_generation_different_eras() {
    for era in [
        Era::Telegraph,
        Era::Telephone,
        Era::EarlyDigital,
        Era::Internet,
        Era::Modern,
        Era::NearFuture,
    ] {
        let config = WorldConfig {
            seed: 42,
            starting_era: era,
            ai_corporations: 2,
            map_size: MapSize::Small,
            ..WorldConfig::default()
        };
        let snap_a = WorldSnapshot::from_world(&GameWorld::new(config.clone()));
        let snap_b = WorldSnapshot::from_world(&GameWorld::new(config));
        assert_eq!(
            snap_a.hash(),
            snap_b.hash(),
            "Era {:?} generation non-deterministic",
            era
        );
    }
}

// ── RNG Determinism ─────────────────────────────────────────────────────────

#[test]
fn determinism_rng_sequence() {
    let config = make_config(42, 0);
    let mut world_a = GameWorld::new(config.clone());
    let mut world_b = GameWorld::new(config);

    // Advance both to the same tick
    for _ in 0..10 {
        world_a.tick();
        world_b.tick();
    }

    // Sample random values — they should be identical
    let mut vals_a = Vec::new();
    let mut vals_b = Vec::new();
    for _ in 0..100 {
        vals_a.push(world_a.deterministic_random());
        vals_b.push(world_b.deterministic_random());
    }

    assert_eq!(vals_a, vals_b, "RNG sequences diverged");
}

// ── Sandbox Mode Determinism ────────────────────────────────────────────────

#[test]
fn determinism_sandbox_mode() {
    let config = WorldConfig {
        seed: 42,
        sandbox: true,
        ai_corporations: 2,
        map_size: MapSize::Small,
        ..WorldConfig::default()
    };
    let snap_a = run_and_snapshot(&config, 100);
    let snap_b = run_and_snapshot(&config, 100);

    assert_eq!(
        snap_a.hash(),
        snap_b.hash(),
        "Sandbox mode determinism failed"
    );
}

// ── HashMap Ordering Issue (documentation test) ─────────────────────────────

/// This test documents the known HashMap iteration order issue.
/// Binary serialization of two logically identical worlds may differ
/// due to Rust's per-instance random HashMap seeds.
/// This is NOT a gameplay bug — it only affects byte-level snapshot comparison.
#[test]
fn hashmap_ordering_documented() {
    let config = make_config(42, 4);
    let world_a = GameWorld::new(config.clone());
    let world_b = GameWorld::new(config);

    let bytes_a = world_a.save_game_binary().expect("save a");
    let bytes_b = world_b.save_game_binary().expect("save b");

    // Binary output MAY differ due to HashMap ordering
    // This is expected behavior, not a determinism bug
    if bytes_a != bytes_b {
        // Confirm logical state is still identical
        let snap_a = WorldSnapshot::from_world(&world_a);
        let snap_b = WorldSnapshot::from_world(&world_b);
        assert_eq!(
            snap_a.hash(),
            snap_b.hash(),
            "Logical state must match even when binary output differs"
        );
    }
    // If bytes happen to match (unlikely but possible), that's fine too
}

// ── Cross-Platform Hash Generation ──────────────────────────────────────────

/// Generate a deterministic hash of world state at tick N.
/// Uses sorted observable state to avoid HashMap ordering issues.
/// This can be compared against WASM output to verify cross-platform determinism.
pub fn world_state_hash(config: &WorldConfig, ticks: u64) -> [u8; 32] {
    run_and_snapshot(config, ticks).hash()
}

/// Generate expected hashes and print them for fixture creation.
/// Run with: `cargo test -p gt-simulation generate_hash_fixtures -- --ignored --nocapture`
#[test]
#[ignore]
fn generate_hash_fixtures() {
    let configs = vec![
        ("seed42_tick100_4ai", make_config(42, 4), 100u64),
        ("seed42_tick500_4ai", make_config(42, 4), 500),
        ("seed123_tick200_2ai", make_config(123, 2), 200),
        ("seed7777_tick100_8ai", make_config(7777, 8), 100),
        ("seed42_tick200_0ai", make_config(42, 0), 200),
    ];

    for (name, config, ticks) in configs {
        let hash = world_state_hash(&config, ticks);
        println!(
            "fixture: {} => {}",
            name,
            hash.iter().map(|b| format!("{:02x}", b)).collect::<String>()
        );
    }
}
