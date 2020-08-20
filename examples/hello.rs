use kudo::*;

fn main() {
    let mut world = World::new();
    world.spawn((10.,));
    world.spawn((13.,));
    world.spawn((13., true));

    let query = world.query::<(&f64,)>();
    for i in query {
        println!("I: {:?}", i);
    }

    let query = world.query::<(&f64, &bool)>();
    for i in query {
        println!("QUERY 2: {:?}", i);
    }
}
