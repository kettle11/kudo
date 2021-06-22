use super::*;
use crate::*;

use std::any::Any;

use crate::{entities::Entities, ChainedIterator};
use std::{
    any::TypeId,
    iter::Zip,
    sync::{RwLockReadGuard, RwLockWriteGuard},
};

use crate::ArchetypeMatch;

pub struct Query<'a, T: QueryParameters + 'static> {
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

pub trait QueryParameters: for<'a> QueryParametersBorrow<'a> + 'static {}

pub trait QueryParametersBorrow<'a> {
    type ComponentBorrows;
}

/// A chunk of data borrowed from an Archetype.
pub struct ArchetypeBorrow<'a, T> {
    /// A tuple of individual components.
    component_borrows: T,
    entities: RwLockReadGuard<'a, Vec<Entity>>,
    #[allow(unused)]
    archetype_index: usize,
}

pub trait QueryParameter: for<'a> QueryParameterBorrow<'a> + 'static {
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

impl<T: 'static> QueryParameter for &'static T {
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

impl<'a, T: 'static> QueryParameterBorrow<'a> for &'static T {
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

impl<T: 'static> QueryParameter for &'static mut T {
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

impl<'a, T: 'static> QueryParameterBorrow<'a> for &'static mut T {
    type ParameterBorrow = RwLockWriteGuard<'a, Vec<T>>;
    fn borrow(
        archetype: &'a Archetype,
        channel_index: Option<usize>,
    ) -> Result<Self::ParameterBorrow, Error> {
        archetype.channel_mut(channel_index.unwrap())
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
                    let archetype = world.borrow_archetype(archetype_info.archetype_index);
                    let [$($name,)*] = archetype_info.channels;

                    archetype_borrows.push(ArchetypeBorrow {
                        component_borrows: (
                            $(<$name as QueryParameterBorrow<'a>>::borrow(archetype, $name)?,)*
                        ),
                        archetype_index: archetype_info.archetype_index,
                        entities: archetype.entities(),
                    })
                }
                Ok(Some(Query { entities: world.entities(), archetype_borrows }))
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

                let mut archetypes = world.storage_lookup().get_matching_archetypes(&type_ids, &[]);

                // Sort archetypes so that we can later binary search the archetypes when
                // finding entities.
                archetypes.sort_by_key(|a| a.archetype_index);

                // Look up resource index.
                /*
                for archetype_match in &mut archetypes {
                    let archetype = &world.borrow_archetype(archetype_match.archetype_index);
                    for (channel, resource_index) in archetype_match.channels.iter().zip(archetype_match.resource_indices.iter_mut()) {
                        *resource_index = channel.map(|channel| archetype.channels[channel].channel_id);
                    }
                }
                */

                let write = [$($name::write(),)*];
                Ok(QueryInfo { archetypes, write})
            }
        }

        impl<'a, $($name: GetComponent<'a>), *> GetComponent<'a> for ($($name,)*) {
            type Component = ($($name::Component,)*);

            #[allow(unused, non_snake_case)]
            fn get_components(&'a self, index: usize) -> Self::Component {
                let ($($name,)*) = self;
                ($($name.get_components(index),)*)
            }

            #[allow(unused, non_snake_case)]
            fn get_component<T: 'static>(&self, index: usize) -> Option<&T> {
                let ($($name,)*) = self;
                $(if let Some(v) = $name.get_component(index) { return Some(v); })*
                None
            }
        }

        impl<'a, $($name: GetComponentMut<'a>), *> GetComponentMut<'a> for ($($name,)*) {
            type Component = ($($name::Component,)*);

            #[allow(unused, non_snake_case)]
            fn get_components_mut(&'a mut self, index: usize) -> Self::Component {
                let ($($name,)*) = self;
                ($($name.get_components_mut(index),)*)
            }

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
                } else if let Some(id) = id {
                    reads.push(*id);
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
pub trait GetComponents<'a> {
    type Components;
    fn get_result(&'a self) -> Self::Components;
}

pub trait GetComponentsMut<'a> {
    type Components;
    fn get_result(&'a mut self) -> Self::Components;
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
    pub fn entities(&'b self) -> ChainedIterator<EntityCloneIter<'b>> {
        ChainedIterator::new(
            self.archetype_borrows
                .iter()
                .map(|i| EntityCloneIter {
                    iter: i.entities.iter(),
                })
                .collect(),
        )
    }
}

pub struct EntityCloneIter<'a> {
    iter: std::slice::Iter<'a, Entity>,
}

impl<'a> Iterator for EntityCloneIter<'a> {
    type Item = Entity;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().copied()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<'a: 'b, 'b, T: 'b + QueryParameters> Query<'a, T>
where
    <T as QueryParametersBorrow<'a>>::ComponentBorrows: GetIterator<'b>,
{
    pub fn entities_and_components(
        &'b self,
    ) -> ChainedIterator<
        Zip<
            EntityCloneIter<'b>,
            <<T as QueryParametersBorrow<'a>>::ComponentBorrows as GetIterator<'b>>::Iter,
        >,
    > {
        ChainedIterator::new(
            self.archetype_borrows
                .iter()
                .map(|i| {
                    EntityCloneIter {
                        iter: i.entities.iter(),
                    }
                    .zip(i.component_borrows.get_iter())
                })
                .collect(),
        )
    }
}

impl<'a: 'b, 'b, T: 'b + QueryParameters> Query<'a, T>
where
    <T as QueryParametersBorrow<'a>>::ComponentBorrows: GetIteratorMut<'b>,
{
    pub fn entities_and_components_mut(
        &'b mut self,
    ) -> ChainedIterator<
        Zip<
            EntityCloneIter<'b>,
            <<T as QueryParametersBorrow<'a>>::ComponentBorrows as GetIteratorMut<'b>>::Iter,
        >,
    > {
        ChainedIterator::new(
            self.archetype_borrows
                .iter_mut()
                .map(|i| {
                    EntityCloneIter {
                        iter: i.entities.iter(),
                    }
                    .zip(i.component_borrows.get_iter_mut())
                })
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
impl<'a, 'b, T: QueryParameters> Query<'a, T>
where
    <T as QueryParametersBorrow<'a>>::ComponentBorrows: GetComponent<'b>,
{
    pub fn get_component<A: 'static>(&self, entity: Entity) -> Option<&A> {
        let entity = self.entities.get_location(entity)??;
        let archetype = self
            .archetype_borrows
            .binary_search_by_key(&entity.archetype_index, |a| a.archetype_index)
            .ok()?;

        self.archetype_borrows[archetype]
            .component_borrows
            .get_component(entity.index_within_archetype)
    }

    /// Gets all components from this `Query` for a specific Entity.
    pub fn get_entity_components(
        &'b self,
        entity: Entity,
    ) -> Option<<<T as QueryParametersBorrow<'a>>::ComponentBorrows as GetComponent<'b>>::Component>
    {
        let entity = self.entities.get_location(entity)??;
        let archetype = self
            .archetype_borrows
            .binary_search_by_key(&entity.archetype_index, |a| a.archetype_index)
            .ok()?;

        Some(
            self.archetype_borrows[archetype]
                .component_borrows
                .get_components(entity.index_within_archetype),
        )
    }
}

impl<'a, 'b, T: QueryParameters> Query<'a, T>
where
    <T as QueryParametersBorrow<'a>>::ComponentBorrows: GetComponentMut<'b>,
{
    pub fn get_component_mut<A: 'static>(&mut self, entity: Entity) -> Option<&mut A> {
        let entity = self.entities.get_location(entity)??;
        let archetype = self
            .archetype_borrows
            .binary_search_by_key(&entity.archetype_index, |a| a.archetype_index)
            .ok()?;

        self.archetype_borrows[archetype]
            .component_borrows
            .get_component_mut(entity.index_within_archetype)
    }

    /// Gets all components from this `Query` for a specific Entity.
    pub fn get_entity_components_mut(
        &'b mut self,
        entity: Entity,
    ) -> Option<
        <<T as QueryParametersBorrow<'a>>::ComponentBorrows as GetComponentMut<'b>>::Component,
    > {
        let entity = self.entities.get_location(entity)??;
        let archetype = self
            .archetype_borrows
            .binary_search_by_key(&entity.archetype_index, |a| a.archetype_index)
            .ok()?;

        Some(
            self.archetype_borrows[archetype]
                .component_borrows
                .get_components_mut(entity.index_within_archetype),
        )
    }
}

pub trait GetComponent<'a> {
    type Component;
    fn get_components(&'a self, index: usize) -> Self::Component;
    fn get_component<T: 'static>(&self, index: usize) -> Option<&T>;
}

pub trait GetComponentMut<'a> {
    type Component;
    fn get_components_mut(&'a mut self, index: usize) -> Self::Component;
    fn get_component_mut<T: 'static>(&mut self, index: usize) -> Option<&mut T>;
}

impl<'a, T: 'static> GetComponent<'a> for RwLockReadGuard<'_, Vec<T>> {
    type Component = &'a T;
    fn get_components(&'a self, index: usize) -> Self::Component {
        &self[index]
    }
    fn get_component<A: 'static>(&self, index: usize) -> Option<&A> {
        let s = (self as &Vec<T> as &dyn Any).downcast_ref::<Vec<A>>()?;
        s.get(index)
    }
}

impl<'a, T: 'static> GetComponent<'a> for RwLockWriteGuard<'_, Vec<T>> {
    type Component = &'a T;
    fn get_components(&'a self, index: usize) -> Self::Component {
        &self[index]
    }

    fn get_component<A: 'static>(&self, index: usize) -> Option<&A> {
        let s = (self as &Vec<T> as &dyn Any).downcast_ref::<Vec<A>>()?;
        s.get(index)
    }
}

impl<'a, G: GetComponent<'a> + 'a> GetComponent<'a> for Option<G> {
    type Component = Option<<G as GetComponent<'a>>::Component>;

    fn get_components(&'a self, index: usize) -> Self::Component {
        self.as_ref().map(|v| v.get_components(index))
    }

    fn get_component<A: 'static>(&self, index: usize) -> Option<&A> {
        self.as_ref().map(|v| v.get_component::<A>(index)).flatten()
    }
}

impl<'a, T: 'static> GetComponentMut<'a> for RwLockWriteGuard<'_, Vec<T>> {
    type Component = &'a mut T;
    fn get_components_mut(&'a mut self, index: usize) -> Self::Component {
        &mut self[index]
    }

    fn get_component_mut<A: 'static>(&mut self, index: usize) -> Option<&mut A> {
        let s = (self as &mut Vec<T> as &mut dyn Any).downcast_mut::<Vec<A>>()?;
        s.get_mut(index)
    }
}

// A ReadGuard cannot mutably get a component.
impl<'a, T: 'static> GetComponentMut<'a> for RwLockReadGuard<'_, Vec<T>> {
    type Component = &'a T;

    fn get_components_mut(&'a mut self, index: usize) -> Self::Component {
        &self[index]
    }

    fn get_component_mut<A: 'static>(&mut self, _index: usize) -> Option<&mut A> {
        // Perhaps this should be a more specific error if the type is within this but not mutable.
        None
    }
}

impl<'a, G: GetComponentMut<'a> + 'a> GetComponentMut<'a> for Option<G> {
    type Component = Option<<G as GetComponentMut<'a>>::Component>;

    fn get_components_mut(&'a mut self, index: usize) -> Self::Component {
        self.as_mut().map(|v| v.get_components_mut(index))
    }

    fn get_component_mut<A: 'static>(&mut self, index: usize) -> Option<&mut A> {
        self.as_mut()
            .map(|v| v.get_component_mut::<A>(index))
            .flatten()
    }
}

impl<'world_borrow, Q: QueryParameters> Query<'world_borrow, Q> {
    pub fn split<'a, Q0: QueryParameters, Q1: QueryParameters>(
        &'a self,
    ) -> Option<(Query<'a, Q0>, Query<'a, Q1>)> {
        todo!()
    }

    pub fn split_mut<'a, Q0: QueryParameters, Q1: QueryParameters>(
        &'a mut self,
    ) -> Option<(Query<'a, Q0>, Query<'a, Q1>)> {
        todo!()
    }
}
