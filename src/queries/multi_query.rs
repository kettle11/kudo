use super::*;
use crate::{
    archetype::Archetype,
    iterators::*,
    storage_lookup::{Filter, FilterType},
    Entity,
};

use std::any::Any;

use crate::{entities::Entities, ChainedIterator};
use std::{
    any::TypeId,
    iter::Zip,
    sync::{RwLockReadGuard, RwLockWriteGuard},
};

use crate::ArchetypeMatch;

pub struct Query<'a, T: QueryParameters> {
    entities: &'a Entities,
    archetype_borrows: Vec<ArchetypeBorrow<'a, <T as QueryParametersBorrow<'a>>::ComponentBorrows>>,
}

impl<'a: 'b, 'b, T: 'b + QueryParameters> Query<'a, T>
where
    &'b Self: IntoIterator,
{
    pub fn iter(&'b self) -> <&'b Self as IntoIterator>::IntoIter {
        self.into_iter()
    }
}

impl<'a: 'b, 'b, T: 'b + QueryParameters> Query<'a, T>
where
    &'b mut Self: IntoIterator,
{
    pub fn iter_mut(&'b mut self) -> <&mut Self as IntoIterator>::IntoIter {
        self.into_iter()
    }
}

pub trait QueryParameters: for<'a> QueryParametersBorrow<'a> {}

pub trait QueryParametersBorrow<'a> {
    type ComponentBorrows;
}

pub struct ArchetypeBorrow<'a, T> {
    /// A tuple of individual components.
    component_borrows: T,
    entities: RwLockReadGuard<'a, Vec<Entity>>,
    #[allow(unused)]
    archetype_index: usize,
}

pub trait QueryParameter: for<'a> QueryParameterBorrow<'a> {
    fn filter() -> Filter;
    fn write() -> bool;
}

pub trait QueryParameterBorrow<'a> {
    type ParameterBorrow;
    fn borrow(
        archetype: &'a Archetype,
        channel_index: Option<usize>,
    ) -> Result<Self::ParameterBorrow, Error>;
}

impl<T: 'static> QueryParameter for &T {
    fn filter() -> Filter {
        Filter {
            filter_type: FilterType::With,
            type_id: TypeId::of::<T>(),
        }
    }
    fn write() -> bool {
        false
    }
}

impl<'a, T: 'static> QueryParameterBorrow<'a> for &T {
    type ParameterBorrow = RwLockReadGuard<'a, Vec<T>>;
    fn borrow(
        archetype: &'a Archetype,
        channel_index: Option<usize>,
    ) -> Result<Self::ParameterBorrow, Error> {
        archetype.borrow_channel(channel_index.unwrap())
    }
}

impl<Q: QueryParameter> QueryParameter for Option<Q> {
    fn filter() -> Filter {
        let inner_filter = Q::filter();
        Filter {
            filter_type: FilterType::Optional,
            type_id: inner_filter.type_id,
        }
    }

    fn write() -> bool {
        Q::write()
    }
}

impl<'a, Q: QueryParameterBorrow<'a>> QueryParameterBorrow<'a> for Option<Q> {
    type ParameterBorrow = Option<Q::ParameterBorrow>;
    fn borrow(
        archetype: &'a Archetype,
        channel_index: Option<usize>,
    ) -> Result<Self::ParameterBorrow, Error> {
        Ok(if let Some(channel_index) = channel_index {
            Some(Q::borrow(archetype, Some(channel_index))?)
        } else {
            None
        })
    }
}

impl<T: 'static> QueryParameter for &mut T {
    fn filter() -> Filter {
        Filter {
            filter_type: FilterType::With,
            type_id: TypeId::of::<T>(),
        }
    }
    fn write() -> bool {
        true
    }
}

impl<'a, T: 'static> QueryParameterBorrow<'a> for &mut T {
    type ParameterBorrow = RwLockWriteGuard<'a, Vec<T>>;
    fn borrow(
        archetype: &'a Archetype,
        channel_index: Option<usize>,
    ) -> Result<Self::ParameterBorrow, Error> {
        archetype.borrow_channel_mut(channel_index.unwrap())
    }
}

pub struct QueryInfo<const CHANNELS: usize> {
    archetypes: Vec<ArchetypeMatch<CHANNELS>>,
    write: [bool; CHANNELS],
}

macro_rules! query_impl{
    ($count: expr, $($name: ident),*) => {
        impl<$($name: QueryParameter),*> QueryParameters for ($($name,)*) {
        }

        impl<'a, $($name: QueryParameterBorrow<'a>),*> QueryParametersBorrow<'a> for ($($name,)*) {
           // type ComponentBorrows = ($($name,)*);
           type ComponentBorrows = ($($name::ParameterBorrow,)*);

        }

        impl<'a, $($name: QueryParameter),*> QueryTrait<'a> for Query<'_, ($($name,)*)> {
            type Result = Option<Query<'a, ($($name,)*)>>;

            #[allow(non_snake_case)]
            fn get_query(world: &'a World, query_info: &Self::QueryInfo) -> Result<Self::Result, Error> {
                let mut archetype_borrows = Vec::with_capacity(query_info.archetypes.len());
                for archetype_info in &query_info.archetypes {
                    let archetype = &world.archetypes[archetype_info.archetype_index];
                    let [$($name,)*] = archetype_info.channels;

                    archetype_borrows.push(ArchetypeBorrow {
                        component_borrows: (
                            $(<$name as QueryParameterBorrow<'a>>::borrow(archetype, $name)?,)*
                        ),
                        archetype_index: archetype_info.archetype_index,
                        entities: archetype.entities.read().unwrap(),
                    })
                }
                Ok(Some(Query { entities: &world.entities, archetype_borrows }))
            }
        }

        // It almost seems like there might be a way to make this more generic.
        // I think these implementations could be made totally generic by making QueryParameters
        // implement a way to get all type ids.
        impl<'a, $($name: QueryParameter), *> GetQueryInfoTrait for Query<'a, ($($name,)*)> {
            type QueryInfo = QueryInfo<$count>;
            fn query_info(world: &World) -> Result<Self::QueryInfo, Error> {
                let type_ids: [Filter; $count] = [
                    $($name::filter()),*
                ];

                let mut archetypes = world.storage_lookup.get_matching_archetypes(&type_ids, &[]);

                // Sort archetypes so that we can later binary search the archetypes when
                // finding entities.
                archetypes.sort_by_key(|a| a.archetype_index);

                // Look up resource index.
                for archetype_match in &mut archetypes {
                    let archetype = &world.archetypes[archetype_match.archetype_index];
                    for (channel, resource_index) in archetype_match.channels.iter().zip(archetype_match.resource_indices.iter_mut()) {
                        *resource_index = channel.map(|channel| archetype.channels[channel].channel_id);
                    }
                }

                let write = [$($name::write(),)*];
                Ok(QueryInfo { archetypes, write})
            }
        }

        impl<'a, $($name: GetComponent), *> GetComponent for ($($name,)*) {
            #[allow(unused, non_snake_case)]
            fn get_component<T: 'static>(&self, index: usize) -> Option<&T> {
                let ($($name,)*) = self;
                $(if let Some(v) = $name.get_component(index) { return Some(v); })*
                None
            }
        }

        impl<'a, $($name: GetComponentMut), *> GetComponentMut for ($($name,)*) {
            #[allow(unused, non_snake_case)]
            fn get_component_mut<T: 'static>(&mut self, index: usize) -> Option<&mut T> {
                let ($($name,)*) = self;
                $(if let Some(v) = $name.get_component_mut(index) { return Some(v); })*
                None
            }
        }
    }
}

query_impl! { 0, }
query_impl! { 1, A }
query_impl! { 2, A, B}
query_impl! { 3, A, B, C}
query_impl! { 4, A, B, C, D}
query_impl! { 5, A, B, C, D, E}
query_impl! { 6, A, B, C, D, E, F}
query_impl! { 7, A, B, C, D, E, F, G}
query_impl! { 8, A, B, C, D, E, F, G, H}
query_impl! { 9, A, B, C, D, E, F, G, H, I}
query_impl! { 10, A, B, C, D, E, F, G, H, I, J}
query_impl! { 11, A, B, C, D, E, F, G, H, I, J, K}

impl<'a, const CHANNELS: usize> QueryInfoTrait for QueryInfo<CHANNELS> {
    fn borrows(&self) -> ResourceBorrows {
        let mut writes = Vec::new();
        let mut reads = Vec::new();

        for archetype_match in self.archetypes.iter() {
            for (id, write) in archetype_match
                .resource_indices
                .iter()
                .zip(self.write.iter())
            {
                if *write {
                    if let Some(id) = id {
                        writes.push(*id);
                    }
                } else {
                    if let Some(id) = id {
                        reads.push(*id);
                    }
                }
            }
        }
        ResourceBorrows { writes, reads }
    }
}

impl<'a, 'b, Q: QueryParameters> AsSystemArg<'b> for Option<Query<'a, Q>> {
    type Arg = Query<'a, Q>;
    fn as_system_arg(&'b mut self) -> Self::Arg {
        self.take().unwrap()
    }
}

pub trait GetQueryDirect {
    type Arg;
    fn get_query_direct(self) -> Self::Arg;
}

impl<'a, 'b, Q: QueryParameters> GetQueryDirect for Option<Query<'a, Q>> {
    type Arg = Query<'a, Q>;
    fn get_query_direct(mut self) -> Self::Arg {
        self.take().unwrap()
    }
}
//-------------- ITERATOR STUFF --------------

// The `GetIterator` and `GetIteratorMut` traits allow a different type of iterator to be
//  returned in mutable vs non-mutable cases.
// I still feel like there might be a way to do without them, but they work for now.

pub trait GetIterator<'a> {
    type Iter: Iterator;
    fn get_iter(&'a self) -> Self::Iter;
}

pub trait GetIteratorMut<'a> {
    type Iter: Iterator;
    fn get_iter_mut(&'a mut self) -> Self::Iter;
}

impl<'a, 'world_borrow, T: 'static> GetIterator<'a> for RwLockReadGuard<'world_borrow, Vec<T>> {
    type Iter = std::slice::Iter<'a, T>;
    fn get_iter(&'a self) -> Self::Iter {
        <[T]>::iter(self)
    }
}

impl<'a, 'world_borrow, T: 'static> GetIteratorMut<'a> for RwLockReadGuard<'world_borrow, Vec<T>> {
    type Iter = std::slice::Iter<'a, T>;
    fn get_iter_mut(&'a mut self) -> Self::Iter {
        <[T]>::iter(self)
    }
}

impl<'a, 'world_borrow, T: 'static> GetIteratorMut<'a> for RwLockWriteGuard<'world_borrow, Vec<T>> {
    type Iter = std::slice::IterMut<'a, T>;
    fn get_iter_mut(&'a mut self) -> Self::Iter {
        <[T]>::iter_mut(self)
    }
}

impl<'a, 'world_borrow, T: 'static> GetIterator<'a> for RwLockWriteGuard<'world_borrow, Vec<T>> {
    type Iter = std::slice::Iter<'a, T>;
    fn get_iter(&'a self) -> Self::Iter {
        <[T]>::iter(self)
    }
}

impl<'a, 'world_borrow, G: GetIterator<'a>> GetIterator<'a> for Option<G> {
    type Iter = OptionIterator<G::Iter>;
    fn get_iter(&'a self) -> Self::Iter {
        OptionIterator::new(self.as_ref().map(|s| s.get_iter()))
    }
}

impl<'a, 'world_borrow, G: GetIteratorMut<'a>> GetIteratorMut<'a> for Option<G> {
    type Iter = OptionIterator<G::Iter>;
    fn get_iter_mut(&'a mut self) -> Self::Iter {
        OptionIterator::new(self.as_mut().map(|s| s.get_iter_mut()))
    }
}

impl<'a, 'b, T: QueryParameters> IntoIterator for &'b Query<'a, T>
where
    <T as QueryParametersBorrow<'a>>::ComponentBorrows: GetIterator<'b>,
{
    type Item = <Self::IntoIter as Iterator>::Item;
    type IntoIter = ChainedIterator<
        <<T as QueryParametersBorrow<'a>>::ComponentBorrows as GetIterator<'b>>::Iter,
    >;

    fn into_iter(self) -> Self::IntoIter {
        ChainedIterator::new(
            self.archetype_borrows
                .iter()
                .map(|i| i.component_borrows.get_iter())
                .collect(),
        )
    }
}

impl<'a, 'b, T: QueryParameters> IntoIterator for &'b mut Query<'a, T>
where
    <T as QueryParametersBorrow<'a>>::ComponentBorrows: GetIteratorMut<'b>,
{
    type Item = <Self::IntoIter as Iterator>::Item;
    type IntoIter = ChainedIterator<
        <<T as QueryParametersBorrow<'a>>::ComponentBorrows as GetIteratorMut<'b>>::Iter,
    >;

    fn into_iter(self) -> Self::IntoIter {
        ChainedIterator::new(
            self.archetype_borrows
                .iter_mut()
                .map(|i| i.component_borrows.get_iter_mut())
                .collect(),
        )
    }
}

impl<'a, 'b, T: 'b + QueryParameters> Query<'a, T> {
    pub fn entities(&'b self) -> ChainedIterator<std::iter::Copied<std::slice::Iter<'b, Entity>>> {
        ChainedIterator::new(
            self.archetype_borrows
                .iter()
                .map(|i| i.entities.iter().copied())
                .collect(),
        )
    }
}

impl<'b, A: GetIterator<'b>> IntoIterator for &'b ArchetypeBorrow<'_, (A,)> {
    type Item = <Self::IntoIter as Iterator>::Item;
    type IntoIter = A::Iter;
    fn into_iter(self) -> Self::IntoIter {
        self.component_borrows.0.get_iter()
    }
}

impl<'b, A: GetIterator<'b>> GetIterator<'b> for (A,) {
    type Iter = A::Iter;
    fn get_iter(&'b self) -> Self::Iter {
        self.0.get_iter()
    }
}

impl<'b, A: GetIterator<'b>, B: GetIterator<'b>> GetIterator<'b> for (A, B) {
    type Iter = Zip<A::Iter, B::Iter>;
    fn get_iter(&'b self) -> Self::Iter {
        self.0.get_iter().zip(self.1.get_iter())
    }
}

impl<'b, A: GetIteratorMut<'b>> GetIteratorMut<'b> for (A,) {
    type Iter = A::Iter;
    fn get_iter_mut(&'b mut self) -> Self::Iter {
        self.0.get_iter_mut()
    }
}

impl<'b, A: GetIteratorMut<'b>, B: GetIteratorMut<'b>> GetIteratorMut<'b> for (A, B) {
    type Iter = Zip<A::Iter, B::Iter>;
    fn get_iter_mut(&'b mut self) -> Self::Iter {
        self.0.get_iter_mut().zip(self.1.get_iter_mut())
    }
}

macro_rules! iterator{
    ($zip_type: ident, $($name: ident),*) => {
        #[allow(non_snake_case)]
        impl<'b, $($name: GetIterator<'b>),*> GetIterator<'b> for ($($name,)*)
             {
            type Iter = $zip_type<$($name::Iter,)*>;
            fn get_iter(&'b self) -> Self::Iter {
                let ($($name,)*) = self;
                $zip_type::new($($name.get_iter(),)*)
            }
        }

        #[allow(non_snake_case)]
        impl<'b, $($name: GetIteratorMut<'b>),*> GetIteratorMut<'b> for ($($name,)*)
             {
            type Iter = $zip_type<$($name::Iter,)*>;
            fn get_iter_mut(&'b mut self) -> Self::Iter {
                let ($($name,)*) = self;
                $zip_type::new($($name.get_iter_mut(),)*)
            }
        }
    }
}

iterator! {Zip3, A, B, C}
iterator! {Zip4, A, B, C, D}
iterator! {Zip5, A, B, C, D, E}
iterator! {Zip6, A, B, C, D, E, F}
iterator! {Zip7, A, B, C, D, E, F, G}
iterator! {Zip8, A, B, C, D, E, F, G, H}
iterator! {Zip9, A, B, C, D, E, F, G, H, I}
iterator! {Zip10, A, B, C, D, E, F, G, H, I, J}
iterator! {Zip11, A, B, C, D, E, F, G, H, I, J, K}

// --------- END ITERATOR STUFF ---------------

// ---------- GET COMPONENT -----------
impl<'a, T: QueryParameters> Query<'a, T>
where
    <T as QueryParametersBorrow<'a>>::ComponentBorrows: GetComponent,
{
    pub fn get_component<A: 'static>(&self, entity: Entity) -> Option<&A> {
        let entity = self.entities.get_location(entity)?;
        let archetype = self
            .archetype_borrows
            .binary_search_by_key(&entity.archetype_index, |a| a.archetype_index)
            .ok()?;

        self.archetype_borrows[archetype]
            .component_borrows
            .get_component(entity.index_within_archetype)
    }
}
impl<'a, T: QueryParameters> Query<'a, T>
where
    <T as QueryParametersBorrow<'a>>::ComponentBorrows: GetComponentMut,
{
    pub fn get_component_mut<A: 'static>(&mut self, entity: Entity) -> Option<&mut A> {
        let entity = self.entities.get_location(entity)?;
        let archetype = self
            .archetype_borrows
            .binary_search_by_key(&entity.archetype_index, |a| a.archetype_index)
            .ok()?;

        self.archetype_borrows[archetype]
            .component_borrows
            .get_component_mut(entity.index_within_archetype)
    }
}

pub trait GetComponent {
    fn get_component<T: 'static>(&self, index: usize) -> Option<&T>;
}

pub trait GetComponentMut {
    fn get_component_mut<T: 'static>(&mut self, index: usize) -> Option<&mut T>;
}

impl<T: 'static> GetComponent for RwLockReadGuard<'_, Vec<T>> {
    fn get_component<A: 'static>(&self, index: usize) -> Option<&A> {
        let s = (self as &Vec<T> as &dyn Any).downcast_ref::<Vec<A>>()?;
        s.get(index)
    }
}

impl<T: 'static> GetComponent for RwLockWriteGuard<'_, Vec<T>> {
    fn get_component<A: 'static>(&self, index: usize) -> Option<&A> {
        let s = (self as &Vec<T> as &dyn Any).downcast_ref::<Vec<A>>()?;
        s.get(index)
    }
}

impl<T: 'static> GetComponentMut for RwLockWriteGuard<'_, Vec<T>> {
    fn get_component_mut<A: 'static>(&mut self, index: usize) -> Option<&mut A> {
        let s = (self as &mut Vec<T> as &mut dyn Any).downcast_mut::<Vec<A>>()?;
        s.get_mut(index)
    }
}

impl<T: 'static> GetComponentMut for RwLockReadGuard<'_, Vec<T>> {
    fn get_component_mut<A: 'static>(&mut self, _index: usize) -> Option<&mut A> {
        // Perhaps this should be a more specific error if the type is within this but not mutable.
        None
    }
}
