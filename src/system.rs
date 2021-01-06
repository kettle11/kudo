use crate::SystemParameter;

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
pub trait System<P> {
    fn run(self, world: &World) -> Result<(), FetchError>;
}

pub trait IntoSystem<P> {
    fn system(self) -> Box<dyn FnMut(&World) -> Result<(), FetchError> + Send + Sync>;
}

pub trait OuterSystem {
    type Input;
    fn run<'world_borrow>(self, world: &'world_borrow World) -> Result<(), FetchError>;
}

// A SystemParameter specifies how its data is fetched and represented with an associated
// type that implements 'Fetch'.
// 'Fetch' has an Item that can be borrow to access its 'InnerItem'.
// This allows the Fetch item to contain data that must be dropped *after*
// the system executes.
type InnerItem<'a, 'b, A> =
    <<<A as SystemParameter>::Fetch as Fetch<'a>>::Item as FetchItem<'b>>::InnerItem;

impl<P, S: System<P> + Sync + Send + 'static + Copy> IntoSystem<P> for S {
    fn system(self) -> Box<dyn FnMut(&World) -> Result<(), FetchError> + Send + Sync> {
        Box::new(move |world| self.run(world))
    }
}

macro_rules! system_impl {
    ($($name: ident),*) => {
        impl<FUNC, $($name: SystemParameter),*> System<($($name,)*)> for FUNC
        where
            FUNC: FnMut($($name,)*) + for<'a, 'b> FnMut($(InnerItem<'a, 'b, $name>,)*),
        {
            #[allow(non_snake_case)]
            #[allow(unused_variables)]
            fn run<'world_borrow>(mut self, world: &'world_borrow World) -> Result<(), FetchError> {
                $(let mut $name = $name::Fetch::fetch(world)?;)*
                self($($name.inner(),)*);
                Ok(())
            }
        }
    };
}

system_impl! {}
system_impl! {A}
system_impl! {A, B}
system_impl! {A, B, C}
system_impl! {A, B, C, D}
system_impl! {A, B, C, D, E}
system_impl! {A, B, C, D, E, F}
system_impl! {A, B, C, D, E, F, G}
system_impl! {A, B, C, D, E, F, G, H}
system_impl! {A, B, C, D, E, F, G, H, I}
system_impl! {A, B, C, D, E, F, G, H, I, J, K}
system_impl! {A, B, C, D, E, F, G, H, I, J, K, L}
