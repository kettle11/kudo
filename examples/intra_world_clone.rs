use kudo::*;

#[derive(Clone)]
struct HydraHead {}

fn main() {
    // Before the World is created we must use a CloneStore to declare
    // all the types that may be cloned.
    // It would be much preferable to automatically infer this, but due to limitations
    // in Rust's type system we cannot.
    let mut clone_store_builder = CloneStore::new();
    clone_store_builder.register_type::<HydraHead>();

    // First we create the world.
    let mut world = World::new_with_clone_store(clone_store_builder.build());

    let first_head_entity = world.spawn((HydraHead {},));

    // Create a new Entity with all possible components cloned from the original Entity.
    let second_head_entity = world.clone_entity(first_head_entity).unwrap();
    world.clone_entity(second_head_entity);

    let mut query = world.query::<(&HydraHead,)>().unwrap();

    println!("The Hyrda has {:?} heads!", query.iter().count())
}
