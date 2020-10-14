//! This file provides easy construction of systems from functions.

use super::{Query, World, WorldBorrow};

pub trait System<'world_borrow, A> {
    fn run(self, world: &'world_borrow World);
}

macro_rules! system_impl {
    ($count: expr, $(($name: ident, $index: tt)),*) => {

        impl<'world_borrow, FUNC, $($name: Query<'world_borrow>),*> System<'world_borrow, ($($name,)*)> for FUNC
        where
            FUNC: Fn(
                $(<<<$name as Query<'world_borrow>>::WorldBorrow as WorldBorrow<'_>>::Iter as Iterator>::Item),*
            ) + Fn($($name,)*),
        {
            #[allow(non_snake_case)]
            fn run(self, world: &'world_borrow World) {
                {
                    let mut query = world.query::<($($name,)*)>();
                    for ($($name,)*) in query.iter() {
                        self($($name,)*);
                    }
                }
            }
        }
    }
}

system_impl! {1, (A, 0)}
system_impl! {2, (A, 0), (B, 1)}
system_impl! {3, (A, 0), (B, 1), (C, 2)}
system_impl! {4, (A, 0), (B, 1), (C, 2), (D, 3)}
system_impl! {5, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4)}
system_impl! {6, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5)}
system_impl! {7, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6)}
system_impl! {8, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6), (H, 7)}
