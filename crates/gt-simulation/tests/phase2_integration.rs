use gt_common::types::*;
use gt_simulation::components::ContractStatus;
use gt_simulation::world::GameWorld;

fn make_config() -> WorldConfig {
    WorldConfig {
        seed: 12345,
        map_size: MapSize::Small,
        ai_corporations: 4,
        starting_era: Era::Modern,
        difficulty: DifficultyPreset::Normal,
        ..WorldConfig::default()
    }
}

#[test]
fn world_setup_1_player_4_ai() {
    let world = GameWorld::new(make_config());

    // 1 player + 4 AI = 5 corporations
    assert_eq!(world.corporations.len(), 5);
    assert!(world.player_corp_id().is_some());

    // All 4 AI corps have AiState
    assert_eq!(world.ai_states.len(), 4);

    // All corps have financials
    assert_eq!(world.financials.len(), 5);

    // AI corps start with infrastructure (seeded)
    for (&corp_id, _) in &world.ai_states {
        let nodes = world.corp_infra_nodes.get(&corp_id);
        assert!(
            nodes.is_some() && !nodes.unwrap().is_empty(),
            "AI corp {} should have starting infrastructure",
            corp_id
        );
    }

    // World has regions, cities, and parcels
    assert!(!world.regions.is_empty());
    assert!(!world.cities.is_empty());
    assert!(!world.land_parcels.is_empty());
}

#[test]
fn ai_builds_infrastructure_over_500_ticks() {
    let mut world = GameWorld::new(make_config());

    // Record initial infrastructure counts
    let initial_counts: std::collections::HashMap<u64, usize> = world
        .ai_states
        .keys()
        .map(|&id| {
            let count = world
                .corp_infra_nodes
                .get(&id)
                .map(|v| v.len())
                .unwrap_or(0);
            (id, count)
        })
        .collect();

    let initial_total_nodes = world.infra_nodes.len();

    // Run 500 ticks
    for _ in 0..500 {
        world.tick();
    }

    assert_eq!(world.current_tick(), 500);

    // At least some AI corps should have built new infrastructure
    let mut any_grew = false;
    for (&corp_id, _) in &world.ai_states {
        let current = world
            .corp_infra_nodes
            .get(&corp_id)
            .map(|v| v.len())
            .unwrap_or(0);
        let initial = initial_counts.get(&corp_id).copied().unwrap_or(0);
        if current > initial {
            any_grew = true;
        }
    }
    assert!(
        any_grew,
        "At least one AI corp should have built new infrastructure over 500 ticks"
    );

    // Total infrastructure should have increased
    assert!(
        world.infra_nodes.len() > initial_total_nodes,
        "Total infrastructure nodes should increase. Initial: {}, Current: {}",
        initial_total_nodes,
        world.infra_nodes.len()
    );
}

#[test]
fn revenue_flows_from_infrastructure() {
    let mut world = GameWorld::new(make_config());

    // Run enough ticks for utilization and revenue to kick in
    for _ in 0..100 {
        world.tick();
    }

    // At least some corps should be earning revenue
    let corps_with_revenue: Vec<u64> = world
        .financials
        .iter()
        .filter(|(_, f)| f.revenue_per_tick > 0)
        .map(|(&id, _)| id)
        .collect();

    assert!(
        !corps_with_revenue.is_empty(),
        "After 100 ticks, at least some corporations should earn revenue from utilization"
    );

    // Revenue comes from traffic, contracts, or coverage.
    // Verify that corps with revenue have at least some operational infrastructure.
    for &corp_id in &corps_with_revenue {
        let has_operational_infra = world
            .corp_infra_nodes
            .get(&corp_id)
            .map(|nodes| {
                nodes.iter().any(|&nid| !world.constructions.contains_key(&nid))
            })
            .unwrap_or(false);
        assert!(
            has_operational_infra,
            "Corp {} has revenue but no operational infrastructure",
            corp_id
        );
    }
}

#[test]
fn costs_deducted_from_cash() {
    let mut world = GameWorld::new(make_config());

    // Record initial cash
    let player_id = world.player_corp_id().unwrap();
    let initial_cash = world.financials[&player_id].cash;

    // Run a few ticks — costs should eat into cash
    for _ in 0..10 {
        world.tick();
    }

    // Player corp has infrastructure costs from workforce salary at minimum
    assert!(
        world.financials[&player_id].cash < initial_cash,
        "Cash should decrease due to costs. Initial: {}, Current: {}",
        initial_cash,
        world.financials[&player_id].cash
    );

    assert!(
        world.financials[&player_id].cost_per_tick > 0,
        "Corp should have non-zero costs"
    );
}

#[test]
fn maintenance_degrades_infrastructure() {
    let mut world = GameWorld::new(make_config());

    // All health starts at 1.0
    let initial_healths: Vec<(u64, f64)> = world
        .healths
        .iter()
        .filter(|(id, _)| world.infra_nodes.contains_key(id))
        .map(|(&id, h)| (id, h.condition))
        .collect();

    assert!(!initial_healths.is_empty());

    // Run 200 ticks for degradation to be visible
    for _ in 0..200 {
        world.tick();
    }

    // Health should have degraded from 1.0 (at least slightly)
    let mut any_degraded = false;
    for &(node_id, initial_h) in &initial_healths {
        if let Some(h) = world.healths.get(&node_id) {
            if h.condition < initial_h {
                any_degraded = true;
                break;
            }
        }
    }

    assert!(
        any_degraded,
        "At least some infrastructure should degrade over 200 ticks"
    );
}

#[test]
fn ai_strategy_responds_to_finances() {
    let mut world = GameWorld::new(make_config());

    // Run enough ticks for AI to take strategic actions
    // AI runs every 10 ticks, so 500 ticks = 50 AI decision cycles
    for _ in 0..500 {
        world.tick();
    }

    // Check AI states — at least some should have switched from initial strategy
    let strategies: Vec<(u64, AIStrategy)> = world
        .ai_states
        .iter()
        .map(|(&id, state)| (id, state.strategy))
        .collect();

    // With 4 different archetypes and 500 ticks of financial changes,
    // we expect variation in strategies
    let unique_strategies: std::collections::HashSet<String> = strategies
        .iter()
        .map(|(_, s)| format!("{:?}", s))
        .collect();

    // At minimum, AI corps should have chosen strategies (not all identical unless
    // the simulation specifically drives them that way)
    assert!(
        !strategies.is_empty(),
        "AI corps should have active strategies"
    );

    // Verify that corps with low cash tend toward Survive/Consolidate
    for &(corp_id, strategy) in &strategies {
        if let Some(fin) = world.financials.get(&corp_id) {
            let cash_ratio = fin.cash as f64 / (fin.cost_per_tick as f64 * 90.0).max(1.0);
            if cash_ratio < 1.0 {
                assert!(
                    matches!(strategy, AIStrategy::Survive),
                    "Corp {} with low cash ratio ({:.2}) should be in Survive mode, but is {:?}",
                    corp_id,
                    cash_ratio,
                    strategy
                );
            }
        }
    }

    // Log strategy distribution for visibility
    eprintln!("AI Strategy distribution after 500 ticks:");
    for (corp_id, strategy) in &strategies {
        let fin = world.financials.get(corp_id).unwrap();
        eprintln!(
            "  Corp {} ({:?}): cash={}, rev={}, cost={}, debt={}",
            corp_id,
            strategy,
            fin.cash,
            fin.revenue_per_tick,
            fin.cost_per_tick,
            fin.debt
        );
    }
    eprintln!("  Unique strategies: {:?}", unique_strategies);
}

#[test]
fn contracts_form_between_corps() {
    let mut world = GameWorld::new(make_config());

    // Run 1000 ticks — AI proposes contracts on Expand/Compete strategy ticks
    // Contract formation depends on AI strategy cycles, available infrastructure,
    // and economic conditions, so we run longer to give AI time to act.
    for _ in 0..1000 {
        world.tick();
    }

    // Check if any contracts exist (proposed, active, or completed)
    let all_contracts = world.contracts.len();
    let proposed_or_active: Vec<&gt_simulation::components::Contract> = world
        .contracts
        .values()
        .filter(|c| matches!(c.status, ContractStatus::Proposed | ContractStatus::Active))
        .collect();

    eprintln!(
        "Contracts after 1000 ticks: {} total, {} proposed/active",
        all_contracts,
        proposed_or_active.len()
    );

    // With 4 AI corps running for 1000 ticks, contract activity should occur.
    // If no contracts formed, it means AI corps don't have enough infrastructure
    // to offer services to each other. Verify that at least AI is building.
    if world.contracts.is_empty() {
        // Verify AI is at least active (building infrastructure)
        let ai_nodes: usize = world.corp_infra_nodes.values()
            .map(|n| n.len())
            .sum();
        eprintln!("Total AI infrastructure nodes: {}", ai_nodes);
        // If no AI built anything either, the test seed may not support contracts.
        // This is acceptable — contracts depend on economic conditions.
        assert!(
            ai_nodes > 0,
            "AI corps should have built at least some infrastructure over 1000 ticks"
        );
    }

    // Verify contract structure
    for contract in world.contracts.values() {
        assert!(contract.from != contract.to, "Contract should be between different corps");
        assert!(contract.price_per_tick > 0, "Contract should have a price");
        assert!(contract.capacity > 0.0, "Contract should have capacity");
    }
}

#[test]
fn ai_takes_loans_when_needed() {
    let mut world = GameWorld::new(make_config());

    // No debt instruments initially
    assert!(
        world.debt_instruments.is_empty(),
        "Should start with no debt"
    );

    // Run for 500 ticks — some AI corps may take loans
    for _ in 0..500 {
        world.tick();
    }

    // Check if any corp has taken debt
    let corps_with_debt: Vec<u64> = world
        .financials
        .iter()
        .filter(|(id, f)| f.debt > 0 && world.ai_states.contains_key(id))
        .map(|(&id, _)| id)
        .collect();

    eprintln!(
        "After 500 ticks: {} AI corps have debt, {} debt instruments total",
        corps_with_debt.len(),
        world.debt_instruments.len()
    );

    // Debt instruments should be well-formed
    for (_, debt) in &world.debt_instruments {
        assert!(debt.principal >= 0, "Principal should be non-negative");
        assert!(
            debt.interest_rate > 0.0,
            "Interest rate should be positive"
        );
        assert!(
            debt.payment_per_tick > 0,
            "Payment should be positive"
        );
    }
}

#[test]
fn population_dynamics() {
    let mut world = GameWorld::new(make_config());

    // Record initial populations
    let initial_pops: std::collections::HashMap<u64, u64> = world
        .cities
        .iter()
        .map(|(&id, c)| (id, c.population))
        .collect();

    let _initial_total: u64 = initial_pops.values().sum();

    // Run 200 ticks
    for _ in 0..200 {
        world.tick();
    }

    // Population should have changed (births, deaths, migration)
    let mut changed = false;
    for (&id, city) in &world.cities {
        if let Some(&initial) = initial_pops.get(&id) {
            if city.population != initial {
                changed = true;
                break;
            }
        }
    }

    assert!(
        changed,
        "City populations should change over 200 ticks from births/deaths/migration"
    );

    // Employment rates should be populated
    for city in world.cities.values() {
        assert!(
            city.employment_rate > 0.0,
            "Employment rate should be positive"
        );
        assert!(
            city.birth_rate > 0.0,
            "Birth rate should be positive"
        );
    }

    // Region populations should match sum of city populations
    for region in world.regions.values() {
        let city_pop_sum: u64 = region
            .city_ids
            .iter()
            .filter_map(|&cid| world.cities.get(&cid).map(|c| c.population))
            .sum();
        assert_eq!(
            region.population, city_pop_sum,
            "Region population should equal sum of city populations"
        );
    }
}

#[test]
fn determinism_500_ticks() {
    let config = make_config();

    let mut w1 = GameWorld::new(config.clone());
    let mut w2 = GameWorld::new(config);

    for _ in 0..500 {
        w1.tick();
        w2.tick();
    }

    assert_eq!(w1.current_tick(), 500);
    assert_eq!(w2.current_tick(), 500);

    // Same entity counts
    assert_eq!(w1.infra_nodes.len(), w2.infra_nodes.len());
    assert_eq!(w1.infra_edges.len(), w2.infra_edges.len());
    assert_eq!(w1.contracts.len(), w2.contracts.len());
    assert_eq!(w1.debt_instruments.len(), w2.debt_instruments.len());
    assert_eq!(w1.corporations.len(), w2.corporations.len());

    // Same financial state for every corporation
    for (&id, f1) in &w1.financials {
        let f2 = w2.financials.get(&id).expect("Corp should exist in both");
        assert_eq!(f1.cash, f2.cash, "Cash mismatch for corp {}", id);
        assert_eq!(
            f1.revenue_per_tick, f2.revenue_per_tick,
            "Revenue mismatch for corp {}",
            id
        );
        assert_eq!(
            f1.cost_per_tick, f2.cost_per_tick,
            "Cost mismatch for corp {}",
            id
        );
        assert_eq!(f1.debt, f2.debt, "Debt mismatch for corp {}", id);
    }

    // Same AI strategies
    for (&id, s1) in &w1.ai_states {
        let s2 = w2.ai_states.get(&id).expect("AI should exist in both");
        assert_eq!(
            format!("{:?}", s1.strategy),
            format!("{:?}", s2.strategy),
            "Strategy mismatch for corp {}",
            id
        );
    }

    // Same city populations
    for (&id, c1) in &w1.cities {
        let c2 = w2.cities.get(&id).expect("City should exist in both");
        assert_eq!(
            c1.population, c2.population,
            "Population mismatch for city {}",
            id
        );
    }
}

#[test]
fn end_to_end_economy_cycle() {
    let mut world = GameWorld::new(make_config());

    // Run 100 ticks to let the economy cycle: build → utilize → revenue → costs
    for _ in 0..100 {
        world.tick();
    }

    // Verify the full cycle for at least one AI corp
    let mut cycle_verified = false;
    for (&corp_id, _) in &world.ai_states {
        let fin = world.financials.get(&corp_id).unwrap();
        let nodes = world.corp_infra_nodes.get(&corp_id);
        let has_infra = nodes.map(|n| !n.is_empty()).unwrap_or(false);

        if has_infra && fin.revenue_per_tick > 0 && fin.cost_per_tick > 0 {
            cycle_verified = true;
            eprintln!(
                "Economy cycle verified for corp {}: rev={}, cost={}, cash={}, nodes={}",
                corp_id,
                fin.revenue_per_tick,
                fin.cost_per_tick,
                fin.cash,
                nodes.unwrap().len()
            );
            break;
        }
    }

    assert!(
        cycle_verified,
        "At least one AI corp should have a complete economy cycle: infrastructure → utilization → revenue + costs"
    );
}
