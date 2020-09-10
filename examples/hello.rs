use kudo::*;

#[derive(Debug)]
struct A {}
#[derive(Debug)]
struct B {}

#[derive(Debug)]
struct C {}

fn main() {
    let mut world = World::new();
    world.spawn((A {},));
    world.spawn((A {}, B {}));

    let mut q = world.query::<(&C,)>();
    for i in q.iterator() {
        println!("i: {:?}", i);
    }
}
