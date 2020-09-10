use kudo::*;

fn main() {
    let mut world = World::new();
    world.spawn((true,));
    world.spawn((false, 10.));

    let mut q = world.query::<(&bool,)>();
    for i in q.iterator() {
        println!("i: {:?}", i);
    }
}
