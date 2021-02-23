use kudo::*;

#[derive(Clone)]
struct HydraHead {}

fn main() {
    let mut clone_store_builder = CloneStore::new();
    clone_store_builder.register_type::<HydraHead>();
    let clone_store = clone_store_builder.build();

    // Create our first World
    let mut first_world = World::new_with_clone_store(clone_store.clone());
    let first_head_entity = first_world.spawn((HydraHead {},));

    let mut second_world = World::new_with_clone_store(clone_store.clone());

    // Create a new Entity with all possible components cloned from the original Entity.
    first_world
        .clone_entity_into_world(first_head_entity, &mut second_world)
        .unwrap();

    let mut query = second_world.query::<(&HydraHead,)>().unwrap();

    println!("The Hyrda has {:?} heads!", query.iter().count())
}
