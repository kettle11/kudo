use super::{Archetype, TypeId, World, WorldBorrow, WorldBorrowImmut, WorldBorrowMut, Zip};

pub trait SystemQuery<'world_borrow> {
    fn get(world: &'world_borrow World) -> Self;
}

pub trait EntityQueryParams<'world_borrow>: Sized {
    type WorldBorrow: for<'iter> WorldBorrow<'iter>;
    fn get_entity_query(world: &'world_borrow World) -> EntityQuery<Self>;
}
pub struct EntityQuery<'world_borrow, PARAMS: EntityQueryParams<'world_borrow>>(
    PARAMS::WorldBorrow,
);

impl<'world_borrow, PARAMS: EntityQueryParams<'world_borrow>> SystemQuery<'world_borrow>
    for EntityQuery<'world_borrow, PARAMS>
{
    fn get(world: &'world_borrow World) -> Self {
        PARAMS::get_entity_query(world)
    }
}

impl<'iter, 'world_borrow, A: EntityQueryItem<'world_borrow>> WorldBorrow<'iter>
    for EntityQuery<'world_borrow, (A,)>
{
    type Iter = Zip<(<A::WorldBorrow as WorldBorrow<'iter>>::Iter,)>;
    fn iter(&'iter mut self) -> Self::Iter {
        Zip {
            t: (self.0.0.iter(),),
        }
    }
}

pub trait EntityQueryItem<'world_borrow> {
    type WorldBorrow: for<'iter> WorldBorrow<'iter>;
    fn get(world: &'world_borrow World, archetypes: &[usize]) -> Self::WorldBorrow;
    fn add_types(types: &mut Vec<TypeId>);
    fn matches_archetype(archetype: &Archetype) -> bool;
}

// Implement EntityQueryItem for immutable borrows
impl<'world_borrow, A: 'static> EntityQueryItem<'world_borrow> for &A {
    type WorldBorrow = WorldBorrowImmut<'world_borrow, A>;

    fn add_types(types: &mut Vec<TypeId>) {
        types.push(TypeId::of::<A>())
    }

    fn get(world: &'world_borrow World, archetypes: &[usize]) -> Self::WorldBorrow {
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

// Implement EntityQueryItem for mutable borrows
impl<'world_borrow, A: 'static> EntityQueryItem<'world_borrow> for &mut A {
    type WorldBorrow = WorldBorrowMut<'world_borrow, A>;

    fn add_types(types: &mut Vec<TypeId>) {
        types.push(TypeId::of::<A>())
    }

    fn get(world: &'world_borrow World, archetypes: &[usize]) -> Self::WorldBorrow {
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

impl<'world_borrow, A: EntityQueryItem<'world_borrow>> EntityQueryParams<'world_borrow> for (A,) {
    type WorldBorrow = (A::WorldBorrow,);
    fn get_entity_query(world: &'world_borrow World) -> EntityQuery<'world_borrow, Self> {
        #[cfg(debug_assertions)]
        {
            let mut types = Vec::new();
            A::add_types(&mut types);
            types.sort();
            debug_assert!(
                types.windows(2).all(|x| x[0] != x[1]),
                "Queries cannot have duplicate types"
            );
        }

        let mut archetype_indices = Vec::new();
        for (i, archetype) in world.archetypes.iter().enumerate() {
            let matches = A::matches_archetype(&archetype);
            if matches {
                archetype_indices.push(i);
            }
        }

        // Find matching archetypes here.
        EntityQuery((A::get(world, &archetype_indices),))
    }
}

/*
// Old stuff
/// A query reference specifies how data will be queried and borrowed from the world.
pub trait Query<'world_borrow> {
    type WorldBorrow: for<'iter> WorldBorrow<'iter>;

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
*/
