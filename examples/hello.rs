use kudo::*;

fn main() {
    let mut world = World::new();
    world.spawn((10.,));
    world.spawn((13.,));
    world.spawn((13., true));

    let query = world.query::<(&f64, &bool)>();
    for i in query {
        println!("I: {:?}", i);
    }
}
