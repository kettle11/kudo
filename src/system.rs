//! This file provides easy construction of systems from functions.

use super::{SystemQuery, World};

pub trait System<'world_borrow, A> {
    fn run(self, world: &'world_borrow World);
}

macro_rules! system_impl {
    ($($name: ident),*) => {

        impl<'world_borrow, FUNC, $($name: SystemQuery<'world_borrow>),*> System<'world_borrow, ($($name,)*)> for FUNC
        where
            FUNC: Fn($($name,)*),
        {
            #[allow(non_snake_case)]
            fn run(self, world: &'world_borrow World) {
                $(let $name = $name::get(world);)*
                self($($name),*)
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
