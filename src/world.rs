use std::{any::TypeId, collections::HashMap, sync::Arc};

use crate::*;

pub trait ComponentTrait: Send + Sync + 'static {}
impl<T: Send + Sync + 'static> ComponentTrait for T {}

pub struct World {
    pub(crate) archetypes: Vec<Archetype>,
    pub(crate) storage_lookup: StorageLookup,
    pub(crate) entities: Entities,
    pub(crate) cloners: HashMap<TypeId, Arc<dyn ClonerTrait>>,
}

#[derive(PartialEq, Debug, Hash, Eq)]
pub struct Entity {
    pub(crate) index: usize,
    pub(crate) generation: usize,
}

impl Entity {
    pub fn index(&self) -> usize {
        self.index
    }
}

impl EntityClone for Entity {
    fn clone_entity(&self) -> Self {
        Self {
            index: self.index,
            generation: self.generation,
        }
    }
}

pub trait EntityClone {
    fn clone_entity(&self) -> Self;
}

impl<E: EntityClone> EntityClone for Option<E> {
    fn clone_entity(&self) -> Self {
        self.as_ref().map(|e| e.clone_entity())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct EntityLocation {
    pub(crate) archetype_index: usize,
    pub(crate) index_within_archetype: usize,
}

impl World {
    pub fn new() -> Self {
        Self {
            archetypes: Vec::new(),
            entities: Entities::new(),
            storage_lookup: StorageLookup::new(),
            cloners: HashMap::new(),
        }
    }

    pub fn register_clone_type<T: Clone + 'static + Send + Sync>(&mut self) {
        self.cloners.insert(
            TypeId::of::<T>(),
            Arc::new(Cloner::<T> {
                phantom: std::marker::PhantomData,
            }),
        );
    }

    fn clone_hierarchy(
        &mut self,
        entity_to_clone: &Entity,
        new_parent: Option<&Entity>,
        new_next_sibling: Option<&Entity>,
    ) -> Option<Entity> {
        let new_entity = self.clone_entity_inner(entity_to_clone)?;

        if let Some(node) = self
            .get_component_mut::<HierarchyNode>(entity_to_clone)
            .map(|h| h.clone_hierarchy())
        {
            let new_previous_sibling = if let Some(previous_sibling) = node.previous_sibling {
                Some(self.clone_hierarchy(&previous_sibling, new_parent, Some(&new_entity))?)
            } else {
                None
            };

            let new_last_child = if let Some(last_child) = node.last_child {
                Some(self.clone_hierarchy(&last_child, Some(&new_entity), None)?)
            } else {
                None
            };

            *self
                .get_component_mut::<HierarchyNode>(&entity_to_clone)
                .unwrap() = HierarchyNode {
                parent: new_parent.map(|e| e.clone_entity()),
                previous_sibling: new_previous_sibling,
                last_child: new_last_child,
                next_sibling: new_next_sibling.map(|e| e.clone_entity()),
            }
        }

        Some(new_entity)
    }

    fn clone_entity_inner(&mut self, entity: &Entity) -> Option<Entity> {
        let entity_location = self.entities.get_location(entity)??;

        let archetype = &mut self.archetypes[entity_location.archetype_index];
        let mut cloners = Vec::with_capacity(archetype.channels.len());

        for (i, channel) in archetype.channels.iter().enumerate() {
            cloners.push((i, self.cloners.get(&channel.type_id)?))
        }

        for (channel, cloner) in cloners {
            cloner.clone_within(
                entity_location.index_within_archetype,
                archetype.channels[channel].channel(),
            )
        }

        let new_entity = self.entities.new_entity_handle();
        let index_within_archetype = archetype.entities.get_mut().unwrap().len();
        archetype
            .entities
            .get_mut()
            .unwrap()
            .push(new_entity.clone_entity());

        *self.entities.get_at_index_mut(new_entity.index) = Some(EntityLocation {
            archetype_index: entity_location.archetype_index,
            index_within_archetype,
        });
        Some(new_entity)
    }

    /*/
    pub fn clone_into_empty_entity(&mut self, target: Entity, from: Entity) -> Option<()> {
        //  self.clone_entity(entity)
        //  let entity_location = self.entities.get_location(target)?;
        //  assert!(entity_location.is_none());
        //  self.entities.get_location(target.lo)
        todo!()
    }*/

    pub fn spawn<CB: ComponentBundle>(&mut self, component_bundle: CB) -> Entity {
        component_bundle.spawn_in_world(self)
    }

    /// Adds a component to an Entity
    /// If the Entity does not exist, this returns None.
    /// If a component of the same type is already attached to the Entity, the component will be replaced.s
    pub fn add_component<T: ComponentTrait>(
        &mut self,
        entity: &Entity,
        component: T,
    ) -> Option<()> {
        // `add_component` is split into two parts. The part here does a small amount of work.
        // This is structured this way so that `World` and `CloneableWorld` can have slightly different implementations.
        let type_id = TypeId::of::<T>();
        let add_info = self.add_component_inner(entity, type_id).ok()?;
        let new_archetype = &mut self.archetypes[add_info.archetype_index];
        if add_info.new_archetype {
            // If a new `Archetype` is constructed insert its new channel.
            new_archetype.channels.insert(
                add_info.channel_index,
                ComponentChannelStorage::new::<T>(self.cloners.get(&type_id).cloned()),
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
    pub fn despawn(&mut self, entity: &Entity) -> Result<(), ()> {
        let entity_location = self.entities.free_entity(entity)?;
        if let Some(entity_location) = entity_location {
            let archetype = &mut self.archetypes[entity_location.archetype_index];
            let swapped_entity = archetype
                .entities
                .get_mut()
                .unwrap()
                .last()
                .map(|e| e.clone_entity())
                .ok_or(())?;
            archetype.swap_remove(entity_location.index_within_archetype);

            if &swapped_entity != entity {
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
                        .get_component_mut::<HierarchyNode>(&child)
                        .map(|n| n.previous_sibling.clone_entity())
                        .flatten();
                    self.despawn(entity)?;
                }
            }
        }
        Ok(())
    }

    pub fn remove_component<T: 'static>(&mut self, entity: &Entity) -> Option<T> {
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
    pub fn get_component_mut<T: 'static>(&mut self, entity: &Entity) -> Option<&mut T> {
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
        entity: &Entity,
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
        entity: &Entity,
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
                .as_mut()
                .unwrap();
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

        new_archetype
            .entities
            .get_mut()
            .unwrap()
            .push(entity.clone_entity());
    }

    /// Clones an `Entity` and returns a new `Entity`.
    /// For now this will return `None` if any components can't be cloned.
    /// If this Entity has child `Entity`s they will be recursively cloned as well.
    pub fn clone_entity(&mut self, entity: &Entity) -> Option<Entity> {
        if self.get_component_mut::<HierarchyNode>(entity).is_some() {
            // Hierarchies need special care when cloning
            // because they will also clone child Entity's
            self.clone_hierarchy(entity, None, None)
        } else {
            self.clone_entity_inner(entity)
        }
    }

    pub fn add_world_to_world(&mut self, other: &mut World) {
        let mut entity_migrator = EntityMigrator::new(&mut other.entities);
        let mut type_ids = Vec::new();

        // For each `Archetype` find a new `Archetype` and then migrate entities to it.
        for archetype in &mut other.archetypes {
            type_ids.clear();
            for channel in &archetype.channels {
                if channel.cloner.is_some() {
                    type_ids.push(channel.type_id)
                }
            }
            if let Some(_archetype_index) =
                self.storage_lookup.get_archetype_with_components(&type_ids)
            {
                // Append the archetype to this archetype
                todo!()
            } else {
                // Create a new archetype from the old one.
                let new_archetype =
                    archetype.clone_archetype(&mut entity_migrator, &mut self.entities);
                self.push_archetype(new_archetype, &type_ids);
            }
        }
    }
}

pub(crate) struct EntityMigrator {
    /// New entities indexed with the index of the old entities.
    new_entities: Vec<Option<Entity>>,
}
impl EntityMigrator {
    pub fn new(old_entities: &mut Entities) -> Self {
        let mut new_entities = Vec::new();
        new_entities.resize_with(
            old_entities
                .inner
                .get_mut()
                .unwrap()
                .generation_and_location
                .len(),
            || None,
        );
        Self { new_entities }
    }

    pub fn get_or_create(&mut self, entity_old: &Entity, entities: &mut Entities) -> Entity {
        if let Some(entity) = &self.new_entities[entity_old.index] {
            entity.clone_entity()
        } else {
            let new_handle = entities.new_entity_handle();
            self.new_entities[entity_old.index] = Some(new_handle.clone_entity());
            new_handle
        }
    }
}

pub struct AddInfo {
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
