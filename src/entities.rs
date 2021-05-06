use crate::{Entity, EntityLocation};
use std::sync::Mutex;

struct Inner {
    generation_and_location: Vec<(usize, Option<EntityLocation>)>,
    free_entity_indices: Vec<usize>,
}

impl Inner {
    pub fn new_handle(&mut self) -> Entity {
        if let Some(index) = self.free_entity_indices.pop() {
            let generation_and_location = &mut self.generation_and_location[index];
            // We don't need to increment the generation here
            // because it was already incremented when the previous entity was freed.
            Entity {
                generation: generation_and_location.0,
                index,
            }
        } else {
            let index = self.generation_and_location.len();
            self.generation_and_location.push((0, None));
            Entity {
                index,
                generation: 0,
            }
        }
    }
}

pub struct Entities {
    // This isn't great, but it's a quick way to make spawning entities thread safe.
    // However this might result in lots of contention, but for now it's probably fine.
    inner: Mutex<Inner>,
}

impl Entities {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Inner {
                generation_and_location: Vec::new(),
                free_entity_indices: Vec::new(),
            }),
        }
    }

    pub fn get_location(&self, entity: Entity) -> Option<Option<EntityLocation>> {
        let inner = self.inner.lock().unwrap();
        let (generation, location) = inner.generation_and_location.get(entity.index)?;
        if *generation == entity.generation {
            Some(*location)
        } else {
            None
        }
    }

    pub fn get_at_index_mut(&mut self, index: usize) -> &mut Option<EntityLocation> {
        &mut self.inner.get_mut().unwrap().generation_and_location[index].1
    }

    pub fn reserve_entity(&self) -> Entity {
        let mut inner = self.inner.lock().unwrap();
        inner.new_handle()
    }

    /// Returns a new Entity handle but does not yet initialize its location within the world.
    pub fn new_entity_handle(&mut self) -> Entity {
        let inner = self.inner.get_mut().unwrap();
        inner.new_handle()
    }

    /// Returns the location of the freed Entity if it exists
    pub fn free_entity(&mut self, entity: Entity) -> Result<Option<EntityLocation>, ()> {
        let inner = self.inner.get_mut().unwrap();
        let (generation, entity_location) = &mut inner.generation_and_location[entity.index];

        if *generation == entity.generation {
            *generation += 1;
            inner.free_entity_indices.push(entity.index);
            Ok(*entity_location)
        } else {
            Err(())
        }
    }
}
