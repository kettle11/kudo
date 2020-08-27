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
        for (_, archetype) in self.archetypes.iter().enumerate() {
            let matches = archetype
                .matches_components(&self.temp_component_types, &mut archetype_indices_out);

            println!("Matches: {:?} {:?}", matches, archetype_indices_out);
        }
    }

    fn add_archetype(&mut self, archetype_id: ArchetypeId, archetype: Archetype) -> usize {
        let archetype_index = self.archetypes.len();
        // Keep track of where components are stored in archetypes.
        for (i, component_id) in archetype.type_ids.iter().enumerate() {
            if let Some(s) = self.component_id_to_archetypes.get_mut(&component_id) {
                s.push((archetype_index, i));
            } else {
                let v = vec![(archetype_index, i)];
                self.component_id_to_archetypes.insert(*component_id, v);
            }
        }

        self.archetypes.push(archetype);
        self.archetype_id_to_index
            .insert(archetype_id, archetype_index);

        archetype_index
    }

    pub fn spawn<B: ComponentBundle>(&mut self, component_bundle: B) -> EntityHandle {
        self.get_component_types::<B>();
        let archetype_id = archetype_id(&self.temp_component_types);

        let (archetype_index, archetype) =
            if let Some(index) = self.archetype_id_to_index.get(&archetype_id).copied() {
                (index, &mut self.archetypes[index])
            } else {
                // Create a new archetype
                let archetype = B::new_archetype();
                let archetype_index = self.add_archetype(archetype_id, archetype);
                (archetype_index, &mut self.archetypes[archetype_index])
            };

        let entity_id = self.entities.len();
        let index_in_archetype = component_bundle.push_to_archetype(
            entity_id,
            &self.temp_component_types_with_index,
            archetype,
        );

        self.entities.push((archetype_index, index_in_archetype));
        EntityHandle(entity_id)
    }

    pub fn move_component<T: 'static>(&mut self, from: EntityHandle, to: EntityHandle) {
        let t = self.remove_component::<T>(from);
        self.add_component(to, t);
    }

    pub fn remove_component<T: 'static>(&mut self, entity: EntityHandle) -> T {
        let type_id = TypeId::of::<T>();
        let (old_archetype_index, entity_index_in_archetype) = self.entities[entity.0];

        // Find the new archetype ID.
        self.temp_component_types.clear();
        let mut removing_component_position = 0;
        for (i, id) in self.archetypes[old_archetype_index]
            .type_ids
            .iter()
            .enumerate()
        {
            if id != &type_id {
                self.temp_component_types.push(*id);
            } else {
                removing_component_position = i;
            }
        }

        self.temp_component_types.sort();

        let new_archetype_id = archetype_id(&self.temp_component_types);

        // If the new archetype exists, push to that, otherwise create the new archetype.
        let new_archetype_index = if let Some(new_archetype_index) =
            self.archetype_id_to_index.get(&new_archetype_id).copied()
        {
            new_archetype_index
        } else {
            let new_archetype =
                self.archetypes[old_archetype_index].copy_structure_with_one_fewer(type_id);
            self.add_archetype(new_archetype_id, new_archetype)
        };
        self.migrate_entity(entity.0, old_archetype_index, new_archetype_index);
        self.archetypes[old_archetype_index]
            .remove_component(removing_component_position, entity_index_in_archetype)
    }

    pub fn add_component<T: 'static>(&mut self, entity: EntityHandle, new_component: T) {
        let type_id = TypeId::of::<T>();
        let (old_archetype_index, index_in_current_archetype) = self.entities[entity.0];
        let old_archetype = &mut self.archetypes[old_archetype_index];
        let new_component_position = old_archetype.type_ids.binary_search(&type_id);

        match new_component_position {
            // The component already exists, replace it
            Ok(position) => {
                old_archetype.replace_component(
                    position,
                    index_in_current_archetype,
                    new_component,
                );
            }
            // The component does not already exist, find or create a new archetype
            Err(new_component_position) => {
                // Find the new archetype ID
                self.temp_component_types.clear();
                self.temp_component_types
                    .extend(old_archetype.type_ids.iter());
                self.temp_component_types
                    .insert(new_component_position, type_id);
                let new_archetype_id = archetype_id(&self.temp_component_types);

                // Lookup or create a new archetype and then migrate data to it.
                let new_archetype_index = if let Some(new_archetype_index) =
                    self.archetype_id_to_index.get(&new_archetype_id).copied()
                {
                    new_archetype_index
                } else {
                    let additional_component_store = Box::new(Vec::<T>::new());
                    let new_archetype = old_archetype
                        .copy_structure_with_one_additional(type_id, additional_component_store);
                    self.add_archetype(new_archetype_id, new_archetype)
                };
                // Migrate the old component data to the new archetype
                self.migrate_entity(entity.0, old_archetype_index, new_archetype_index);
                // Push the new component data to the new archetype
                self.archetypes[new_archetype_index]
                    .push_component(new_component_position, new_component);
            }
        }
    }

    /// Migrates an entity's components from one archetype into another
    fn migrate_entity(
        &mut self,
        entity_index: usize,
        old_archetype_index: usize,
        new_archetype_index: usize,
    ) {
        let (old_archetype, new_archetype) = get_two_references(
            &mut self.archetypes,
            old_archetype_index,
            new_archetype_index,
        );

        let (_, entity_index_in_archetype) = self.entities[entity_index];
        let mut old_iter = old_archetype
            .type_ids
            .iter()
            .zip(old_archetype.components.iter_mut());
        let mut new_iter = new_archetype
            .type_ids
            .iter()
            .zip(new_archetype.components.iter_mut());

        let mut old_component = old_iter.next();
        let mut new_component = new_iter.next();

        while old_component.is_some() && new_component.is_some() {
            let (self_id, old_component_store) = old_component.as_mut().unwrap();
            let (new_id, new_component_store) = new_component.as_mut().unwrap();
            match self_id.cmp(&new_id) {
                Ordering::Less => old_component = old_iter.next(),
                Ordering::Greater => new_component = new_iter.next(),
                Ordering::Equal => {
                    old_component_store.migrate(entity_index_in_archetype, new_component_store);
                    old_component = old_iter.next();
                    new_component = new_iter.next();
                }
            }
        }

        new_archetype.entity_ids.push(entity_index);

        let world_entity_index = old_archetype
            .entity_ids
            .swap_remove(entity_index_in_archetype);
        self.entities[world_entity_index] = (new_archetype_index, new_archetype.len() - 1);

        // If an entity was swapped to the old index during the migration the world's reference to it must be updated.
        if let Some(entity_moved) = old_archetype.entity_ids.get(entity_index_in_archetype) {
            self.entities[*entity_moved] = (old_archetype_index, entity_index_in_archetype);
        }
    }

    fn query() {}
}

// Is there a way to not use this to mutably borrow twice from the same array?
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

pub type ArchetypeId = u64;

/// A unique identifier for a component.
/// Internally based on TypeId so it will change between Rust compiler versions.
pub type ComponentId = TypeId;

trait ComponentStore: Any {
    fn to_any(&self) -> &dyn Any;
    fn to_any_mut(&mut self) -> &mut dyn Any;
    fn migrate(&mut self, index: usize, other: &mut Box<dyn ComponentStore>);
    fn new_same_type(&self) -> Box<dyn ComponentStore>;
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
        let other = other.to_any_mut().downcast_mut::<Vec<T>>().unwrap();
        other.push(data);
    }

    fn new_same_type(&self) -> Box<dyn ComponentStore> {
        Box::new(Vec::<T>::new())
    }
}

/// Entities that share the same components share the same 'archetype'
/// Archetypes internally store Vecs of components
pub struct Archetype {
    entity_ids: Vec<usize>,
    // The dyn Any is always a ComponentStore
    type_ids: Vec<TypeId>,
    components: Vec<Box<dyn ComponentStore>>,
}

impl Archetype {
    /// Just the initialization of the archetype's vec
    /// The actual members are added later.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entity_ids: Vec::new(),
            components: Vec::with_capacity(capacity),
            type_ids: Vec::with_capacity(capacity),
        }
    }

    /// Copies the structure of this archetype but removes a component store.
    fn copy_structure_with_one_fewer(&self, skip: TypeId) -> Archetype {
        let mut new_archetype = Archetype::with_capacity(self.components.len());
        let types_and_components = self.type_ids.iter().zip(self.components.iter());
        for (type_id, component) in types_and_components {
            if *type_id != skip {
                new_archetype.components.push(component.new_same_type());
                new_archetype.type_ids.push(*type_id);
            }
        }
        new_archetype
    }

    /// Copies the structure of this archetype but adds an additional component store.
    fn copy_structure_with_one_additional(
        &self,
        additional_type_id: TypeId,
        additional_component_store: Box<dyn ComponentStore>,
    ) -> Archetype {
        let mut additional_component_store = Some(additional_component_store);
        let mut new_archetype = Archetype::with_capacity(self.components.len());
        let types_and_components = self.type_ids.iter().zip(self.components.iter());
        for (type_id, component) in types_and_components {
            if *type_id > additional_type_id {
                new_archetype
                    .components
                    .push(additional_component_store.take().unwrap());
                new_archetype.type_ids.push(additional_type_id);
            }
            new_archetype.components.push(component.new_same_type());
            new_archetype.type_ids.push(*type_id);
        }
        // If the new component store wasn't inserted, it's the last item so insert it here.
        if let Some(additional_component_store) = additional_component_store {
            new_archetype.components.push(additional_component_store);
            new_archetype.type_ids.push(additional_type_id);
        }
        new_archetype
    }

    fn get_component_store_mut<T: 'static>(&mut self, index: usize) -> &mut Vec<T> {
        self.components[index]
            .to_any_mut()
            .downcast_mut::<Vec<T>>()
            .unwrap()
    }

    fn push_component<T: 'static>(&mut self, component_index: usize, t: T) {
        self.get_component_store_mut(component_index).push(t);
    }

    fn remove_component<T: 'static>(&mut self, component_index: usize, entity_index: usize) -> T {
        self.get_component_store_mut(component_index)
            .swap_remove(entity_index)
    }

    fn len(&self) -> usize {
        self.entity_ids.len()
    }
    fn replace_component<T: 'static>(&mut self, component_index: usize, entity_index: usize, t: T) {
        self.get_component_store_mut(component_index)[entity_index] = t;
    }

    pub fn matches_components(
        &self,
        components: &[TypeId],
        archetype_indices_out: &mut [usize],
    ) -> bool {
        let mut component_iter = components.iter().enumerate();
        let mut component = component_iter.next();
        for (archetype_index, archetype_component_type) in self.type_ids.iter().enumerate() {
            if let Some((component_index, component_type)) = component {
                match archetype_component_type.partial_cmp(component_type) {
                    // Components are sorted, if we've passed a component
                    // it can no longer be found.
                    Some(Ordering::Greater) => return false,
                    Some(Ordering::Equal) => {
                        archetype_indices_out[component_index] = archetype_index;
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

trait Query {
    type Item;
}
trait QueryParameter {
    type Item;
}

impl<T> QueryParameter for &T {
    type Item = T;
}
impl<T> QueryParameter for &mut T {
    type Item = T;
}

macro_rules! query_impl {
    ($count: expr, $(($name: ident, $index: tt)),*) => {
        impl<$($name: QueryParameter),*> Query for ($($name,)*) {
            type Item = ($($name,) *);

        }
    };
}

query_impl! {1, (A, 0)}

/// A component bundle is a collection of types.
pub trait ComponentBundle {
    /// Retrieves sorted component ids
    fn component_ids(ids_out: &mut Vec<ComponentId>);
    fn new_archetype() -> Archetype;
    /// Add an instance of this component bundle to the archetype.
    fn push_to_archetype(
        self,
        entity_id: usize,
        component_order: &[(usize, ComponentId)],
        archetype: &mut Archetype,
    ) -> usize;
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

                // This is a little funky, but it's a way to sort the component stores by the TypeIds
                // and still store them separately.
                let mut component_stores: [(TypeId, Option<Box<dyn ComponentStore>>); $count] = [
                    $((TypeId::of::<$name>(), Some(Box::new(Vec::<$name>::new()))),) *
                ];
                component_stores.sort_by(|a, b| a.0.cmp(&b.0));

                for (t, c) in component_stores.iter_mut() {
                    archetype.components.push(c.take().unwrap());
                    archetype.type_ids.push(*t);
                }

                archetype
            }

            fn push_to_archetype(
                self,
                entity_id: usize,
                component_order: &[(usize, ComponentId)],
                archetype: &mut Archetype,
            ) -> usize {
                archetype.entity_ids.push(entity_id);
                $(archetype.push_component(component_order[$index].0, self.$index);)*
                archetype.len() - 1
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
