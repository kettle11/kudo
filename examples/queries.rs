//! This example is the same as `hello.rs` except a function is used as the system
//! and multiple queries are passed into the system.

use kudo::*;

struct ProgramInfo {
    name: String,
}
fn main() {
    let mut world = World::new();

    world.spawn((ProgramInfo {
        name: "Queries Example".to_string(),
    },));

    print_names.run(&world).unwrap();
}

// The call to Single here ensures that exactly one component is retrieved.
// If multiple of the component exist then an arbitrary one is returned.
// Accessing the Single will panic if no instance of the component exists in the world.
// This is useful for accessing global singleton data.
fn print_names(program_info: &mut ProgramInfo) {
    println!("This program's name is: {}", program_info.name);
}
