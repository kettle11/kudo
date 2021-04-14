use crate::{Entity, EntityLocation};

pub(crate) struct Entities {
    pub(crate) generation_and_location: Vec<(usize, EntityLocation)>,
    free_entity_indices: Vec<usize>,
}

impl Entities {
    pub fn new() -> Self {
        Self {
            generation_and_location: Vec::new(),
            free_entity_indices: Vec::new(),
        }
    }

    pub fn get_location(&self, entity: Entity) -> Option<EntityLocation> {
        let (generation, location) = self.generation_and_location.get(entity.index)?;
        if *generation == entity.generation {
            Some(*location)
        } else {
            None
        }
    }

    pub fn set_entity_location(&mut self, entity: Entity, entity_location: EntityLocation) {
        self.generation_and_location[entity.index].1 = entity_location;
    }

    /// Returns a new Entity handle but does not yet initialize its location within the world.
    pub fn new_entity_handle(&mut self) -> Entity {
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
            self.generation_and_location.push((
                0,
                // This data will be overriden later.
                EntityLocation {
                    archetype_index: 0,
                    index_within_archetype: 0,
                },
            ));
            Entity {
                index,
                generation: 0,
            }
        }
    }

    /// Returns the location of the freed Entity if it exists
    pub fn free_entity(&mut self, entity: Entity) -> Result<EntityLocation, ()> {
        let (generation, entity_location) = &mut self.generation_and_location[entity.index];

        if *generation == entity.generation {
            *generation += 1;
            self.free_entity_indices.push(entity.index);
            Err(())
        } else {
            Ok(*entity_location)
        }
    }
}
