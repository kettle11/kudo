use super::{Archetype, ChainedIterator, Entity, EntityId, World};
use std::any::TypeId;
use std::sync::{RwLockReadGuard, RwLockWriteGuard};

/// A trait for data that has been borrowed from the world.
/// Call `iter` to get an iterator over the data.
pub trait WorldBorrow<'iter> {
    type Iter: Iterator;
    fn iter(&'iter mut self) -> Self::Iter;
}

/// A read-only borrow from the world.
pub struct WorldBorrowImmut<'world_borrow, T> {
    world: &'world_borrow World,
    locks: Vec<ArchetypeBorrowRead<'world_borrow, T>>,
}

struct ArchetypeBorrowRead<'world_borrow, T> {
    archetype_index: EntityId,
    read_guard: RwLockReadGuard<'world_borrow, Vec<T>>,
}

impl<'world_borrow, T: 'static> WorldBorrowImmut<'world_borrow, T> {
    pub(crate) fn new(world: &'world_borrow World) -> Self {
        Self {
            world,
            locks: Vec::new(),
        }
    }

    pub(crate) fn add_archetype(
        &mut self,
        id: TypeId,
        archetype_index: EntityId,
        archetype: &'world_borrow Archetype,
    ) -> Result<(), ()> {
        // In theory this index may have already been found, but it's not too bad to do it again here.
        let index = archetype
            .components
            .iter()
            .position(|c| c.type_id == id)
            .unwrap();
        if let Ok(read_guard) = archetype.get(index).try_read() {
            self.locks.push(ArchetypeBorrowRead {
                archetype_index,
                read_guard,
            });
            Ok(())
        } else {
            Err(())
        }
    }

    /// If the entity is part of this query then return a reference to its component.
    pub fn get_component(&self, entity: Entity) -> Result<&T, ()> {
        let entity_info = self.world.entities[entity.index as usize];

        if entity_info.generation == entity.generation {
            let archetype_index = entity_info.location.archetype_index;
            for lock in self.locks.iter() {
                if archetype_index == lock.archetype_index {
                    return Ok(&lock.read_guard[entity_info.location.index_in_archetype as usize]);
                }
            }
            Err(())
        } else {
            Err(())
        }
    }
}

impl<'iter, 'world_borrow, T: 'static> WorldBorrow<'iter> for WorldBorrowImmut<'world_borrow, T> {
    type Iter = ChainedIterator<std::slice::Iter<'iter, T>>;

    fn iter(&'iter mut self) -> Self::Iter {
        let mut iters: Vec<std::slice::Iter<'iter, T>> =
            self.locks.iter().map(|l| l.read_guard.iter()).collect();
        // If no iters, add an empty iter to iterate over.
        if iters.is_empty() {
            iters.push([].iter())
        }
        ChainedIterator::new(iters)
    }
}

/// A write/read capable borrow from the world.
pub struct WorldBorrowMut<'world_borrow, T> {
    world: &'world_borrow World,
    locks: Vec<ArchetypeBorrowWrite<'world_borrow, T>>,
}

struct ArchetypeBorrowWrite<'world_borrow, T> {
    archetype_index: EntityId,
    write_guard: RwLockWriteGuard<'world_borrow, Vec<T>>,
}

impl<'world_borrow, T: 'static> WorldBorrowMut<'world_borrow, T> {
    pub(crate) fn new(world: &'world_borrow World) -> Self {
        Self {
            world,
            locks: Vec::new(),
        }
    }

    pub(crate) fn add_archetype(
        &mut self,
        id: TypeId,
        archetype_index: u32,
        archetype: &'world_borrow Archetype,
    ) -> Result<(), ()> {
        // In theory this index have already been found, but it's not too bad to do it again here.
        let index = archetype
            .components
            .iter()
            .position(|c| c.type_id == id)
            .unwrap();

        if let Ok(write_guard) = archetype.get(index).try_write() {
            self.locks.push(ArchetypeBorrowWrite {
                archetype_index,
                write_guard,
            });
            Ok(())
        } else {
            Err(())
        }
    }

    /// If the entity is part of this query then return a mutable reference to its component.
    pub fn get_component_mut(&mut self, entity: Entity) -> Result<&mut T, ()> {
        let entity_info = self.world.entities[entity.index as usize];

        if entity_info.generation == entity.generation {
            let archetype_index = entity_info.location.archetype_index;
            for lock in self.locks.iter_mut() {
                if archetype_index == lock.archetype_index {
                    return Ok(
                        &mut lock.write_guard[entity_info.location.index_in_archetype as usize]
                    );
                }
            }
            Err(())
        } else {
            Err(())
        }
    }
}

impl<'iter, 'world_borrow, T: 'static> WorldBorrow<'iter> for WorldBorrowMut<'world_borrow, T> {
    type Iter = ChainedIterator<std::slice::IterMut<'iter, T>>;

    fn iter(&'iter mut self) -> Self::Iter {
        let mut iters: Vec<std::slice::IterMut<'iter, T>> = self
            .locks
            .iter_mut()
            .map(|l| l.write_guard.iter_mut())
            .collect();
        // If no iters, add an empty iter to iterate over.
        if iters.is_empty() {
            iters.push([].iter_mut())
        }
        ChainedIterator::new(iters)
    }
}

/*
/// A borrow of the world sufficient to provide EntityIds for the archetypes.
pub struct WorldBorrowEntity<'world_borrow> {
    world: &'world_borrow World,
    locks: Vec<RwLockReadGuard<'world_borrow, Vec<EntityId>>>,
}

impl<'iter, 'world_borrow> WorldBorrow<'iter> for WorldBorrowEntity<'world_borrow> {
    type Iter = ChainedIterator<std::slice::Iter<'iter, EntityId>>;

    fn iter(&'iter mut self) -> Self::Iter {
        let mut iters: Vec<std::slice::Iter<'iter, EntityId>> =
            self.locks.iter().map(|l| l.iter()).collect();
        // If no iters, add an empty iter to iterate over.
        if iters.is_empty() {
            iters.push([].iter())
        }
        ChainedIterator::new(iters)
    }
}
*/