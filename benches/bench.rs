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
        (|mut query: Query<(&mut Position, &Velocity)>| {
            for (pos, vel) in &mut query {
                pos.0 += vel.0;
            }
        })
        .run(&world)
        .unwrap();
    })
}

fn add_remove(b: &mut Bencher) {
    struct A(f32);
    struct B(f32);

    let mut world = World::new();
    let mut entities = Vec::new();
    for _ in 0..10_000 {
        entities.push(world.spawn((A(0.0),)))
    }
    b.iter(|| {
        for entity in &entities {
            world.add_component(*entity, B(0.0)).unwrap();
        }

        for entity in &entities {
            world.remove_component::<B>(*entity).unwrap();
        }
    })
}

pub fn fragmented_iter(b: &mut Bencher) {
    macro_rules! create_entities {
        ($world:ident; $( $variants:ident ),*) => {
            $(
                struct $variants(f32);
                for _ in 0..20 {
                    $world.spawn(($variants(0.0), Data(1.0)));
                }
            )*
        };
    }
    struct Data(f32);

    let mut world = World::new();

    create_entities!(world; A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z);

    b.iter(|| {
        let mut query = world.query::<(&mut Data,)>().unwrap();

        for mut data in &mut query {
            data.0 *= 2.0;
        }
    });

    /*
    for data in &query {
        assert!(data.0 == 2.0)
    }
    */
}
benchmark_group!(benches, iterate_100k, fragmented_iter, add_remove);
benchmark_main!(benches);
