## This repository is no longer maintained. But `kudo` lives on as a crate within the `koi` game engine project: https://github.com/kettle11/koi/tree/main/crates/kecs

# 👏 kudo

[![Documentation](https://docs.rs/kudo/badge.svg)](https://docs.rs/kudo/)
[![Crates.io](https://img.shields.io/crates/v/kudo.svg)](https://crates.io/crates/kudo)
[![License: Zlib](https://img.shields.io/badge/License-Zlib-lightgrey.svg)](https://opensource.org/licenses/Zlib)

## WORK IN PROGRESS

An Entity Component System for Rust. Fast, easy, and predictable.

* No `unsafe`
* No dependencies

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

`Kudo` was inspired by the library [`hecs`](https://github.com/Ralith/hecs). If you need a more feature-rich ECS, give `hecs` a try!
