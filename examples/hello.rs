use kudo::*;

struct A(f32);
struct B(f32);
fn main() {
    let mut world = World::new();
    world.spawn((true,));
    world.spawn((true, 10.));

    let queryable = world.into_queryable_world();
    {
        let mut query = queryable.query::<(&bool,)>();
        {
            let iter = query.iterator();
            //  for q in iter {}
        }
    }
    println!("HELLO");
}
