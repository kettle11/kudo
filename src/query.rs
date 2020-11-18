use super::{Archetype, ChainedIterator, Entity, GetIter, World};
use std::any::TypeId;
use std::ops::{Deref, DerefMut};
use std::sync::{RwLockReadGuard, RwLockWriteGuard};

#[doc(hidden)]
/// Get data from the world
/// Fetch returns something that implements `FetchItem`.
/// This use of `FetchItem` allows for returning something that has a meaningful `Drop`
/// but can still be passed as a more ergonomic type to the system.
/// For example `RwLockReadGuard<T>` has a `Drop` but for singletons we want to pass
/// it to the system as a `&T`. In this case we use FetchItem to get a temporary
/// reference to the RwLockReadGuard<T>.
pub trait Fetch<'a>: Sized {
    type FetchItem: for<'b> FetchItem<'b>;
    fn fetch(world: &'a World) -> Result<Self::FetchItem, FetchError>;
}

#[doc(hidden)]
pub trait FetchItem<'a> {
    type Item;
    fn get(&'a mut self) -> Self::Item;
}

#[derive(Debug)]
pub enum FetchError {
    ComponentAlreadyBorrowed(ComponentAlreadyBorrowed),
    ComponentDoesNotExist(ComponentDoesNotExist),
}

#[derive(Debug)]
pub struct ComponentAlreadyBorrowed(&'static str);

impl ComponentAlreadyBorrowed {
    pub fn new<T>() -> Self {
        Self(std::any::type_name::<T>())
    }
}

impl std::fmt::Display for ComponentAlreadyBorrowed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] is already borrowed from the archetype", self.0)
    }
}

impl std::error::Error for ComponentAlreadyBorrowed {}

#[derive(Debug)]
pub struct ComponentDoesNotExist(&'static str);

impl ComponentDoesNotExist {
    pub fn new<T>() -> Self {
        Self(std::any::type_name::<T>())
    }
}

impl std::fmt::Display for ComponentDoesNotExist {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] does not exist", self.0)
    }
}

impl std::error::Error for ComponentDoesNotExist {}

/// `QueryFetch` implements fetching slightly differently than regular `Regular` Fetch.
/// `QueryFetch` is used as part of `Query`s
#[doc(hidden)]
pub trait QueryFetch<'a> {
    type Item;
    fn fetch_param(world: &'a World, archetypes: usize) -> Result<Self::Item, FetchError>;
}

/// A dummy struct that is never constructed.
/// It is used to specify a Fetch trait.
#[doc(hidden)]
pub struct QueryFetchRead<T> {
    phantom: std::marker::PhantomData<T>,
}

// Borrow a single component channel from an archetype.
impl<'world_borrow, T: 'static> QueryFetch<'world_borrow> for QueryFetchRead<T> {
    type Item = RwLockReadGuard<'world_borrow, Vec<T>>;
    fn fetch_param(
        world: &'world_borrow World,
        archetype: usize,
    ) -> Result<Self::Item, FetchError> {
        fetch_component_channel(world, archetype)
    }
}

/// A dummy struct that is never constructed.
/// It is used to specify a Fetch trait.
#[doc(hidden)]
pub struct QueryFetchWrite<T> {
    phantom: std::marker::PhantomData<T>,
}

impl<'world_borrow, T: 'static> QueryFetch<'world_borrow> for QueryFetchWrite<T> {
    type Item = RwLockWriteGuard<'world_borrow, Vec<T>>;
    fn fetch_param(
        world: &'world_borrow World,
        archetype: usize,
    ) -> Result<Self::Item, FetchError> {
        fetch_component_channel_mut(world, archetype)
    }
}

/// Gets a component channel from an archetype.
fn fetch_component_channel_mut<T: 'static>(
    world: &World,
    archetype: usize,
) -> Result<RwLockWriteGuard<'_, Vec<T>>, FetchError> {
    let archetype = &world.archetypes[archetype];
    let type_id = TypeId::of::<T>();

    let component_index = archetype
        .components
        .iter()
        .position(|c| c.type_id == type_id)
        .unwrap();
    if let Ok(write_guard) = archetype.get(component_index).try_write() {
        Ok(write_guard)
    } else {
        Err(FetchError::ComponentAlreadyBorrowed(
            ComponentAlreadyBorrowed::new::<T>(),
        ))
    }
}

fn fetch_component_channel<T: 'static>(
    world: &World,
    archetype: usize,
) -> Result<RwLockReadGuard<'_, Vec<T>>, FetchError> {
    let archetype = &world.archetypes[archetype];
    let type_id = TypeId::of::<T>();

    let index = archetype
        .components
        .iter()
        .position(|c| c.type_id == type_id)
        .unwrap();
    if let Ok(read_guard) = archetype.get(index).try_read() {
        Ok(read_guard)
    } else {
        Err(FetchError::ComponentAlreadyBorrowed(
            ComponentAlreadyBorrowed::new::<T>(),
        ))
    }
}

/// Used to get a single *immutable* instance of a component from the world.
/// If there are multiple of the component in the world an arbitrary
/// instance is returned.
pub struct Single<'world_borrow, T> {
    entity: Entity,
    pub borrow: RwLockReadGuard<'world_borrow, Vec<T>>,
}

impl<'world_borrow, T> Single<'world_borrow, T> {
    pub fn entity(&self) -> Entity {
        self.entity
    }
}

impl<'world_borrow, T> Deref for Single<'world_borrow, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        // This unwrap should never panic because a Single cannot be constructed
        // unless there is a single element
        self.borrow.get(0).unwrap()
    }
}

impl<'world_borrow, T> DerefMut for SingleMut<'world_borrow, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.borrow.get_mut(0).unwrap()
    }
}

impl<'world_borrow, T> Deref for SingleMut<'world_borrow, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.borrow.get(0).unwrap()
    }
}

/// Used to get a single *mutable* instance of a component from the world.
/// If there are multiple of the component in the world an arbitrary
/// instance is returned.
pub struct SingleMut<'world_borrow, T> {
    entity: Entity,
    pub borrow: RwLockWriteGuard<'world_borrow, Vec<T>>,
}

impl<'world_borrow, T> SingleMut<'world_borrow, T> {
    pub fn entity(&self) -> Entity {
        self.entity
    }
}

impl<'a, T: 'static> Fetch<'a> for Single<'_, T> {
    type FetchItem = Single<'a, T>;
    fn fetch(world: &'a World) -> Result<Self::FetchItem, FetchError> {
        // The archetypes must be found here.
        let mut archetype_index = None;
        let type_id = TypeId::of::<T>();
        for (i, archetype) in world.archetypes.iter().enumerate() {
            if archetype.components.iter().any(|c| c.type_id == type_id) {
                archetype_index = Some(i);
            }
        }

        if let Some(archetype_index) = archetype_index {
            // This feels a bit messy to just get the entity.
            let index = world.archetypes[archetype_index].entities[0];
            let generation = world.entities[index as usize].generation;
            let entity = Entity { index, generation };
            let borrow = fetch_component_channel(&world, archetype_index)?;

            if !borrow.is_empty() {
                return Ok(Single { entity, borrow });
            }
        }
        Err(FetchError::ComponentDoesNotExist(
            ComponentDoesNotExist::new::<T>(),
        ))
    }
}

impl<'a, 'b, T: 'static> FetchItem<'b> for Single<'a, T> {
    type Item = &'b T;
    fn get(&'b mut self) -> Self::Item {
        &self.borrow[0]
    }
}

impl<'a, T: 'static> Fetch<'a> for &T {
    type FetchItem = Single<'a, T>;
    fn fetch(world: &'a World) -> Result<Self::FetchItem, FetchError> {
        Self::FetchItem::fetch(world)
    }
}

impl<'a, T: 'static> Fetch<'a> for &mut T {
    type FetchItem = SingleMut<'a, T>;
    fn fetch(world: &'a World) -> Result<Self::FetchItem, FetchError> {
        Self::FetchItem::fetch(world)
    }
}

impl<'a, 'b, T: 'static> FetchItem<'b> for SingleMut<'a, T> {
    type Item = &'b mut T;
    fn get(&'b mut self) -> Self::Item {
        &mut self.borrow[0]
    }
}

impl<'a, 'b, T: 'static> Fetch<'a> for SingleMut<'_, T> {
    type FetchItem = SingleMut<'a, T>;

    fn fetch(world: &'a World) -> Result<Self::FetchItem, FetchError> {
        // The archetypes must be found here.
        let mut archetype_index = None;
        let type_id = TypeId::of::<T>();
        for (i, archetype) in world.archetypes.iter().enumerate() {
            if archetype.components.iter().any(|c| c.type_id == type_id) {
                archetype_index = Some(i);
            }
        }

        if let Some(archetype_index) = archetype_index {
            // This feels a bit messy to just get the entity.
            let index = world.archetypes[archetype_index].entities[0];
            let generation = world.entities[index as usize].generation;
            let entity = Entity { index, generation };

            let borrow = fetch_component_channel_mut(&world, archetype_index)?;

            if !borrow.is_empty() {
                return Ok(SingleMut { entity, borrow });
            }
        }
        Err(FetchError::ComponentDoesNotExist(
            ComponentDoesNotExist::new::<T>(),
        ))
    }
}

/// Query for entities with specific components.
pub struct Query<'world_borrow, T: QueryParams> {
    // The archetype borrow will be based on the QueryParams borrow type.
    pub borrow: <T as QueryFetch<'world_borrow>>::Item,
    pub(crate) phantom: std::marker::PhantomData<&'world_borrow ()>,
}

impl<'world_borrow, 'iter, D: QueryParams> Query<'world_borrow, D>
where
    <D as QueryFetch<'world_borrow>>::Item: GetIter<'iter>,
{
    /// Gets an iterator over the components in this `Query`.
    pub fn iter(
        &'iter mut self,
    ) -> <<D as QueryFetch<'world_borrow>>::Item as GetIter<'iter>>::Iter {
        self.borrow.get_iter()
    }
}

impl<'iter, T: GetIter<'iter>> GetIter<'iter> for Vec<T> {
    type Iter = ChainedIterator<<T as GetIter<'iter>>::Iter>;
    fn get_iter(&'iter mut self) -> Self::Iter {
        ChainedIterator::new(self.iter_mut().map(|t| t.get_iter()).collect())
    }
}

impl<'iter, 'world_borrow, T: 'static> GetIter<'iter> for RwLockReadGuard<'world_borrow, Vec<T>> {
    type Iter = std::slice::Iter<'iter, T>;
    fn get_iter(&'iter mut self) -> Self::Iter {
        <[T]>::iter(self)
    }
}

impl<'iter, 'world_borrow, T: 'static> GetIter<'iter> for RwLockWriteGuard<'world_borrow, Vec<T>> {
    type Iter = std::slice::IterMut<'iter, T>;
    fn get_iter(&'iter mut self) -> Self::Iter {
        <[T]>::iter_mut(self)
    }
}

/// The parameters passed into a query. Like: `(&bool, &String)`
pub trait QueryParams: for<'a> QueryFetch<'a> {}

/// A member of a `Query`, like `&A` or `&mut A`
pub trait QueryParam {
    type Fetch: for<'a> QueryFetch<'a>;

    #[doc(hidden)]
    fn add_types(types: &mut Vec<TypeId>);
    #[doc(hidden)]
    fn matches_archetype(archetype: &Archetype) -> bool;
}

// Implement EntityQueryItem for immutable borrows
impl<A: 'static> QueryParam for &A {
    type Fetch = QueryFetchRead<A>;

    fn add_types(types: &mut Vec<TypeId>) {
        types.push(TypeId::of::<A>())
    }

    fn matches_archetype(archetype: &Archetype) -> bool {
        let type_id = TypeId::of::<A>();
        archetype.components.iter().any(|c| c.type_id == type_id)
    }
}

// Implement EntityQueryItem for mutable borrows
impl<A: 'static> QueryParam for &mut A {
    type Fetch = QueryFetchWrite<A>;

    fn add_types(types: &mut Vec<TypeId>) {
        types.push(TypeId::of::<A>())
    }

    fn matches_archetype(archetype: &Archetype) -> bool {
        let type_id = TypeId::of::<A>();
        archetype.components.iter().any(|c| c.type_id == type_id)
    }
}

impl<'a, 'b, Q: QueryParams + 'static> FetchItem<'b> for Option<Query<'a, Q>> {
    type Item = Query<'a, Q>;
    fn get(&'b mut self) -> Self::Item {
        self.take().unwrap()
    }
}

impl<'world_borrow, Q: QueryParams + 'static> Fetch<'world_borrow> for Query<'_, Q> {
    type FetchItem = Option<Query<'world_borrow, Q>>;
    fn fetch(world: &'world_borrow World) -> Result<Self::FetchItem, FetchError> {
        Ok(Some(Query {
            borrow: Q::fetch_param(world, 0 /* Ignored */)?,
            phantom: std::marker::PhantomData,
        }))
    }
}

macro_rules! entity_query_params_impl {
    ($($name: ident),*) => {
        #[allow(unused_parens)]
        impl<$($name: QueryParam,)*> QueryParams for ($($name,)*) {}

        #[allow(unused_parens)]
        impl <'world_borrow, $($name: QueryParam,)*> QueryFetch<'world_borrow> for ($($name,)*) {
            type Item = Vec<($(<$name::Fetch as QueryFetch<'world_borrow>>::Item),*)>;

            fn fetch_param(world: &'world_borrow World, _archetype: usize) -> Result<Self::Item, FetchError> {
                #[cfg(debug_assertions)]
                {
                    let mut types: Vec<TypeId> = Vec::new();
                    $($name::add_types(&mut types);)*
                    types.sort();
                    debug_assert!(
                        types.windows(2).all(|x| x[0] != x[1]),
                        "Queries cannot have duplicate types"
                    );
                }

                let mut archetype_indices = Vec::new();
                for (i, archetype) in world.archetypes.iter().enumerate() {
                    let matches = $($name::matches_archetype(&archetype))&&*;

                    if matches {
                        archetype_indices.push(i);
                    }
                }

                let mut result = Vec::with_capacity(archetype_indices.len());
                for index in archetype_indices {
                   result.push(($(<$name::Fetch>::fetch_param(world, index)?),*))
                }

                Ok(result)
            }
        }

    };
}

//entity_query_params_impl! {}
entity_query_params_impl! {A}
entity_query_params_impl! {A, B}
entity_query_params_impl! {A, B, C}
entity_query_params_impl! {A, B, C, D}
entity_query_params_impl! {A, B, C, D, E}
entity_query_params_impl! {A, B, C, D, E, F}
entity_query_params_impl! {A, B, C, D, E, F, G}
entity_query_params_impl! {A, B, C, D, E, F, G, H}
