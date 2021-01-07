use crate::errors::*;
use crate::query_infrastructure::*;
use crate::World;

use std::any::TypeId;
use std::sync::{RwLockReadGuard, RwLockWriteGuard};

pub struct Single<'world_borrow, T> {
    borrow: RwLockReadGuard<'world_borrow, Vec<T>>,
}

impl<'a, 'world_borrow, T: 'a> FetchItem<'a> for Single<'world_borrow, T> {
    type InnerItem = &'a T;
    fn inner(&'a mut self) -> Self::InnerItem {
        &self.borrow[0]
    }
}

pub struct SingleMut<'world_borrow, T> {
    borrow: RwLockWriteGuard<'world_borrow, Vec<T>>,
}

impl<'a, 'world_borrow, T: 'a> FetchItem<'a> for SingleMut<'world_borrow, T> {
    type InnerItem = &'a mut T;
    fn inner(&'a mut self) -> Self::InnerItem {
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
