use core::cell::UnsafeCell;
use std::any::{Any, TypeId};
use std::cmp::Ordering;
use std::collections::{hash_map::DefaultHasher, HashMap};
use std::hash::{Hash, Hasher};

pub struct Archetype {
    // Sorted type IDs
    type_ids: Vec<TypeId>,
    // The dyn Any is a Vec of T where T is this archetype's component type
    components: Vec<ComponentStore>,
    entity_indices: Vec<usize>,
    length: usize,
}

trait ComponentStoreInner {
    fn to_any(&self) -> &dyn Any;
    fn to_any_mut(&mut self) -> &mut dyn Any;
    fn remove(&mut self, index: usize);
}

impl<T: 'static> ComponentStoreInner for Vec<T> {
    fn to_any(&self) -> &dyn Any {
        self
    }
    fn to_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn remove(&mut self, index: usize) {
        self.swap_remove(index);
    }
}

pub struct ComponentStore(UnsafeCell<Box<dyn ComponentStoreInner>>);

impl ComponentStore {
    pub fn new<T: 'static>() -> Self {
        Self(UnsafeCell::new(Box::new(Vec::<T>::new())))
    }
}
impl Archetype {
    unsafe fn add_component<T: 'static>(&mut self, index: usize, t: T) {
        (*self.components[index].0.get())
            .to_any_mut()
            .downcast_mut::<Vec<T>>()
            .unwrap()
            .push(t);
    }
}

pub struct EntityLocation {
    #[allow(dead_code)]
    generation: u32,
    #[allow(dead_code)]
    archetype: u32,
    #[allow(dead_code)]
    index_in_archetype: u32,
}

/// A unique per entity identifier.
#[derive(Clone, Copy)]
pub struct Entity {
    index: u32,
    generation: u32,
}

pub struct World {
    archetypes: Vec<Archetype>,
    archetype_id_to_index: HashMap<u64, usize>,
    entities: Vec<EntityLocation>,
    free_entity_ids: Vec<u32>,
}

#[derive(Debug)]
pub enum EntityError {
    AlreadyRemoved,
}

impl World {
    pub fn new() -> Self {
        Self {
            archetypes: Vec::new(),
            archetype_id_to_index: HashMap::new(),
            entities: Vec::new(),
            free_entity_ids: Vec::new(),
        }
    }

    pub fn remove_entity(&mut self, entity: Entity) -> Result<(), EntityError> {
        let location = &self
            .entities
            .get(entity.index as usize)
            .ok_or(EntityError::AlreadyRemoved)?;

        // If the generations do not match then this entity has already been removed
        if location.generation == entity.generation {
            let archetype = &mut self.archetypes[location.archetype as usize];

            // Remove the entity from each of the archetype's components.
            for component_store in archetype.components.iter_mut() {
                unsafe {
                    (*component_store.0.get()).remove(location.index_in_archetype as usize);
                }
            }

            // When each ComponentStore removes a component it swaps the last item in the store to the
            // removed index. The world's "index_in_archetype" for the entity must be updated to reflect that.
            let swapped_entity = *archetype.entity_indices.last().unwrap();
            archetype
                .entity_indices
                .swap_remove(location.index_in_archetype as usize);
            self.entities[swapped_entity].index_in_archetype = location.index_in_archetype;

            // Increment the generation to avoid a double remove being OK.
            self.entities[entity.index as usize].generation += 1;

            self.free_entity_ids.push(entity.index);
            Ok(())
        } else {
            Err(EntityError::AlreadyRemoved)
        }
    }

    pub fn add_component<T>(&mut self, entity: Entity, t: T) {
        unimplemented!()
    }

    pub fn remove_component<T>(&mut self, entity: Entity, t: T) {
        unimplemented!()
    }

    /// This is unsafe for now because queries can overlap
    pub unsafe fn query<'a, Q: QueryParams<'a>>(&'a self) -> Q::Query {
        let mut id_and_index: Vec<(usize, TypeId)> =
            Q::type_ids_unsorted().iter().copied().enumerate().collect();
        id_and_index.sort_unstable_by(|a, b| a.1.cmp(&b.1));

        // This isn't a great approach. Multiple Vecs are allocated here.
        // Total the 'query' function allocates at least 4 Vecs.
        let type_ids: Vec<TypeId> = id_and_index.iter().map(|(_, id)| id).copied().collect();
        let type_order: Vec<usize> = id_and_index.iter().map(|(i, _)| i).copied().collect();

        fn matches_archetype(
            query_type_ids: &[TypeId],
            archetype_type_ids1: &[TypeId],
            type_order: &[usize],
            indices_out: &mut [usize],
        ) -> bool {
            let mut query_ids = query_type_ids.iter().enumerate();
            let mut query_index_and_id = query_ids.next();
            // Look through archetype components until every component has been matched.
            for (archetype_index, archetype_id) in archetype_type_ids1.iter().enumerate() {
                if let Some((query_index, query_id)) = query_index_and_id {
                    match archetype_id.partial_cmp(query_id) {
                        // Components are sorted, if we've passed a component
                        // it can no longer be found.
                        Some(Ordering::Greater) => return false,
                        Some(Ordering::Equal) => {
                            indices_out[type_order[query_index]] = archetype_index;
                            query_index_and_id = query_ids.next();
                        }
                        _ => {}
                    }
                }
            }
            query_index_and_id.is_none()
        }

        let mut query = Q::Query::new();
        let mut locations = vec![0; type_ids.len()];
        for a in self.archetypes.iter() {
            let matches = matches_archetype(&type_ids, &a.type_ids, &type_order, &mut locations);
            println!("MATCHES: {:?}", matches);
            if matches {
                // The ordering of the type IDs must be found.
                // The type ids used for matching are ordered.
                // So here they can be incorrect.
                println!("locations: {:?}", locations);
                query.add_archetype(&a, &locations);
            }
        }
        query
    }

    pub fn spawn<B: ComponentBundle>(&mut self, b: B) -> Entity {
        let archetype_id = B::archetype_id();
        let archetype_index = self
            .archetype_id_to_index
            .get(&archetype_id)
            .copied()
            .unwrap_or_else(|| {
                let archetype_index = self.archetypes.len();
                self.archetypes.push(B::new_archetype());
                self.archetype_id_to_index
                    .insert(archetype_id, archetype_index);
                archetype_index
            });

        let index_in_archetype = self.archetypes[archetype_index].length as u32;

        let free_id = self.free_entity_ids.pop();
        let entity = if let Some(index) = free_id {
            let generation = self.entities[index as usize].generation + 1;
            self.entities[index as usize] = EntityLocation {
                archetype: archetype_index as u32,
                generation,
                index_in_archetype,
            };
            Entity { index, generation }
        } else {
            self.entities.push(EntityLocation {
                archetype: archetype_index as u32,
                generation: 0,
                index_in_archetype,
            });
            Entity {
                index: (self.entities.len() - 1) as u32,
                generation: 0,
            }
        };

        let archetype = &mut self.archetypes[archetype_index];
        b.add_to_archetype(entity.index as usize, archetype);
        entity
    }
}

pub trait ComponentQueryTrait<'a> {
    type Iterator: Iterator;
    fn new() -> Self;
    fn iterator(&'a mut self) -> Self::Iterator;
    fn add_component_store(&mut self, component_store: &ComponentStore);
}

pub struct ComponentQuery<'a, T> {
    components: Vec<&'a Box<dyn ComponentStoreInner>>,
    phantom: std::marker::PhantomData<T>,
}

impl<'a, 'b: 'a, T: 'static> ComponentQueryTrait<'a> for ComponentQuery<'b, T> {
    type Iterator = ChainedIterator<std::slice::Iter<'a, T>>;
    fn new() -> Self {
        Self {
            components: Vec::new(),
            phantom: std::marker::PhantomData,
        }
    }

    fn iterator(&mut self) -> Self::Iterator {
        ChainedIterator::new(if self.components.len() > 0 {
            self.components
                .iter()
                .map(|i| i.to_any().downcast_ref::<Vec<T>>().unwrap().iter())
                .collect()
        } else {
            vec![[].iter()] // An empty iterator
        })
    }

    fn add_component_store(&mut self, component_store: &ComponentStore) {
        unsafe { self.components.push(&*component_store.0.get()) }
    }
}

pub struct MutableComponentQuery<'a, T> {
    components: Vec<&'a mut Box<dyn ComponentStoreInner>>,
    phantom: std::marker::PhantomData<T>,
}

impl<'a, 'b: 'a, T: 'static> ComponentQueryTrait<'a> for MutableComponentQuery<'b, T> {
    type Iterator = ChainedIterator<std::slice::IterMut<'a, T>>;
    fn new() -> Self {
        Self {
            components: Vec::new(),
            phantom: std::marker::PhantomData,
        }
    }

    fn iterator(&'a mut self) -> Self::Iterator {
        ChainedIterator::new(if self.components.len() > 0 {
            self.components
                .iter_mut()
                .map(|i| i.to_any_mut().downcast_mut::<Vec<T>>().unwrap().iter_mut())
                .collect()
        } else {
            vec![[].iter_mut()] // An empty iterator
        })
    }

    fn add_component_store(&mut self, component_store: &ComponentStore) {
        unsafe { self.components.push(&mut *component_store.0.get()) }
    }
}

pub trait QueryParam<'a> {
    type ComponentQuery: ComponentQueryTrait<'a>;
    fn type_id() -> TypeId;
}

// Is it OK to use the lifetime for T here?
impl<'a, T: 'static> QueryParam<'a> for &T {
    type ComponentQuery = ComponentQuery<'a, T>;
    fn type_id() -> TypeId {
        TypeId::of::<T>()
    }
}

// Is it OK to use the lifetime for T here?
impl<'a, T: 'static> QueryParam<'a> for &mut T {
    type ComponentQuery = MutableComponentQuery<'a, T>;
    fn type_id() -> TypeId {
        TypeId::of::<T>()
    }
}

pub trait QueryParams<'a> {
    type Query: Query<'a>;
    fn type_ids() -> Vec<TypeId>;
    fn type_ids_unsorted() -> Vec<TypeId>;
}

pub trait Query<'a> {
    type Iterator: Iterator;
    fn new() -> Self;
    fn add_archetype(&mut self, archetype: &Archetype, indices: &[usize]);
    fn iterator(&'a mut self) -> Self::Iterator;
}

pub trait ComponentBundle {
    fn archetype_id() -> u64;
    fn new_archetype() -> Archetype;
    fn add_to_archetype(self, index: usize, archetype: &mut Archetype);
    fn type_ids_and_order(ids_and_order: &mut [(usize, TypeId)]);
}

macro_rules! component_bundle_impl {
    ($count: expr, $(($name: ident, $index: tt)),*) => {
        impl<'a, $($name: ComponentQueryTrait<'a>),*> Query<'a> for ($($name,)*) {
            type Iterator = QueryIterator<($($name::Iterator,)*)>;
            fn new() -> Self {
                ($($name::new(),)*)
            }
            fn add_archetype(&mut self, archetype: &Archetype, indices: &[usize]) {
                $(self.$index.add_component_store(&archetype.components[indices[$index]]);)*
            }
            fn iterator(&'a mut self) -> Self::Iterator {
                QueryIterator (($(self.$index.iterator(),)*))
            }
        }

        impl<$($name: Iterator),*> Iterator for QueryIterator<($($name,)*)> {
            type Item = ($($name::Item,)*);
            fn next(&mut self) -> Option<Self::Item> {
                Some(($(self.0.$index.next()?,)*))
            }
        }

        impl<'a, $($name: QueryParam<'a>),*> QueryParams<'a> for ($($name,)*) {
            type Query = ($($name::ComponentQuery,)*);
            fn type_ids() -> Vec<TypeId> {
                let mut ids = vec![$($name::type_id()), *];
                ids.sort_unstable();
                debug_assert!(ids.windows(2).all(|w| w[0] != w[1]), "Cannot query for multiple components of the same type");
                ids
            }
            fn type_ids_unsorted() -> Vec<TypeId> {
                vec![$($name::type_id()), *]
            }
        }

        impl< $($name: 'static),*> ComponentBundle for ($($name,)*) {
            // By calculating the hash here two hashes are done for
            // every entity spawned.
            // The alternative is to use a Vec<TypeId> as the key for
            // archetypes.
            fn archetype_id() -> u64 {
                let mut ids = [$(TypeId::of::<$name>()), *];
                ids.sort_unstable();
                debug_assert!(ids.windows(2).all(|w| w[0] != w[1]), "Cannot spawn entities with multiple components of the same type");

                let mut s = DefaultHasher::new();
                ids.hash(&mut s);
                s.finish()
            }

            fn type_ids_and_order(ids_and_order: &mut [(usize, TypeId)]) {
                $(ids_and_order[$index] = ($index, TypeId::of::<$name>());)*
                ids_and_order.sort_unstable_by(|a, b| a.1.cmp(&b.1));
            }

            fn new_archetype() -> Archetype {
                let mut data = [$((TypeId::of::<$name>(), Some(ComponentStore::new::<$name>())),)*];
                data.sort_unstable_by(|(id0, _), (id1, _)| id0.cmp(&id1));
                Archetype {
                    type_ids: data.iter().map(|(id, _)| *id).collect(),
                    components: data.iter_mut().map(|(_, component_store)| component_store.take().unwrap()).collect(),
                    length: 0,
                    entity_indices: Vec::new(),
                }
            }

            fn add_to_archetype(self, index: usize, archetype: &mut Archetype) {
                let mut component_ordering = [(0, TypeId::of::<()>()); $count];
                Self::type_ids_and_order(&mut component_ordering);
                unsafe {
                    $(archetype.add_component(component_ordering[$index].0, self.$index);)*
                }
                archetype.entity_indices.push(index);
                archetype.length += 1;
            }
        }
    };
}

component_bundle_impl! {1, (A, 0)}
component_bundle_impl! {2, (A, 0), (B, 1)}
component_bundle_impl! {3, (A, 0), (B, 1), (C, 2)}
component_bundle_impl! {4, (A, 0), (B, 1), (C, 2), (D, 3)}
component_bundle_impl! {5, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4)}
component_bundle_impl! {6, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5)}
component_bundle_impl! {7, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6)}
component_bundle_impl! {8, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6), (H, 7)}

pub struct QueryIterator<T>(T);

pub struct ChainedIterator<I: Iterator> {
    current_iter: I,
    iterators: Vec<I>,
}

impl<I: Iterator> ChainedIterator<I> {
    pub fn new(mut iterators: Vec<I>) -> Self {
        let current_iter = iterators.pop().unwrap();
        Self {
            current_iter,
            iterators,
        }
    }
}

impl<I: Iterator> Iterator for ChainedIterator<I> {
    type Item = I::Item;
    fn next(&mut self) -> Option<Self::Item> {
        // Chain the iterators together.
        // If the end of one iterator is reached go to the next.
        self.current_iter.next().or_else(|| {
            self.iterators.pop().map_or(None, |i| {
                self.current_iter = i;
                self.current_iter.next()
            })
        })
    }
}
