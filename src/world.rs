use std::{any::TypeId, cell::RefCell, collections::HashMap, sync::Arc};

use crate::*;

pub trait ComponentTrait: Send + 'static {}
impl<T: Send + 'static> ComponentTrait for T {}

#[derive(Clone)]
pub struct Cloners(pub(crate) HashMap<TypeId, Arc<dyn ClonerTrait>>);

impl Cloners {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn register_clone_type<T: WorldClone + 'static + Send>(&mut self) {
        self.0.insert(
            TypeId::of::<T>(),
            Arc::new(Cloner::<T> {
                phantom: std::marker::PhantomData,
            }),
        );
    }
}

thread_local! {
    static CLONERS: RefCell<Arc<Cloners>> = RefCell::new(Arc::new(Cloners::new()));
}

/// Set the `Cloners` used by all future worlds created.
pub fn set_cloners(cloners: Cloners) {
    CLONERS.with(|w| *w.borrow_mut() = Arc::new(cloners))
}

pub struct World {
    pub(crate) archetypes: Vec<Archetype>,
    pub(crate) storage_lookup: StorageLookup,
    pub(crate) entities: Entities,
    pub cloners: Arc<Cloners>,
}

#[derive(PartialEq, Debug, Hash, Eq, Clone, Copy)]
pub struct Entity {
    pub(crate) index: usize,
    pub(crate) generation: usize,
}

impl Entity {
    pub fn index(&self) -> usize {
        self.index
    }

    pub fn generation(&self) -> usize {
        self.generation
    }

    /// Useful when storing an `Entity` handle in another library for reconstruction later.
    pub fn from_index_and_generation(index: usize, generation: usize) -> Self {
        Self { index, generation }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct EntityLocation {
    pub(crate) archetype_index: usize,
    pub(crate) index_within_archetype: usize,
}

impl World {
    pub fn new() -> Self {
        let cloners = CLONERS.with(|w| w.borrow().clone());

        Self {
            archetypes: Vec::new(),
            entities: Entities::new(),
            storage_lookup: StorageLookup::new(),
            cloners,
        }
    }

    /*
    pub fn new_with_cloners(cloners: Arc<Cloners>) -> Self {
        Self {
            archetypes: Vec::new(),
            entities: Entities::new(),
            storage_lookup: StorageLookup::new(),
            cloners: cloners.clone(),
        }
    }
    */

    pub fn spawn<CB: ComponentBundle>(&mut self, component_bundle: CB) -> Entity {
        component_bundle.spawn_in_world(self)
    }

    /// Adds a component to an Entity
    /// If the Entity does not exist, this returns None.
    /// If a component of the same type is already attached to the Entity, the component will be replaced.s
    pub fn add_component<T: ComponentTrait>(&mut self, entity: Entity, component: T) -> Option<()> {
        // `add_component` is split into two parts. The part here does a small amount of work.
        // This is structured this way so that `World` and `CloneableWorld` can have slightly different implementations.
        let type_id = TypeId::of::<T>();
        let add_info = self.add_component_inner(entity, type_id).ok()?;
        let new_archetype = &mut self.archetypes[add_info.archetype_index];
        if add_info.new_archetype {
            // If a new `Archetype` is constructed insert its new channel.
            new_archetype.channels.insert(
                add_info.channel_index,
                ComponentChannelStorage::new::<T>(self.cloners.0.get(&type_id).cloned()),
            );
        }

        if let Some(replace_index) = add_info.replace_index {
            new_archetype.get_channel_mut(add_info.channel_index)[replace_index] = component;
        } else {
            new_archetype
                .get_channel_mut(add_info.channel_index)
                .push(component);
        }
        Some(())
    }

    pub fn query<'world_borrow, T: QueryParameters>(
        &'world_borrow self,
    ) -> Result<
        <<Query<'world_borrow, T> as QueryTrait<'world_borrow>>::Result as GetQueryDirect>::Arg,
        Error,
    >
    where
        Query<'world_borrow, T>: QueryTrait<'world_borrow>,
        <Query<'world_borrow, T> as QueryTrait<'world_borrow>>::Result: GetQueryDirect,
    {
        let query_info = <Query<'world_borrow, T> as GetQueryInfoTrait>::query_info(self)?;
        let result = <Query<'world_borrow, T>>::get_query(self, &query_info)?;
        Ok(result.get_query_direct())
    }

    pub(crate) fn storage_lookup(&self) -> &StorageLookup {
        &self.storage_lookup
    }

    pub(crate) fn entities(&self) -> &Entities {
        &self.entities
    }

    pub(crate) fn borrow_archetype(&self, index: usize) -> &Archetype {
        &self.archetypes[index]
    }

    pub(crate) fn push_archetype(&mut self, archetype: Archetype, type_ids: &[TypeId]) -> usize {
        let new_archetype_index = self.archetypes.len();
        self.archetypes.push(archetype);
        self.storage_lookup
            .new_archetype(new_archetype_index, &type_ids);
        new_archetype_index
    }

    pub fn reserve_entity(&self) -> Entity {
        self.entities.reserve_entity()
    }

    /// Remove an entity and all its components from the world.
    /// An error is returned if the entity does not exist.
    pub fn despawn(&mut self, entity: Entity) -> Result<(), ()> {
        let entity_location = self.entities.free_entity(entity)?;
        if let Some(entity_location) = entity_location {
            let archetype = &mut self.archetypes[entity_location.archetype_index];
            let swapped_entity = archetype
                .entities
                .get_mut()
                .unwrap()
                .last()
                .copied()
                .ok_or(())?;
            archetype.swap_remove(entity_location.index_within_archetype);

            if swapped_entity != entity {
                *self.entities.get_at_index_mut(swapped_entity.index) = Some(entity_location);
            }

            // If this is a hierarchy node then child `Entity`s and their children should be removed.
            if let Some(hierarchy_node) = self
                .get_component_mut::<HierarchyNode>(entity)
                .map(|h| h.clone_hierarchy())
            {
                // Despawn all children and their siblings
                let mut current_child = hierarchy_node.last_child;
                while let Some(child) = current_child {
                    current_child = self
                        .get_component_mut::<HierarchyNode>(child)
                        .map(|n| n.previous_sibling)
                        .flatten();
                    self.despawn(entity)?;
                }
            }
        }
        Ok(())
    }

    pub fn remove_component<T: 'static>(&mut self, entity: Entity) -> Option<T> {
        let entity_location = self.entities.get_location(entity)??;
        let old_archetype_index = entity_location.archetype_index;
        let mut type_ids = self.archetypes[old_archetype_index].type_ids();
        let removing_component_id = TypeId::of::<T>();

        let remove_channel_index = match type_ids.binary_search(&removing_component_id) {
            Ok(index) => index,
            Err(_) => None?, // Entity does not have this component
        };

        type_ids.remove(remove_channel_index);
        let new_archetype_index = match self.storage_lookup.get_archetype_with_components(&type_ids)
        {
            Some(index) => index,
            None => {
                // Create a new archetype with one additional component.
                let mut new_archetype = Archetype::new();
                for (i, c) in self.archetypes[old_archetype_index]
                    .channels
                    .iter()
                    .enumerate()
                {
                    // Skip the channel we're removing.
                    if i != remove_channel_index {
                        new_archetype.new_channel_same_type(c);
                    }
                }
                self.push_archetype(new_archetype, &type_ids)
            }
        };

        self.migrate_entity_between_archetypes(
            entity,
            entity_location.index_within_archetype,
            old_archetype_index,
            new_archetype_index,
        );

        Some(
            self.archetypes[old_archetype_index]
                .get_channel_mut::<T>(remove_channel_index)
                .swap_remove(entity_location.index_within_archetype),
        )
    }

    /// This will return None if the `Entity` does not exist or the `Entity` does not have the component.
    pub fn get_component_mut<T: 'static>(&mut self, entity: Entity) -> Option<&mut T> {
        let entity_location = self
            .entities
            .get_location(entity)
            .flatten()
            .ok_or(())
            .ok()?;
        let archetype = &mut self.archetypes[entity_location.archetype_index as usize];
        archetype
            .get_component_mut(entity_location.index_within_archetype)
            .ok()
    }

    pub(crate) fn add_component_inner(
        &mut self,
        entity: Entity,
        new_component_id: TypeId,
    ) -> Result<AddInfo, ()> {
        let old_entity_location = self.entities.get_location(entity).ok_or(())?;

        // If the `EntityLocation` is `None` then this `Entity` is reserved is not yet
        // in an Archetype.
        let (type_ids, insert_position) = if let Some(old_entity_location) = old_entity_location {
            let mut type_ids = self.archetypes[old_entity_location.archetype_index].type_ids();
            let insert_position = match type_ids.binary_search(&new_component_id) {
                Ok(i) => {
                    // If the component already exists, simply replace it with the new one.
                    return Ok(AddInfo {
                        archetype_index: old_entity_location.archetype_index,
                        channel_index: i,
                        new_archetype: false,
                        replace_index: Some(old_entity_location.index_within_archetype),
                    });
                }
                Err(position) => {
                    type_ids.insert(position, new_component_id);
                    position
                }
            };
            (type_ids, insert_position)
        } else {
            (Vec::new(), 0)
        };

        let mut is_new_archetype = false;
        // Find or create a new archetype for this entity.
        let new_archetype_index = match self.storage_lookup.get_archetype_with_components(&type_ids)
        {
            Some(index) => index,
            None => {
                // Create a new archetype with one additional component.
                let mut new_archetype = Archetype::new();
                if let Some(entity_location) = old_entity_location {
                    for c in &self.archetypes[entity_location.archetype_index].channels {
                        new_archetype.new_channel_same_type(c);
                    }
                }

                is_new_archetype = true;
                self.push_archetype(new_archetype, &type_ids)
            }
        };

        if let Some(old_entity_location) = old_entity_location {
            self.migrate_entity_between_archetypes(
                entity,
                old_entity_location.index_within_archetype,
                old_entity_location.archetype_index,
                new_archetype_index,
            );
        }

        Ok(AddInfo {
            archetype_index: new_archetype_index,
            channel_index: insert_position,
            replace_index: None,
            new_archetype: is_new_archetype,
        })
    }

    pub fn migrate_entity_between_archetypes(
        &mut self,
        entity: Entity,
        entity_index_within_archetype: usize,
        first_archetype: usize,
        second_archetype: usize,
    ) {
        let (old_archetype, new_archetype) =
            index_mut_twice(&mut self.archetypes, first_archetype, second_archetype);

        // Migrate components from old archetype

        let mut first_channel_iter = old_archetype.channels.iter_mut();
        let mut second_channel_iter = new_archetype.channels.iter_mut();

        let mut first_channel = first_channel_iter.next();
        let mut second_channel = second_channel_iter.next();

        while let Some(first) = &mut first_channel {
            if let Some(second) = &mut second_channel {
                match first.get_type_id().cmp(&second.get_type_id()) {
                    std::cmp::Ordering::Less => {
                        first_channel = first_channel_iter.next();
                    }
                    std::cmp::Ordering::Greater => {
                        second_channel = second_channel_iter.next();
                    }
                    std::cmp::Ordering::Equal => {
                        first.channel_mut().migrate_component(
                            entity_index_within_archetype,
                            &mut *second.channel_mut(),
                        );
                        first_channel = first_channel_iter.next();
                        second_channel = second_channel_iter.next();
                    }
                }
            } else {
                break;
            }
        }

        // `migrate_component` uses `swap_remove` internally, so another Entity's location
        // is swapped and need updating.
        let swapped_entity_index = old_archetype
            .entities
            .get_mut()
            .unwrap()
            .last()
            .unwrap()
            .index;

        {
            // Update the location of the swapped entity.
            let swapped_entity_location = self
                .entities
                .get_at_index_mut(swapped_entity_index)
                .as_mut();

            let swapped_entity_location = swapped_entity_location.unwrap();
            swapped_entity_location.index_within_archetype = entity_index_within_archetype;

            // Update the location of the entity
            let location = self.entities.get_at_index_mut(entity.index);
            *location = Some(EntityLocation {
                archetype_index: second_archetype,
                index_within_archetype: new_archetype.entities.get_mut().unwrap().len(),
            });
        }

        old_archetype
            .entities
            .get_mut()
            .unwrap()
            .swap_remove(entity_index_within_archetype);

        new_archetype.entities.get_mut().unwrap().push(entity);
    }

    /// Returns a list of all new `Entity`s created.
    pub fn add_world_to_world(&mut self, other: &mut World) -> Vec<Entity> {
        let cloners = &mut self.cloners;
        let Self {
            archetypes,
            storage_lookup,
            entities,
            ..
        } = self;
        let mut new_entities = HashMap::new();
        for old_archetype in &mut other.archetypes {
            let mut entity_migrator = EntityMigrator::new(&mut new_entities, entities);

            let mut new_archetype = old_archetype.clone_archetype(&mut entity_migrator, &cloners);
            let type_ids = new_archetype.type_ids();

            // Check if there is an existing archetype or not.
            if let Some(archetype_index) = storage_lookup.get_archetype_with_components(&type_ids) {
                // If there is an existing [Archetype] merge into that.
                archetypes[archetype_index].append_archetype(
                    new_archetype,
                    entities,
                    archetype_index,
                );
            } else {
                let new_archetype_index = archetypes.len();
                storage_lookup.new_archetype(new_archetype_index, &type_ids);
                // Update the location of each `Entity`.
                for (index_within_archetype, entity) in
                    new_archetype.entities.get_mut().unwrap().iter().enumerate()
                {
                    *entities.get_at_index_mut(entity.index) = Some(EntityLocation {
                        archetype_index: new_archetype_index,
                        index_within_archetype,
                    });
                }
                archetypes.push(new_archetype);
            }
        }

        new_entities.values().copied().collect()
    }

    /// Creates a new [World] cloning all components declared to this [World]'s` [Cloner]
    pub fn clone(&mut self) -> Self {
        let mut new_world = World::new();
        new_world.add_world_to_world(self);
        new_world
    }
}

pub(crate) struct AddInfo {
    pub(crate) archetype_index: usize,
    /// If an entity already has a component replace its component at this index.
    pub(crate) replace_index: Option<usize>,
    pub(crate) channel_index: usize,
    pub(crate) new_archetype: bool,
}

/// A helper to get two mutable borrows from the same slice.
fn index_mut_twice<T>(slice: &mut [T], first: usize, second: usize) -> (&mut T, &mut T) {
    if first < second {
        let (a, b) = slice.split_at_mut(second);
        (&mut a[first], &mut b[0])
    } else {
        let (a, b) = slice.split_at_mut(first);
        (&mut b[0], &mut a[second])
    }
}
