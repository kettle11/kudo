use std::any::TypeId;

use crate::{
    Archetype, AsSystemArg, ComponentBundle, ComponentChannelStorage, Entities, GetQueryDirect,
    GetQueryInfoTrait, InsertHandle, Query, QueryParameters, QueryTrait, StorageGraph,
};
pub struct World {
    pub(crate) archetypes: Vec<Archetype>,
    pub(crate) storage_graph: StorageGraph,
    pub(crate) entities: Entities,
}

#[derive(Clone, Copy, PartialEq)]
pub struct Entity {
    pub(crate) index: usize,
    pub(crate) generation: usize,
}

impl Entity {
    pub fn index(&self) -> usize {
        self.index
    }
}

#[derive(Clone, Copy)]
pub struct EntityLocation {
    pub(crate) archetype_index: usize,
    pub(crate) index_within_archetype: usize,
}

impl World {
    pub fn new() -> Self {
        Self {
            archetypes: Vec::new(),
            entities: Entities::new(),
            storage_graph: StorageGraph::new(),
        }
    }

    pub fn spawn<CB: ComponentBundle>(&mut self, component_bundle: CB) -> Entity {
        component_bundle.spawn_in_world(self)
    }

    /// Remove an entity and all its components from the world.
    /// An error is returned if the entity does not exist.
    pub fn despawn(&mut self, entity: Entity) -> Result<(), ()> {
        let entity_location = self.entities.free_entity(entity)?;
        let archetype = &mut self.archetypes[entity_location.archetype_index];
        let swapped_entity = *archetype.entities.get_mut().unwrap().last().ok_or(())?;
        archetype.swap_remove(entity_location.index_within_archetype);

        if swapped_entity != entity {
            self.entities
                .set_entity_location(swapped_entity, entity_location);
        }
        Ok(())
    }

    pub fn remove_component<T: 'static>(&mut self, entity: Entity) -> Option<T> {
        let entity_location = self.entities.get_location(entity)?;
        let old_archetype_index = entity_location.archetype_index;
        let mut type_ids = self.archetypes[old_archetype_index].type_ids();
        let removing_component_id = TypeId::of::<T>();

        let remove_channel_index = match type_ids.binary_search(&removing_component_id) {
            Ok(index) => index,
            Err(_) => None?, // Entity does not have this component
        };

        type_ids.remove(remove_channel_index);
        let new_archetype_index = match self.storage_graph.find_storage(&type_ids) {
            Ok(index) => {
                // Archetype already exists
                index
            }
            Err(insert_handle) => {
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

                self.insert_archetype(insert_handle, new_archetype)
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
            let location = &mut self.entities.generation_and_location[entity.index].1;
            location.archetype_index = new_archetype_index;
            location.index_within_archetype = new_archetype.entities.get_mut().unwrap().len();

            // Update the location of the swapped entity.
            let swapped_entity_location =
                &mut self.entities.generation_and_location[swapped_entity_index].1;
            swapped_entity_location.index_within_archetype = entity_location.index_within_archetype;
        }

        old_archetype
            .entities
            .get_mut()
            .unwrap()
            .swap_remove(entity_location.index_within_archetype);
        new_archetype.entities.get_mut().unwrap().push(entity);

        Some(
            old_archetype
                .get_channel_mut::<T>(remove_channel_index)
                .swap_remove(entity_location.index_within_archetype),
        )
    }

    pub fn add_component<T: 'static + Send + Sync>(
        &mut self,
        entity: Entity,
        component: T,
    ) -> Option<()> {
        let entity_location = self.entities.get_location(entity)?;
        let old_archetype_index = entity_location.archetype_index;
        let mut type_ids = self.archetypes[old_archetype_index].type_ids();
        let new_component_id = TypeId::of::<T>();
        let insert_position = match type_ids.binary_search(&new_component_id) {
            Ok(_) => None?, // Entity already has this component
            Err(position) => {
                type_ids.insert(position, new_component_id);
                position
            }
        };

        // Find or create a new archetype for this entity.
        let new_archetype_index = match self.storage_graph.find_storage(&type_ids) {
            Ok(index) => {
                // Archetype already exists
                index
            }
            Err(insert_handle) => {
                // Create a new archetype with one additional component.
                let mut new_archetype = Archetype::new();
                let mut i = 0;
                for c in self.archetypes[old_archetype_index].channels.iter() {
                    if i == insert_position {
                        new_archetype.push_channel(ComponentChannelStorage::new::<T>());
                    }
                    new_archetype.push_channel(c.new_same_type());
                    i += 1;
                }

                if i == insert_position {
                    new_archetype.push_channel(ComponentChannelStorage::new::<T>());
                }
                self.insert_archetype(insert_handle, new_archetype)
            }
        };

        let (old_archetype, new_archetype) = index_mut_twice(
            &mut self.archetypes,
            old_archetype_index,
            new_archetype_index,
        );

        // Insert the new component
        new_archetype
            .get_channel_mut(insert_position)
            .push(component);

        // Migrate components from old archetype
        let mut new_channel_index = 0;
        for c in old_archetype.channels.iter_mut() {
            if new_channel_index == insert_position {
                new_channel_index += 1;
            }

            c.component_channel.migrate_component(
                entity_location.index_within_archetype,
                &mut *new_archetype.channels[new_channel_index].component_channel,
            )
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
            let location = &mut self.entities.generation_and_location[entity.index].1;
            location.archetype_index = new_archetype_index;
            location.index_within_archetype = new_archetype.entities.get_mut().unwrap().len();

            // Update the location of the swapped entity.
            let swapped_entity_location =
                &mut self.entities.generation_and_location[swapped_entity_index].1;
            swapped_entity_location.index_within_archetype = entity_location.index_within_archetype;
        }

        old_archetype
            .entities
            .get_mut()
            .unwrap()
            .swap_remove(entity_location.index_within_archetype);
        new_archetype.entities.get_mut().unwrap().push(entity);

        Some(())
    }

    pub(crate) fn insert_archetype(
        &mut self,
        insert_handle: InsertHandle,
        archetype: Archetype,
    ) -> usize {
        let archetype_index = self.archetypes.len();
        self.archetypes.push(archetype);
        self.storage_graph
            .insert_storage(insert_handle, archetype_index);
        archetype_index
    }

    pub fn query<'world_borrow, T: QueryParameters + 'world_borrow>(
        &'world_borrow self,
    ) -> Result<
        <<Query<'world_borrow, T> as QueryTrait<'world_borrow>>::Result as GetQueryDirect>::Arg,
        (),
    >
    where
        Query<'world_borrow, T>: QueryTrait<'world_borrow>,
        <Query<'world_borrow, T> as QueryTrait<'world_borrow>>::Result: GetQueryDirect,
    {
        let query_info =
            <Query<'world_borrow, T> as GetQueryInfoTrait>::query_info(self).ok_or(())?;
        let result = <Query<'world_borrow, T>>::get_query(self, &query_info).ok_or(())?;
        Ok(result.get_query_direct())
    }

    pub fn get_component_mut<T: 'static>(&mut self, entity: Entity) -> Result<&mut T, ()> {
        let entity_location = self.entities.get_location(entity).ok_or(())?;
        let archetype = &mut self.archetypes[entity_location.archetype_index as usize];
        archetype
            .get_component_mut(entity_location.index_within_archetype)
            .map_err(|_| ())
    }
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

/*
#[test]
fn simple_spawn() {
    use crate::*;

    let mut world = World::new();
    world.spawn((1 as i32,));
}

#[test]
fn add_component0() {
    use crate::*;

    let mut world = World::new();
    let entity = world.spawn((1 as i32,));
    world.add_component(entity, true);
    let result = (|q: Query<(&i32, &bool)>| -> bool { *q.iter().next().unwrap().1 })
        .run(&world)
        .unwrap();

    world.add_component(entity, true);

    assert!(result == true);
}

#[test]
fn add_component1() {
    use crate::*;

    let mut world = World::new();
    let entity = world.spawn((true,));
    world.add_component(entity, 10 as i32);
    let result = (|q: Query<(&i32, &bool)>| -> bool { *q.iter().next().unwrap().1 })
        .run(&world)
        .unwrap();

    world.add_component(entity, true);

    assert!(result == true);
}

#[test]
fn remove_component0() {
    use crate::*;

    let mut world = World::new();
    let entity = world.spawn((1 as i32, true));
    assert!(world.remove_component::<bool>(entity) == Some(true));
}
*/
