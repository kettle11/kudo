use std::{any::TypeId, collections::HashMap};

use crate::sparse_set::*;

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

#[derive(Clone, Copy, Debug)]
pub enum FilterType {
    With,
    Without,
    Optional,
}
#[derive(Clone, Copy, Debug)]
pub struct Filter {
    pub filter_type: FilterType,
    pub type_id: TypeId,
}

#[derive(Clone, Debug)]
pub struct ArchetypeMatch<const CHANNEL_COUNT: usize> {
    pub archetype_index: usize,
    // A channel will be None if this archetype does not contain an optional channel.
    pub channels: [Option<usize>; CHANNEL_COUNT],
    // Used for scheduling
    pub resource_indices: [Option<usize>; CHANNEL_COUNT],
}

/*
/// Which archetypes match this query
pub struct MatchInfo<const CHANNEL_COUNT: usize> {
    archetypes: Vec<ArchetypeMatch>
}
*/

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

    pub fn get_matching_archetypes<const REQUIREMENT_COUNT: usize>(
        &self,
        requirements: &[Filter; REQUIREMENT_COUNT],
        filters: &[Filter],
    ) -> Vec<ArchetypeMatch<REQUIREMENT_COUNT>> {
        #[derive(Clone, Copy)]
        struct InnerFilter<'a> {
            requirement_index: Option<usize>,
            component_info: Option<&'a ComponentInfo>,
            filter: Filter,
        }

        let mut inner_filters: Vec<InnerFilter> =
            Vec::with_capacity(REQUIREMENT_COUNT + filters.len());

        for (i, filter) in requirements.iter().enumerate() {
            let component_info = self.component_info.get(&filter.type_id);
            inner_filters.push(InnerFilter {
                requirement_index: Some(i),
                component_info,
                filter: Filter {
                    type_id: filter.type_id,
                    filter_type: filter.filter_type,
                },
            });

            // If we don't have this component nothing can match the query.
            match filter.filter_type {
                FilterType::With => {
                    if component_info.is_none() {
                        return Vec::new();
                    }
                }
                _ => {}
            }
        }

        // Identical to the above, but without a requirement_index.
        for filter in filters.iter() {
            let component_info = self.component_info.get(&filter.type_id);
            inner_filters.push(InnerFilter {
                requirement_index: None,
                component_info,
                filter: Filter {
                    type_id: filter.type_id,
                    filter_type: filter.filter_type,
                },
            });

            // If we don't have this component nothing can match the query.
            match filter.filter_type {
                FilterType::With => {
                    if component_info.is_none() {
                        return Vec::new();
                    }
                }
                _ => {}
            }
        }

        inner_filters.sort_by_key(|filter| match filter.filter.filter_type {
            FilterType::With => filter.component_info.unwrap().archetypes.len(),
            FilterType::Optional => usize::MAX,
            FilterType::Without => {
                if let Some(c) = filter.component_info {
                    self.archetype_count - c.archetypes.len()
                } else {
                    usize::MAX
                }
            }
        });

        fn check_further_matches<const REQUIREMENT_COUNT: usize>(
            archetype: usize,
            inner_filters: &[InnerFilter],
            archetype_match: &mut ArchetypeMatch<REQUIREMENT_COUNT>,
        ) -> bool {
            for inner_filter in inner_filters.iter() {
                let matches = inner_filter
                    .component_info
                    .unwrap()
                    .archetypes
                    .get(archetype);

                match inner_filter.filter.filter_type {
                    FilterType::With => {
                        if !matches.is_some() {
                            return false;
                        }
                        if let Some(component_archetype_info) = matches {
                            if let Some(requirement_index) = inner_filter.requirement_index {
                                archetype_match.channels[requirement_index] =
                                    Some(component_archetype_info.channel_in_archetype);
                            }
                        }
                    }
                    FilterType::Optional => {
                        if let Some(component_archetype_info) = matches {
                            if let Some(requirement_index) = inner_filter.requirement_index {
                                archetype_match.channels[requirement_index] = if matches.is_some() {
                                    Some(component_archetype_info.channel_in_archetype)
                                } else {
                                    None
                                };
                            }
                        }
                    }
                    FilterType::Without => {
                        if matches.is_some() {
                            return false;
                        }
                    }
                }
            }
            true
        }

        let mut archetype_match = ArchetypeMatch {
            archetype_index: 0,
            channels: [None; REQUIREMENT_COUNT],
            resource_indices: [None; REQUIREMENT_COUNT],
        };

        let mut archetype_matches: Vec<ArchetypeMatch<REQUIREMENT_COUNT>> = Vec::new();

        let (first_filter, tail_filters) = inner_filters.split_first().unwrap();
        match first_filter.filter.filter_type {
            FilterType::With => {
                // Iterate through the archetypes of the component with the fewest archetypes.
                // For each archetype check if the next component contain that archetype.
                for component_archetype_info in first_filter
                    .component_info
                    .unwrap()
                    .archetypes
                    .data()
                    .iter()
                {
                    // Reset the data so it's not used in multiple matches.
                    archetype_match.channels = [None; REQUIREMENT_COUNT];
                    archetype_match.resource_indices = [None; REQUIREMENT_COUNT];

                    archetype_match.archetype_index = component_archetype_info.archetype_index;
                    if let Some(requirement_index) = first_filter.requirement_index {
                        archetype_match.channels[requirement_index] =
                            Some(component_archetype_info.channel_in_archetype)
                    }

                    if check_further_matches(
                        component_archetype_info.archetype_index,
                        tail_filters,
                        &mut archetype_match,
                    ) {
                        // This archetype matches!
                        archetype_matches.push(archetype_match.clone());
                    }
                }
            }
            FilterType::Without => {
                todo!()
            }
            // This is only possible if every Archetype matches!
            // This is most likely to happen if all queries are Optional.
            FilterType::Optional => todo!(),
        }
        archetype_matches
    }
}
