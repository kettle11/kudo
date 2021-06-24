//! This example is the same as `hello.rs` except a function is used as the system
//! and multiple queries are passed into the system.

use kudo::*;

struct Health(f32);
struct Name(String);
struct CreepySnakeHair(u32);

impl ComponentTrait for Health {}
impl ComponentTrait for Name {}
impl ComponentTrait for CreepySnakeHair {}

fn main() {
    let mut world = World::new();

    // Create entities with components.
    world.spawn((Name("Perseus".to_string()), Health(50.)));
    world.spawn((
        Name("Medusa".to_string()),
        Health(100.),
        CreepySnakeHair(300),
    ));

    // The unwrap here checks that the system ran successfully.
    // The system will fail to run if its queries need mutable access to the same components.
    print_names.run(&world).unwrap();
}

// Find every entity with a `Name` and a `Health` component.
fn print_names(query: Query<(&Name, &Health)>, creepy_hair: Query<(&Name, &CreepySnakeHair)>) {
    // Iterate through all entities with those components.
    for (name, health) in query.iter() {
        println!("{}'s health is: {:?}", name.0, health.0);
    }

    // Iterate through all entities with those components.
    for (name, _) in creepy_hair.iter() {
        println!("{} has creepy snake hair", name.0);
    }
}
