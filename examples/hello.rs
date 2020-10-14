use kudo::*;

#[derive(Debug)]
struct A(f32);
#[derive(Debug)]
struct B(f32);

#[derive(Debug)]
struct C(f32);

fn main() {
    let mut world = World::new();

    let entities: Vec<Entity> = (0..10_000).map(|_| world.spawn((A(0.0),))).collect();

    for entity in entities.iter() {
        world.add_component(*entity, B(0.0)).unwrap();
    }

    (test_system).run(&world);
    (test_system).run(&world);

    /*
        for entity in entities.iter() {
            world.remove_component::<B>(*entity).unwrap();
        }
    */
    // world.query::<(&A, &B)>();
    /*
    (|a: &A| {
        println!("A: {:?}", a);
    })
    .run(&world);
    */
}

fn test_system(a: &A, b: &mut B) {
    println!("A: {:?} B: {:?}", a, b);
    b.0 = 30.;
}
