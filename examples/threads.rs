use kudo::*;
use std::sync::Arc;

#[derive(Debug)]
struct A {}

fn main() {
    let mut world = World::new();

    world.spawn((true, 10));

    let world = Arc::new(world);
    let world_other_thread = world.clone();

    // Run a query on another thread.
    let thread = std::thread::spawn(move || {
        let mut query = world_other_thread.query::<(&bool,)>().unwrap();
        for b in query.iter() {
            println!("Boolean: {:?}", b);
        }
    });

    // Even though this query accesses the same data as the other thread
    // it's ok because both threads are only reading the data.
    let mut query = world.query::<(&bool,)>().unwrap();
    for b in query.iter() {
        println!("Boolean: {:?}", b);
    }

    // This is also OK because the query does not overlap with the query on the other thread
    let mut query = world.query::<(&mut i32,)>().unwrap();
    for i in query.iter() {
        println!("I: {:?}", i);
    }

    // Presently there are not appropriate ways to guarantee queries from other threads won't overlap.
    // Some sort of scheduling primitives are needed.

    thread.join().unwrap();
}
