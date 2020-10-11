use kudo::*;

#[derive(Debug)]
struct A {}
#[derive(Debug)]
struct B {}

#[derive(Debug)]
struct C {}

fn main() {
    let mut world = World::new();
    let a = world.spawn((A {},));
    world.spawn((A {}, B {}));
    world.despawn(a).unwrap();

    world.spawn((A {}, B {}));

    let mut q = world.query::<(&A,)>();

    for i in q.iter() {
        println!("i: {:?}", i);
    }
}
