use super::*;
use crate::{archetype::Archetype, iterators::*, Entity};
use crate::{storage_graph::Requirement, ChainedIterator};
use std::convert::TryInto;
use std::{
    any::TypeId,
    iter::Zip,
    sync::{RwLockReadGuard, RwLockWriteGuard},
};

pub struct Query<'a, T: QueryParameters> {
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
    archetype_index: usize,
}

pub trait QueryParameter: for<'a> QueryParameterBorrow<'a> {
    fn type_id() -> TypeId;
}

pub trait QueryParameterBorrow<'a> {
    type ParameterBorrow;
    fn borrow(archetype: &'a Archetype, channel_index: usize) -> Option<Self::ParameterBorrow>;
}

impl<T: 'static> QueryParameter for &T {
    fn type_id() -> TypeId {
        TypeId::of::<T>()
    }
}

impl<'a, T: 'static> QueryParameterBorrow<'a> for &T {
    type ParameterBorrow = RwLockReadGuard<'a, Vec<T>>;
    fn borrow(archetype: &'a Archetype, channel_index: usize) -> Option<Self::ParameterBorrow> {
        archetype.borrow_channel(channel_index)
    }
}

impl<T: 'static> QueryParameter for &mut T {
    fn type_id() -> TypeId {
        TypeId::of::<T>()
    }
}

impl<'a, T: 'static> QueryParameterBorrow<'a> for &mut T {
    type ParameterBorrow = RwLockWriteGuard<'a, Vec<T>>;
    fn borrow(archetype: &'a Archetype, channel_index: usize) -> Option<Self::ParameterBorrow> {
        archetype.borrow_channel_mut(channel_index)
    }
}

#[derive(Debug)]
struct ArchetypeInfo<const CHANNELS: usize> {
    index: usize,
    channels: [usize; CHANNELS],
}

pub struct QueryInfo<const CHANNELS: usize> {
    archetypes: Vec<ArchetypeInfo<CHANNELS>>,
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
            fn get_query(world: &'a World, query_info: &Self::QueryInfo) -> Option<Self::Result> {
                let mut archetype_borrows = Vec::with_capacity(query_info.archetypes.len());
                for archetype_info in &query_info.archetypes {
                    let archetype = &world.archetypes[archetype_info.index];
                    let [$($name,)*] = archetype_info.channels;

                    archetype_borrows.push(ArchetypeBorrow {
                        component_borrows: (
                            $(<$name as QueryParameterBorrow<'a>>::borrow(archetype, $name)?,)*
                        ),
                        archetype_index: archetype_info.index,
                        entities: archetype.entities.read().unwrap(),
                    })
                }
                Some(Some(Query { archetype_borrows }))
            }
        }


        // It almost seems like there might be a way to make this more generic.
        // I think these implementations could be made totally generic by making QueryParameters
        // implement a way to get all type ids.

        impl<'a, $($name: QueryParameter), *> GetQueryInfoTrait for Query<'a, ($($name,)*)> {
            type QueryInfo = QueryInfo<$count>;
            fn query_info(world: &World) -> Option<Self::QueryInfo> {
                let mut type_ids: [Requirement; $count] = [
                    $(Requirement::with_(0, $name::type_id())),*
                ];
                // This is a poor way of filling out this requirement.
                for (i, r) in type_ids.iter_mut().enumerate() {
                    r.original_index = i;
                }

                let archetypes = get_archetype_info(world, type_ids)?;
                Some(QueryInfo { archetypes })
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

// This helper means that below block of code can be moved out of a macro.
fn get_archetype_info<const SIZE: usize>(
    world: &World,
    mut type_ids: [Requirement; SIZE],
) -> Option<Vec<ArchetypeInfo<SIZE>>> {
    // type_ids.sort_unstable_by_key(|r| r.type_id);

    let archetypes = world.storage_lookup.get_matching_archetypes(&type_ids);
    let archetypes = archetypes
        .into_iter()
        .map(|index| {
            let mut channels = [0; SIZE];
            for (channel, requirement) in
                world.archetypes[index].channels.iter().zip(type_ids.iter())
            {}
            ArchetypeInfo { index, channels }
        })
        .collect();

    /*
    world
        .storage_graph
        .iterate_matching_storage(
            &type_ids,
            #[allow(unused_variables)]
            |archetype_index, channels| -> Result<(), ()> {
                // This feels a bit inelegant.
                let mut new_channels = [0; SIZE];
                for i in 0..SIZE {
                    new_channels[type_ids[i].original_index] = channels[i];
                }

                archetypes.push(ArchetypeInfo {
                    index: archetype_index,
                    // LEFT OFF HERE: This needs to be rearranged into the original arrangement.
                    channels: new_channels,
                });
                Ok(())
            },
        )
        .ok()?;
        */
    Some(archetypes)
}

impl<'a, const CHANNELS: usize> QueryInfoTrait for QueryInfo<CHANNELS> {
    fn borrows(&self) -> &[WorldBorrow] {
        todo!()
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

impl<'a, 'b, T: QueryParameters> IntoIterator for &'b Query<'a, T>
where
    &'b ArchetypeBorrow<'a, <T as QueryParametersBorrow<'a>>::ComponentBorrows>: IntoIterator,
{
    type Item = <Self::IntoIter as Iterator>::Item;
    type IntoIter =
        ChainedIterator<<&'b ArchetypeBorrow<'a, <T as QueryParametersBorrow<'a>>::ComponentBorrows> as IntoIterator>::IntoIter>;

    fn into_iter(self) -> Self::IntoIter {
        ChainedIterator::new(
            self.archetype_borrows
                .iter()
                .map(|i| i.into_iter())
                .collect(),
        )
    }
}

impl<'a, 'b, T: QueryParameters> IntoIterator for &'b mut Query<'a, T>
where
    &'b mut ArchetypeBorrow<'a, <T as QueryParametersBorrow<'a>>::ComponentBorrows>: IntoIterator,
{
    type Item = <Self::IntoIter as Iterator>::Item;
    type IntoIter = ChainedIterator<
        <&'b mut ArchetypeBorrow<'a, <T as QueryParametersBorrow<'a>>::ComponentBorrows> as IntoIterator>::IntoIter,
    >;

    fn into_iter(self) -> Self::IntoIter {
        ChainedIterator::new(
            self.archetype_borrows
                .iter_mut()
                .map(|i| i.into_iter())
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

impl<'b, A: GetIteratorMut<'b>> IntoIterator for &'b mut ArchetypeBorrow<'_, (A,)> {
    type Item = <Self::IntoIter as Iterator>::Item;
    type IntoIter = A::Iter;
    fn into_iter(self) -> Self::IntoIter {
        self.component_borrows.0.get_iter_mut()
    }
}

impl<'b, A: GetIterator<'b>, B: GetIterator<'b>> IntoIterator for &'b ArchetypeBorrow<'_, (A, B)> {
    type Item = <Self::IntoIter as Iterator>::Item;
    type IntoIter = Zip<A::Iter, B::Iter>;
    fn into_iter(self) -> Self::IntoIter {
        self.component_borrows
            .0
            .get_iter()
            .zip(self.component_borrows.1.get_iter())
    }
}

impl<'b, A: GetIteratorMut<'b>, B: GetIteratorMut<'b>> IntoIterator
    for &'b mut ArchetypeBorrow<'_, (A, B)>
{
    type Item = <Self::IntoIter as Iterator>::Item;
    type IntoIter = Zip<A::Iter, B::Iter>;
    fn into_iter(self) -> Self::IntoIter {
        self.component_borrows
            .0
            .get_iter_mut()
            .zip(self.component_borrows.1.get_iter_mut())
    }
}

macro_rules! iterator{
    ($zip_type: ident, $($name: ident),*) => {
        #[allow(non_snake_case)]
        impl<'b, $($name: GetIterator<'b>),*> IntoIterator for &'b ArchetypeBorrow<'_, ($($name,)*)>
             {
            type Item = <Self::IntoIter as Iterator>::Item;
            type IntoIter = $zip_type<$($name::Iter,)*>;
            fn into_iter(self) -> Self::IntoIter {
                let ($($name,)*) = &self.component_borrows;
                $zip_type::new($($name.get_iter(),)*)
            }
        }

        #[allow(non_snake_case)]
        impl<'b, $($name: GetIteratorMut<'b>),*> IntoIterator for &'b mut ArchetypeBorrow<'_, ($($name,)*)>
             {
            type Item = <Self::IntoIter as Iterator>::Item;
            type IntoIter = $zip_type<$($name::Iter,)*>;
            fn into_iter(self) -> Self::IntoIter {
                let ($($name,)*) = &mut self.component_borrows;
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

#[test]
fn iterate_entities() {
    use crate::*;
    let mut world = World::new();
    world.spawn((3 as i32,));
    world.spawn((4 as i32,));

    (|i: Query<(&i32,)>| {
        let entities: Vec<Entity> = i.entities().collect();
        assert!(entities[0].index() == 0);
        assert!(entities[1].index() == 1);
    })
    .run(&world)
    .unwrap()
}

#[test]
fn sum() {
    struct Position([f32; 3]);
    struct Velocity([f32; 3]);
    struct Rotation([f32; 3]);

    let mut world = World::new();

    for _ in 0..10 {
        world.spawn((
            Position([1., 0., 0.]),
            Rotation([1., 0., 0.]),
            Velocity([1., 0., 0.]),
        ));
    }

    let mut query = world
        .query::<(&Velocity, &mut Position, &Rotation)>()
        .unwrap();
    for (velocity, position, _rotation) in query.iter_mut() {
        position.0[0] += velocity.0[0];
        position.0[1] += velocity.0[1];
        position.0[2] += velocity.0[2];
    }

    for (_velocity, position, _rotation) in query.iter_mut() {
        assert!(position.0 == [2., 0., 0.]);
    }
}
