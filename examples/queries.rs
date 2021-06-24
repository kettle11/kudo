//! This example is the same as `hello.rs` except a function is used as the system
//! and multiple queries are passed into the system.

use kudo::*;
struct ProgramInfo {
    name: String,
}

impl ComponentTrait for ProgramInfo {}

fn main() {
    let mut world = World::new();

    world.spawn((ProgramInfo {
        name: "Queries Example".to_string(),
    },));

    print_names.run(&world).unwrap();
}

// This call ensures that exactly one component is retrieved.
// If multiple of the component exist then an arbitrary one is returned.
// This system will panic if no instances of the component exist.
fn print_names(program_info: &mut ProgramInfo) {
    println!("This program's name is: {}", program_info.name);
}
