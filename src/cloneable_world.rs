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
    fn despawn(&mut self, entity: Entity) -> Result<(), ()> {
        self.inner.despawn(entity)
    }

    fn remove_component<T: 'static>(&mut self, entity: Entity) -> Option<T> {
        self.inner.remove_component(entity)
    }

    /// This will return None if the `Entity` does not exist or the `Entity` does not have the component.
    fn get_component_mut<T: 'static>(&mut self, entity: Entity) -> Option<&mut T> {
        self.inner.get_component_mut(entity)
    }

    fn clone_entity(&mut self, entity: Entity) -> Option<Entity> {
        // This implementation can avoid looking up if types are clone
        // because it can be assumed that they are.
        todo!()
    }
}

impl CloneableWorld {
    pub fn spawn<CB: ComponentBundle + Clone>(&mut self, component_bundle: CB) -> Entity {
        component_bundle.spawn_in_world(&mut self.inner)
    }
}

impl WorldPrivate for CloneableWorld {
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
