use super::{Archetype, FetchRead, FetchWrite, GetIter, GetSingle, TypeId, World};

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
impl<'world_borrow, T: QueryParam> TopLevelQuery for Single<'world_borrow, T> {}

impl<'a, T: QueryParams> Fetch<'a> for Query<'_, T> {
    type Item = Query<'a, T>;
    fn get(world: &'a World, archetypes: &[usize]) -> Result<Self::Item, ()> {
        Ok(Query {
            borrow: <<T as QueryParams>::Fetch as Fetch<'a>>::get(&world, &archetypes)?,
            phantom: std::marker::PhantomData,
        })
    }
}

/// Used to get a single instance of a component from the world.
/// If there are multiple of the component in the world an arbitrary
/// instance is returned.
pub struct Single<'world_borrow, T: QueryParam> {
    pub borrow: <<T as QueryParam>::Fetch as Fetch<'world_borrow>>::Item,
    pub(crate) phantom: std::marker::PhantomData<&'world_borrow ()>,
}

impl<'world_borrow, 'a, T: QueryParam> Single<'world_borrow, T>
where
    <<T as QueryParam>::Fetch as Fetch<'world_borrow>>::Item: GetSingle<'a>,
{
    pub fn get(
        &'a mut self,
    ) -> Option<<<<T as QueryParam>::Fetch as Fetch<'world_borrow>>::Item as GetSingle<'a>>::Item>
    {
        self.borrow.get()
    }

    pub fn unwrap(
        &'a mut self,
    ) -> <<<T as QueryParam>::Fetch as Fetch<'world_borrow>>::Item as GetSingle<'a>>::Item {
        self.borrow.get().unwrap()
    }
}

impl<'a, T: QueryParam> Fetch<'a> for Single<'_, T> {
    type Item = Single<'a, T>;
    fn get(world: &'a World, _archetypes: &[usize]) -> Result<Self::Item, ()> {
        // The archetypes must be found here.
        let mut archetype_index = None;
        for (i, archetype) in world.archetypes.iter().enumerate() {
            if T::matches_archetype(&archetype) {
                archetype_index = Some(i);
            }
        }

        if let Some(archetype_index) = archetype_index {
            Ok(Single {
                borrow: <<T as QueryParam>::Fetch as Fetch<'a>>::get(&world, &[archetype_index])?,
                phantom: std::marker::PhantomData,
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
        impl<$($name: QueryParam,)*> QueryParams for ($($name,)*) {
            type Fetch = ($($name,)*);
        }

        impl<'world_borrow, $($name: QueryParam,)*> Fetch<'world_borrow> for ($($name,)*) {
            type Item = ($(<<$name as QueryParam>::Fetch as Fetch<'world_borrow>>::Item,)*);
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

                Ok(($(<<$name as QueryParam>::Fetch as Fetch>::get(world, &archetype_indices)?,)*))
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
