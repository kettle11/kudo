use kudo::*;

fn main() {
    let mut world = World::new();
    struct Health(i32);

    let medusa_entity = world.spawn(("Medusa".to_string(), Health(0)));
    let query = world.query::<(&String, &Health)>().unwrap();

    // If the entity is part of this query (which it is in this case)
    // then return a reference to the requested component.
    // let medusa_health = query.borrow.1.get_component(medusa_entity).unwrap();
    //  println!("Medusa's health: {:?}", medusa_health.0)
}
