use crate::*;
use std::any::{Any, TypeId};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::RwLock;
use std::sync::{RwLockReadGuard, RwLockWriteGuard};

pub struct Archetype<STORAGE: ComponentStorageTrait + 'static> {
    pub(crate) entities: RwLock<Vec<Entity>>,
    pub(crate) channels: Vec<STORAGE>,
}

pub trait ArchetypeTrait {
    type Storage: ComponentStorageTrait + 'static;
    fn new() -> Self;
    fn type_ids(&self) -> Vec<TypeId>;
    fn get_channel_mut<T: 'static>(&mut self, channel_index: usize) -> &mut Vec<T>;
    fn get_component_mut<T: 'static>(&mut self, entity_index: usize) -> Result<&mut T, ()>;
    fn borrow_channel<T: 'static>(
        &self,
        channel_index: usize,
    ) -> Result<RwLockReadGuard<Vec<T>>, Error>;
    fn channel_mut<T: 'static>(
        &self,
        channel_index: usize,
    ) -> Result<RwLockWriteGuard<Vec<T>>, Error>;

    fn new_channel_same_type(&mut self, c: &Self::Storage);
    fn channel_dyn(&mut self, index: usize) -> &mut dyn ComponentChannelTrait;
    fn sort_channels(&mut self);
    fn swap_remove(&mut self, index: usize);
    fn entities(&self) -> RwLockReadGuard<Vec<Entity>>;
}

impl<STORAGE: ComponentStorageTrait> ArchetypeTrait for Archetype<STORAGE> {
    type Storage = STORAGE;
    fn new() -> Self {
        Self {
            entities: RwLock::new(Vec::new()),
            channels: Vec::new(),
        }
    }

    fn type_ids(&self) -> Vec<TypeId> {
        self.channels.iter().map(|c| c.get_type_id()).collect()
    }

    fn entities(&self) -> RwLockReadGuard<Vec<Entity>> {
        self.entities.read().unwrap()
    }

    /// Directly access the channel
    fn get_channel_mut<T: 'static>(&mut self, channel_index: usize) -> &mut Vec<T> {
        // self.channels[channel_index].component_channel.print_type();
        self.channels[channel_index]
            .channel_mut()
            .to_any_mut()
            .downcast_mut::<RwLock<Vec<T>>>()
            .unwrap()
            .get_mut()
            .unwrap()
    }

    fn get_component_mut<T: 'static>(&mut self, entity_index: usize) -> Result<&mut T, ()> {
        let type_id = TypeId::of::<T>();
        for channel in self.channels.iter_mut() {
            if channel.get_type_id() == type_id {
                return Ok(channel
                    .channel_mut()
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
        &mut *self.channels[index].channel_mut()
    }

    fn borrow_channel<T: 'static>(
        &self,
        channel_index: usize,
    ) -> Result<RwLockReadGuard<Vec<T>>, Error> {
        Ok(self.channels[channel_index]
            .channel()
            .to_any()
            .downcast_ref::<RwLock<Vec<T>>>()
            .unwrap()
            .try_read()
            .map_err(|_| Error::CouldNotBorrowComponent(std::any::type_name::<T>()))?)
    }

    fn channel_mut<T: 'static>(
        &self,
        channel_index: usize,
    ) -> Result<RwLockWriteGuard<Vec<T>>, Error> {
        self.channels[channel_index]
            .channel()
            .to_any()
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

    fn new_channel_same_type(&mut self, c: &Self::Storage) {
        self.channels.push(c.new_same_type());
    }

    /// To be used after an Archetype is done being constructed
    fn sort_channels(&mut self) {
        self.channels
            .sort_unstable_by(|a, b| a.get_type_id().cmp(&b.get_type_id()));
    }

    fn swap_remove(&mut self, index: usize) {
        for channel in &mut self.channels {
            channel.channel_mut().swap_remove(index)
        }
        self.entities.get_mut().unwrap().swap_remove(index);
    }
}

// This may be used later when scheduling.
static CHANNEL_COUNT: AtomicUsize = AtomicUsize::new(0);

pub trait ComponentStorageTrait {
    // fn new<T: ComponentTrait>() -> Self;
    fn new_same_type(&self) -> Self;
    fn get_type_id(&self) -> TypeId;
    fn channel_mut(&mut self) -> &mut dyn ComponentChannelTrait;
    fn channel(&self) -> &dyn ComponentChannelTrait;
}

pub struct ComponentChannelStorage {
    pub(crate) type_id: TypeId,
    pub(crate) channel_id: usize,
    pub(crate) component_channel: Box<dyn ComponentChannelTrait>,
}

impl ComponentStorageTrait for ComponentChannelStorage {
    fn new_same_type(&self) -> ComponentChannelStorage {
        Self {
            type_id: self.type_id,
            component_channel: self.component_channel.new_same_type(),
            channel_id: CHANNEL_COUNT.fetch_add(1, Ordering::Relaxed),
        }
    }
    fn channel_mut(&mut self) -> &mut dyn ComponentChannelTrait {
        &mut *self.component_channel
    }

    fn channel(&self) -> &dyn ComponentChannelTrait {
        &*self.component_channel
    }

    fn get_type_id(&self) -> TypeId {
        println!("TYPE ID HERE0: {:?}", self.type_id);

        self.type_id
    }
}

impl ComponentChannelStorage {
    pub(crate) fn new<T: ComponentTrait>() -> Self {
        println!("TYPE ID HERE: {:?}", TypeId::of::<T>());

        Self {
            component_channel: Box::new(RwLock::new(Vec::<T>::new())),
            channel_id: CHANNEL_COUNT.fetch_add(1, Ordering::Relaxed),
            type_id: TypeId::of::<T>(),
        }
    }
}

pub struct ComponentChannelStorageClone {
    pub(crate) type_id: TypeId,
    pub(crate) channel_id: usize,
    component_channel: Box<dyn CloneComponentChannelTrait>,
}

impl ComponentStorageTrait for ComponentChannelStorageClone {
    fn new_same_type(&self) -> ComponentChannelStorageClone {
        Self {
            type_id: self.type_id,
            component_channel: self.component_channel.new_same_type_clone(),
            channel_id: CHANNEL_COUNT.fetch_add(1, Ordering::Relaxed),
        }
    }

    fn channel_mut(&mut self) -> &mut dyn ComponentChannelTrait {
        self.component_channel.as_component_channel_mut()
    }

    fn channel(&self) -> &dyn ComponentChannelTrait {
        self.component_channel.as_component_channel()
    }

    fn get_type_id(&self) -> TypeId {
        self.type_id
    }
}

impl ComponentChannelStorageClone {
    pub(crate) fn new<T: ComponentTrait + WorldClone>() -> Self {
        Self {
            component_channel: Box::new(RwLock::new(Vec::<T>::new())),
            channel_id: CHANNEL_COUNT.fetch_add(1, Ordering::Relaxed),
            type_id: TypeId::of::<T>(),
        }
    }
}

pub trait CloneComponentChannelTrait: ComponentChannelTrait {
    fn clone_into(
        &mut self,
        into: &mut dyn ComponentChannelTrait,
        index: usize,
        entity_migrator: &mut EntityMigrator,
    );
    fn new_same_type_clone(&self) -> Box<dyn CloneComponentChannelTrait>;
    fn as_component_channel_mut(&mut self) -> &mut dyn ComponentChannelTrait;
    fn as_component_channel(&self) -> &dyn ComponentChannelTrait;
    fn world_clone_self(&self) -> Box<dyn CloneComponentChannelTrait>;
    fn to_component_channel(self: Box<Self>) -> Box<dyn ComponentChannelTrait>;
}

impl<T: ComponentTrait + WorldClone> CloneComponentChannelTrait for RwLock<Vec<T>> {
    fn clone_into(
        &mut self,
        other: &mut dyn ComponentChannelTrait,
        index: usize,
        entity_migrator: &mut EntityMigrator,
    ) {
        let data: T = self.get_mut().unwrap()[index].world_clone(entity_migrator);
        let other = other
            .to_any_mut()
            .downcast_mut::<RwLock<Vec<T>>>()
            .unwrap()
            .get_mut()
            .unwrap();
        other.push(data);
    }

    fn new_same_type_clone(&self) -> Box<dyn CloneComponentChannelTrait> {
        Box::new(RwLock::new(Vec::<T>::new()))
    }

    fn as_component_channel_mut(&mut self) -> &mut dyn ComponentChannelTrait {
        self
    }
    fn as_component_channel(&self) -> &dyn ComponentChannelTrait {
        self
    }

    fn world_clone_self(&self) -> Box<dyn CloneComponentChannelTrait> {
        let data = self.read().unwrap();
        let v = data
            .iter()
            .map(|d| d.world_clone(&mut DoNothingEntityMigrator {}))
            .collect();
        Box::new(RwLock::new(v))
    }

    fn to_component_channel(self: Box<Self>) -> Box<dyn ComponentChannelTrait> {
        self
    }
}

impl Archetype<ComponentChannelStorageClone> {
    fn to_component_channel_archetype() -> Archetype<ComponentChannelStorage> {
        todo!()
    }
    pub fn push_new_channel<T: Sync + Send + 'static + WorldClone>(&mut self) {
        self.channels.push(ComponentChannelStorageClone::new::<T>())
    }
}

impl Archetype<ComponentChannelStorage> {
    pub(crate) fn push_new_channel<T: Sync + Send + 'static>(&mut self) {
        self.channels.push(ComponentChannelStorage::new::<T>())
    }
}
/*
impl<T: WorldClone> WorldClone for Vec<T> {
    fn world_clone(&self) -> Self {
        let mut v = Vec::with_capacity(self.len());
        for item in self {
            v.push(item.world_clone())
        }
    }
}
*/

impl WorldClone for Box<dyn CloneComponentChannelTrait> {
    fn world_clone(&self, _entity_migrator: &mut impl EntityMigratorTrait) -> Self {
        self.world_clone_self()
    }
}

impl WorldClone for ComponentChannelStorageClone {
    fn world_clone(&self, entity_migrator: &mut impl EntityMigratorTrait) -> Self {
        Self {
            type_id: self.type_id,
            channel_id: CHANNEL_COUNT.fetch_add(1, Ordering::Relaxed),
            component_channel: self.component_channel.world_clone(entity_migrator),
        }
    }
}

impl Archetype<ComponentChannelStorageClone> {
    pub(crate) fn world_clone_with_entities(
        &self,
        entity_migrator: &mut impl EntityMigratorTrait,
        entities: Vec<Entity>,
    ) -> Self {
        let channels = self
            .channels
            .iter()
            .map(|c| c.world_clone(entity_migrator))
            .collect();
        Self {
            channels,
            // Entities will be filled in later.
            entities: RwLock::new(entities),
        }
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
