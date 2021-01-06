//! This file contains a number of workarounds to get around the lack of generic associated types (GATs)
//! so expect some weirdness and convolutedness.
//!
//! A `Query` has `QueryParameters` that is a tuple of `QueryParameter`s
//! A `QueryParameter` has code to filter archetypes from the world.
//! A `QueryParameter implements `QueryParameterFetch` which borrows from the `World`.
//! `QueryParameterFetch` has a `FetchItem` which is a borrow from the world.
//! `FetchItem` has `Item` which is the final value passed to a system.
//!
//! `FetchItem` exists so that RwLocks can be held in the scope that calls the user system.
//! but the user system receives a simple &T or &mut T.

use crate::iterators::*;
use crate::{
    Archetype, ChainedIterator, ComponentAlreadyBorrowed, ComponentDoesNotExist, FetchError, World,
};
use std::iter::Zip;
use std::sync::{RwLockReadGuard, RwLockWriteGuard};
use std::{any::TypeId, usize};
pub trait FetchItem<'a> {
    type Item;
    fn inner(&'a mut self) -> Self::Item;
}

pub trait Fetch<'world_borrow> {
    type Item: for<'a> FetchItem<'a>;
    fn fetch(world: &'world_borrow World) -> Result<Self::Item, FetchError>;
}

pub struct Query<'world_borrow, T: QueryParameters> {
    data: <T as QueryParameterFetch<'world_borrow>>::FetchItem,
    world: &'world_borrow World,
}

impl<'a, 'world_borrow, T: QueryParameters> FetchItem<'a> for Option<Query<'world_borrow, T>> {
    type Item = Query<'world_borrow, T>;
    fn inner(&'a mut self) -> Self::Item {
        self.take().unwrap()
    }
}

impl<'world_borrow, T: QueryParameters> Fetch<'world_borrow> for Query<'world_borrow, T> {
    type Item = Option<Query<'world_borrow, T>>;
    fn fetch(world: &'world_borrow World) -> Result<Self::Item, FetchError> {
        Ok(Some(Self {
            data: T::fetch(world, 0)?,
            world,
        }))
    }
}

impl<'a, 'world_borrow, T: 'a> FetchItem<'a> for RwLockReadGuard<'world_borrow, T> {
    type Item = &'a T;
    fn inner(&'a mut self) -> Self::Item {
        self
    }
}

impl<'a, 'world_borrow, T: 'a> FetchItem<'a> for RwLockWriteGuard<'world_borrow, T> {
    type Item = &'a mut T;
    fn inner(&'a mut self) -> Self::Item {
        &mut *self
    }
}

pub struct Single<'world_borrow, T> {
    borrow: RwLockReadGuard<'world_borrow, Vec<T>>,
}

impl<'a, 'world_borrow, T: 'a> FetchItem<'a> for Single<'world_borrow, T> {
    type Item = &'a T;
    fn inner(&'a mut self) -> Self::Item {
        &self.borrow[0]
    }
}

pub struct SingleMut<'world_borrow, T> {
    borrow: RwLockWriteGuard<'world_borrow, Vec<T>>,
}

impl<'a, 'world_borrow, T: 'a> FetchItem<'a> for SingleMut<'world_borrow, T> {
    type Item = &'a mut T;
    fn inner(&'a mut self) -> Self::Item {
        &mut self.borrow[0]
    }
}

impl<'world_borrow, T: 'static> Fetch<'world_borrow> for &T {
    type Item = Single<'world_borrow, T>;
    fn fetch(world: &'world_borrow World) -> Result<Self::Item, FetchError> {
        // The archetypes must be found here.
        let type_id = TypeId::of::<T>();
        for archetype in world.archetypes.iter() {
            for (i, c) in archetype.components.iter().enumerate() {
                if c.type_id == type_id {
                    let borrow = archetype.get(i).try_read().unwrap();
                    return Ok(Single { borrow });
                }
            }
        }

        Err(FetchError::ComponentDoesNotExist(
            ComponentDoesNotExist::new::<T>(),
        ))
    }
}

impl<'world_borrow, T: 'static> Fetch<'world_borrow> for &mut T {
    type Item = SingleMut<'world_borrow, T>;
    fn fetch(world: &'world_borrow World) -> Result<Self::Item, FetchError> {
        // The archetypes must be found here.
        let type_id = TypeId::of::<T>();
        for archetype in world.archetypes.iter() {
            for (i, c) in archetype.components.iter().enumerate() {
                if c.type_id == type_id {
                    let borrow = archetype.get(i).try_write().unwrap();
                    return Ok(SingleMut { borrow });
                }
            }
        }

        Err(FetchError::ComponentDoesNotExist(
            ComponentDoesNotExist::new::<T>(),
        ))
    }
}

// Request the data from the world for a specific lifetime.
// This could instead be part of QueryParameter if Generic Associated Types were done.
pub trait QueryParameterFetch<'a> {
    type FetchItem;
    fn fetch(world: &'a World, archetype: usize) -> Result<Self::FetchItem, FetchError>;
}

#[doc(hidden)]
pub struct ReadQueryParameterFetch<T> {
    phantom: std::marker::PhantomData<T>,
}

impl<'a, T: 'static> QueryParameterFetch<'a> for ReadQueryParameterFetch<T> {
    type FetchItem = RwLockReadGuard<'a, Vec<T>>;
    fn fetch(world: &'a World, archetype: usize) -> Result<Self::FetchItem, FetchError> {
        let archetype = &world.archetypes[archetype];
        let type_id = TypeId::of::<T>();

        let index = archetype
            .components
            .iter()
            .position(|c| c.type_id == type_id)
            .unwrap();
        if let Ok(read_guard) = archetype.get(index).try_read() {
            Ok(read_guard)
        } else {
            Err(FetchError::ComponentAlreadyBorrowed(
                ComponentAlreadyBorrowed::new::<T>(),
            ))
        }
    }
}

// QueryParameter should fetch its own data, but the data must be requested for any lifetime
// so an inner trait must be used instead.
// 'QueryParameter' specifies the nature of the data requested, but not the lifetime.
// In the future this can (hopefully) be made better with Generic Associated Types.
pub trait QueryParameter {
    type QueryParameterFetch: for<'a> QueryParameterFetch<'a>;
    fn matches_archetype(archetype: &Archetype) -> bool;
}

impl<T: 'static> QueryParameter for &T {
    type QueryParameterFetch = ReadQueryParameterFetch<T>;

    fn matches_archetype(archetype: &Archetype) -> bool {
        let type_id = TypeId::of::<T>();
        archetype.components.iter().any(|c| c.type_id == type_id)
    }
}

#[doc(hidden)]
pub struct WriteQueryParameterFetch<T> {
    phantom: std::marker::PhantomData<T>,
}

impl<'world_borrow, T: 'static> QueryParameterFetch<'world_borrow> for WriteQueryParameterFetch<T> {
    type FetchItem = RwLockWriteGuard<'world_borrow, Vec<T>>;
    fn fetch(world: &'world_borrow World, archetype: usize) -> Result<Self::FetchItem, FetchError> {
        let archetype = &world.archetypes[archetype];
        let type_id = TypeId::of::<T>();

        let index = archetype
            .components
            .iter()
            .position(|c| c.type_id == type_id)
            .unwrap();
        if let Ok(write_guard) = archetype.get(index).try_write() {
            Ok(write_guard)
        } else {
            Err(FetchError::ComponentAlreadyBorrowed(
                ComponentAlreadyBorrowed::new::<T>(),
            ))
        }
    }
}

impl<T: 'static> QueryParameter for &mut T {
    type QueryParameterFetch = WriteQueryParameterFetch<T>;

    fn matches_archetype(archetype: &Archetype) -> bool {
        let type_id = TypeId::of::<T>();
        archetype.components.iter().any(|c| c.type_id == type_id)
    }
}

pub trait QueryParameters: for<'a> QueryParameterFetch<'a> {}

macro_rules! query_parameters_impl {
    ($($name: ident),*) => {
        impl<'world_borrow, $($name: QueryParameter,)*> QueryParameters
            for ($($name,)*)
        {}

        impl<'world_borrow, $($name: QueryParameter,)*> QueryParameterFetch<'world_borrow> for ($($name,)*) {
            #[allow(unused_parens)]
            type FetchItem = Vec<($(<$name::QueryParameterFetch as QueryParameterFetch<'world_borrow>>::FetchItem),*)>;

            fn fetch(world: &'world_borrow World, _archetype: usize) -> Result<Self::FetchItem, FetchError> {
                let mut archetype_indices = Vec::new();
                for (i, archetype) in world.archetypes.iter().enumerate() {
                    let matches = $($name::matches_archetype(&archetype))&&*;
                    if matches {
                        archetype_indices.push(i);
                    }
                }

                let mut result = Vec::with_capacity(archetype_indices.len());
                for index in archetype_indices {
                    result.push(($(<$name::QueryParameterFetch as QueryParameterFetch<'world_borrow>>::fetch(world, index)?),*));
                }

                Ok(result)
            }

        }
    };
}

query_parameters_impl! {A}
query_parameters_impl! {A, B}
query_parameters_impl! {A, B, C}
query_parameters_impl! {A, B, C, D}
query_parameters_impl! {A, B, C, D, E}
query_parameters_impl! {A, B, C, D, E, F}
query_parameters_impl! {A, B, C, D, E, F, G}
query_parameters_impl! {A, B, C, D, E, F, G, H}
query_parameters_impl! {A, B, C, D, E, F, G, H, I}
query_parameters_impl! {A, B, C, D, E, F, G, H, I, J, K}
query_parameters_impl! {A, B, C, D, E, F, G, H, I, J, K, L}

type QueryParameterItem<'world_borrow, Q> =
    <<Q as QueryParameter>::QueryParameterFetch as QueryParameterFetch<'world_borrow>>::FetchItem;

pub trait QueryIter<'a> {
    type Iter: Iterator;
    fn iter(&'a mut self) -> Self::Iter;
}

impl<'a, 'world_borrow, T: 'static> QueryIter<'a> for RwLockReadGuard<'world_borrow, Vec<T>> {
    type Iter = std::slice::Iter<'a, T>;
    fn iter(&'a mut self) -> Self::Iter {
        <[T]>::iter(self)
    }
}

impl<'a, 'world_borrow, T: 'static> QueryIter<'a> for RwLockWriteGuard<'world_borrow, Vec<T>> {
    type Iter = std::slice::IterMut<'a, T>;
    fn iter(&'a mut self) -> Self::Iter {
        <[T]>::iter_mut(self)
    }
}

impl<'a, 'world_borrow, A: QueryParameter> QueryIter<'a> for Query<'world_borrow, (A,)>
where
    QueryParameterItem<'world_borrow, A>: QueryIter<'a>,
{
    type Iter = ChainedIterator<QueryParameterIter<'a, 'world_borrow, A>>;
    fn iter(&'a mut self) -> Self::Iter {
        ChainedIterator::new(self.data.iter_mut().map(|v| v.iter()).collect())
    }
}

type QueryParameterIter<'a, 'world_borrow, A> =
    <QueryParameterItem<'world_borrow, A> as QueryIter<'a>>::Iter;
impl<'a, 'world_borrow, A: QueryParameter, B: QueryParameter> QueryIter<'a>
    for Query<'world_borrow, (A, B)>
where
    QueryParameterItem<'world_borrow, A>: QueryIter<'a>,
    QueryParameterItem<'world_borrow, B>: QueryIter<'a>,
{
    type Iter = ChainedIterator<
        Zip<QueryParameterIter<'a, 'world_borrow, A>, QueryParameterIter<'a, 'world_borrow, B>>,
    >;
    fn iter(&'a mut self) -> Self::Iter {
        ChainedIterator::new(
            self.data
                .iter_mut()
                .map(|(a, b)| a.iter().zip(b.iter()))
                .collect(),
        )
    }
}

macro_rules! query_iter {
    ($zip_type: ident, $($name: ident),*) => {
        #[allow(non_snake_case)]
        impl<'a, 'world_borrow, $($name: QueryParameter),*> QueryIter<'a> for Query<'world_borrow, ($($name,)*)>
        where
            $(QueryParameterItem<'world_borrow, $name>: QueryIter<'a>),*
             {
            type Iter = ChainedIterator<$zip_type<$(QueryParameterIter<'a, 'world_borrow, $name>,)*>>;
            fn iter(&'a mut self) -> Self::Iter {
                ChainedIterator::new(
                    self.data
                    .iter_mut()
                    .map(|($(ref mut $name,)*)| $zip_type::new($($name.iter(),)*))
                    .collect()
                )
            }
        }
    }
}

query_iter! {Zip3, A, B, C}
query_iter! {Zip4, A, B, C, D}
query_iter! {Zip5, A, B, C, D, E}
query_iter! {Zip6, A, B, C, D, E, F}
query_iter! {Zip7, A, B, C, D, E, F, G}
query_iter! {Zip8, A, B, C, D, E, F, G, H}
