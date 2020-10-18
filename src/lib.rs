//! A simple and predictable Entity Component System.
//!
//! An entity-component-system (ECS) is a data structure and organizational pattern often
//! used by game-like code.
//!
//! An ECS is made up of three parts:
//!
//! * **Entities**: IDs associated with various components
//! * **Components**: Individual units of data associated with an entity.
//! * **Systems**: Code that iterates over components
//!
//! ```
//! # use kudo::*;
//! // First we create the world.
//! let mut world = World::new();
//!
//! // Let's create a new entity with a name and a health component.
//! // Components are just plain structs.
//!
//! // This will be our health component
//! struct Health(i32);
//!
//! // Spawn the entity with a String component we'll use for the name and a Health component.
//! // Within the call to spawn we pass in a tuple that can have multiple components.
//! world.spawn(("Medusa".to_string(), Health(-2)));
//!
//! // Query the world for entities that have a String component and a Health component.
//! // The '&' before each component requests read-only access to the component.
//! // Using '&mut' would request write/read access for that component.
//! let mut query = world.query::<(&String, &Health)>().unwrap();
//!
//! // Iterate over all the components we found and check if their health is less than 0.
//! for (name, health) in query.iter() {
//!     if health.0 < 0 {
//!         println!("{} has perished!", name);
//!     }
//! }
//! ```

mod query;
mod system;
mod world_borrow;

pub use query::*;
pub use system::*;
pub use world_borrow::*;

use std::any::{Any, TypeId};
use std::collections::{hash_map::DefaultHasher, HashMap};
use std::hash::{Hash, Hasher};
use std::sync::RwLock;

// This can be used to easily change the size of an EntityId.
type EntityId = u32;

trait ComponentVec {
    fn to_any(&self) -> &dyn Any;
    fn to_any_mut(&mut self) -> &mut dyn Any;
    fn len(&mut self) -> usize;
    fn swap_remove(&mut self, index: EntityId);
    fn migrate(&mut self, entity_index: EntityId, other_archetype: &mut dyn ComponentVec);
    fn new_same_type(&self) -> Box<dyn ComponentVec + Send + Sync>;
}

impl<T: 'static + Send + Sync> ComponentVec for RwLock<Vec<T>> {
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
fn component_vec_to_mut<T: 'static>(c: &mut dyn ComponentVec) -> &mut Vec<T> {
    c.to_any_mut()
        .downcast_mut::<RwLock<Vec<T>>>()
        .unwrap()
        .get_mut()
        .unwrap()
}

/// Stores components for a component type
struct ComponentStore {
    type_id: TypeId,
    data: Box<dyn ComponentVec + Send + Sync>,
}

impl ComponentStore {
    pub fn new<T: 'static + Send + Sync>() -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            data: Box::new(RwLock::new(Vec::<T>::new())),
        }
    }

    /// Creates a new ComponentStore with the same internal storage type
    pub fn new_same_type(&self) -> Self {
        Self {
            type_id: self.type_id,
            data: self.data.new_same_type(),
        }
    }

    pub fn len(&mut self) -> usize {
        self.data.len()
    }
}

#[doc(hidden)]
/// An archetype stores entities with the same set of components.
pub struct Archetype {
    entities: Vec<EntityId>,
    components: Vec<ComponentStore>,
}

impl Archetype {
    pub fn new() -> Self {
        Self {
            entities: Vec::new(),
            components: Vec::new(),
        }
    }
    pub fn get<T: 'static>(&self, index: usize) -> &RwLock<Vec<T>> {
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

    fn mutable_component_store<T: 'static>(&mut self, component_index: usize) -> &mut Vec<T> {
        component_vec_to_mut(&mut *self.components[component_index].data)
    }

    fn replace_component<T: 'static>(&mut self, component_index: usize, index: EntityId, t: T) {
        self.mutable_component_store(component_index)[index as usize] = t;
    }

    fn push<T: 'static>(&mut self, component_index: usize, t: T) {
        self.mutable_component_store(component_index).push(t)
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
        self.components[0].len()
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
struct EntityInfo {
    generation: EntityId,
    location: EntityLocation,
}

/// A handle to an entity within the world.
#[derive(Debug, Clone, Copy, Hash, Eq, Ord, PartialEq, PartialOrd)]
pub struct Entity {
    index: EntityId,
    generation: EntityId,
}

/// The world holds all components and associated entities.
pub struct World {
    archetypes: Vec<Archetype>,
    bundle_id_to_archetype: HashMap<u64, usize>,
    entities: Vec<EntityInfo>,
    free_entities: Vec<EntityId>,
}

impl World {
    /// Create the world.
    pub fn new() -> Self {
        Self {
            archetypes: Vec::new(),
            bundle_id_to_archetype: HashMap::new(),
            entities: Vec::new(),
            free_entities: Vec::new(),
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
        let (index, generation) = if let Some(index) = self.free_entities.pop() {
            let (generation, _) = self.entities[index as usize].generation.overflowing_add(1);
            (index, generation)
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
            ((self.entities.len() - 1) as EntityId, 0)
        };

        let location = b.add_to_world(self, index);

        self.entities[index as usize] = EntityInfo {
            location,
            generation: generation,
        };

        Entity { index, generation }
    }

    /// Remove an entity and all its components from the world.
    /// An error is returned if the entity does not exist.
    pub fn despawn(&mut self, entity: Entity) -> Result<(), ()> {
        // Remove an entity
        // Update swapped entity position if an entity was moved.
        let entity_info = self.entities[entity.index as usize];
        if entity_info.generation == entity.generation {
            self.entities[entity.index as usize].generation += 1;
            let moved_entity = self.archetypes[entity_info.location.archetype_index as usize]
                .remove_entity(entity_info.location.index_in_archetype);
            self.free_entities.push(entity.index);

            // Update the position of an entity that was moved.
            self.entities[moved_entity as usize].location = entity_info.location;

            Ok(())
        } else {
            Err(())
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
    pub fn remove_component<T: 'static>(&mut self, entity: Entity) -> Result<T, ()> {
        let entity_info = self.entities[entity.index as usize];

        if entity_info.generation == entity.generation {
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
                    let mut archetype = Archetype::new();
                    for c in current_archetype.components.iter() {
                        if c.type_id != type_id {
                            archetype.components.push(c.new_same_type());
                        }
                    }

                    let new_archetype_index = self.archetypes.len();

                    self.bundle_id_to_archetype
                        .insert(bundle_id, new_archetype_index);
                    self.archetypes.push(archetype);
                    new_archetype_index
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
                Err(())
            }
        } else {
            // Entity is not in world
            Err(())
        }
    }

    /// Adds a component to an entity.
    /// If the component already exists its data will be replaced.
    pub fn add_component<T: 'static + Send + Sync>(
        &mut self,
        entity: Entity,
        t: T,
    ) -> Result<(), ()> {
        // In an archetypal ECS adding and removing components are the most expensive operations.
        // The volume of code in this function reflects that.
        // When a component is added the entity can be either migrated to a brand new archetype
        // or migrated to an existing archetype.

        // First find if the entity exists
        let entity_info = self.entities[entity.index as usize];
        if entity_info.generation == entity.generation {
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
                    let mut archetype = Archetype::new();
                    for c in current_archetype.components.iter() {
                        archetype.components.push(c.new_same_type());
                    }

                    let new_archetype_index = self.archetypes.len();
                    archetype
                        .components
                        .insert(insert_index, ComponentStore::new::<T>());
                    self.bundle_id_to_archetype
                        .insert(bundle_id, new_archetype_index);

                    self.archetypes.push(archetype);

                    new_archetype_index
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
            Err(())
        }
    }

    pub fn query<T: QueryParams>(&self) -> Result<<Query<T> as TopLevelFetch>::Item, ()> {
        Query::<T>::get(self)
    }
}

/// An iterator that iterates multiple iterators in a row
// Can this be replaced with something from the standard library?
pub struct ChainedIterator<I: Iterator> {
    current_iter: I,
    iterators: Vec<I>,
}

impl<I: Iterator> ChainedIterator<I> {
    #[doc(hidden)]
    pub fn new(mut iterators: Vec<I>) -> Self {
        let current_iter = iterators.pop().unwrap();
        Self {
            current_iter,
            iterators,
        }
    }
}

impl<I: Iterator> Iterator for ChainedIterator<I> {
    type Item = I::Item;
    fn next(&mut self) -> Option<Self::Item> {
        // Chain the iterators together.
        // If the end of one iterator is reached go to the next.

        // Given that this is going to be part of a group of iterators there
        // could be an additional function call that does this without checking if it needs to step
        // to the next iter.
        // That check would be done only for the first iterator.
        self.current_iter.next().or_else(|| {
            self.iterators.pop().map_or(None, |i| {
                self.current_iter = i;
                self.current_iter.next()
            })
        })
    }
}

/// A bundle of components
pub trait ComponentBundle {
    #[doc(hidden)]
    fn new_archetype() -> Archetype;
    #[doc(hidden)]
    fn add_to_world(self, world: &mut World, entity_index: EntityId) -> EntityLocation;
}

fn calculate_bundle_id(types: &[TypeId]) -> u64 {
    // Calculate the bundle ID
    let mut s = DefaultHasher::new();
    types.hash(&mut s);
    s.finish()
}

macro_rules! component_bundle_impl {
    ($count: expr, $(($name: ident, $index: tt)),*) => {
        impl< $($name: 'static + Send + Sync),*> ComponentBundle for ($($name,)*) {
            fn new_archetype() -> Archetype {
                let mut components = vec![$(ComponentStore::new::<$name>()), *];
                components.sort_unstable_by(|a, b| a.type_id.cmp(&b.type_id));
                Archetype { components, entities: Vec::new() }
            }

            fn add_to_world(self, world: &mut World, entity_index: EntityId) -> EntityLocation {
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
                    let archetype = Self::new_archetype();
                    let index = world.archetypes.len();

                    world.bundle_id_to_archetype.insert(bundle_id, index);
                    world.archetypes.push(archetype);
                    index
                };

                world.archetypes[archetype_index].entities.push(entity_index);
                $(world.archetypes[archetype_index].push(order[$index], self.$index);)*
                EntityLocation {
                    archetype_index: archetype_index as EntityId,
                    index_in_archetype: (world.archetypes[archetype_index].len() - 1) as EntityId
                }
            }
        }

        #[allow(non_snake_case)]
        impl<'iter, $($name: GetIter<'iter>),*> GetIter<'iter> for ($($name,)*){
            type Iter = Zip<($($name::Iter,)*)>;
            fn iter(&'iter mut self) -> Self::Iter {
                let ($(ref mut $name,)*) = self;

                Zip {
                    t: ($($name.iter(),)*)
                }
            }
        }

        impl<'world_borrow, $($name: WorldBorrow<'world_borrow>),*> WorldBorrow<'world_borrow> for ($($name,)*){}


        #[allow(non_snake_case)]
        impl<$($name: Iterator),*> Iterator for Zip<($($name,)*)> {
            type Item = ($($name::Item,)*);
            fn next(&mut self) -> Option<Self::Item> {
                let ($(ref mut $name,)*) = self.t;
                // This should be an unwrap unchecked for performance.
                // Because iterators will always be the same length.
                Some(($($name.next()?,)*))
            }
        }
    }
}

/// An iterator over multiple iterators at once.
pub struct Zip<T> {
    t: T,
}

component_bundle_impl! {1, (A, 0)}
component_bundle_impl! {2, (A, 0), (B, 1)}
component_bundle_impl! {3, (A, 0), (B, 1), (C, 2)}
component_bundle_impl! {4, (A, 0), (B, 1), (C, 2), (D, 3)}
component_bundle_impl! {5, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4)}
component_bundle_impl! {6, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5)}
component_bundle_impl! {7, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6)}
component_bundle_impl! {8, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6), (H, 7)}

fn index_twice<T>(slice: &mut [T], first: usize, second: usize) -> (&mut T, &mut T) {
    if first < second {
        let (a, b) = slice.split_at_mut(second);
        (&mut a[first], &mut b[0])
    } else {
        let (a, b) = slice.split_at_mut(first);
        (&mut b[0], &mut a[second])
    }
}
