use super::{ComponentAlreadyBorrowed, Fetch, TopLevelQuery, World};

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
pub trait System<A> {
    fn run(self, world: &World) -> Result<(), ComponentAlreadyBorrowed>;
    fn system(self) -> Box<dyn Fn(&World) -> Result<(), ComponentAlreadyBorrowed>>;
}

// The value accepted as part of a function should be different from the SystemQuery passed in.
// Even if they appear the same to the library user.
macro_rules! system_impl {
    ($($name: ident),*) => {
        impl<FUNC, $($name: TopLevelQuery + 'static),*> System<($($name,)*)>  for FUNC
        where
            FUNC: Fn($($name,)*) + Fn($(<$name as Fetch>::Item,)*) + 'static + Copy,
        {
            #[allow(non_snake_case)]
            fn run<'world_borrow>(self, world: &'world_borrow World) -> Result<(), ComponentAlreadyBorrowed> {
                $(let $name = <$name as Fetch<'world_borrow>>::get(world, 0)?;)*

                self($($name,)*);
                Ok(())
            }
            fn system(self) -> Box<dyn Fn(&World) -> Result<(), ComponentAlreadyBorrowed>> {
                Box::new(move|world|
                    self.run(world)
                )
            }
        }
    };
}

system_impl! {A}
system_impl! {A, B}
system_impl! {A, B, C}
system_impl! {A, B, C, D}
system_impl! {A, B, C, D, E}
system_impl! {A, B, C, D, E, F}
system_impl! {A, B, C, D, E, F, G}
system_impl! {A, B, C, D, E, F, G, H}
