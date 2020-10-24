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
        for (pos, vel) in world
            .query::<(&mut Position, &Velocity)>()
            .unwrap()
            .get_iter()
        {
            pos.0 += vel.0;
        }
    })
}

benchmark_group!(benches, iterate_100k,);
benchmark_main!(benches);
