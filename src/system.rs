use super::{Fetch, FetchError, FetchItem, World};

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
    fn run(self, world: &World) -> Result<(), FetchError>;
}

pub trait IntoSystem<A> {
    fn system(self) -> Box<dyn FnMut(&World) -> Result<(), FetchError>>;
}

// The value accepted as part of a function should be different from the SystemQuery passed in.
// Even if they appear the same to the library user.
macro_rules! system_impl {
    ($($name: ident),*) => {
        impl<FUNC, $($name: for<'a> Fetch<'a>),*> System<($($name,)*)> for FUNC
        where
            FUNC: FnMut($($name,)*) + FnMut($(<<$name as Fetch>::FetchItem as FetchItem>::Item,)*),
        {
            #[allow(non_snake_case)]
            #[allow(unused_variables)]
            fn run<'world_borrow>(mut self, world: &'world_borrow World) -> Result<(), FetchError> {
                $(let mut $name = <$name as Fetch<'world_borrow>>::fetch(world)?;)*
                self($($name.get(),)*);
                Ok(())
            }
        }


        impl<'world_borrow, FUNC, $($name: for<'a> Fetch<'a> ),*> IntoSystem<($($name,)*)> for FUNC
        where
            FUNC: System<($($name,)*)> + 'static + Copy,
        {
            fn system(self) -> Box<dyn FnMut(&World) -> Result<(), FetchError>> {
                Box::new(move|world|
                    self.run(world)
                )
            }
        }


    };
}

//system_impl! {}
system_impl! {A}
system_impl! {A, B}
system_impl! {A, B, C}
system_impl! {A, B, C, D}
system_impl! {A, B, C, D, E}
system_impl! {A, B, C, D, E, F}
system_impl! {A, B, C, D, E, F, G}
system_impl! {A, B, C, D, E, F, G, H}
