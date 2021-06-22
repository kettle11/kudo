use crate::*;
use std::{
    any::{Any, TypeId},
    sync::Arc,
};
//use std::sync::atomic::AtomicUsize;
use std::sync::RwLock;
use std::sync::{RwLockReadGuard, RwLockWriteGuard};

pub struct Archetype {
    pub(crate) entities: RwLock<Vec<Entity>>,
    pub(crate) channels: Vec<ComponentChannelStorage>,
}

impl Archetype {
    pub(crate) fn new() -> Self {
        Self {
            entities: RwLock::new(Vec::new()),
            channels: Vec::new(),
        }
    }

    pub(crate) fn type_ids(&self) -> Vec<TypeId> {
        self.channels.iter().map(|c| c.get_type_id()).collect()
    }

    pub(crate) fn entities(&self) -> RwLockReadGuard<Vec<Entity>> {
        self.entities.read().unwrap()
    }

    /// Clones the channels that can be cloned.
    pub(crate) fn clone_archetype(
        &mut self,
        entity_migrator: &mut EntityMigrator,
    ) -> Option<Archetype> {
        let mut archetype = Archetype::new();
        for old_channel_storage in &mut self.channels {
            let ComponentChannelStorage {
                cloner,
                component_channel,
                ..
            } = old_channel_storage;
            if let Some(cloner) = cloner {
                let component_channel = cloner.clone_channel(component_channel.as_mut());
                let channel_storage = ComponentChannelStorage {
                    type_id: old_channel_storage.type_id,
                    component_channel,
                    cloner: Some(cloner.clone()),
                };
                archetype.channels.push(channel_storage)
            }
        }

        let new_entities = self
            .entities
            .get_mut()
            .unwrap()
            .iter()
            .map(|e| entity_migrator.migrate(*e))
            .collect();
        *archetype.entities.get_mut().unwrap() = new_entities;
        Some(archetype)
    }

    /// Move all components from `other_archetype` into `self`
    pub(crate) fn append_archetype(
        &mut self,
        other_archetype: &mut Archetype,
        entity_migrator: &mut EntityMigrator,
    ) {
        for (new_channel, old_channel) in self
            .channels
            .iter_mut()
            .zip(other_archetype.channels.iter_mut())
        {
            new_channel
                .channel_mut()
                .append_channel(old_channel.channel_mut())
        }

        let self_entities = self.entities.get_mut().unwrap();
        let other_entities = other_archetype.entities.get_mut().unwrap();
        self_entities.reserve(other_entities.len());
        for entity in other_entities {
            let migrated_entity = entity_migrator.migrate(*entity);
            self_entities.push(migrated_entity);
        }
    }

    /// Directly access the channel
    pub(crate) fn get_channel_mut<T: 'static>(&mut self, channel_index: usize) -> &mut Vec<T> {
        // self.channels[channel_index].component_channel.print_type();
        self.channels[channel_index]
            .channel_mut()
            .as_any_mut()
            .downcast_mut::<RwLock<Vec<T>>>()
            .unwrap()
            .get_mut()
            .unwrap()
    }

    pub(crate) fn get_component_mut<T: 'static>(
        &mut self,
        entity_index: usize,
    ) -> Result<&mut T, ()> {
        let type_id = TypeId::of::<T>();
        for channel in self.channels.iter_mut() {
            if channel.get_type_id() == type_id {
                return Ok(channel
                    .channel_mut()
                    .as_any_mut()
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
        self.channels[channel_index]
            .channel()
            .as_any()
            .downcast_ref::<RwLock<Vec<T>>>()
            .unwrap()
            .try_read()
            .map_err(|_| Error::CouldNotBorrowComponent(std::any::type_name::<T>()))
    }

    pub(crate) fn channel_mut<T: 'static>(
        &self,
        channel_index: usize,
    ) -> Result<RwLockWriteGuard<Vec<T>>, Error> {
        self.channels[channel_index]
            .channel()
            .as_any()
            .downcast_ref::<RwLock<Vec<T>>>()
            .unwrap()
            .try_write()
            .map_err(|_| Error::CouldNotBorrowComponent(std::any::type_name::<T>()))
    }

    /*
    fn insert_new_channel<T: Sync + Send + 'static>(&mut self, index: usize) {
        self.channels
            .insert(index, ComponentChannelStorage::new::<T>())
    }
    */

    pub(crate) fn new_channel_same_type(&mut self, c: &ComponentChannelStorage) {
        println!("NEW CHANNEL SAME TYPE: {:?}", c.type_id());
        self.channels.push(c.new_same_type());
    }

    /// To be used after an Archetype is done being constructed
    pub(crate) fn sort_channels(&mut self) {
        self.channels.sort_unstable_by_key(|a| a.get_type_id());
    }

    pub(crate) fn swap_remove(&mut self, index: usize) {
        for channel in &mut self.channels {
            channel.channel_mut().swap_remove(index)
        }
        self.entities.get_mut().unwrap().swap_remove(index);
    }

    pub(crate) fn push_new_channel<T: Sync + Send + 'static>(
        &mut self,
        cloner: Option<Arc<dyn ClonerTrait>>,
    ) {
        self.channels
            .push(ComponentChannelStorage::new::<T>(cloner))
    }
}

// This may be used later when scheduling.
// static CHANNEL_COUNT: AtomicUsize = AtomicUsize::new(0);

pub struct ComponentChannelStorage {
    pub(crate) type_id: TypeId,
    //  pub(crate) channel_id: usize,
    pub(crate) component_channel: Box<dyn ComponentChannelTrait>,
    pub(crate) cloner: Option<Arc<dyn ClonerTrait>>,
}

impl ComponentChannelStorage {
    pub(crate) fn new_same_type(&self) -> ComponentChannelStorage {
        Self {
            type_id: self.type_id,
            component_channel: self.component_channel.new_same_type(),
            cloner: self.cloner.clone(),
            // channel_id: CHANNEL_COUNT.fetch_add(1, Ordering::Relaxed),
        }
    }
    pub(crate) fn channel_mut(&mut self) -> &mut dyn ComponentChannelTrait {
        &mut *self.component_channel
    }

    pub(crate) fn channel(&self) -> &dyn ComponentChannelTrait {
        &*self.component_channel
    }

    pub(crate) fn get_type_id(&self) -> TypeId {
        self.type_id
    }
}

impl ComponentChannelStorage {
    pub(crate) fn new<T: ComponentTrait>(cloner: Option<Arc<dyn ClonerTrait>>) -> Self {
        Self {
            component_channel: Box::new(RwLock::new(Vec::<T>::new())),
            // channel_id: CHANNEL_COUNT.fetch_add(1, Ordering::Relaxed),
            type_id: TypeId::of::<T>(),
            cloner,
        }
    }
}

pub trait ComponentChannelTrait: Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn new_same_type(&self) -> Box<dyn ComponentChannelTrait>;
    fn migrate_component(&mut self, index: usize, other: &mut dyn ComponentChannelTrait);
    fn swap_remove(&mut self, index: usize);
    fn print_type(&self);
    fn append_channel(&mut self, other: &mut dyn ComponentChannelTrait);
}

impl<T: ComponentTrait> ComponentChannelTrait for RwLock<Vec<T>> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
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
            .as_any_mut()
            .downcast_mut::<RwLock<Vec<T>>>()
            .unwrap()
            .get_mut()
            .unwrap();
        other.push(data);
    }

    fn append_channel(&mut self, other: &mut dyn ComponentChannelTrait) {
        let other = other
            .as_any_mut()
            .downcast_mut::<RwLock<Vec<T>>>()
            .unwrap()
            .get_mut()
            .unwrap();
        self.get_mut().unwrap().append(other);
    }
}

pub(crate) trait ClonerTrait: Send + Sync {
    fn clone_within(&self, clone_from_index: usize, channel: &dyn ComponentChannelTrait);
    fn clone_channel(
        &self,
        channel: &mut dyn ComponentChannelTrait,
    ) -> Box<dyn ComponentChannelTrait>;
}

#[derive(Clone)]
pub(crate) struct Cloner<T> {
    pub phantom: std::marker::PhantomData<fn() -> T>,
}

impl<T: Clone + 'static + Send + Sync> ClonerTrait for Cloner<T> {
    fn clone_within(&self, clone_from_index: usize, channel: &dyn ComponentChannelTrait) {
        let mut channel = channel
            .as_any()
            .downcast_ref::<RwLock<Vec<T>>>()
            .unwrap()
            .try_write()
            .unwrap();
        let t = channel[clone_from_index].clone();
        channel.push(t)
    }

    fn clone_channel(
        &self,
        channel: &mut dyn ComponentChannelTrait,
    ) -> Box<dyn ComponentChannelTrait> {
        let channel = channel
            .as_any_mut()
            .downcast_mut::<RwLock<Vec<T>>>()
            .unwrap()
            .get_mut()
            .unwrap();
        Box::new(RwLock::new(channel.clone()))
    }
}
