use criterion::{criterion_group, criterion_main, Criterion};
use gt_common::types::WorldConfig;
use gt_simulation::world::GameWorld;

fn bench_tick_empty(c: &mut Criterion) {
    let config = WorldConfig {
        seed: 42,
        ai_corporations: 0,
        ..Default::default()
    };
    let mut world = GameWorld::new(config);
    c.bench_function("tick_empty_world", |b| b.iter(|| world.tick()));
}

fn bench_tick_4_ai(c: &mut Criterion) {
    let config = WorldConfig {
        seed: 42,
        ai_corporations: 4,
        ..Default::default()
    };
    let mut world = GameWorld::new(config);
    // Run 50 ticks to build up entities
    for _ in 0..50 {
        world.tick();
    }
    c.bench_function("tick_4_ai_50ticks_warm", |b| b.iter(|| world.tick()));
}

fn bench_tick_8_ai(c: &mut Criterion) {
    let config = WorldConfig {
        seed: 42,
        ai_corporations: 8,
        ..Default::default()
    };
    let mut world = GameWorld::new(config);
    // Run 100 ticks to build up entities
    for _ in 0..100 {
        world.tick();
    }
    c.bench_function("tick_8_ai_100ticks_warm", |b| b.iter(|| world.tick()));
}

fn bench_world_creation(c: &mut Criterion) {
    let config = WorldConfig {
        seed: 42,
        ai_corporations: 4,
        ..Default::default()
    };
    c.bench_function("world_creation", |b| {
        b.iter(|| GameWorld::new(config.clone()))
    });
}

fn bench_snapshot_serialize(c: &mut Criterion) {
    let config = WorldConfig {
        seed: 42,
        ai_corporations: 4,
        ..Default::default()
    };
    let mut world = GameWorld::new(config);
    for _ in 0..100 {
        world.tick();
    }
    c.bench_function("snapshot_json_serialize", |b| {
        b.iter(|| world.save_game().unwrap())
    });
}

fn bench_snapshot_binary(c: &mut Criterion) {
    let config = WorldConfig {
        seed: 42,
        ai_corporations: 4,
        ..Default::default()
    };
    let mut world = GameWorld::new(config);
    for _ in 0..100 {
        world.tick();
    }
    c.bench_function("snapshot_binary_serialize", |b| {
        b.iter(|| world.save_game_binary().unwrap())
    });
}

criterion_group!(
    benches,
    bench_tick_empty,
    bench_tick_4_ai,
    bench_tick_8_ai,
    bench_world_creation,
    bench_snapshot_serialize,
    bench_snapshot_binary,
);
criterion_main!(benches);
