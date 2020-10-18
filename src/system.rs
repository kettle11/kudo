use super::{Fetch, QueryParams, TopLevelFetch, TopLevelQuery, World};

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
    fn run(self, world: &World) -> Result<(), ()>;
}

pub trait BoxSystem<A> {
    fn system(self) -> Box<dyn Fn(&World) -> Result<(), ()>>;
}

// The value accepted as part of a function should be different from the SystemQuery passed in.
// Even if they appear the same to the library user.
macro_rules! system_impl {
    ($($name: ident),*) => {
        impl< FUNC, $($name: TopLevelQuery),*> System<($($name,)*)> for FUNC
        where
            FUNC: Fn($(<$name as TopLevelFetch>::Item,)*) + 'static,
        {
            #[allow(non_snake_case)]
            fn run(self, world: &World) -> Result<(), ()> {
                $(let $name = <$name as TopLevelQuery>::get(world)?;)*
                self($($name),*);
                Ok(())
            }
        }

        /*
        impl<'a, FUNC, $($name: F),*> BoxSystem<'a,($($name,)*)> for FUNC
        where
            FUNC: Fn($($name,)*) + 'static,
        {
            #[allow(non_snake_case)]
            // This value needs to be valid for any lifetime.
            // Because of the definition here the 'a becomes part of the type.
            fn system(self) -> Box<dyn Fn(&World) -> Result<(),()>> {
                Box::new( move |world| {
                        $(let $name = <$name::EntityQueryParams>::get_entity_query(world)?;)*
                        self($($name),*);
                        Ok(())
                    }
                )
            }
        }
        */
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
