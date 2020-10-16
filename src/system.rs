use super::{SystemQuery, World};

/// A function that can be run as system by pulling in queries from the world.
/// # Example
/// ```
/// # use kudo::*;
/// # struct A {}
/// # struct B {}
/// # struct C {}
/// # struct D {}
/// # let world = World::new();
/// fn my_system(
///     mut query0: Query<(&A, &B)>,
///     mut query1: Query<(&C, &D)>,
/// ) {
///     for (a, b) in query0.iter() {
///         /* Do stuff */
///     }
///     for (c, d) in query1.iter() {
///         /* Do other stuff */
///     }
/// }
/// 
/// my_system.run(&world).unwrap();
/// ```
pub trait System<'world_borrow, A> {
    fn run(self, world: &'world_borrow World) -> Result<(), ()>;
}

macro_rules! system_impl {
    ($($name: ident),*) => {

        impl<'world_borrow, FUNC, $($name: SystemQuery<'world_borrow>),*> System<'world_borrow, ($($name,)*)> for FUNC
        where
            FUNC: Fn($($name,)*),
        {
            #[allow(non_snake_case)]
            fn run(self, world: &'world_borrow World) -> Result<(), ()> {
                $(let $name = $name::get(world)?;)*
                self($($name),*);
                Ok(())
            }
        }
    }
}

system_impl! {A}
system_impl! {A, B}
system_impl! {A, B, C}
system_impl! {A, B, C, D}
system_impl! {A, B, C, D, E}
system_impl! {A, B, C, D, E, F}
system_impl! {A, B, C, D, E, F, G}
system_impl! {A, B, C, D, E, F, G, H}
