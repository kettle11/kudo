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
pub trait System<'world_borrow, P> {
    fn run(self, world: &'world_borrow World) -> Result<(), FetchError>;
}

/*
pub trait IntoSystem<A> {
    fn system(self) -> Box<dyn FnMut(&World) -> Result<(), FetchError> + Send + Sync>;
}
*/

impl<'world_borrow, A: Fetch<'world_borrow>, FUNC> System<'world_borrow, A> for FUNC
where
    FUNC: FnMut(A) + Fn(<A::Item as FetchItem>::Item),
{
    fn run(self, world: &'world_borrow World) -> Result<(), FetchError> {
        let mut a = A::fetch(world)?;
        let a = a.inner();
        self(a);
        Ok(())
    }
}

impl<'world_borrow, A: Fetch<'world_borrow>, B: Fetch<'world_borrow>, FUNC>
    System<'world_borrow, (A, B)> for FUNC
where
    FUNC: FnMut(A, B) + Fn(<A::Item as FetchItem>::Item, <B::Item as FetchItem>::Item),
{
    fn run(self, world: &'world_borrow World) -> Result<(), FetchError> {
        let mut a = A::fetch(world)?;
        let mut b = B::fetch(world)?;
        let a = a.inner();
        let b = b.inner();
        self(a, b);
        Ok(())
    }
}

// The value accepted as part of a function should be different from the SystemQuery passed in.
// Even if they appear the same to the library user.
/*
macro_rules! system_impl {
    ($($name: ident),*) => {
        impl<FUNC, $($name: for<'a> Fetch<'a>),*> System<($($name,)*)> for FUNC
        where
            FUNC: FnMut($($name,)*) + FnMut($(<<$name as Fetch>::Item as FetchItem>::Item,)*),
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
            FUNC: System<($($name,)*)> + 'static + Copy + Send + Sync,
        {
            fn system(self) -> Box<dyn FnMut(&World) -> Result<(), FetchError> + Send + Sync> {
                Box::new(move|world|
                    self.run(world)
                )
            }
        }
    };
}
*/

//system_impl! {}

//system_impl! {A}
/*
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
*/
