# :clap: kudo

An Entity Component System for Rust. Fast, easy, and predictable. (Work in progress)

* No `unsafe`
* No dependencies
* Only `660` lines of code

```rust 
struct Health(f32);
struct Name(String);
struct CreepySnakeHair(u32);

let mut world = World::new();

// Create entities with components.
world.spawn((Name("Perseus".to_string()), Health(50.)));
world.spawn((
    Name("Medusa".to_string()),
    Health(100.),
    CreepySnakeHair(300),
));

// Find every entity with a `Name` and a `Health` component.
let mut query = world.query::<(&Name, &Health)>().unwrap();

// Iterate through all entities with those components.
for (name, health) in query.iter() {
    println!("{}'s health is: {:?}", name.0, health.0);
}
```
