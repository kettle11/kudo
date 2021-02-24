//! A `Query` has `QueryParameters` that define the contents of the Query.
//! Each `QueryParameters` is a tuple of multiple `QueryParameters`.
//! Each `QueryParameter` implements `QueryParameterFetch` which defines how to borrow
//! the parameter from the world.

use crate::{
    Archetype, ChainedIterator, ComponentAlreadyBorrowed, Entity, EntityLocation, Fetch,
    FetchError, FetchItem, World,
};
use std::sync::{RwLockReadGuard, RwLockWriteGuard};

use crate::iterators::*;
use std::any::TypeId;
use std::iter::Zip;

pub struct Query<'world_borrow, T: QueryParameters> {
    data: <T as QueryParametersFetch<'world_borrow>>::FetchItem,
    _world: &'world_borrow World,
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

impl<T: 'static> QueryParameter for &mut T {
    type QueryParameterFetch = WriteQueryParameterFetch<T>;

    fn matches_archetype(archetype: &Archetype) -> bool {
        let type_id = TypeId::of::<T>();
        archetype.components.iter().any(|c| c.type_id == type_id)
    }
}

impl<T: QueryParameter> QueryParameter for Option<T> {
    type QueryParameterFetch = Option<T::QueryParameterFetch>;

    fn matches_archetype(_archetype: &Archetype) -> bool {
        true
    }
}

impl<'world_borrow, T: QueryParameters> Fetch<'world_borrow> for Query<'_, T> {
    type Item = Option<Query<'world_borrow, T>>;
    fn fetch(world: &'world_borrow World) -> Result<Self::Item, FetchError> {
        Ok(Some(Query {
            data: T::fetch(world)?,
            _world: world,
        }))
    }
}

impl<'a, 'world_borrow, T: QueryParameters> FetchItem<'a> for Option<Query<'world_borrow, T>> {
    type InnerItem = Query<'world_borrow, T>;
    fn inner(&'a mut self) -> Self::InnerItem {
        self.take().unwrap()
    }
}

impl<'a, 'world_borrow, T: 'a> FetchItem<'a> for RwLockReadGuard<'world_borrow, T> {
    type InnerItem = &'a T;
    fn inner(&'a mut self) -> Self::InnerItem {
        self
    }
}

impl<'a, 'world_borrow, T: 'a> FetchItem<'a> for RwLockWriteGuard<'world_borrow, T> {
    type InnerItem = &'a mut T;
    fn inner(&'a mut self) -> Self::InnerItem {
        self
    }
}

pub trait QueryParameters: for<'a> QueryParametersFetch<'a> {}
pub trait QueryParametersFetch<'world_borrow> {
    type FetchItem;
    fn fetch(world: &'world_borrow World) -> Result<Self::FetchItem, FetchError>;
}

macro_rules! query_parameters_impl {
    ($($name: ident),*) => {
        impl<'world_borrow, $($name: QueryParameter,)*> QueryParameters
            for ($($name,)*)
        {
        }

        impl<'world_borrow, $($name: QueryParameter,)*> QueryParametersFetch<'world_borrow> for ($($name,)*) {
            // This stores a Vec of archetype borrows.
            // An archetype borrow is stored as a tuple of individual channel borrows.
            // Some sort of additional info about which archetype is which needs to be stored here.
            // Perhaps just an archetype ID.
            #[allow(unused_parens)]
            type FetchItem = Vec<ArchetypeBorrow<($(<$name::QueryParameterFetch as QueryParameterFetch<'world_borrow>>::FetchItem),*)>>;

            fn fetch(world: &'world_borrow World) -> Result<Self::FetchItem, FetchError> {
                let mut archetype_indices = Vec::new();
                for (i, archetype) in world.archetypes.iter().enumerate() {
                    let matches = $($name::matches_archetype(&archetype))&&*;
                    if matches {
                        archetype_indices.push(i);
                    }
                }

                let mut result = Vec::with_capacity(archetype_indices.len());
                for archetype_index in archetype_indices {
                    result.push(ArchetypeBorrow {
                        _archetype_index: archetype_index,
                        data: ($(<$name::QueryParameterFetch as QueryParameterFetch<'world_borrow>>::fetch(world, archetype_index)?),*)
                    });
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
query_parameters_impl! {A, B, C, D, E, F, G, H, I, J}
query_parameters_impl! {A, B, C, D, E, F, G, H, I, J, K}
query_parameters_impl! {A, B, C, D, E, F, G, H, I, J, K, L}

#[doc(hidden)]
pub struct ArchetypeBorrow<T> {
    _archetype_index: usize,
    data: T,
}

// Request the data from the world for a specific lifetime.
// This could instead be part of QueryParameter if Generic Associated Types were done.
pub trait QueryParameterFetch<'a> {
    type FetchItem;
    fn fetch(world: &'a World, archetype: usize) -> Result<Self::FetchItem, FetchError>;
}

/// If the component is already borrowed this is an error to request.
/// Otherwise None is returned if the component does not exist.
impl<'a, T: QueryParameterFetch<'a>> QueryParameterFetch<'a> for Option<T> {
    type FetchItem = Option<T::FetchItem>;
    fn fetch(world: &'a World, archetype: usize) -> Result<Self::FetchItem, FetchError> {
        match T::fetch(world, archetype) {
            Ok(result) => Ok(Some(result)),
            Err(FetchError::MissingComponent) => Ok(None),
            Err(e) => Err(e),
        }
    }
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
            .position(|c| c.type_id == type_id);

        if let Some(index) = index {
            if let Ok(read_guard) = archetype.get(index).try_read() {
                Ok(read_guard)
            } else {
                Err(FetchError::ComponentAlreadyBorrowed(
                    ComponentAlreadyBorrowed::new::<T>(),
                ))
            }
        } else {
            Err(FetchError::MissingComponent)
        }
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
            .position(|c| c.type_id == type_id);

        if let Some(index) = index {
            if let Ok(write_guard) = archetype.get(index).try_write() {
                Ok(write_guard)
            } else {
                Err(FetchError::ComponentAlreadyBorrowed(
                    ComponentAlreadyBorrowed::new::<T>(),
                ))
            }
        } else {
            Err(FetchError::MissingComponent)
        }
    }
}

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
        ChainedIterator::new(self.data.iter_mut().map(|v| v.data.iter()).collect())
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
                .map(|ArchetypeBorrow { data: (a, b), .. }| a.iter().zip(b.iter()))
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
                    .map(|ArchetypeBorrow{data: ($(ref mut $name,)*), ..}| $zip_type::new($($name.iter(),)*))
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
query_iter! {Zip9, A, B, C, D, E, F, G, H, I}
query_iter! {Zip10, A, B, C, D, E, F, G, H, I, J}
query_iter! {Zip11, A, B, C, D, E, F, G, H, I, J, K}

macro_rules! get_entity_component {
    ($($name: ident),*) => {
impl<'a, 'world_borrow, $($name: QueryParameter),*> GetEntityComponent<'a> for Query<'world_borrow, ($($name,)*)>
where
    $(QueryParameterItem<'world_borrow, $name>: GetEntityComponentInner<'a>),*
     {
        fn get_entity_component<T>(&'a mut self, entity: Entity) -> Option<&T> {
            let entity_info = self._world.get_entity_info(entity)?;
            for d in self.data.iter_mut() {
                if let Some(result) = d.data.get_entity_component::<T>(entity_info.location) {
                    return Some(result);
                }
            }
            None
        }
    }
}
}

get_entity_component! {A}
/*
get_entity_component! {A, B}
get_entity_component! {A, B, C}
get_entity_component! {A, B, C, D}
get_entity_component! {A, B, C, D, E}
get_entity_component! {A, B, C, D, E, F}
get_entity_component! {A, B, C, D, E, F, G}
get_entity_component! {A, B, C, D, E, F, G, H}
get_entity_component! {A, B, C, D, E, F, G, H, I}
get_entity_component! { A, B, C, D, E, F, G, H, I, J}
get_entity_component! { A, B, C, D, E, F, G, H, I, J, K}
*/

pub trait GetEntityComponent<'a> {
    fn get_entity_component<T>(&'a mut self, entity: Entity) -> Option<&T>;
}

pub trait GetEntityComponentInner<'a> {
    fn get_entity_component<T>(&'a mut self, entity: EntityLocation) -> Option<&T>;
}

// ------------ Other types of query parameters----------------------

/// This is used to test if an entity has a component, without actually
/// needing to read or write to that component.
pub struct Has<T> {
    pub value: bool,
    phantom: std::marker::PhantomData<T>,
}

impl<'world_borrow, T: 'static> QueryParameterFetch<'world_borrow> for Has<T> {
    type FetchItem = bool;
    fn fetch(world: &'world_borrow World, archetype: usize) -> Result<Self::FetchItem, FetchError> {
        let archetype = &world.archetypes[archetype];
        let type_id = TypeId::of::<T>();

        let contains = archetype.components.iter().any(|c| c.type_id == type_id);
        Ok(contains)
    }
}

// If a boolean value is reported, just repeat its result.
impl<'a, 'world_borrow> QueryIter<'a> for bool {
    type Iter = std::iter::Repeat<bool>;
    fn iter(&'a mut self) -> Self::Iter {
        std::iter::repeat(*self)
    }
}

impl<T: 'static> QueryParameter for Has<T> {
    type QueryParameterFetch = Self;

    fn matches_archetype(_archetype: &Archetype) -> bool {
        true
    }
}

impl<'a, 'world_borrow, T: QueryIter<'a>> QueryIter<'a> for Option<T> {
    type Iter = OptionIterator<T::Iter>;
    fn iter(&'a mut self) -> Self::Iter {
        if let Some(t) = self {
            OptionIterator {
                iter: Some(t.iter()),
            }
        } else {
            OptionIterator { iter: None }
        }
    }
}

use std::iter::Iterator;
pub struct OptionIterator<T: Iterator> {
    iter: Option<T>,
}

impl<T: Iterator> Iterator for OptionIterator<T> {
    type Item = Option<T::Item>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(iter) = &mut self.iter {
            Some(iter.next())
        } else {
            Some(None)
        }
    }
}
