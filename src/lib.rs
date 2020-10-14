mod scheduler;
mod system;

use std::any::{Any, TypeId};
use std::collections::{hash_map::DefaultHasher, HashMap};
use std::hash::{Hash, Hasher};
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
pub use system::*;

// This can be used to easily change the size of an EntityId.
type EntityId = u32;

trait ComponentVec {
    fn to_any(&self) -> &dyn Any;
    fn to_any_mut(&mut self) -> &mut dyn Any;
    fn len(&mut self) -> usize;
    fn swap_remove(&mut self, index: EntityId);
    fn migrate(&mut self, entity_index: EntityId, other_archetype: &mut dyn ComponentVec);
    fn new_same_type(&self) -> Box<dyn ComponentVec>;
}

impl<T: 'static> ComponentVec for RwLock<Vec<T>> {
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

    fn new_same_type(&self) -> Box<dyn ComponentVec> {
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
    data: Box<dyn ComponentVec>,
}

impl ComponentStore {
    pub fn new<T: 'static>() -> Self {
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

    /*
    fn matches_types(&self, types: &[TypeId]) -> bool {
        use std::cmp::Ordering;

        let mut query_types = types.iter();
        let mut query_type = query_types.next();
        for t in self.components.iter().map(|c| c.type_id) {
            if let Some(q) = query_type {
                match t.partial_cmp(q) {
                    // Components are sorted, if we've passed a component
                    // it can no longer be found.
                    Some(Ordering::Greater) => return false,
                    Some(Ordering::Equal) => {
                        query_type = query_types.next();
                    }
                    _ => {}
                }
            }
        }
        query_type.is_none()
    }
    */
}

/// A trait for data that has been borrowed from the world.
/// Call `iter` to get an iterator over the data.
pub trait WorldBorrow<'iter> {
    type Iter: Iterator;
    fn iter(&'iter mut self) -> Self::Iter;
}

impl<'iter, 'world_borrow, T: 'static> WorldBorrow<'iter> for WorldBorrowImmut<'world_borrow, T> {
    type Iter = ChainedIterator<std::slice::Iter<'iter, T>>;

    fn iter(&'iter mut self) -> Self::Iter {
        let mut iters: Vec<std::slice::Iter<'iter, T>> =
            self.locks.iter().map(|l| l.iter()).collect();
        // If no iters, add an empty iter to iterate over.
        if iters.is_empty() {
            iters.push([].iter())
        }
        ChainedIterator::new(iters)
    }
}

impl<'iter, 'world_borrow, T: 'static> WorldBorrow<'iter>
    for WorldBorrowMut<'world_borrow, T>
{
    type Iter = ChainedIterator<std::slice::IterMut<'iter, T>>;

    fn iter(&'iter mut self) -> Self::Iter {
        let mut iters: Vec<std::slice::IterMut<'iter, T>> =
            self.locks.iter_mut().map(|l| l.iter_mut()).collect();
        // If no iters, add an empty iter to iterate over.
        if iters.is_empty() {
            iters.push([].iter_mut())
        }
        ChainedIterator::new(iters)
    }
}

/// A query reference specifies how data will be queried and borrowed from the world.
pub trait Query<'world_borrow> {
    type WorldBorrow: for<'a> WorldBorrow<'a>;

    /// Used to verify that there are no duplicates queries in a query.
    fn add_types(types: &mut Vec<TypeId>);

    /// Get the query data from the world for the archetypes indice passed in.
    fn get_query(world: &'world_borrow World, archetypes: &[usize]) -> Self::WorldBorrow;

    // Because of the way this is implemented the worst case for finding
    // archetypes for a query is approximately O(a * c * q)
    // where a is the number of archetypes
    // c is the number of components in an archetype (which varies)
    // and q is the number of queries in this query.
    fn matches_archetype(archetype: &Archetype) -> bool;
}

impl<'world_borrow, A: 'static> Query<'world_borrow> for &'world_borrow A {
    type WorldBorrow = WorldBorrowImmut<'world_borrow, A>;

    fn add_types(types: &mut Vec<TypeId>) {
        types.push(TypeId::of::<A>())
    }

    fn get_query(world: &'world_borrow World, archetypes: &[usize]) -> Self::WorldBorrow {
        let type_id = TypeId::of::<A>();
        let mut query = WorldBorrowImmut::new();
        for i in archetypes {
            query.add_archetype(type_id, &world.archetypes[*i]);
        }
        query
    }

    fn matches_archetype(archetype: &Archetype) -> bool {
        let type_id = TypeId::of::<A>();
        archetype.components.iter().any(|c| c.type_id == type_id)
    }
}

impl<'world_borrow, A: 'static> Query<'world_borrow> for &mut A {
    type WorldBorrow = WorldBorrowMut<'world_borrow, A>;

    fn add_types(types: &mut Vec<TypeId>) {
        types.push(TypeId::of::<A>())
    }

    fn get_query(world: &'world_borrow World, archetypes: &[usize]) -> Self::WorldBorrow {
        let type_id = TypeId::of::<A>();
        let mut query = WorldBorrowMut::new();
        for i in archetypes {
            query.add_archetype(type_id, &world.archetypes[*i]);
        }
        query
    }
    fn matches_archetype(archetype: &Archetype) -> bool {
        let type_id = TypeId::of::<A>();
        archetype.components.iter().any(|c| c.type_id == type_id)
    }
}

pub struct WorldBorrowImmut<'a, T> {
    locks: Vec<RwLockReadGuard<'a, Vec<T>>>,
}

impl<'a, T: 'static> WorldBorrowImmut<'a, T> {
    fn new() -> Self {
        Self { locks: Vec::new() }
    }

    fn add_archetype(&mut self, id: TypeId, archetype: &'a Archetype) {
        // In theory this index may have already been found, but it's not too bad to do it again here.
        let index = archetype
            .components
            .iter()
            .position(|c| c.type_id == id)
            .unwrap();
        self.locks.push(
            archetype
                .get(index)
                .try_read()
                .expect("Cannot read: Component store already borrowed"),
        )
    }
}

pub struct WorldBorrowMut<'a, T> {
    locks: Vec<RwLockWriteGuard<'a, Vec<T>>>,
}

impl<'a, T: 'static> WorldBorrowMut<'a, T> {
    fn new() -> Self {
        Self { locks: Vec::new() }
    }

    fn add_archetype(&mut self, id: TypeId, archetype: &'a Archetype) {
        // In theory this index have already been found, but it's not too bad to do it again here.
        let index = archetype
            .components
            .iter()
            .position(|c| c.type_id == id)
            .unwrap();
        self.locks.push(
            archetype
                .get(index)
                .try_write()
                .expect("Cannot write: Component store already borrowed"),
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EntityLocation {
    archetype_index: EntityId,
    index_in_archetype: EntityId,
}

#[derive(Clone, Copy)]
struct EntityInfo {
    generation: EntityId,
    location: EntityLocation,
}

#[derive(Clone, Copy, Hash, Eq, Ord, PartialEq, PartialOrd)]
pub struct Entity {
    index: EntityId,
    generation: EntityId,
}

pub struct World {
    archetypes: Vec<Archetype>,
    bundle_id_to_archetype: HashMap<u64, usize>,
    entities: Vec<EntityInfo>,
    free_entities: Vec<EntityId>,
}

impl World {
    pub fn new() -> Self {
        Self {
            archetypes: Vec::new(),
            bundle_id_to_archetype: HashMap::new(),
            entities: Vec::new(),
            free_entities: Vec::new(),
        }
    }

    pub fn run_system<'world_borrow, A>(
        &'world_borrow self,
        system: impl System<'world_borrow, A>,
    ) {
        (system).run(&self);
    }

    pub fn spawn(&mut self, b: impl ComponentBundle) -> Entity {
        let (index, generation) = if let Some(index) = self.free_entities.pop() {
            let generation = self.entities[index as usize].generation + 1;
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
    pub fn add_component<T: 'static>(&mut self, entity: Entity, t: T) -> Result<(), ()> {
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
                    // println!("Found matching archetype for component addition");
                    *new_archetype_index
                } else {
                    // Create a new archetype with the structure of the current archetype and one additional component.
                    // println!("Creating new archetype");
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

    pub fn query<'world_borrow, Q: Query<'world_borrow>>(&'world_borrow self) -> Q::WorldBorrow {
        #[cfg(debug_assertions)]
        {
            let mut types = Vec::new();
            Q::add_types(&mut types);
            types.sort();
            debug_assert!(
                types.windows(2).all(|x| x[0] != x[1]),
                "Queries cannot have duplicate types"
            );
        }

        let mut archetype_indices = Vec::new();
        for (i, archetype) in self.archetypes.iter().enumerate() {
            if Q::matches_archetype(&archetype) {
                archetype_indices.push(i);
            }
        }

        Q::get_query(self, &archetype_indices)
    }
}

pub struct ChainedIterator<I: Iterator> {
    current_iter: I,
    iterators: Vec<I>,
}

impl<I: Iterator> ChainedIterator<I> {
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
    fn new_archetype() -> Archetype;
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
        impl< $($name: 'static),*> ComponentBundle for ($($name,)*) {
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

        impl<'world_borrow, $($name: Query<'world_borrow>),*> Query<'world_borrow>
            for ($($name,)*)
        {
            type WorldBorrow = ($($name::WorldBorrow,)*);

            fn add_types(types: &mut Vec<TypeId>) {
                $($name::add_types(types);)*
            }

            fn get_query(world: &'world_borrow World, archetypes: &[usize]) -> Self::WorldBorrow {
                (
                    $($name::get_query(world, archetypes),)*
                )
            }
            fn matches_archetype(archetype: &Archetype) -> bool {
                $($name::matches_archetype(archetype))&&*
            }
        }

        #[allow(non_snake_case)]
        impl<'a, $($name: WorldBorrow<'a>),*> WorldBorrow<'a> for ($($name,)*){
            type Iter = Zip<($($name::Iter,)*)>;
            fn iter(&'a mut self) -> Self::Iter {
                let ($(ref mut $name,)*) = self;

                Zip {
                    t: ($($name.iter(),)*)
                }
            }
        }

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
