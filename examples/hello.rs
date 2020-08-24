use kudo::*;
fn main() {
    struct Hello {}
    struct Test {}
    let mut world = World::new();
    world.spawn((10.,));
    world.spawn((Hello {}, Test {}, 10.));
    // world.spawn((Hello {},));

    world.find_matching_archetypes::<(Hello,)>();
}
