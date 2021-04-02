use bencher::{benchmark_group, benchmark_main, Bencher};
use kudo::*;

struct Position(f32);
struct Velocity(f32);

fn iterate_100k(b: &mut Bencher) {
    let mut world = World::new();
    for i in 0..100_000 {
        world.spawn((Position(-(i as f32)), Velocity(i as f32)));
    }
    b.iter(|| {
        for (pos, vel) in world.query::<(&mut Position, &Velocity)>().unwrap().iter() {
            pos.0 += vel.0;
        }
    })
}

fn spawn_100k(b: &mut Bencher) {
    let mut world = World::new();

    b.iter(|| {
        for i in 0..100_000 {
            world.spawn((Position(-(i as f32)), Velocity(i as f32)));
        }
    })
}

fn add_components_100k(b: &mut Bencher) {
    let mut world = World::new();

    let mut entities = Vec::new();
    for _ in 0..100_000 {
        entities.push(world.spawn((Position(0.),)));
    }

    b.iter(|| {
        for e in entities.iter() {
            let _ = world.add_component(*e, Velocity(10.));
        }
    })
}

fn get_query_100k(b: &mut Bencher) {
    struct A;
    struct B;
    struct C;
    struct D;

    let mut world = World::new();
    world.spawn((A {},));
    world.spawn((A {}, B {}));
    world.spawn((A {}, B {}, C {}));
    world.spawn((A {}, B {}, C {}, D {}));
    world.spawn((A {}, B {}, D {}));
    world.spawn((A {}, D {}));
    world.spawn((B {}, D {}));
    world.spawn((D {},));

    b.iter(|| {
        for _ in 0..100_000 {
            let _ = world.query::<(&D,)>();
        }
    })
}

benchmark_group!(
    benches,
    iterate_100k,
    spawn_100k,
    add_components_100k,
    get_query_100k
);
benchmark_main!(benches);
