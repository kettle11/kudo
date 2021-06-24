/// A malicious example intended to break the ID system
/// This should crash, but does not when EntityId in
/// lib.rs is changed to a u8.
/// This illustrates a weakness in a generational ID system.
/// But in a typical scenario it's incredibly unlikely to have this occur.
use kudo::*;

struct Thing;
impl ComponentTrait for Thing {}

fn main() {
    let mut world = World::new();
    let first_entity = world.spawn((Thing,));
    world.despawn(first_entity).unwrap();

    // For this test the generation is stored with just a u8.
    // 128 because kudo increments generation on remove.
    for i in 0..128 {
        let entity = world.spawn((Thing,));

        if i != 127 {
            world.despawn(entity).unwrap();
        }
    }

    // This unwrap should panic, but it does not!
    world.despawn(first_entity).unwrap();
}
