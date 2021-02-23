//! The hierarchy that builds the world is the following
//! World
//!     * Various entitiy metadata
//!     Vec<Archetype>
//!         components: Vec<ComponentStore>
//!             TypeId
//!             ComponentVec (which can be downcast into a RwLock<Vec<T>>
//!
//! The world contains entity metadata and archetypes.
//! Archetypes contain Vecs of component data.

use super::{Fetch, FetchError, Query, QueryParameters, Single, SingleMut};

use std::any::{Any, TypeId};
use std::hash::{Hash, Hasher};
use std::sync::RwLock;
use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    usize,
};

use crate::clone_store::*;

// This can be used to easily change the size of an EntityId.
pub(crate) type EntityId = u32;

pub trait Component: Sync + Send + 'static {}
impl<T: Sync + Send + 'static> Component for T {}
/// The ComponentVec trait is used to define a set of things that can be done on
/// an Any without knowing its exact type.
pub(crate) trait ComponentVec: Sync + Send {
    fn to_any(&self) -> &dyn Any;
    fn to_any_mut(&mut self) -> &mut dyn Any;
    fn len(&mut self) -> usize;
    fn swap_remove(&mut self, index: EntityId);
    fn migrate(&mut self, entity_index: EntityId, other_archetype: &mut dyn ComponentVec);
    fn new_same_type(&self) -> Box<dyn ComponentVec + Send + Sync>;
}

impl<T: Component> ComponentVec for RwLock<Vec<T>> {
    fn to_any(&self) -> &dyn Any {
        self
    }
    fn to_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn len(&mut self) -> usize {
        // Perhaps this call to read incurs unnecessary overhead.
        self.get_mut().unwrap().len()
    }

    fn swap_remove(&mut self, index: EntityId) {
        self.get_mut().unwrap().swap_remove(index as usize);
    }

    fn migrate(&mut self, entity_index: EntityId, other_component_vec: &mut dyn ComponentVec) {
        let data: T = self.get_mut().unwrap().swap_remove(entity_index as usize);
        component_vec_to_mut(other_component_vec).push(data);
    }

    fn new_same_type(&self) -> Box<dyn ComponentVec + Send + Sync> {
        Box::new(RwLock::new(Vec::<T>::new()))
    }
}

// This could be made unchecked in the future if there's a high degree of confidence in everything else.
pub(crate) fn component_vec_to_mut<T: 'static>(c: &mut dyn ComponentVec) -> &mut Vec<T> {
    c.to_any_mut()
        .downcast_mut::<RwLock<Vec<T>>>()
        .unwrap()
        .get_mut()
        .unwrap()
}

/// Stores components for a component type
pub(crate) struct ComponentStore {
    pub(crate) type_id: TypeId,
    pub(crate) data: Box<dyn ComponentVec>,
}

impl ComponentStore {
    pub fn new<T: 'static + Send + Sync>() -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            data: Box::new(RwLock::new(Vec::<T>::new())),
        }
    }

    /// Creates a new ComponentStore with the same internal storage type as self
    pub fn new_same_type(&self) -> Self {
        Self {
            type_id: self.type_id,
            data: self.data.new_same_type(),
        }
    }
}

struct ArchetypeBuilder {
    component_channels: Vec<ComponentStore>,
}

impl ArchetypeBuilder {
    fn new(reserve_size: usize) -> Self {
        Self {
            component_channels: Vec::with_capacity(reserve_size),
        }
    }

    fn add_component_store(&mut self, component_store: ComponentStore) {
        self.component_channels.push(component_store)
    }

    fn build(mut self) -> Archetype {
        self.component_channels
            .sort_unstable_by(|a, b| a.type_id.cmp(&b.type_id));

        Archetype {
            entities: Vec::new(),
            components: self.component_channels,
        }
    }
}

#[doc(hidden)]
/// An archetype stores entities with the same set of components.
pub struct Archetype {
    pub(crate) entities: Vec<EntityId>,
    pub(crate) components: Vec<ComponentStore>,
}

impl Archetype {
    pub(crate) fn get<T: 'static>(&self, index: usize) -> &RwLock<Vec<T>> {
        self.components[index]
            .data
            .to_any()
            .downcast_ref::<RwLock<Vec<T>>>()
            .unwrap()
    }

    /// Returns the index of the entity moved
    fn remove_entity(&mut self, index: EntityId) -> EntityId {
        for c in self.components.iter_mut() {
            c.data.swap_remove(index)
        }

        let moved = *self.entities.last().unwrap();
        self.entities.swap_remove(index as usize);
        moved
    }

    pub(crate) fn mutable_component_store<T: 'static>(
        &mut self,
        component_index: usize,
    ) -> &mut Vec<T> {
        component_vec_to_mut(&mut *self.components[component_index].data)
    }

    fn replace_component<T: 'static>(&mut self, component_index: usize, index: EntityId, t: T) {
        self.mutable_component_store(component_index)[index as usize] = t;
    }

    fn push<T: 'static>(&mut self, component_index: usize, t: T) {
        self.mutable_component_store(component_index).push(t)
    }

    fn get_component_mut<T: 'static>(
        &mut self,
        index: EntityId,
    ) -> Result<&mut T, EntityMissingComponent> {
        let type_id = TypeId::of::<T>();
        let mut component_index = None;
        for (i, c) in self.components.iter().enumerate() {
            if c.type_id == type_id {
                component_index = Some(i);
                break;
            }
        }

        if let Some(component_index) = component_index {
            Ok(&mut self.mutable_component_store(component_index)[index as usize])
        } else {
            Err(EntityMissingComponent::new::<T>(index))
        }
    }

    /// Removes the component from an entity and pushes it to the other archetype
    /// The type does not need to be known to call this function.
    /// But the types of component_index and other_index need to match.
    fn migrate_component(
        &mut self,
        component_index: usize,
        entity_index: EntityId,
        other_archetype: &mut Archetype,
        other_index: usize,
    ) {
        self.components[component_index].data.migrate(
            entity_index,
            &mut *other_archetype.components[other_index].data,
        );
    }

    /// This takes a mutable reference so that the inner RwLock does not need to be locked
    /// by instead using get_mut.
    fn len(&mut self) -> usize {
        self.entities.len()
    }
}

/// An entity's location within the world
#[derive(Debug, Clone, Copy)]
#[doc(hidden)]
pub struct EntityLocation {
    archetype_index: EntityId,
    index_in_archetype: EntityId,
}

#[derive(Clone, Copy)]
pub(crate) struct EntityInfo {
    pub(crate) generation: EntityId,
    pub(crate) location: EntityLocation,
}

/// A handle to an entity within the world.
#[derive(Debug, Clone, Copy, Hash, Eq, Ord, PartialEq, PartialOrd)]
pub struct Entity {
    pub(crate) index: EntityId,
    pub(crate) generation: EntityId,
}

/// This entity has been despawned so operations can no longer
/// be performed on it.
#[derive(Debug)]
pub struct NoSuchEntity;

impl std::fmt::Display for NoSuchEntity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "The entity no longer exists so the operation cannot be performed"
        )
    }
}

impl std::error::Error for NoSuchEntity {}

#[derive(Debug)]
pub struct EntityMissingComponent(EntityId, &'static str);

impl EntityMissingComponent {
    pub fn new<T>(entity_id: EntityId) -> Self {
        Self(entity_id, std::any::type_name::<T>())
    }
}

impl std::fmt::Display for EntityMissingComponent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Entity {:?} does not have a [{}] component",
            self.0, self.1
        )
    }
}

impl std::error::Error for EntityMissingComponent {}

#[derive(Debug)]
pub enum ComponentError {
    EntityMissingComponent(EntityMissingComponent),
    NoSuchEntity(NoSuchEntity),
}

/// The world holds all components and associated entities.
pub struct World {
    pub(crate) archetypes: Vec<Archetype>,
    bundle_id_to_archetype: HashMap<u64, usize>,
    pub(crate) entities: Vec<EntityInfo>,
    free_entities: Vec<EntityId>,
    clone_store: CloneStore,
}

impl World {
    /// Create the World with a `CloneStore` that specifies which components can be cloned.
    pub fn new_with_clone_store(clone_store: CloneStore) -> Self {
        Self {
            archetypes: Vec::new(),
            bundle_id_to_archetype: HashMap::new(),
            entities: Vec::new(),
            free_entities: Vec::new(),
            clone_store,
        }
    }

    /// Create the world.
    pub fn new() -> Self {
        Self::new_with_clone_store(CloneStore::new().build())
    }

    /// Clone all the clonable components of an Entity into the destination World.
    pub fn clone_entity_into_world(
        &mut self,
        entity: Entity,
        destination_world: &mut World,
    ) -> Option<Entity> {
        // Much of the code here could be shared with the function below.

        let entity_info = self.get_entity_info(entity)?;
        let old_archetype_index = entity_info.location.archetype_index as usize;
        let archetype = &self.archetypes[old_archetype_index];

        // Calculate new Archetype ID.
        let mut type_ids = Vec::new();
        for c in archetype.components.iter() {
            if self.clone_store.get(c.type_id).is_some() {
                type_ids.push(c.type_id);
            }
        }

        let bundle_id = calculate_bundle_id(&type_ids);
        let destination_archetype_index = if let Some(archetype_index) =
            destination_world.bundle_id_to_archetype.get(&bundle_id)
        {
            *archetype_index
        } else {
            let mut archetype_builder = ArchetypeBuilder::new(type_ids.len());
            for c in archetype.components.iter() {
                // This additional hash should be avoidable.
                if self.clone_store.get(c.type_id).is_some() {
                    archetype_builder.add_component_store(c.new_same_type())
                }
            }
            destination_world.add_archetype(bundle_id, archetype_builder.build())
        };

        let old_archetype = &mut self.archetypes[old_archetype_index];
        let new_archetype = &mut destination_world.archetypes[destination_archetype_index];

        let mut channel_in_destination = 0;

        for c in old_archetype.components.iter_mut() {
            if let Some(cloner) = self.clone_store.get(c.type_id) {
                cloner.clone_component(
                    entity_info.location.index_in_archetype as usize,
                    c,
                    &mut new_archetype.components[channel_in_destination],
                );
                channel_in_destination += 1;
            }
        }
        let entity = self.get_entity_id();
        self.entities[entity.index as usize].location = EntityLocation {
            archetype_index: destination_archetype_index as u32,
            index_in_archetype: destination_world.archetypes[destination_archetype_index].len()
                as u32,
        };
        Some(entity)
    }

    /// Clones an entity within this world.
    pub fn clone_entity(&mut self, entity: Entity) -> Option<Entity> {
        // The code in this has a bunch of places for improvement.

        // 1. Find the Entity
        // 2. Iterate the Entity's components, check if they're clonable, and calculate a bundle ID to look up the Archetype
        // 3. Find or create a new Archetype
        // 4. Copy the data to the new Archetype (with a special case if it's the original Archetype)

        let entity_info = self.get_entity_info(entity)?;
        let old_archetype_index = entity_info.location.archetype_index as usize;
        let archetype = &self.archetypes[old_archetype_index];

        // Calculate new Archetype ID.
        let mut type_ids = Vec::new();
        for c in archetype.components.iter() {
            if self.clone_store.get(c.type_id).is_some() {
                type_ids.push(c.type_id);
            }
        }

        let bundle_id = calculate_bundle_id(&type_ids);

        let new_archetype_index =
            if let Some(archetype_index) = self.bundle_id_to_archetype.get(&bundle_id) {
                *archetype_index
            } else {
                let mut archetype_builder = ArchetypeBuilder::new(type_ids.len());
                for c in archetype.components.iter() {
                    // This additional hash should be avoidable.
                    if self.clone_store.get(c.type_id).is_some() {
                        archetype_builder.add_component_store(c.new_same_type())
                    }
                }
                self.add_archetype(bundle_id, archetype_builder.build())
            };

        if old_archetype_index as usize == new_archetype_index {
            for c in self.archetypes[old_archetype_index].components.iter_mut() {
                if let Some(cloner) = self.clone_store.get(c.type_id) {
                    cloner.clone_component_into_self(
                        entity_info.location.index_in_archetype as usize,
                        c,
                    );
                }
            }
        } else {
            let mut channel_in_destination = 0;
            let (old_archetype, new_archtype) = index_twice(
                &mut self.archetypes,
                old_archetype_index as usize,
                new_archetype_index,
            );

            for c in old_archetype.components.iter_mut() {
                if let Some(cloner) = self.clone_store.get(c.type_id) {
                    cloner.clone_component(
                        entity_info.location.index_in_archetype as usize,
                        c,
                        &mut new_archtype.components[channel_in_destination],
                    );
                    channel_in_destination += 1;
                }
            }
        }

        let entity = self.get_entity_id();
        self.entities[entity.index as usize].location = EntityLocation {
            archetype_index: new_archetype_index as u32,
            index_in_archetype: self.archetypes[new_archetype_index].len() as u32,
        };
        Some(entity)
    }

    fn add_archetype(&mut self, bundle_id: u64, archetype: Archetype) -> usize {
        let new_archetype_index = self.archetypes.len();
        self.bundle_id_to_archetype
            .insert(bundle_id, new_archetype_index);
        self.archetypes.push(archetype);
        new_archetype_index
    }

    fn get_entity_id(&mut self) -> Entity {
        if let Some(index) = self.free_entities.pop() {
            let (generation, _) = self.entities[index as usize].generation.overflowing_add(1);
            Entity { index, generation }
        } else {
            // Push placeholder data
            self.entities.push(EntityInfo {
                location: EntityLocation {
                    archetype_index: 0,
                    index_in_archetype: 0,
                },
                generation: 0,
            });

            // Error if too many entities are allocated.
            debug_assert!(self.entities.len() <= EntityId::MAX as usize);
            Entity {
                index: (self.entities.len() - 1) as EntityId,
                generation: 0,
            }
        }
    }
    /// Spawn an entity with components passed in through a tuple.
    /// Multiple components can be passed in through the tuple.
    /// # Example
    /// ```
    /// # use kudo::*;
    /// let mut world = World::new();
    /// let entity = world.spawn((456, true));
    /// ```
    pub fn spawn(&mut self, b: impl ComponentBundle) -> Entity {
        let entity = self.get_entity_id();
        let location = b.spawn_in_world(self, entity.index);
        self.entities[entity.index as usize].location = location;
        entity
    }

    /// Spawn an entity with just a single component.
    pub fn spawn_single<T: Component>(&mut self, t: T) -> Entity {
        self.spawn((t,))
    }

    // This should return an error.
    pub(crate) fn get_entity_info(&self, entity: Entity) -> Option<EntityInfo> {
        let entity_info = self.entities[entity.index as usize];
        if entity_info.generation == entity.generation {
            Some(entity_info)
        } else {
            None
        }
    }

    /// Remove an entity and all its components from the world.
    /// An error is returned if the entity does not exist.
    pub fn despawn(&mut self, entity: Entity) -> Result<(), NoSuchEntity> {
        // Remove an entity
        // Update swapped entity position if an entity was moved.

        if let Some(entity_info) = self.get_entity_info(entity) {
            self.entities[entity.index as usize].generation += 1;
            let moved_entity = self.archetypes[entity_info.location.archetype_index as usize]
                .remove_entity(entity_info.location.index_in_archetype);
            self.free_entities.push(entity.index);

            // Update the position of an entity that was moved.
            self.entities[moved_entity as usize].location = entity_info.location;

            Ok(())
        } else {
            Err(NoSuchEntity)
        }
    }

    /// Gets mutable access to a single component on an `Entity`.
    pub fn get_component_mut<T: 'static>(
        &mut self,
        entity: Entity,
    ) -> Result<&mut T, ComponentError> {
        if let Some(entity_info) = self.get_entity_info(entity) {
            let archetype = &mut self.archetypes[entity_info.location.archetype_index as usize];
            archetype
                .get_component_mut(entity_info.location.index_in_archetype)
                .map_err(|e| ComponentError::EntityMissingComponent(e))
        } else {
            // Entity no longer exists
            Err(ComponentError::NoSuchEntity(NoSuchEntity))
        }
    }

    /// Remove a single component from an entity.
    /// If successful the component is returned.
    /// # Example
    /// ```
    /// # use kudo::*;
    /// let mut world = World::new();
    /// let entity = world.spawn((456, true));
    /// let b = world.remove_component::<bool>(entity).unwrap();
    /// ```
    pub fn remove_component<T: 'static>(&mut self, entity: Entity) -> Result<T, ComponentError> {
        if let Some(entity_info) = self.get_entity_info(entity) {
            let current_archetype = &self.archetypes[entity_info.location.archetype_index as usize];

            let type_id = TypeId::of::<T>();
            let mut type_ids: Vec<TypeId> = current_archetype
                .components
                .iter()
                .map(|c| c.type_id)
                .collect();
            let binary_search_index = type_ids.binary_search(&type_id);

            if let Ok(remove_index) = binary_search_index {
                type_ids.remove(remove_index);
                let bundle_id = calculate_bundle_id(&type_ids);
                let new_archetype_index = if let Some(new_archetype_index) =
                    self.bundle_id_to_archetype.get(&bundle_id)
                {
                    *new_archetype_index
                } else {
                    // Create a new archetype
                    let mut archetype_builder =
                        ArchetypeBuilder::new(current_archetype.components.len() - 1);
                    for c in current_archetype.components.iter() {
                        if c.type_id != type_id {
                            archetype_builder.add_component_store(c.new_same_type());
                        }
                    }
                    // A slight optimization here would be to skip the sort that occurs
                    // in ArchetypeBuilder.
                    let archetype = archetype_builder.build();
                    self.add_archetype(bundle_id, archetype)
                };

                // Much of this code is similar to the code for adding a component.

                // index_twice lets us mutably borrow from the world twice.
                let (old_archetype, new_archetype) = index_twice(
                    &mut self.archetypes,
                    entity_info.location.archetype_index as usize,
                    new_archetype_index,
                );

                // If an entity is being moved then update its location
                if let Some(last) = old_archetype.entities.last() {
                    self.entities[*last as usize].location = entity_info.location;
                }

                // First update the entity's location to reflect the changes about to be made.
                self.entities[entity.index as usize].location = EntityLocation {
                    archetype_index: new_archetype_index as EntityId,
                    index_in_archetype: (new_archetype.len()) as EntityId,
                };

                // The new archetype is the same as the old one but with one fewer components.
                for i in 0..remove_index {
                    old_archetype.migrate_component(
                        i,
                        entity_info.location.index_in_archetype,
                        new_archetype,
                        i,
                    );
                }

                let components_in_archetype = old_archetype.components.len();

                for i in (remove_index + 1)..components_in_archetype {
                    old_archetype.migrate_component(
                        i,
                        entity_info.location.index_in_archetype,
                        new_archetype,
                        i - 1,
                    );
                }

                old_archetype
                    .entities
                    .swap_remove(entity_info.location.index_in_archetype as usize);
                new_archetype.entities.push(entity.index);

                Ok(
                    component_vec_to_mut::<T>(&mut *old_archetype.components[remove_index].data)
                        .swap_remove(entity_info.location.index_in_archetype as usize),
                )
            } else {
                // Component is not in entity
                Err(ComponentError::EntityMissingComponent(
                    EntityMissingComponent::new::<T>(entity.index),
                ))
            }
        } else {
            // Entity is not in world
            Err(ComponentError::NoSuchEntity(NoSuchEntity))
        }
    }

    /// Adds a component to an entity.
    /// If the component already exists its data will be replaced.
    pub fn add_component<T: 'static + Send + Sync>(
        &mut self,
        entity: Entity,
        t: T,
    ) -> Result<(), NoSuchEntity> {
        // In an archetypal ECS adding and removing components are the most expensive operations.
        // The volume of code in this function reflects that.
        // When a component is added the entity can be either migrated to a brand new archetype
        // or migrated to an existing archetype.

        // First find if the entity exists
        if let Some(entity_info) = self.get_entity_info(entity) {
            let type_id = TypeId::of::<T>();

            // First check if the component already exists for this entity.
            let current_archetype = &self.archetypes[entity_info.location.archetype_index as usize];

            let mut type_ids: Vec<TypeId> = current_archetype
                .components
                .iter()
                .map(|c| c.type_id)
                .collect();
            let binary_search_index = type_ids.binary_search(&type_id);

            if let Ok(insert_index) = binary_search_index {
                // The component already exists, replace it.
                let current_archetype =
                    &mut self.archetypes[entity_info.location.archetype_index as usize];

                // Replace the existing component.
                current_archetype.replace_component(
                    insert_index,
                    entity_info.location.index_in_archetype,
                    t,
                );
            } else {
                // The component does not already exist in the current archetype.
                // Find an existing archetype to migrate to or create a new archetype

                let insert_index = binary_search_index.unwrap_or_else(|i| i);

                type_ids.insert(insert_index, type_id);
                let bundle_id = calculate_bundle_id(&type_ids);

                let new_archetype_index = if let Some(new_archetype_index) =
                    self.bundle_id_to_archetype.get(&bundle_id)
                {
                    // Found an existing archetype to migrate data to
                    *new_archetype_index
                } else {
                    // Create a new archetype with the structure of the current archetype and one additional component.
                    let mut archetype_builder =
                        ArchetypeBuilder::new(current_archetype.components.len() + 1);

                    for c in current_archetype.components.iter() {
                        archetype_builder.add_component_store(c.new_same_type());
                    }
                    archetype_builder.add_component_store(ComponentStore::new::<T>());
                    let archetype = archetype_builder.build();
                    self.add_archetype(bundle_id, archetype)
                };

                // index_twice lets us mutably borrow from the world twice.
                let (old_archetype, new_archetype) = index_twice(
                    &mut self.archetypes,
                    entity_info.location.archetype_index as usize,
                    new_archetype_index,
                );

                // If an entity is being moved then update its location
                if let Some(last) = old_archetype.entities.last() {
                    self.entities[*last as usize].location = entity_info.location;
                }

                // First update the entity's location to reflect the changes about to be made.
                self.entities[entity.index as usize].location = EntityLocation {
                    archetype_index: new_archetype_index as EntityId,
                    index_in_archetype: (new_archetype.len()) as EntityId,
                };

                // The new archetype is the same as the old one but with one additional component.
                for i in 0..insert_index {
                    old_archetype.migrate_component(
                        i,
                        entity_info.location.index_in_archetype,
                        new_archetype,
                        i,
                    );
                }

                // Push the new component to the new archetype
                new_archetype.push(insert_index, t);

                let components_in_archetype = old_archetype.components.len();

                for i in insert_index..components_in_archetype {
                    old_archetype.migrate_component(
                        i,
                        entity_info.location.index_in_archetype,
                        new_archetype,
                        i + 1,
                    );
                }

                old_archetype
                    .entities
                    .swap_remove(entity_info.location.index_in_archetype as usize);
                new_archetype.entities.push(entity.index);
            }

            Ok(())
        } else {
            Err(NoSuchEntity)
        }
    }

    /// Query for an immutable reference to the first instance of a component found.
    pub fn get_single<T: 'static>(&self) -> Result<Single<T>, FetchError> {
        <&T>::fetch(self)
    }

    /// Query for a mutable reference to the first instance of a component found.
    pub fn get_single_mut<T: 'static>(&self) -> Result<SingleMut<T>, FetchError> {
        <&mut T>::fetch(self)
    }

    /// Get a query from the world.
    /// # Example
    /// ```
    /// # use kudo::*;
    /// # let mut world = World::new();
    /// let query = world.query<(&bool, &String)>();
    /// ```
    pub fn query<'world_borrow, T: QueryParameters>(
        &'world_borrow self,
    ) -> Result<Query<T>, FetchError> {
        Ok(Query::<T>::fetch(self)?.take().unwrap())
    }
}

/// A bundle of components
/// Used to spawn new
pub trait ComponentBundle: 'static + Send + Sync {
    #[doc(hidden)]
    fn new_archetype(&self) -> Archetype;
    #[doc(hidden)]
    fn spawn_in_world(self, world: &mut World, entity_index: EntityId) -> EntityLocation;
}

fn calculate_bundle_id(types: &[TypeId]) -> u64 {
    let mut s = DefaultHasher::new();
    types.hash(&mut s);
    s.finish()
}

macro_rules! component_bundle_impl {
    ($count: expr, $(($name: ident, $index: tt)),*) => {
        impl< $($name: 'static + Send + Sync),*> ComponentBundle for ($($name,)*) {
            fn new_archetype(&self) -> Archetype {
                let mut archetype_builder = ArchetypeBuilder::new($count);
                $(archetype_builder.add_component_store(ComponentStore::new::<$name>());)*
                archetype_builder.build()
            }

            fn spawn_in_world(self, world: &mut World, entity_index: EntityId) -> EntityLocation {
                let mut types = [$(($index, TypeId::of::<$name>())), *];
                types.sort_unstable_by(|a, b| a.1.cmp(&b.1));
                debug_assert!(
                    types.windows(2).all(|x| x[0].1 != x[1].1),
                    "`ComponentBundle`s cannot have duplicate types"
                );

                // Is there a better way to map the original ordering to the sorted ordering?
                let mut order = [0; $count];
                for i in 0..order.len() {
                    order[types[i].0] = i;
                }
                let types = [$(types[$index].1), *];

                let bundle_id = calculate_bundle_id(&types);

                // Find the appropriate archetype
                // If it doesn't exist create a new archetype.
                let archetype_index = if let Some(archetype) = world.bundle_id_to_archetype.get(&bundle_id) {
                    *archetype
                } else {
                    let archetype = self.new_archetype();
                    world.add_archetype(bundle_id, archetype)
                };

                world.archetypes[archetype_index].entities.push(entity_index);
                $(world.archetypes[archetype_index].push(order[$index], self.$index);)*
                EntityLocation {
                    archetype_index: archetype_index as EntityId,
                    index_in_archetype: (world.archetypes[archetype_index].len() - 1) as EntityId
                }
            }
        }
    }
}

component_bundle_impl! {1, (A, 0)}
component_bundle_impl! {2, (A, 0), (B, 1)}
component_bundle_impl! {3, (A, 0), (B, 1), (C, 2)}
component_bundle_impl! {4, (A, 0), (B, 1), (C, 2), (D, 3)}
component_bundle_impl! {5, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4)}
component_bundle_impl! {6, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5)}
component_bundle_impl! {7, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6)}
component_bundle_impl! {8, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6), (H, 7)}
component_bundle_impl! {9, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6), (H, 7), (I, 8)}
component_bundle_impl! {10, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6), (H, 7), (I, 8), (J, 9)}
component_bundle_impl! {11, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6), (H, 7), (I, 8), (J, 9), (K, 10)}
component_bundle_impl! {12, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6), (H, 7), (I, 8), (J, 9), (K, 10), (L, 11)}

/// A helper to get two mutable borrows from the same slice.
fn index_twice<T>(slice: &mut [T], first: usize, second: usize) -> (&mut T, &mut T) {
    if first < second {
        let (a, b) = slice.split_at_mut(second);
        (&mut a[first], &mut b[0])
    } else {
        let (a, b) = slice.split_at_mut(first);
        (&mut b[0], &mut a[second])
    }
}
