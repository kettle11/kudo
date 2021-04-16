use crate::{Entity, Error};
use std::any::{Any, TypeId};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::RwLock;
use std::sync::{RwLockReadGuard, RwLockWriteGuard};

pub struct Archetype {
    pub(crate) entities: RwLock<Vec<Entity>>,
    pub(crate) channels: Vec<ComponentChannelStorage>,
}

impl Archetype {
    pub fn new() -> Self {
        Self {
            entities: RwLock::new(Vec::new()),
            channels: Vec::new(),
        }
    }

    pub fn type_ids(&self) -> Vec<TypeId> {
        self.channels.iter().map(|c| c.type_id).collect()
    }

    /// Directly access the channel
    pub fn get_channel_mut<T: 'static>(&mut self, channel_index: usize) -> &mut Vec<T> {
        // self.channels[channel_index].component_channel.print_type();
        self.channels[channel_index]
            .component_channel
            .to_any_mut()
            .downcast_mut::<RwLock<Vec<T>>>()
            .unwrap()
            .get_mut()
            .unwrap()
    }

    pub fn get_component_mut<T: 'static>(&mut self, entity_index: usize) -> Result<&mut T, ()> {
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

    pub(crate) fn borrow_channel<T: 'static>(
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

    pub(crate) fn borrow_channel_mut<T: 'static>(
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

    pub(crate) fn push_channel(&mut self, c: ComponentChannelStorage) {
        self.channels.push(c);
    }

    /// To be used after an Archetype is done being constructed
    pub(crate) fn sort_channels(&mut self) {
        self.channels
            .sort_unstable_by(|a, b| a.type_id.cmp(&b.type_id));
    }

    pub(crate) fn swap_remove(&mut self, index: usize) {
        for channel in &mut self.channels {
            channel.component_channel.swap_remove(index)
        }
        self.entities.get_mut().unwrap().swap_remove(index);
    }
}

pub(crate) struct ComponentChannelStorage {
    pub(crate) type_id: TypeId,
    pub(crate) component_channel: Box<dyn ComponentChannel>,
    pub(crate) channel_id: usize,
}
static CHANNEL_COUNT: AtomicUsize = AtomicUsize::new(0);

impl ComponentChannelStorage {
    pub fn new<T: 'static + Send + Sync>() -> Self {
        ComponentChannelStorage {
            component_channel: Box::new(RwLock::new(Vec::<T>::new())),
            type_id: TypeId::of::<T>(),
            channel_id: CHANNEL_COUNT.fetch_add(1, Ordering::Relaxed),
        }
    }

    pub fn new_same_type(&self) -> ComponentChannelStorage {
        Self {
            type_id: self.type_id,
            component_channel: self.component_channel.new_same_type(),
            channel_id: CHANNEL_COUNT.fetch_add(1, Ordering::Relaxed),
        }
    }
}

pub trait ComponentChannel: Send + Sync {
    fn to_any(&self) -> &dyn Any;
    fn to_any_mut(&mut self) -> &mut dyn Any;
    fn new_same_type(&self) -> Box<dyn ComponentChannel>;
    fn migrate_component(&mut self, index: usize, other: &mut dyn ComponentChannel);
    fn swap_remove(&mut self, index: usize);
    fn print_type(&self);
}

impl<T: 'static + Send + Sync> ComponentChannel for RwLock<Vec<T>> {
    fn to_any(&self) -> &dyn Any {
        self
    }

    fn to_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn new_same_type(&self) -> Box<dyn ComponentChannel> {
        Box::new(RwLock::new(Vec::<T>::new()))
    }

    // Purely for debugging purposes.
    fn print_type(&self) {
        println!("TYPE OF SELF: {:?}", std::any::type_name::<Self>());
    }

    fn swap_remove(&mut self, index: usize) {
        self.get_mut().unwrap().swap_remove(index);
    }

    fn migrate_component(&mut self, index: usize, other: &mut dyn ComponentChannel) {
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
