use super::{
    Archetype, FetchRead, FetchWrite, GetIter, GetSingle, GetSingleMut, TypeId, World,
    WorldBorrowImmut, WorldBorrowMut,
};
use std::ops::{Deref, DerefMut};

/// Get data from the world
pub trait Fetch<'a> {
    type Item;
    fn get(world: &'a World, archetypes: &[usize]) -> Result<Self::Item, ()>;
}

pub trait QueryParams {
    type Fetch: for<'a> Fetch<'a>;
}

pub trait TopLevelQuery: for<'a> Fetch<'a> {}
impl<'world_borrow, T: QueryParams> TopLevelQuery for Query<'world_borrow, T> {}
impl<'world_borrow, T: 'static> TopLevelQuery for Single<'world_borrow, T> {}
impl<'world_borrow, T: 'static> TopLevelQuery for SingleMut<'world_borrow, T> {}

impl<'a, T: QueryParams> Fetch<'a> for Query<'_, T> {
    type Item = Query<'a, T>;
    fn get(world: &'a World, archetypes: &[usize]) -> Result<Self::Item, ()> {
        Ok(Query {
            borrow: <<T as QueryParams>::Fetch as Fetch<'a>>::get(&world, &archetypes)?,
            phantom: std::marker::PhantomData,
        })
    }
}

/// Used to get a single *immutable* instance of a component from the world.
/// If there are multiple of the component in the world an arbitrary
/// instance is returned.
pub struct Single<'world_borrow, T> {
    pub borrow: WorldBorrowImmut<'world_borrow, T>,
}

impl<'world_borrow, 'a, T> Single<'world_borrow, T> {
    pub fn get(&'a self) -> Option<&T> {
        self.borrow.get()
    }

    pub fn unwrap(&'a self) -> &'a T {
        self.borrow.get().unwrap()
    }
}

impl<'world_borrow, T> Deref for Single<'world_borrow, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // This unwrap may be bad. If a Single fails to get its query then this unwrap
        // will panic when attempting to access a member
        self.unwrap()
    }
}

/// Used to get a single *mutable* instance of a component from the world.
/// If there are multiple of the component in the world an arbitrary
/// instance is returned.
pub struct SingleMut<'world_borrow, T> {
    pub borrow: WorldBorrowMut<'world_borrow, T>,
}

impl<'world_borrow, 'a, T> SingleMut<'world_borrow, T> {
    pub fn get(&'a mut self) -> Option<&mut T> {
        self.borrow.get_mut()
    }

    pub fn unwrap(&'a mut self) -> &mut T {
        self.borrow.get_mut().unwrap()
    }
}

impl<'world_borrow, T> Deref for SingleMut<'world_borrow, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.borrow.get().unwrap()
    }
}

impl<'world_borrow, T> DerefMut for SingleMut<'world_borrow, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.borrow.get_mut().unwrap()
    }
}

impl<'a, T: 'static> Fetch<'a> for Single<'_, T> {
    type Item = Single<'a, T>;
    fn get(world: &'a World, _archetypes: &[usize]) -> Result<Self::Item, ()> {
        // The archetypes must be found here.
        let mut archetype_index = None;
        let type_id = TypeId::of::<T>();
        for (i, archetype) in world.archetypes.iter().enumerate() {
            if archetype.components.iter().any(|c| c.type_id == type_id) {
                archetype_index = Some(i);
            }
        }

        if let Some(archetype_index) = archetype_index {
            Ok(Single {
                borrow: FetchRead::<T>::get(&world, &[archetype_index])?,
            })
        } else {
            Err(())
        }
    }
}

impl<'a, T: 'static> Fetch<'a> for SingleMut<'_, T> {
    type Item = SingleMut<'a, T>;
    fn get(world: &'a World, _archetypes: &[usize]) -> Result<Self::Item, ()> {
        // The archetypes must be found here.
        let mut archetype_index = None;
        let type_id = TypeId::of::<T>();
        for (i, archetype) in world.archetypes.iter().enumerate() {
            if archetype.components.iter().any(|c| c.type_id == type_id) {
                archetype_index = Some(i);
            }
        }

        if let Some(archetype_index) = archetype_index {
            Ok(SingleMut {
                borrow: FetchWrite::<T>::get(&world, &[archetype_index])?,
            })
        } else {
            Err(())
        }
    }
}

/// Query for entities with specific components.
pub struct Query<'world_borrow, T: QueryParams> {
    pub borrow: <<T as QueryParams>::Fetch as Fetch<'world_borrow>>::Item,
    pub(crate) phantom: std::marker::PhantomData<&'world_borrow ()>,
}

// I'm skeptical of the lifetimes here.
impl<'world_borrow, 'iter, D: QueryParams> GetIter<'iter> for Query<'world_borrow, D>
where
    <<D as QueryParams>::Fetch as Fetch<'world_borrow>>::Item: GetIter<'iter>,
{
    type Iter = <<<D as QueryParams>::Fetch as Fetch<'world_borrow>>::Item as GetIter<'iter>>::Iter;
    fn iter(&'iter mut self) -> Self::Iter {
        self.borrow.iter()
    }
}

/// A member of a `Query`, like `&A` or `&mut A`
pub trait QueryParam {
    type Fetch: for<'a> Fetch<'a>;

    #[doc(hidden)]
    fn add_types(types: &mut Vec<TypeId>);
    #[doc(hidden)]
    fn matches_archetype(archetype: &Archetype) -> bool;
}

// Implement EntityQueryItem for immutable borrows
impl<'world_borrow, A: 'static> QueryParam for &A {
    type Fetch = FetchRead<A>;

    fn add_types(types: &mut Vec<TypeId>) {
        types.push(TypeId::of::<A>())
    }

    fn matches_archetype(archetype: &Archetype) -> bool {
        let type_id = TypeId::of::<A>();
        archetype.components.iter().any(|c| c.type_id == type_id)
    }
}

// Implement EntityQueryItem for mutable borrows
impl<'world_borrow, A: 'static> QueryParam for &mut A {
    type Fetch = FetchWrite<A>;

    fn add_types(types: &mut Vec<TypeId>) {
        types.push(TypeId::of::<A>())
    }

    fn matches_archetype(archetype: &Archetype) -> bool {
        let type_id = TypeId::of::<A>();
        archetype.components.iter().any(|c| c.type_id == type_id)
    }
}

macro_rules! entity_query_params_impl {
    ($($name: ident),*) => {
        #[allow(unused_parens)]
        impl<$($name: QueryParam,)*> QueryParams for ($($name),*) {
            type Fetch = ($($name),*);
        }

        #[allow(unused_parens)]
        impl<'world_borrow, $($name: QueryParam,)*> Fetch<'world_borrow> for ($($name),*) {
            type Item = ($(<<$name as QueryParam>::Fetch as Fetch<'world_borrow>>::Item),*);
            fn get(world: &'world_borrow World, _archetypes: &[usize]) -> Result<Self::Item, ()> {
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

                Ok(($(<<$name as QueryParam>::Fetch as Fetch>::get(world, &archetype_indices)?),*))
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
