use std::{any::TypeId, collections::HashMap};

use crate::sparse_set::*;
use crate::{Requirement, RequirementType};

// This operates under the assumptipn that Archetypes are never deallocated.
pub struct StorageLookup {
    archetype_count: usize,
    component_info: HashMap<TypeId, ComponentInfo>,
}

pub struct ComponentArchetypeInfo {
    archetype_index: usize,
    channel_in_archetype: usize,
}

pub struct ComponentInfo {
    pub archetypes: SparseSet<ComponentArchetypeInfo>,
}

impl StorageLookup {
    pub fn new() -> Self {
        Self {
            archetype_count: 0,
            component_info: HashMap::new(),
        }
    }

    /// Declare a new archetype with the TypeIds.
    /// This assumes TypeIds are already sorted
    pub fn new_archetype(&mut self, archetype_index: usize, type_ids: &[TypeId]) {
        for (channel_in_archetype, type_id) in type_ids.iter().enumerate() {
            if !self.component_info.contains_key(type_id) {
                self.component_info.insert(
                    *type_id,
                    ComponentInfo {
                        archetypes: SparseSet::new(),
                    },
                );
            }

            let component_info = self.component_info.get_mut(&type_id).unwrap();
            component_info.archetypes.insert(
                archetype_index,
                ComponentArchetypeInfo {
                    archetype_index,
                    channel_in_archetype,
                },
            );
            self.archetype_count += 1;
        }
    }

    /// Find all archetypes that match the requirements.
    /// Type IDs do not need to be sorted.
    // This should be amended to take a separate list of filters
    // that can include `With` or `Without`.
    // The actual requirements create additional filters that must be checked.
    // So some filters are associated with a storage and some are not.
    // The filters are then sorted by how many archetypes they match, and they
    // are iterated in order.
    // It might be better to just rewrite the below function to achieve that goal.
    pub fn get_matching_archetypes<const SIZE: usize>(
        &self,
        requirements: &[Requirement; SIZE],
    ) -> Vec<usize> {
        #[derive(Clone, Copy)]
        struct TempComponentInfo<'a> {
            component_info: Option<&'a ComponentInfo>,
            requirement: Requirement,
        }

        let mut matching_archetypes = Vec::new();

        // If there were a way to collect into a fixed size array we wouldn't have to use an Option and a bunch of unwraps.
        // This stores the original index (before being sorted)
        let mut component_info: [TempComponentInfo; SIZE] = [TempComponentInfo {
            component_info: None,
            // We aren't actually using the TypeID of bool, it's replaced in the next step.
            requirement: Requirement::with_(0, TypeId::of::<bool>()),
        }; SIZE];
        for (requirement, component_info) in requirements.iter().zip(component_info.iter_mut()) {
            if let Some(info) = self.component_info.get(&requirement.type_id) {
                *component_info = TempComponentInfo {
                    component_info: Some(info),
                    requirement: *requirement,
                }
            } else {
                match requirement.requirement_type {
                    RequirementType::Without | RequirementType::Optional => {}
                    RequirementType::With => {}
                }
            }
        }

        // Sort so that we can iterate the requirements with the fewest matching archetypes first.
        component_info.sort_by_key(|f| match f.requirement.requirement_type {
            RequirementType::With => f.component_info.unwrap().archetypes.len(),
            RequirementType::Without => {
                if let Some(c) = f.component_info {
                    self.archetype_count - c.archetypes.len()
                } else {
                    usize::MAX
                }
            }
            RequirementType::Optional => usize::MAX,
        });

        fn check_further_matches(archetype: usize, component_info: &[TempComponentInfo]) -> bool {
            for v in component_info[1..].iter() {
                let matches = v
                    .component_info
                    .unwrap()
                    .archetypes
                    .get(archetype)
                    .is_some();

                match v.requirement.requirement_type {
                    RequirementType::With => {
                        if !matches {
                            return false;
                        }
                    }
                    RequirementType::Without => {
                        if matches {
                            return false;
                        }
                    }
                    RequirementType::Optional => {}
                }
            }
            true
        }
        match component_info[0].requirement.requirement_type {
            RequirementType::With => {
                // Iterate through the archetypes of the component with the fewest archetypes.
                // For each archetype check if the next component contain that archetype.
                for archetype in component_info[0]
                    .component_info
                    .unwrap()
                    .archetypes
                    .data()
                    .iter()
                {
                    if check_further_matches(archetype.archetype_index, &component_info[1..]) {
                        // This archetype matches!
                        matching_archetypes.push(archetype.archetype_index);
                    }
                }
            }
            RequirementType::Without => {
                let c = component_info[0].component_info.unwrap();
                for archetype in 0..self.archetype_count {
                    if c.archetypes.get(archetype).is_none()
                        && check_further_matches(archetype, &component_info[1..])
                    {
                        // This archetype matches!
                        matching_archetypes.push(archetype);
                    }
                }
            }
            // This is only possible if every Archetype matches!
            // This is most likely to happen if all queries are Optional.
            RequirementType::Optional => matching_archetypes.extend(0..self.archetype_count),
        }
        matching_archetypes
    }
}
