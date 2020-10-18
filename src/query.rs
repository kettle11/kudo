use super::{
    Archetype, EntityId, FetchRead, GetIter, TypeId, World, WorldBorrow, WorldBorrowImmut,
    WorldBorrowMut,
};

/// A query that can be passed into a `System` function.
pub trait SystemQuery: Sized {
    type Fetch: for<'a> Fetch<'a>;
    // #[doc(hidden)]
    // fn get(world: &World) -> Result<Self, ()>;
}

/// Get data from the world
pub trait Fetch<'a> {
    type Item;
    fn get(world: &'a World, archetypes: &[usize]) -> Result<Self::Item, ()>;
}

pub trait EntityQueryParams {
    type Fetch: for<'a> Fetch<'a>;
}
/// Query for entities with specific components.
pub struct Query<'world_borrow, T> {
    pub borrow: T,
    phantom: std::marker::PhantomData<&'world_borrow T>,
}

impl<'world_borrow, PARAMS: EntityQueryParams> Query<'world_borrow, PARAMS> {}

/// A member of a `Query`, like `&A` or `&mut A`
pub trait EntityQueryItem {
    type Fetch: for<'a> Fetch<'a>;

    #[doc(hidden)]
    fn add_types(types: &mut Vec<TypeId>);
    #[doc(hidden)]
    fn matches_archetype(archetype: &Archetype) -> bool;
}

// Implement EntityQueryItem for immutable borrows
impl<'world_borrow, A: 'static> EntityQueryItem for &A {
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
/*
impl<'world_borrow, A: 'static> EntityQueryItem for &mut A {
    type WorldBorrow = /* Mutable borrow of some sort */;

    fn add_types(types: &mut Vec<TypeId>) {
        types.push(TypeId::of::<A>())
    }

    fn matches_archetype(archetype: &Archetype) -> bool {
        let type_id = TypeId::of::<A>();
        archetype.components.iter().any(|c| c.type_id == type_id)
    }
}
*/

trait QueryParams {}

// Could there be a way to implement this for a &'world_borrow World to get the lifetime from there?

macro_rules! entity_query_params_impl {
    ($($name: ident),*) => {
        // Very important here is that the lifetime of the Query this is implemented for is not the same
        // as the lifetime of the Item returned.
        // This means that the outer lifetime is ignored, the Query<'_,..> is just a way to inform the creation of the
        // Query with the final lifetime.
        impl<'world_borrow, $($name: EntityQueryItem + 'world_borrow,)*> Fetch<'world_borrow> for Query<'_, ($($name,)*)> {
            type Item = Query<'world_borrow, ($(<<$name as EntityQueryItem>::Fetch as Fetch<'world_borrow>>::Item,)*)>;
            fn get(world: &'world_borrow World, _archetypes: &[usize]) -> Result<Self::Item, ()> {
                #[cfg(debug_assertions)]
                {
                    let mut types = Vec::new();
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

                // Find matching archetypes here.
                Ok(Query{borrow: ($(<<$name as EntityQueryItem>::Fetch as Fetch>::get(world, &archetype_indices)?,)*), phantom: std::marker::PhantomData})
            }
        }


    };
}

entity_query_params_impl! {A}
entity_query_params_impl! {A, B}
entity_query_params_impl! {A, B, C}
entity_query_params_impl! {A, B, C, D}
entity_query_params_impl! {A, B, C, D, E}
entity_query_params_impl! {A, B, C, D, E, F}
entity_query_params_impl! {A, B, C, D, E, F, G}
entity_query_params_impl! {A, B, C, D, E, F, G, H}
