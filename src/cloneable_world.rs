use crate::*;
pub struct CloneableWorld {
    pub(crate) inner: WorldInner,
}

impl WorldTrait for CloneableWorld {
    fn new() -> Self {
        Self {
            inner: WorldInner::new(),
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
    pub fn spawn<CB: ComponentBundle + WorldClone>(&mut self, component_bundle: CB) -> Entity {
        component_bundle.spawn_in_world(&mut self.inner)
    }

    /// Adds this CloneableWorld to another world.
    pub fn add_to_world(self, entity: &Entity, world: impl WorldTrait) {
        // This needs to iterate through all of the `Entity`'s components and clone
        // them into the new world.
        todo!()
    }
}

/// Can be cloned between worlds.
/// This trait works similarly to `Clone`, but it is implemented in a way that
/// preserves `Entity` relationships when cloning into different worlds.
pub trait WorldClone {
    fn world_clone(&self) -> Self;
}

impl<T: Clone> WorldClone for T {
    fn world_clone(&self) -> Self {
        self.clone()
    }
}

impl WorldClone for Entity {
    fn world_clone(&self) -> Self {
        todo!()
    }
}

impl WorldPrivate for CloneableWorld {
    type Archetype = Archetype;
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
