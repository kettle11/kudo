//! This example is the same as `hello.rs` except a function is used as the system
//! and multiple queries are passed into the system.

use kudo::*;

#[derive(Debug)]
struct A {}

#[derive(Debug)]
struct B {}
fn main() {
    let mut world = World::new();

    world.spawn((A {}, B {}));
    world.spawn((A {},));

    print_names.run(&world).unwrap();
}

// This call ensures that exactly one component is retrieved.
// If multiple of the component exist then an arbitrary one is returned.
// This system will panic if no instances of the component exist.
fn print_names(mut items: Query<(&A, Option<&B>)>) {
    for item in items.iter() {
        println!("{:?}", item);
    }
}
