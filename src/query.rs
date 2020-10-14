use super::{Archetype, TypeId, World, WorldBorrow, WorldBorrowImmut, WorldBorrowMut};

/// A query reference specifies how data will be queried and borrowed from the world.
pub trait Query<'world_borrow> {
    type WorldBorrow: for<'a> WorldBorrow<'a>;

    /// Used to verify that there are no duplicates queries in a query.
    fn add_types(types: &mut Vec<TypeId>);

    /// Get the query data from the world for the archetypes indice passed in.
    fn get_query(world: &'world_borrow World, archetypes: &[usize]) -> Self::WorldBorrow;

    // Because of the way this is implemented the worst case for finding
    // archetypes for a query is approximately O(a * c * q)
    // where a is the number of archetypes
    // c is the number of components in an archetype (which varies)
    // and q is the number of queries in this query.
    fn matches_archetype(archetype: &Archetype) -> bool;
}

// Implement Query for immutable borrows
impl<'world_borrow, A: 'static> Query<'world_borrow> for &A {
    type WorldBorrow = WorldBorrowImmut<'world_borrow, A>;

    fn add_types(types: &mut Vec<TypeId>) {
        types.push(TypeId::of::<A>())
    }

    fn get_query(world: &'world_borrow World, archetypes: &[usize]) -> Self::WorldBorrow {
        let type_id = TypeId::of::<A>();
        let mut query = WorldBorrowImmut::new();
        for i in archetypes {
            query.add_archetype(type_id, &world.archetypes[*i]);
        }
        query
    }

    fn matches_archetype(archetype: &Archetype) -> bool {
        let type_id = TypeId::of::<A>();
        archetype.components.iter().any(|c| c.type_id == type_id)
    }
}

// Implement Query for mutable borrows
impl<'world_borrow, A: 'static> Query<'world_borrow> for &mut A {
    type WorldBorrow = WorldBorrowMut<'world_borrow, A>;

    fn add_types(types: &mut Vec<TypeId>) {
        types.push(TypeId::of::<A>())
    }

    fn get_query(world: &'world_borrow World, archetypes: &[usize]) -> Self::WorldBorrow {
        let type_id = TypeId::of::<A>();
        let mut query = WorldBorrowMut::new();
        for i in archetypes {
            query.add_archetype(type_id, &world.archetypes[*i]);
        }
        query
    }

    fn matches_archetype(archetype: &Archetype) -> bool {
        let type_id = TypeId::of::<A>();
        archetype.components.iter().any(|c| c.type_id == type_id)
    }
}
