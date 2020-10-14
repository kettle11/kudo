//! This file provides easy construction of systems from functions.

use super::{GetQueryIter, Query, World};

pub trait System<'world_borrow, A> {
    fn run(self, world: &'world_borrow World);
}

impl<'world_borrow, A: Query<'world_borrow>, F> System<'world_borrow, (A,)> for F
where
    F: Fn(
        <<<A as Query<'world_borrow>>::GetQueryIter as GetQueryIter<'_>>::Iter as Iterator>::Item,
    ) + Fn(A),
{
    fn run(self, world: &'world_borrow World) {
        {
            let mut query = world.query::<(A,)>();
            for (a,) in query.iter() {
                self(a);
            }
        }
    }
}

impl<'world_borrow, A: Query<'world_borrow>, B: Query<'world_borrow>, F> System<'world_borrow, (A,B)> for F
where
    F: Fn(
        <<<A as Query<'world_borrow>>::GetQueryIter as GetQueryIter<'_>>::Iter as Iterator>::Item,
        <<<B as Query<'world_borrow>>::GetQueryIter as GetQueryIter<'_>>::Iter as Iterator>::Item,
    ) + Fn(A, B),
{
    fn run(self, world: &'world_borrow World) {
        {
            let mut query = world.query::<(A,B)>();
            for (a,b) in query.iter() {
                self(a, b);
            }
        }
    }
}
