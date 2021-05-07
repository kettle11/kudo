use crate::{ComponentTrait, Entity, Error};
use std::any::{Any, TypeId};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::RwLock;
use std::sync::{RwLockReadGuard, RwLockWriteGuard};

pub struct Archetype {
    pub(crate) entities: RwLock<Vec<Entity>>,
    pub(crate) channels: Vec<ComponentChannelStorage>,
}

pub trait ArchetypeTrait {
    fn new() -> Self;
    fn type_ids(&self) -> Vec<TypeId>;
    fn get_channel_mut<T: 'static>(&mut self, channel_index: usize) -> &mut Vec<T>;
    fn get_component_mut<T: 'static>(&mut self, entity_index: usize) -> Result<&mut T, ()>;
    fn borrow_channel<T: 'static>(
        &self,
        channel_index: usize,
    ) -> Result<RwLockReadGuard<Vec<T>>, Error>;
    fn borrow_channel_mut<T: 'static>(
        &self,
        channel_index: usize,
    ) -> Result<RwLockWriteGuard<Vec<T>>, Error>;
    fn push_new_channel<T: Sync + Send + 'static>(&mut self);
    fn insert_new_channel<T: Sync + Send + 'static>(&mut self, index: usize);
    fn new_channel_same_type(&mut self, c: &ComponentChannelStorage);
    fn channel_dyn(&mut self, index: usize) -> &mut dyn ComponentChannelTrait;
    fn sort_channels(&mut self);
    fn swap_remove(&mut self, index: usize);
    fn entities(&self) -> RwLockReadGuard<Vec<Entity>>;
}

impl ArchetypeTrait for Archetype {
    fn new() -> Self {
        Self {
            entities: RwLock::new(Vec::new()),
            channels: Vec::new(),
        }
    }

    fn type_ids(&self) -> Vec<TypeId> {
        self.channels.iter().map(|c| c.type_id).collect()
    }

    fn entities(&self) -> RwLockReadGuard<Vec<Entity>> {
        self.entities.read().unwrap()
    }

    /// Directly access the channel
    fn get_channel_mut<T: 'static>(&mut self, channel_index: usize) -> &mut Vec<T> {
        // self.channels[channel_index].component_channel.print_type();
        self.channels[channel_index]
            .component_channel
            .to_any_mut()
            .downcast_mut::<RwLock<Vec<T>>>()
            .unwrap()
            .get_mut()
            .unwrap()
    }

    fn get_component_mut<T: 'static>(&mut self, entity_index: usize) -> Result<&mut T, ()> {
        let type_id = TypeId::of::<T>();
        for channel in self.channels.iter_mut() {
            if channel.type_id == type_id {
                return Ok(channel
                    .component_channel
                    .to_any_mut()
                    .downcast_mut::<RwLock<Vec<T>>>()
                    .unwrap()
                    .get_mut()
                    .unwrap()
                    .get_mut(entity_index)
                    .unwrap());
            }
        }
        Err(())
    }

    fn channel_dyn(&mut self, index: usize) -> &mut dyn ComponentChannelTrait {
        &mut *self.channels[index].component_channel
    }

    fn borrow_channel<T: 'static>(
        &self,
        channel_index: usize,
    ) -> Result<RwLockReadGuard<Vec<T>>, Error> {
        Ok(self.channels[channel_index]
            .component_channel
            .to_any()
            .downcast_ref::<RwLock<Vec<T>>>()
            .unwrap()
            .try_read()
            .map_err(|_| Error::CouldNotBorrowComponent(std::any::type_name::<T>()))?)
    }

    fn borrow_channel_mut<T: 'static>(
        &self,
        channel_index: usize,
    ) -> Result<RwLockWriteGuard<Vec<T>>, Error> {
        Ok(self.channels[channel_index]
            .component_channel
            .to_any()
            .downcast_ref::<RwLock<Vec<T>>>()
            .unwrap()
            .try_write()
            .map_err(|_| Error::CouldNotBorrowComponent(std::any::type_name::<T>()))?)
    }

    fn push_new_channel<T: Sync + Send + 'static>(&mut self) {
        self.channels.push(ComponentChannelStorage::new::<T>())
    }
    fn insert_new_channel<T: Sync + Send + 'static>(&mut self, index: usize) {
        self.channels
            .insert(index, ComponentChannelStorage::new::<T>())
    }

    fn new_channel_same_type(&mut self, c: &ComponentChannelStorage) {
        self.channels.push(c.new_same_type());
    }

    /// To be used after an Archetype is done being constructed
    fn sort_channels(&mut self) {
        self.channels
            .sort_unstable_by(|a, b| a.type_id.cmp(&b.type_id));
    }

    fn swap_remove(&mut self, index: usize) {
        for channel in &mut self.channels {
            channel.component_channel.swap_remove(index)
        }
        self.entities.get_mut().unwrap().swap_remove(index);
    }
}

// This may be used later when scheduling.
static CHANNEL_COUNT: AtomicUsize = AtomicUsize::new(0);

trait ComponentStorageTrait {
    fn new<T: ComponentTrait>() -> Self;
    fn new_same_type(&self) -> Self;
}

pub struct ComponentChannelStorage {
    pub(crate) type_id: TypeId,
    pub(crate) channel_id: usize,
    pub(crate) component_channel: Box<dyn ComponentChannelTrait>,
}

impl ComponentStorageTrait for ComponentChannelStorage {
    fn new<T: ComponentTrait>() -> Self {
        Self {
            component_channel: Box::new(RwLock::new(Vec::<T>::new())),
            channel_id: CHANNEL_COUNT.fetch_add(1, Ordering::Relaxed),
            type_id: TypeId::of::<T>(),
        }
    }

    fn new_same_type(&self) -> ComponentChannelStorage {
        Self {
            type_id: self.type_id,
            component_channel: self.component_channel.new_same_type(),
            channel_id: CHANNEL_COUNT.fetch_add(1, Ordering::Relaxed),
        }
    }
}

pub(crate) struct ComponentChannelStorageClone {
    pub(crate) type_id: TypeId,
    pub(crate) channel_id: usize,
    component_channel: Box<dyn CloneComponentChannel>,
}

/*
impl ComponentStorageTrait for ComponentChannelStorageClone {
    fn new<T: ComponentTrait + Clone>() -> Self {
        Self {
            component_channel: Box::new(RwLock::new(Vec::<T>::new())),
            channel_id: CHANNEL_COUNT.fetch_add(1, Ordering::Relaxed),
            type_id: TypeId::of::<T>(),
        }
    }

    fn new_same_type(&self) -> ComponentChannelStorageClone {
        Self {
            type_id: self.type_id,
            component_channel: self.component_channel.new_same_type_clone(),
            channel_id: CHANNEL_COUNT.fetch_add(1, Ordering::Relaxed),
        }
    }
}
*/

pub trait CloneComponentChannel: ComponentChannelTrait {
    fn clone_into(&mut self, into: &mut dyn ComponentChannelTrait, index: usize);
    fn new_same_type_clone(&self) -> Box<dyn CloneComponentChannel>;
}

impl<T: ComponentTrait + Clone> CloneComponentChannel for RwLock<Vec<T>> {
    fn clone_into(&mut self, other: &mut dyn ComponentChannelTrait, index: usize) {
        let data: T = self.get_mut().unwrap()[index].clone();
        let other = other
            .to_any_mut()
            .downcast_mut::<RwLock<Vec<T>>>()
            .unwrap()
            .get_mut()
            .unwrap();
        other.push(data);
    }

    fn new_same_type_clone(&self) -> Box<dyn CloneComponentChannel> {
        Box::new(RwLock::new(Vec::<T>::new()))
    }
}

pub trait ComponentChannelTrait: Send + Sync {
    fn to_any(&self) -> &dyn Any;
    fn to_any_mut(&mut self) -> &mut dyn Any;
    fn new_same_type(&self) -> Box<dyn ComponentChannelTrait>;
    fn migrate_component(&mut self, index: usize, other: &mut dyn ComponentChannelTrait);
    fn swap_remove(&mut self, index: usize);
    fn print_type(&self);
}

impl<T: ComponentTrait> ComponentChannelTrait for RwLock<Vec<T>> {
    fn to_any(&self) -> &dyn Any {
        self
    }

    fn to_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn new_same_type(&self) -> Box<dyn ComponentChannelTrait> {
        Box::new(RwLock::new(Vec::<T>::new()))
    }

    // Purely for debugging purposes.
    fn print_type(&self) {
        println!("TYPE OF SELF: {:?}", std::any::type_name::<Self>());
    }

    fn swap_remove(&mut self, index: usize) {
        self.get_mut().unwrap().swap_remove(index);
    }

    fn migrate_component(&mut self, index: usize, other: &mut dyn ComponentChannelTrait) {
        let data: T = self.get_mut().unwrap().swap_remove(index);
        let other = other
            .to_any_mut()
            .downcast_mut::<RwLock<Vec<T>>>()
            .unwrap()
            .get_mut()
            .unwrap();
        other.push(data);
    }
}
