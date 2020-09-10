use crate::{ComponentStore, QueryableArchetype, QueryableWorld};
use std::any::TypeId;
use std::iter::Zip;
use std::slice::{Iter, IterMut};
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

// There are a few relevant concepts here:
// * QueryBundle:
//      A collection of QueryParams that define the data that will be produced by a query's iterator.
//      A component bundle is passed in when constructing a Query.
// * QueryParam
//      An individual item within a QueryBundle. Implements a function to get a ComponentQuery from the world.
// * ComponentQuery (mutable and immutable)
//      Hold borrows from the world and can be converted into a ComponentIterator (immutable or mutable)
//      When it is dropped the world's data is unborrowed.
// * ComponentIterator
//      Iterators over a single component's data.
// * Query
//      A collection of ComponentQueries. Can be used to produce an iterator of some sort that produces
//      the data requested by the QueryBundle.

// Need to implement a Query that is a bundle of ComponentQueries.
// The component query types need to be associated with reference types passed in a query bundle.
// Then those associated types need to be bundled together into a final query.
// Potentially as part of the bundle's macro.

pub trait Query<'a, 'b: 'a> {
    type ITERATOR: Iterator;
    fn iterator(&'b mut self) -> Self::ITERATOR;
}

macro_rules! query_impl {
    ($count: expr, $(($name: ident, $index: tt)),*) => {
        impl<'a, 'b: 'a, $($name: ComponentQueryTrait<'a, 'b>),*> Query<'a, 'b> for ($($name,)*) {
            type ITERATOR = QueryIterator<($($name::ITERATOR,)*)>;

            fn iterator(&'b mut self) -> Self::ITERATOR {
               QueryIterator(($(self.$index.iterator(),)*))
            }
        }
    };
}

// Manual implementation for a query of one item
impl<'a, 'b: 'a, A: ComponentQueryTrait<'a, 'b>> Query<'a, 'b> for (A,) {
    type ITERATOR = A::ITERATOR;
    fn iterator(&'b mut self) -> Self::ITERATOR {
        self.0.iterator()
    }
}

// Manual implementation for a query of two items
impl<'a, 'b: 'a, A: ComponentQueryTrait<'a, 'b>, B: ComponentQueryTrait<'a, 'b>> Query<'a, 'b>
    for (A, B)
{
    type ITERATOR = Zip<A::ITERATOR, B::ITERATOR>;
    fn iterator(&'b mut self) -> Self::ITERATOR {
        self.0.iterator().zip(self.1.iterator())
    }
}

/*

query_impl! {3, (A, 0), (B, 1), (C, 2)}
query_impl! {4, (A, 0), (B, 1), (C, 2), (D, 3)}
query_impl! {5, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4)}
query_impl! {6, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5)}
query_impl! {7, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6)}
query_impl! {8, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6), (H, 7)}
query_impl! {9, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6), (H, 7), (I, 8)}
*/
/// QueryBundle defines a collection of QueryParams that can be used to construct a Query.
/// A query bundle defines how to construct its query.
/// Each of its sub-QueryParams define how to construct their individual component queries.
pub trait QueryBundle<'a, 'b> {
    type QUERY;
    fn component_ids() -> Vec<TypeId>;
    fn get_query(world: &QueryableWorld) -> Self::QUERY;
}

macro_rules! query_bundle_impl {
    ($count: expr, $(($name: ident, $index: tt)),*) => {
        impl<'a,'b: 'a, $($name: QueryParam<'a, 'b> + 'static),*> QueryBundle<'a, 'b> for ($($name,)*) {
            type QUERY = ($($name::ComponentQuery,)*);

            fn component_ids() -> Vec<TypeId> {
                vec![$($name::type_id()), *]
            }

            fn get_query(world: &QueryableWorld) -> Self::QUERY {
                let mut ids = Self::component_ids();
                let mut component_types_with_index: Vec<(usize, TypeId)> = ids.iter().copied().enumerate().collect();
                ids.sort();
                component_types_with_index.sort_by(|a, b| a.1.cmp(&b.1));
                let mut archetype_indices_out = vec![0; $count];

              //  let component_queries = [$($name::get_com()), *]
                let mut component_queries = ($($name::ComponentQuery::new(),) *);
                /*
                for archetype in world.archetypes.iter() {
                    let matches = archetype
                        .matches_components(&ids, &mut archetype_indices_out);

                    // This needs an additional layer of indirection.
                    $(component_queries.$index.add_archetype(archetype, archetype_indices_out[$index]);)*
                    println!("Matches: {:?} {:?}", matches, archetype_indices_out);
                }

                // Search the world for archetypes that match this query and update individual component
                // queries with the correct data.
                unimplemented!()
                }*/
                component_queries
            }
        }
    };
}

query_bundle_impl! {1, (A, 0)}
query_bundle_impl! {2, (A, 0), (B, 0)}
/*
query_bundle_impl! {3, (A, 0), (B, 1), (C, 2)}
query_bundle_impl! {4, (A, 0), (B, 1), (C, 2), (D, 3)}
query_bundle_impl! {5, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4)}
query_bundle_impl! {6, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5)}
query_bundle_impl! {7, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6)}
query_bundle_impl! {8, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6), (H, 7)}
query_bundle_impl! {9, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6), (H, 7), (I, 8)}
*/
/// Part of a query bundle
pub trait QueryParam<'a, 'b: 'a> {
    type ComponentQuery: ComponentQueryTrait<'a, 'b>;
    fn type_id() -> TypeId;
    fn get_component_query(archetypes: &Vec<QueryableArchetype>) -> Self::ComponentQuery;
}

impl<'a, 'b: 'a, T: 'static> QueryParam<'a, 'b> for &T {
    type ComponentQuery = ComponentQuery<'b, T>;
    fn type_id() -> TypeId {
        TypeId::of::<T>()
    }

    fn get_component_query(archetypes: &Vec<QueryableArchetype>) -> Self::ComponentQuery {
        // Need to get immutable access to all the world components here.
        unimplemented!()
    }
}

impl<'a, 'b: 'a, T: 'static> QueryParam<'a, 'b> for &mut T {
    type ComponentQuery = MutableComponentQuery<'b, T>;
    fn type_id() -> TypeId {
        TypeId::of::<T>()
    }

    fn get_component_query(archetypes: &Vec<QueryableArchetype>) -> Self::ComponentQuery {
        // Need to get mutable access to all the world components here.
        unimplemented!()
    }
}

pub trait ComponentQueryTrait<'a, 'b: 'a> {
    type ITERATOR: Iterator;
    fn new() -> Self;
    fn add_archetype(&'b mut self, archetype: &'b QueryableArchetype, component_index: usize);
    fn iterator(&'b mut self) -> Self::ITERATOR;
}

pub struct MutableComponentQuery<'a, T: 'a> {
    guards: Vec<RwLockWriteGuard<'a, Box<dyn ComponentStore>>>,
    phantom: std::marker::PhantomData<T>,
}

impl<'a, 'b: 'a, T: 'static> ComponentQueryTrait<'a, 'b> for MutableComponentQuery<'b, T> {
    type ITERATOR = ComponentIterMut<'a, T>;

    fn new() -> Self {
        Self {
            guards: Vec::new(),
            phantom: std::marker::PhantomData,
        }
    }
    fn add_archetype(&'b mut self, archetype: &'b QueryableArchetype, component_index: usize) {
        let lock_write_guard = archetype.components[component_index].write().unwrap();
        self.guards.push(lock_write_guard);
    }

    fn iterator(&'b mut self) -> Self::ITERATOR {
        let iters = self
            .guards
            .iter_mut()
            .map(|g| g.to_any_mut().downcast_mut::<Vec<T>>().unwrap().iter_mut())
            .collect();
        ComponentIterMut::new(iters)
    }
}

pub struct ComponentQuery<'a, T: 'a> {
    guards: Vec<RwLockReadGuard<'a, Box<dyn ComponentStore>>>,
    phantom: std::marker::PhantomData<T>,
}

impl<'a, 'b: 'a, T: 'static> ComponentQueryTrait<'a, 'b> for ComponentQuery<'b, T> {
    type ITERATOR = ComponentIter<'a, T>;
    fn new() -> Self {
        Self {
            guards: Vec::new(),
            phantom: std::marker::PhantomData,
        }
    }
    fn add_archetype(&'b mut self, archetype: &'b QueryableArchetype, component_index: usize) {
        let lock_read_guard = archetype.components[component_index].read().unwrap();
        self.guards.push(lock_read_guard);
    }
    fn iterator(&'b mut self) -> Self::ITERATOR {
        let iters = self
            .guards
            .iter()
            .map(|g| g.to_any().downcast_ref::<Vec<T>>().unwrap().iter())
            .collect();
        ComponentIter::new(iters)
    }
}

pub struct ComponentIter<'a, T> {
    current_iter: Iter<'a, T>,
    iterators: Vec<Iter<'a, T>>,
}

impl<'a, T> ComponentIter<'a, T> {
    pub fn new(mut iterators: Vec<Iter<'a, T>>) -> Self {
        let current_iter = iterators.pop().unwrap();
        Self {
            current_iter,
            iterators,
        }
    }
}

impl<'a, T: 'a> Iterator for ComponentIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        // Chain the iterators together.
        // If the end of one iterator is reached go to the next.
        self.current_iter.next().or_else(|| {
            self.iterators.pop().map_or(None, |i| {
                self.current_iter = i;
                self.current_iter.next()
            })
        })
    }
}

/// A mutable iterator over a series of components.
/// This is created from a ComponentQuery.
pub struct ComponentIterMut<'a, T> {
    current_iter: IterMut<'a, T>,
    iterators: Vec<IterMut<'a, T>>,
}

impl<'a, T> ComponentIterMut<'a, T> {
    pub fn new(mut iterators: Vec<IterMut<'a, T>>) -> Self {
        let current_iter = iterators.pop().unwrap();
        Self {
            current_iter,
            iterators,
        }
    }
}

impl<'a, T: 'a> Iterator for ComponentIterMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        // Chain the iterators together.
        // If the end of one iterator is reached go to the next.
        self.current_iter.next().or_else(|| {
            self.iterators.pop().map_or(None, |i| {
                self.current_iter = i;
                self.current_iter.next()
            })
        })
    }
}

// This is a way to implement a flattened iterator over multiple components at once.
// but it's doing performing unnecessary checks with all the unwraps implied by the `?`s

/*
// This could be used in the above checks to avoid additional unwrapping costs.
fn unwrap_unchecked<T>(item: Option<T>) -> T {
    unsafe {
        if let Some(value) = item {
            value
        } else {
            core::hint::unreachable_unchecked()
        }
    }
}
*/
pub struct QueryIterator<T>(T);

macro_rules! iterator_impl {
    ($count: expr, $(($name: ident, $index: tt)),*) => {
        impl<$($name: Iterator),*> Iterator for QueryIterator<($($name),*)> {
            type Item = ($($name::Item),*);

            fn next(&mut self) -> Option<Self::Item> {
                Some(($(self.0.$index.next()?),*))
            }
        }
    };
}

iterator_impl! {3, (A, 0), (B, 1), (C, 2)}
iterator_impl! {4, (A, 0), (B, 1), (C, 2), (D, 3)}
iterator_impl! {5, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4)}
iterator_impl! {6, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5)}
iterator_impl! {7, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6)}
iterator_impl! {8, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6), (H, 7)}
iterator_impl! {9, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6), (H, 7), (I, 8)}
