use std::{any::TypeId, collections::HashMap};

use crate::*;

pub trait ComponentTrait: Send + Sync + 'static {}
impl<T: Send + Sync + 'static> ComponentTrait for T {}

pub struct WorldInner {
    pub(crate) archetypes: Vec<Archetype>,
    pub(crate) storage_lookup: StorageLookup,
    pub(crate) entities: Entities,
}

pub struct World {
    pub(crate) inner: WorldInner,
    cloners: HashMap<TypeId, Box<dyn ClonerTrait>>,
}

pub(crate) trait ClonerTrait: Send + Sync {
    fn clone_within(&self, index: usize, channel: usize, archetype: &mut Archetype);
}

struct Cloner<T> {
    phantom: std::marker::PhantomData<fn() -> T>,
}

impl<T: Clone + 'static> ClonerTrait for Cloner<T> {
    fn clone_within(&self, index: usize, channel: usize, archetype: &mut Archetype) {
        let channel = archetype.get_channel_mut::<T>(channel);
        channel.push(channel[index].clone())
    }
}

#[derive(PartialEq, Debug)]
pub struct Entity {
    pub(crate) index: usize,
    pub(crate) generation: usize,
}

pub trait EntityClone {
    fn clone(&self) -> Self;
}
impl Entity {
    pub fn index(&self) -> usize {
        self.index
    }
}

impl EntityClone for Entity {
    fn clone(&self) -> Self {
        Self {
            index: self.index,
            generation: self.generation,
        }
    }
}

impl<E: EntityClone> EntityClone for Option<E> {
    fn clone(&self) -> Self {
        self.as_ref().map(|e| e.clone())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct EntityLocation {
    pub(crate) archetype_index: usize,
    pub(crate) index_within_archetype: usize,
}

impl World {
    pub fn register_clone_type<T: Clone + 'static>(&mut self) {
        self.cloners.insert(
            TypeId::of::<T>(),
            Box::new(Cloner::<T> {
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
            .map(|h| h.clone())
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
                parent: new_parent.map(|e| e.clone()),
                previous_sibling: new_previous_sibling,
                last_child: new_last_child,
                next_sibling: new_next_sibling.map(|e| e.clone()),
            }
        }

        Some(new_entity)
    }

    fn clone_entity_inner(&mut self, entity: &Entity) -> Option<Entity> {
        let entity_location = self.inner.entities.get_location(entity)??;

        let archetype = &mut self.inner.archetypes[entity_location.archetype_index];
        let mut cloners = Vec::with_capacity(archetype.channels.len());

        for (i, channel) in archetype.channels.iter().enumerate() {
            cloners.push((i, self.cloners.get(&channel.type_id)?))
        }

        for (channel, cloner) in cloners {
            cloner.clone_within(entity_location.index_within_archetype, channel, archetype)
        }

        let new_entity = self.inner.entities.new_entity_handle();
        let index_within_archetype = archetype.entities.get_mut().unwrap().len();
        archetype
            .entities
            .get_mut()
            .unwrap()
            .push(new_entity.clone());

        *self.inner.entities.get_at_index_mut(new_entity.index) = Some(EntityLocation {
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
        component_bundle.spawn_in_world(&mut self.inner)
    }

    /// Adds a component to an Entity
    /// If the Entity does not exist, this returns None.
    /// If a component of the same type is already attached to the Entity, the component will be replaced.s
    pub fn add_component<T: ComponentTrait>(
        &mut self,
        entity: &Entity,
        component: T,
    ) -> Option<()> {
        self.inner.add_component(entity, component)
    }

    pub fn query<'world_borrow, T: QueryParameters>(
        &'world_borrow self,
    ) -> Result<
        <<Query<'world_borrow, T> as QueryTrait<'world_borrow, Self>>::Result as GetQueryDirect>::Arg,
        Error,
    >
    where
        Query<'world_borrow, T>: QueryTrait<'world_borrow, Self>,
        <Query<'world_borrow, T> as QueryTrait<'world_borrow, Self>>::Result: GetQueryDirect,
    {
        let query_info = <Query<'world_borrow, T> as GetQueryInfoTrait<Self>>::query_info(self)?;
        let result = <Query<'world_borrow, T>>::get_query(self, &query_info)?;
        Ok(result.get_query_direct())
    }
}

impl WorldTrait for World {
    fn new() -> Self {
        Self {
            inner: WorldInner::new(),
            cloners: HashMap::new(),
        }
    }

    /// Clones an `Entity` and returns a new `Entity`.
    /// For now this will return `None` if any components can't be cloned.
    /// If this Entity has child `Entity`s they will be recursively cloned as well.
    fn clone_entity(&mut self, entity: &Entity) -> Option<Entity> {
        if self.get_component_mut::<HierarchyNode>(entity).is_some() {
            // Hierarchies need special care when cloning
            // because they will also clone child Entity's
            self.clone_hierarchy(entity, None, None)
        } else {
            self.clone_entity_inner(entity)
        }
    }

    fn reserve_entity(&self) -> Entity {
        self.inner.reserve_entity()
    }

    /// Remove an entity and all its components from the world.
    /// An error is returned if the entity does not exist.
    fn despawn(&mut self, entity: &Entity) -> Result<(), ()> {
        self.inner.despawn(entity)
    }

    fn remove_component<T: 'static>(&mut self, entity: &Entity) -> Option<T> {
        self.inner.remove_component(entity)
    }

    /// This will return None if the `Entity` does not exist or the `Entity` does not have the component.
    fn get_component_mut<T: 'static>(&mut self, entity: &Entity) -> Option<&mut T> {
        self.inner.get_component_mut(entity)
    }
}

impl WorldPrivate for World {
    fn storage_lookup(&self) -> &StorageLookup {
        self.inner.storage_lookup()
    }

    fn entities(&self) -> &Entities {
        self.inner.entities()
    }

    fn borrow_archetype(&self, index: usize) -> &Archetype {
        self.inner.borrow_archetype(index)
    }
}

impl WorldInner {
    pub(crate) fn new() -> Self {
        Self {
            archetypes: Vec::new(),
            entities: Entities::new(),
            storage_lookup: StorageLookup::new(),
        }
    }

    pub(crate) fn reserve_entity(&self) -> Entity {
        self.entities.reserve_entity()
    }

    /// Remove an entity and all its components from the world.
    /// An error is returned if the entity does not exist.
    pub(crate) fn despawn(&mut self, entity: &Entity) -> Result<(), ()> {
        let entity_location = self.entities.free_entity(entity)?;
        if let Some(entity_location) = entity_location {
            let archetype = &mut self.archetypes[entity_location.archetype_index];
            let swapped_entity = archetype
                .entities
                .get_mut()
                .unwrap()
                .last()
                .map(|e| e.clone())
                .ok_or(())?;
            archetype.swap_remove(entity_location.index_within_archetype);

            if &swapped_entity != entity {
                *self.entities.get_at_index_mut(swapped_entity.index) = Some(entity_location);
            }

            // If this is a hierarchy node then child `Entity`s and their children should be removed.
            if let Some(hierarchy_node) = self
                .get_component_mut::<HierarchyNode>(entity)
                .map(|h| h.clone())
            {
                // Despawn all children and their siblings
                let mut current_child = hierarchy_node.last_child;
                while let Some(child) = current_child {
                    current_child = self
                        .get_component_mut::<HierarchyNode>(&child)
                        .map(|n| n.previous_sibling.clone())
                        .flatten();
                    self.despawn(entity)?;
                }
            }
        }
        Ok(())
    }

    pub(crate) fn remove_component<T: 'static>(&mut self, entity: &Entity) -> Option<T> {
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
                let mut i = 0;
                for c in self.archetypes[old_archetype_index].channels.iter() {
                    // Skip the channel we're removing.
                    if i != remove_channel_index {
                        new_archetype.push_channel(c.new_same_type());
                    }
                    i += 1;
                }

                let new_archetype_index = self.archetypes.len();
                self.archetypes.push(new_archetype);
                self.storage_lookup
                    .new_archetype(new_archetype_index, &type_ids);
                new_archetype_index
            }
        };

        let (old_archetype, new_archetype) = index_mut_twice(
            &mut self.archetypes,
            old_archetype_index,
            new_archetype_index,
        );

        // Migrate components from old archetype
        let mut i = 0;
        for c in new_archetype.channels.iter_mut() {
            if i != remove_channel_index {
                old_archetype.channels[i]
                    .component_channel
                    .migrate_component(
                        entity_location.index_within_archetype,
                        &mut *c.component_channel,
                    )
            }
            i += 1;
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
            // Update the location of the entity
            let location = self.entities.get_at_index_mut(entity.index);
            *location = Some(EntityLocation {
                archetype_index: new_archetype_index,
                index_within_archetype: new_archetype.entities.get_mut().unwrap().len(),
            });

            // Update the location of the swapped entity.
            let swapped_entity_location = self
                .entities
                .get_at_index_mut(swapped_entity_index)
                .as_mut()
                .unwrap();
            swapped_entity_location.index_within_archetype = entity_location.index_within_archetype;
        }

        old_archetype
            .entities
            .get_mut()
            .unwrap()
            .swap_remove(entity_location.index_within_archetype);
        new_archetype
            .entities
            .get_mut()
            .unwrap()
            .push(entity.clone());

        Some(
            old_archetype
                .get_channel_mut::<T>(remove_channel_index)
                .swap_remove(entity_location.index_within_archetype),
        )
    }

    /// This will return None if the `Entity` does not exist or the `Entity` does not have the component.
    pub(crate) fn get_component_mut<T: 'static>(&mut self, entity: &Entity) -> Option<&mut T> {
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

    pub(crate) fn add_component<T: ComponentTrait>(
        &mut self,
        entity: &Entity,
        component: T,
    ) -> Option<()> {
        let old_entity_location = self.entities.get_location(entity)?;

        let (type_ids, insert_position) = if let Some(old_entity_location) = old_entity_location {
            let mut type_ids = self.archetypes[old_entity_location.archetype_index].type_ids();
            let new_component_id = TypeId::of::<T>();
            let insert_position = match type_ids.binary_search(&new_component_id) {
                Ok(i) => {
                    // If the component already exists, simply replace it with the new one.
                    self.archetypes[old_entity_location.archetype_index]
                        .borrow_channel_mut::<T>(i)
                        .unwrap()[old_entity_location.index_within_archetype] = component;
                    return Some(());
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

        // Find or create a new archetype for this entity.
        let new_archetype_index = match self.storage_lookup.get_archetype_with_components(&type_ids)
        {
            Some(index) => index,
            None => {
                // Create a new archetype with one additional component.
                let mut new_archetype = Archetype::new();
                let mut i = 0;

                if let Some(old_entity_location) = old_entity_location {
                    for c in self.archetypes[old_entity_location.archetype_index]
                        .channels
                        .iter()
                    {
                        if i == insert_position {
                            new_archetype.push_channel(ComponentChannelStorage::new::<T>());
                        }
                        new_archetype.push_channel(c.new_same_type());
                        i += 1;
                    }
                }

                if i == insert_position {
                    new_archetype.push_channel(ComponentChannelStorage::new::<T>());
                }

                let new_archetype_index = self.archetypes.len();
                self.archetypes.push(new_archetype);
                self.storage_lookup
                    .new_archetype(new_archetype_index, &type_ids);

                new_archetype_index
            }
        };

        if let Some(old_entity_location) = old_entity_location {
            let (old_archetype, new_archetype) = index_mut_twice(
                &mut self.archetypes,
                old_entity_location.archetype_index,
                new_archetype_index,
            );
            // Migrate components from old archetype
            let mut new_channel_index = 0;

            for c in old_archetype.channels.iter_mut() {
                if new_channel_index == insert_position {
                    new_channel_index += 1;
                }

                c.component_channel.migrate_component(
                    old_entity_location.index_within_archetype,
                    &mut *new_archetype.channels[new_channel_index].component_channel,
                );
                new_channel_index += 1;
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
                // Update the location of the entity
                let location = self.entities.get_at_index_mut(entity.index);
                *location = Some(EntityLocation {
                    archetype_index: new_archetype_index,
                    index_within_archetype: new_archetype.entities.get_mut().unwrap().len(),
                });

                // Update the location of the swapped entity.
                let swapped_entity_location = self
                    .entities
                    .get_at_index_mut(swapped_entity_index)
                    .as_mut()
                    .unwrap();
                swapped_entity_location.index_within_archetype =
                    old_entity_location.index_within_archetype;
            }

            old_archetype
                .entities
                .get_mut()
                .unwrap()
                .swap_remove(old_entity_location.index_within_archetype);
        }

        let new_archetype = &mut self.archetypes[new_archetype_index];
        // Insert the new component
        new_archetype
            .get_channel_mut(insert_position)
            .push(component);

        new_archetype
            .entities
            .get_mut()
            .unwrap()
            .push(entity.clone());

        Some(())
    }
}
impl WorldPrivate for WorldInner {
    fn storage_lookup(&self) -> &StorageLookup {
        &self.storage_lookup
    }

    fn entities(&self) -> &Entities {
        &self.entities
    }

    fn borrow_archetype(&self, index: usize) -> &Archetype {
        &self.archetypes[index]
    }
}

pub trait WorldTrait: WorldPrivate {
    fn new() -> Self;
    fn clone_entity(&mut self, entity: &Entity) -> Option<Entity>;
    fn despawn(&mut self, entity: &Entity) -> Result<(), ()>;
    fn reserve_entity(&self) -> Entity;
    fn remove_component<T: 'static>(&mut self, entity: &Entity) -> Option<T>;
    fn get_component_mut<T: 'static>(&mut self, entity: &Entity) -> Option<&mut T>;
}

pub trait WorldPrivate {
    fn storage_lookup(&self) -> &StorageLookup;
    fn entities(&self) -> &Entities;
    fn borrow_archetype(&self, index: usize) -> &Archetype;
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
