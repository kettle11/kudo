use std::any::Any;
use std::any::TypeId;
use std::cmp::Ordering;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

#[derive(Copy, Clone)]
pub struct EntityHandle(usize);

/// A bunch of data that can be queried together.
pub struct World {
    archetypes: Vec<Archetype>,
    archetype_id_to_index: HashMap<ArchetypeId, usize>,
    component_id_to_archetypes: HashMap<ComponentId, Vec<(usize, usize)>>,
    /// Used to provide a temporary location to store and sort ComponentIds
    temp_component_types: Vec<ComponentId>,
    /// Also a temporary used to sort and find where the
    temp_component_types_with_index: Vec<(usize, ComponentId)>,
    // Archetype index and then index within that archetype
    entities: Vec<(usize, usize)>,
}

impl World {
    pub fn new() -> Self {
        Self {
            archetypes: Vec::new(),
            archetype_id_to_index: HashMap::new(),
            component_id_to_archetypes: HashMap::new(),
            temp_component_types: Vec::with_capacity(8),
            temp_component_types_with_index: Vec::with_capacity(8),
            entities: Vec::new(),
        }
    }

    fn get_component_types<B: ComponentBundle>(&mut self) {
        self.temp_component_types.clear();
        B::component_ids(&mut self.temp_component_types);

        // This is a bit messy, but the order the components are sorted into must be known.
        // So what we do here is sort a vec of tuples of the ComponentId and its index by the ComponentId.
        // This is used to insert components into the correct ComponentStore of the archetype.
        self.temp_component_types_with_index.clear();
        // This is extended to avoid an additional allocation.
        self.temp_component_types_with_index
            .extend(self.temp_component_types.iter().copied().enumerate());

        self.temp_component_types_with_index
            .sort_by(|a, b| a.1.cmp(&b.1));

        self.temp_component_types.sort();
    }

    pub fn find_matching_archetypes<B: ComponentBundle>(&mut self) {
        self.get_component_types::<B>();
        let mut archetype_indices_out = vec![0; self.temp_component_types.len()];
        for (i, archetype) in self.archetypes.iter().enumerate() {
            let matches = archetype
                .matches_components(&self.temp_component_types, &mut archetype_indices_out);

            println!("Matches: {:?} {:?}", matches, archetype_indices_out);
        }
    }

    pub fn spawn<B: ComponentBundle>(&mut self, component_bundle: B) -> EntityHandle {
        self.get_component_types::<B>();

        let archetype_id = archetype_id(&self.temp_component_types);

        /*
        println!(
            "TEMP COMPONENT TYPES: {:?} ARCHETYPE ID: {:?}",
            self.temp_component_types, archetype_id
        );
        */
        let (archetype_index, archetype) =
            if let Some(index) = self.archetype_id_to_index.get(&archetype_id).copied() {
                (index, &mut self.archetypes[index])
            } else {
                // Create a new archetype
                let archetype_index = self.archetypes.len();
                let archetype = B::new_archetype();
                self.archetypes.push(archetype);
                self.archetype_id_to_index
                    .insert(archetype_id, archetype_index);

                // Keep track of where components are stored in archetypes.
                for (i, component_id) in self.temp_component_types.iter().enumerate() {
                    if let Some(s) = self.component_id_to_archetypes.get_mut(&component_id) {
                        s.push((archetype_index, i));
                    } else {
                        let v = vec![(archetype_index, i)];
                        self.component_id_to_archetypes.insert(*component_id, v);
                    }
                }
                (archetype_index, &mut self.archetypes[archetype_index])
            };

        let index_in_archetype =
            component_bundle.push_to_archetype(&self.temp_component_types_with_index, archetype);
        self.entities.push((archetype_index, index_in_archetype));
        EntityHandle(self.entities.len() - 1)
    }

    pub fn remove_component<T: 'static>(&mut self, entity: EntityHandle) {
        let type_id = TypeId::of::<T>();
        let (archetype_index, index_in_archetype) = self.entities[entity.0];

        // Find the archetype ID
        self.temp_component_types.clear();
        for (i, _) in self.archetypes[archetype_index].components.iter() {
            if i != &type_id {
                self.temp_component_types.push(*i);
            }
        }
        let new_archetype_id = archetype_id(&self.temp_component_types);

        if let Some(other_archetype_index) =
            self.archetype_id_to_index.get(&new_archetype_id).copied()
        {
            let (current_archetype, new_archetype) =
                get_two_references(&mut self.archetypes, archetype_index, other_archetype_index);

            // Migrate data to the new archetype.
            // The ordering will be the same in the new archetype except with one fewer component.
            let mut index = 0;
            for (component_type_id, component_store) in current_archetype.components.iter_mut() {
                if component_type_id != &type_id {
                    component_store
                        .migrate(index_in_archetype, &mut new_archetype.components[index].1);
                    index += 1;
                }
            }
        } else {
            unimplemented!("Creating a new archetype while removing components isn't implemented")
        }
    }

    pub fn add_component<T: 'static>(&mut self, entity: EntityHandle, t: T) {
        let type_id = TypeId::of::<T>();
        let (archetype_index, index_in_archetype) = self.entities[entity.0];
        let archetype = &self.archetypes[archetype_index];

        // Find if this component matches a component already attached to this archetype.
        let existing_component = archetype
            .components
            .iter()
            .position(|(store_type_id, _)| store_type_id == &type_id);

        if let Some(c) = existing_component {
            self.archetypes[archetype_index].replace_component(c, index_in_archetype, t);
        } else {
            // Find the archetype ID
            self.temp_component_types.clear();
            self.temp_component_types.extend(
                self.archetypes[archetype_index]
                    .components
                    .iter()
                    .map(|(i, _)| i),
            );
            self.temp_component_types.push(type_id);
            self.temp_component_types.sort();
            let archetype_id = archetype_id(&self.temp_component_types);

            /*
            println!(
                "TEMP COMPONENT TYPES: {:?} ARCHETYPE ID: {:?}",
                self.temp_component_types, archetype_id
            );
            */

            // Lookup or create a new archetype and then migrate data to it.
            if let Some(other_archetype_index) =
                self.archetype_id_to_index.get(&archetype_id).copied()
            {
                let (current_archetype, new_archetype) = get_two_references(
                    &mut self.archetypes,
                    archetype_index,
                    other_archetype_index,
                );

                // Migrate data to the new archetype.
                // The ordering will be the same in the new archetype except for one additional component.
                // When we pass that component offset insertion into the new archetype by 1.
                let mut index = 0;
                let mut new_data_index = 0;
                for (_, component_store) in current_archetype.components.iter_mut() {
                    if new_archetype.components[index].0 == type_id {
                        new_data_index = index;
                        index += 1
                    } else {
                        component_store
                            .migrate(index_in_archetype, &mut new_archetype.components[index].1);
                    }
                }
                new_archetype.push_component(new_data_index, t);
            } else {
                unimplemented!("Creating a new archetype while adding components isn't implemented")
            }
        }
    }
}

// Is there a way to remove this?
#[inline]
fn get_two_references<'a, T>(
    data: &'a mut [T],
    first_index: usize,
    second_index: usize,
) -> (&'a mut T, &'a mut T) {
    if first_index < second_index {
        let (left, right) = data.split_at_mut(second_index);
        (&mut left[first_index], &mut right[0])
    } else {
        let (left, right) = data.split_at_mut(first_index);
        (&mut right[0], &mut left[second_index])
    }
}

/// Calculates an archetype id from the ids produced from a ComponentBundle
fn archetype_id(component_ids: &[ComponentId]) -> ArchetypeId {
    let mut s = DefaultHasher::new();
    component_ids.hash(&mut s);
    s.finish()
}

/// A component bundle is a collection of types.
pub trait ComponentBundle {
    /// Retrieves sorted component ids
    fn component_ids(ids_out: &mut Vec<ComponentId>);
    fn new_archetype() -> Archetype;
    /// Add an instance of this component bundle to the archetype.
    fn push_to_archetype(
        self,
        component_order: &[(usize, ComponentId)],
        archetype: &mut Archetype,
    ) -> usize;
}

pub type ArchetypeId = u64;

/// A unique identifier for a component.
/// Internally based on TypeId so it will change between Rust compiler versions.
pub type ComponentId = TypeId;

trait ComponentStore: Any {
    fn to_any(&self) -> &dyn Any;
    fn to_any_mut(&mut self) -> &mut dyn Any;
    fn migrate(&mut self, index: usize, other: &mut Box<dyn ComponentStore>);
}

impl<T: 'static> ComponentStore for Vec<T> {
    fn to_any(&self) -> &dyn Any {
        self
    }
    fn to_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    // This is dynamic dispatch and includes a downcast.
    // Is there a way to change that?
    // Perhaps with unsafe the size could be known and the bytes could just be copied
    // directly.
    fn migrate(&mut self, index: usize, other: &mut Box<dyn ComponentStore>) {
        let data = self.swap_remove(index);
        other
            .to_any_mut()
            .downcast_mut::<Vec<T>>()
            .unwrap()
            .push(data);
    }
}

/// Entities that share the same components share the same 'archetype'
/// Archetypes internally store Vecs of components
pub struct Archetype {
    size: usize,
    // The dyn Any is always a ComponentStore
    components: Vec<(TypeId, Box<dyn ComponentStore>)>,
}

impl Archetype {
    /// Just the initialization of the archetype's vec
    /// The actual members are added later.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            components: Vec::with_capacity(capacity),
            size: 0,
        }
    }

    fn get_component_store_mut<T: 'static>(&mut self, index: usize) -> &mut Vec<T> {
        self.components[index]
            .1
            .to_any_mut()
            .downcast_mut::<Vec<T>>()
            .unwrap()
    }

    pub fn new_component_store<T: 'static>(&mut self) {
        let v = Vec::<T>::new();
        self.components.push((TypeId::of::<T>(), Box::new(v)));
    }

    pub fn push_component<T: 'static>(&mut self, component_index: usize, t: T) {
        self.get_component_store_mut(component_index).push(t);
    }

    pub fn replace_component<T: 'static>(
        &mut self,
        component_index: usize,
        entity_index: usize,
        t: T,
    ) {
        self.get_component_store_mut(component_index)[entity_index] = t;
    }

    pub fn migrate(&mut self, indices: &[usize], other: &mut Archetype) {
        for (i, (_, component)) in self.components.iter_mut().enumerate() {
            component.migrate(i, &mut other.components[indices[i]].1);
        }
    }

    pub fn matches_components(
        &self,
        components: &[TypeId],
        archetyped_indices_out: &mut [usize],
    ) -> bool {
        let mut component_iter = components.iter().enumerate();
        let mut component = component_iter.next();
        for (archetype_index, (archetype_component_type, _)) in self.components.iter().enumerate() {
            if let Some((component_index, component_type)) = component {
                match archetype_component_type.partial_cmp(component_type) {
                    // Components are sorted, if we've passed a component
                    // it can no longer be found.
                    Some(Ordering::Greater) => return false,
                    Some(Ordering::Equal) => {
                        archetyped_indices_out[component_index] = archetype_index;
                        component = component_iter.next();
                    }
                    _ => {}
                }
            } else {
                return true;
            }
        }
        component.is_none()
    }
}

/// It feels like these implementations shouldn't necessarily be made public.
/// All logic that must vary between bundle types must be implemented per bundle type.
macro_rules! component_bundle_impl {
    ($count: expr, $(($name: ident, $index: tt)),*) => {
        impl< $($name: 'static),*> ComponentBundle for ($($name,)*) {

            fn component_ids(ids_out: &mut Vec<ComponentId>) {
                ids_out.extend_from_slice(&[
                    $(TypeId::of::<$name>()), *
                ]);
            }

            fn new_archetype() -> Archetype {
                let mut archetype = Archetype::with_capacity($count);
                $(archetype.new_component_store::<$name>();) *
                archetype.components.sort_by(|a, b| a.0.cmp(&b.0));
                archetype
            }

            fn push_to_archetype(
                self,
                component_order: &[(usize, ComponentId)],
                archetype: &mut Archetype,
            ) -> usize {
                $(archetype.push_component(component_order[$index].0, self.$index);)*
                archetype.size += 1;
                archetype.size - 1
            }
        }
    };
}

// Implement ComponentBundle for a bunch of different sizes of tuples.
component_bundle_impl! {1, (A, 0)}
component_bundle_impl! {2, (A, 0), (B, 1)}
component_bundle_impl! {3, (A, 0), (B, 1), (C, 2)}
component_bundle_impl! {4, (A, 0), (B, 1), (C, 2), (D, 3)}
component_bundle_impl! {5, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4)}
component_bundle_impl! {6, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5)}
component_bundle_impl! {7, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6)}
component_bundle_impl! {8, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6), (H, 7)}
component_bundle_impl! {9, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6), (H, 7), (I, 8)}
