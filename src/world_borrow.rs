use super::{Fetch, World};
use std::any::TypeId;
use std::sync::{RwLockReadGuard, RwLockWriteGuard};

pub trait GetIter<'iter> {
    type Iter: Iterator;

    // Named get_iter to disambiguate from into_iter
    // But will be renamed because it's annoying.
    fn get_iter(&'iter mut self) -> Self::Iter;
}

impl<'iter> GetIter<'iter> for () {
    type Iter = std::iter::Empty<()>;
    fn get_iter(&'iter mut self) -> Self::Iter {
        std::iter::empty()
    }
}

pub struct FetchRead<T> {
    phantom: std::marker::PhantomData<T>,
}

// Borrow a single component channel from an archetype.
impl<'world_borrow, T: 'static> Fetch<'world_borrow> for FetchRead<T> {
    type Item = RwLockReadGuard<'world_borrow, Vec<T>>;
    fn get(world: &'world_borrow World, archetype: usize) -> Result<Self::Item, ()> {
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
            Err(())
        }
    }
}

pub struct FetchWrite<T> {
    phantom: std::marker::PhantomData<T>,
}

// Immutably borrow a single component channel from an archetype.
impl<'world_borrow, T: 'static> Fetch<'world_borrow> for FetchWrite<T> {
    type Item = RwLockWriteGuard<'world_borrow, Vec<T>>;
    fn get(world: &'world_borrow World, archetype: usize) -> Result<Self::Item, ()> {
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
            Err(())
        }
    }
}
