use super::{Fetch, World};
use std::any::TypeId;
use std::sync::{RwLockReadGuard, RwLockWriteGuard};

pub trait GetIter<'iter> {
    type Iter: Iterator;

    // Named get_iter to disambiguate from into_iter
    fn get_iter(&'iter mut self) -> Self::Iter;
}

pub trait GetSingle<'a> {
    type Item;
    fn get(&'a self) -> Option<Self::Item>;
}

pub struct FetchRead<T> {
    phantom: std::marker::PhantomData<T>,
}

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

impl<'iter> GetIter<'iter> for () {
    type Iter = std::iter::Empty<()>;
    fn get_iter(&'iter mut self) -> Self::Iter {
        std::iter::empty()
    }
}

pub trait GetSingleMut<'a> {
    type Item;
    fn get_mut(&'a mut self) -> Option<Self::Item>;
}

/*
impl<'a, 'world_borrow: 'a, T> GetSingleMut<'a> for WorldBorrowMut<'world_borrow, T> {
    type Item = &'a mut T;
    fn get_mut(&'a mut self) -> Option<Self::Item> {
        self.locks.get_mut(0)?.write_guard.get_mut(0)
    }
}

impl<'a, 'world_borrow: 'a, T> GetSingle<'a> for WorldBorrowMut<'world_borrow, T> {
    type Item = &'a T;
    fn get(&'a self) -> Option<Self::Item> {
        self.locks.get(0)?.write_guard.get(0)
    }
}
*/
