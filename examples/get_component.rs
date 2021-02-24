use kudo::*;

fn main() {
    let mut world = World::new();
    struct Health(i32);

    let medusa_entity = world.spawn(("Medusa".to_string(), Health(0)));
    let mut query = world.query::<(&String, &Health)>().unwrap();

    // If the entity is part of this query (which it is in this case)
    // then return a reference to the requested component.
    let medusa_health = query.get_entity_components(medusa_entity).unwrap().1;
    
    // let medusa_health = query.data.1.get_component(medusa_entity).unwrap();
    println!("Medusa's health: {:?}", medusa_health.0)
}
