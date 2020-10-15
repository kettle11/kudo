/// A malicious example intended to break the ID system
/// This example will not fail as expected unless EntityId in lib.rs is changed to a u8.

use kudo::*;

fn main() {
    let mut world = World::new();
    let first_entity = world.spawn((true,));
    world.despawn(first_entity).unwrap();

    // For this test the generation is stored with just a u8.
    // 128 because kudo increments generation on remove.
    for i in 0..128 {
        let e = world.spawn((true,));

        if i != 127 {
            world.despawn(e).unwrap();
        }
    }

    // This unwrap should panic, but it does not!
    world.despawn(first_entity).unwrap();
}
