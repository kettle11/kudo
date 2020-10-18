use super::{Archetype, FetchRead, FetchWrite, GetIter, TypeId, World};

/// A query that can be passed into a `System` function.
/*
pub trait TopLevelQuery<'world_borrow> {
    type Item;
    fn get(world: &'world_borrow World) -> Result<Self::Item, ()>;
}
*/

// Very important here is that the lifetime of the Query this is implemented for is not the same
// as the lifetime of the Item returned.
// This means that the outer lifetime is ignored, the Query<'_, PARAMS> is just a way to guide the creation of the
// Query with the final lifetime.
/*
impl<'world_borrow, PARAMS: QueryParams + 'static> TopLevelQuery<'world_borrow>
    for Query<'_, PARAMS>
{
    type Item = Query<'world_borrow, <PARAMS as Fetch<'world_borrow>>::Item>;
    fn get(world: &'world_borrow World) -> Result<Self::Item, ()> {
        Ok(Query {
            borrow: <PARAMS as Fetch<'world_borrow>>::get(world, &[])?,
            phantom: std::marker::PhantomData,
        })
    }
}
*/

// Maybe the lifetime of Fetch<'world_borrow> within the Item is too specific?
// So the Query within a function definition can't be cast to Fetch?
// Perhaps something about this trait makes it impossible to 'as' into it?
/*
impl<'world_borrow, PARAMS: QueryParams + 'static> Fetch<'world_borrow> for Query<'_, PARAMS> {
    type Item =
        Query<'world_borrow, <<PARAMS as QueryParams>::Fetch as Fetch<'world_borrow>>::Item>;
    fn get(world: &'world_borrow World, _archetypes: &[usize]) -> Result<Self::Item, ()> {
        Ok(Query {
            borrow: <<PARAMS as QueryParams>::Fetch as Fetch<'world_borrow>>::get(world, &[])?,
            phantom: std::marker::PhantomData,
        })
    }
}
*/

/// Get data from the world
pub trait Fetch<'a> {
    type Item: for<'iter> GetIter<'iter>;
    fn get(world: &'a World, archetypes: &[usize]) -> Result<Self::Item, ()>;
}

pub trait QueryParams {
    type Fetch: for<'a> Fetch<'a>;
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
    type Fetch = FetchRead<A>; /* Immutable borrow of some sort */

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

// Could there be a way to implement this for a &'world_borrow World to get the lifetime from there?

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
                    let matches = false; //$($name::matches_archetype(&archetype))&&*;

                    if matches {
                        archetype_indices.push(i);
                    }
                }

                // Find matching archetypes here.
                Ok(($(<<$name as QueryParam>::Fetch as Fetch>::get(world, &archetype_indices)?,)*))
            }
        }
    };
}

entity_query_params_impl! {}
entity_query_params_impl! {A}
entity_query_params_impl! {A, B}
entity_query_params_impl! {A, B, C}
entity_query_params_impl! {A, B, C, D}
entity_query_params_impl! {A, B, C, D, E}
entity_query_params_impl! {A, B, C, D, E, F}
entity_query_params_impl! {A, B, C, D, E, F, G}
entity_query_params_impl! {A, B, C, D, E, F, G, H}
