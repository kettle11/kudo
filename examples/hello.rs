use kudo::*;

fn main() {
    // First we create the world.
    let mut world = World::new();

    // Let's create a new entity with a name and a health component.
    // With Kudo components are just plain structs.

    // This will be our health component
    struct Health(i32);

    // Spawn the entity with a String component we'll use for the name and a Health component.
    // Within the call to spawn we pass in a tuple that can have multiple components.
    world.spawn(("Medusa".to_string(), Health(0)));

    // Query the world for entities that have a String component and a Health component.
    // The '&' before each component requests read-only access to the component.
    // Using '&mut' would request write/read access for that component.
    let mut query = world.query::<(&String, &Health)>().unwrap();

    // Iterate over all the components we found and check if their health is below 0.
    for (name, health) in query.iter() {
        if health.0 <= 0 {
            println!("{} has perished!", name);
        }
    }
}
