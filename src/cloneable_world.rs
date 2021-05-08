use std::any::TypeId;

use crate::*;
pub struct CloneableWorld {
    pub(crate) inner: ArchetypeWorld<ComponentChannelStorageClone>,
}

impl WorldTrait for CloneableWorld {
    fn new() -> Self {
        Self {
            inner: ArchetypeWorld::new(),
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

    fn clone_entity(&mut self, entity: &Entity) -> Option<Entity> {
        // This implementation can avoid looking up if types can be cloned
        // because it can be assumed that they are.
        todo!()
    }
}

impl CloneableWorld {
    pub fn spawn<CB: ComponentBundleClone + WorldClone>(&mut self, component_bundle: CB) -> Entity {
        component_bundle.spawn_in_world_clone(&mut self.inner)
    }

    /// Adds this CloneableWorld to another world.
    pub fn add_to_world<WORLD: WorldTrait>(self, world: &mut WORLD)
    where
        WORLD::Archetype: From<Archetype<ComponentChannelStorageClone>>,
    {
        let old_entities = self.inner.entities.inner.lock().unwrap();

        // Temporarily take the new world's `Entities` structure.
        let new_entities = &mut Entities::new();
        std::mem::swap(world.entities_mut(), new_entities);
        let mut entity_migrator = EntityMigrator::new(&old_entities, new_entities);

        for mut archetype in self.inner.archetypes {
            let type_ids = archetype.type_ids();
            let new_entities: Vec<Entity> = archetype
                .entities
                .get_mut()
                .unwrap()
                .iter()
                .map(|entity| entity_migrator.get_new_entity(entity))
                .collect();

            if let Some(target_archetype) = world
                .storage_lookup()
                .get_archetype_with_components(&type_ids)
            {
                // Append the data in this archetype to the new world.
                todo!()
            } else {
                // Insert this archetype into the new world but modify its entities list.
                let new_archetype: WORLD::Archetype = archetype
                    .world_clone_with_entities(&mut entity_migrator, new_entities)
                    .into();
                let type_ids = new_archetype.type_ids();
                world.push_archetype(new_archetype, &type_ids);
            }
        }

        std::mem::swap(world.entities_mut(), new_entities);
    }

    /// Adds a component to an Entity
    /// If the Entity does not exist, this returns None.
    /// If a component of the same type is already attached to the Entity, the component will be replaced.s
    pub fn add_component<T: ComponentTrait + WorldClone>(
        &mut self,
        entity: &Entity,
        component: T,
    ) -> Option<()> {
        // `add_component` is split into two parts. The part here does a small amount of work.
        // This is structured this way so that `World` and `CloneableWorld` can have slightly different implementations.
        let type_id = TypeId::of::<T>();
        let add_info = self.inner.add_component_inner(entity, type_id).ok()?;
        let new_archetype = &mut self.inner.archetypes[add_info.archetype_index];
        if add_info.new_archetype {
            // If a new `Archetype` is constructed insert its new channel.
            new_archetype.channels.insert(
                add_info.channel_index,
                ComponentChannelStorageClone::new::<T>(),
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

impl Clone for ArchetypeWorld<ComponentChannelStorageClone> {
    fn clone(&self) -> Self {
        let mut do_nothing_entity_migrator = DoNothingEntityMigrator {};
        Self {
            storage_lookup: self.storage_lookup.clone(),
            entities: self.entities.clone(),
            archetypes: self
                .archetypes
                .iter()
                .map(|a| {
                    a.world_clone_with_entities(
                        &mut do_nothing_entity_migrator,
                        a.entities
                            .read()
                            .unwrap()
                            .iter()
                            .map(|e| e.clone())
                            .collect(),
                    )
                })
                .collect(),
        }
    }
}

impl Clone for CloneableWorld {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

pub(crate) struct DoNothingEntityMigrator {}

impl EntityMigratorTrait for DoNothingEntityMigrator {
    fn get_new_entity(&mut self, entity: &Entity) -> Entity {
        entity.clone()
    }
}

#[derive(Clone)]
struct TempEntityInfo {
    index: usize,
    generation: usize,
}

pub trait EntityMigratorTrait {
    fn get_new_entity(&mut self, entity: &Entity) -> Entity;
}

pub struct EntityMigrator<'a> {
    old_to_new: Vec<Option<TempEntityInfo>>,
    old_entities: &'a EntitiesInner,
    new_entities: &'a Entities,
}

impl<'a> EntityMigrator<'a> {
    fn new(old_entities: &'a EntitiesInner, new_entities: &'a Entities) -> Self {
        Self {
            old_to_new: vec![None; old_entities.generation_and_location.len()],
            old_entities,
            new_entities,
        }
    }
}

impl<'a> EntityMigratorTrait for EntityMigrator<'a> {
    fn get_new_entity(&mut self, entity: &Entity) -> Entity {
        if self.old_entities.generation_and_location[entity.index].0 != entity.generation {
            // Entities that no longer exist in the original world should be replaced with a
            // similarly invalid Entity in the new world.
            todo!()
        } else {
            let old_to_new = &mut self.old_to_new[entity.index];
            if let Some(old_to_new) = old_to_new {
                Entity {
                    index: old_to_new.index,
                    generation: old_to_new.generation,
                }
            } else {
                let entity = self.new_entities.reserve_entity();
                *old_to_new = Some(TempEntityInfo {
                    index: entity.index,
                    generation: entity.generation,
                });
                entity
            }
        }
    }
}

/// Can be cloned between worlds.
/// This trait works similarly to `Clone`, but it is implemented in a way that
/// preserves `Entity` relationships when cloning into different worlds.
pub trait WorldClone {
    fn world_clone(&self, entity_migrator: &mut impl EntityMigratorTrait) -> Self;
}

impl<T: Clone> WorldClone for T {
    fn world_clone(&self, _: &mut impl EntityMigratorTrait) -> Self {
        self.clone()
    }
}

impl WorldClone for Entity {
    fn world_clone(&self, entity_migrator: &mut impl EntityMigratorTrait) -> Self {
        entity_migrator.get_new_entity(self)
    }
}

impl WorldPrivate for CloneableWorld {
    type Archetype = Archetype<ComponentChannelStorageClone>;
    fn storage_lookup(&self) -> &StorageLookup {
        self.inner.storage_lookup()
    }

    fn entities(&self) -> &Entities {
        self.inner.entities()
    }

    fn entities_mut(&mut self) -> &mut Entities {
        self.inner.entities_mut()
    }

    fn borrow_archetype(&self, index: usize) -> &Self::Archetype {
        self.inner.borrow_archetype(index)
    }

    fn push_archetype(&mut self, archetype: Self::Archetype, type_ids: &[TypeId]) -> usize {
        self.inner.push_archetype(archetype, type_ids)
    }
}
