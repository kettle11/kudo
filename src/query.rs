use super::{Archetype, EntityId, TypeId, World, WorldBorrow, WorldBorrowImmut, WorldBorrowMut};

/// A query that can be passed into a `System` function.
pub trait SystemQuery<'world_borrow>: Sized {
    fn get(world: &'world_borrow World) -> Result<Self, ()>;
}

/// Parameters passed in as part of a `Query`.
pub trait EntityQueryParams<'world_borrow> {
    type WorldBorrow: for<'iter> WorldBorrow<'iter>;
    fn get_entity_query(world: &'world_borrow World) -> Result<Query<Self>, ()>;
}

/// Query for entities with specific components.
pub struct Query<'world_borrow, PARAMS: EntityQueryParams<'world_borrow> + ?Sized> {
    /// Direct access to the borrow can be used to query for components by
    /// calling `get_component` or `get_component_mut` on members of the borrow tuple.
    pub borrow: PARAMS::WorldBorrow,
}

impl<'world_borrow, PARAMS: EntityQueryParams<'world_borrow>> Query<'world_borrow, PARAMS> {
    /// Gets an iterator over the components of this query.
    pub fn iter(&mut self) -> <PARAMS::WorldBorrow as WorldBorrow>::Iter {
        self.borrow.iter()
    }
}

impl<'world_borrow, PARAMS: EntityQueryParams<'world_borrow>> SystemQuery<'world_borrow>
    for Query<'world_borrow, PARAMS>
{
    fn get(world: &'world_borrow World) -> Result<Self, ()> {
        PARAMS::get_entity_query(world)
    }
}

/// A member of a `Query`, like `&A` or `&mut A`
pub trait EntityQueryItem<'world_borrow> {
    type WorldBorrow: for<'iter> WorldBorrow<'iter>;
    fn get(world: &'world_borrow World, archetypes: &[usize]) -> Result<Self::WorldBorrow, ()>;
    fn add_types(types: &mut Vec<TypeId>);
    fn matches_archetype(archetype: &Archetype) -> bool;
}

// Implement EntityQueryItem for immutable borrows
impl<'world_borrow, A: 'static> EntityQueryItem<'world_borrow> for &A {
    type WorldBorrow = WorldBorrowImmut<'world_borrow, A>;

    fn add_types(types: &mut Vec<TypeId>) {
        types.push(TypeId::of::<A>())
    }

    fn get(world: &'world_borrow World, archetypes: &[usize]) -> Result<Self::WorldBorrow, ()> {
        let type_id = TypeId::of::<A>();
        let mut query = WorldBorrowImmut::new(world);
        for i in archetypes {
            query.add_archetype(type_id, *i as EntityId, &world.archetypes[*i])?;
        }
        Ok(query)
    }

    fn matches_archetype(archetype: &Archetype) -> bool {
        let type_id = TypeId::of::<A>();
        archetype.components.iter().any(|c| c.type_id == type_id)
    }
}

// Implement EntityQueryItem for mutable borrows
impl<'world_borrow, A: 'static> EntityQueryItem<'world_borrow> for &mut A {
    type WorldBorrow = WorldBorrowMut<'world_borrow, A>;

    fn add_types(types: &mut Vec<TypeId>) {
        types.push(TypeId::of::<A>())
    }

    fn get(world: &'world_borrow World, archetypes: &[usize]) -> Result<Self::WorldBorrow, ()> {
        let type_id = TypeId::of::<A>();
        let mut query = WorldBorrowMut::new(world);
        for i in archetypes {
            query.add_archetype(type_id, *i as EntityId, &world.archetypes[*i])?;
        }
        Ok(query)
    }

    fn matches_archetype(archetype: &Archetype) -> bool {
        let type_id = TypeId::of::<A>();
        archetype.components.iter().any(|c| c.type_id == type_id)
    }
}

macro_rules! entity_query_params_impl {
    ($($name: ident),*) => {
        impl<'world_borrow, $($name: EntityQueryItem<'world_borrow>,)*> EntityQueryParams<'world_borrow> for ($($name,)*) {
            type WorldBorrow = ($($name::WorldBorrow,)*);
            fn get_entity_query(world: &'world_borrow World) -> Result<Query<'world_borrow, Self>, ()> {
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
                Ok(Query {borrow: ($($name::get(world, &archetype_indices)?,)*)})
            }
        }

    }
}

entity_query_params_impl! {A}
entity_query_params_impl! {A, B}
entity_query_params_impl! {A, B, C}
entity_query_params_impl! {A, B, C, D}
entity_query_params_impl! {A, B, C, D, E}
entity_query_params_impl! {A, B, C, D, E, F}
entity_query_params_impl! {A, B, C, D, E, F, G}
entity_query_params_impl! {A, B, C, D, E, F, G, H}
