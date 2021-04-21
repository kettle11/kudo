use kudo::*;

struct Node {
    parent: Option<Entity>,
    children: Vec<Entity>,
}

impl Node {
    fn new() -> Self {
        Self {
            parent: None,
            children: Vec::new(),
        }
    }
}

struct HierarchyBuilder<'a: 'b, 'b> {
    entity: Entity,
    query: &'b Query<'a, (&'a mut Node,)>,
}

impl<'a: 'b, 'b> HierarchyBuilder<'a, 'b> {
    fn new(query: &'b Query<'a, (&'a mut Node,)>, entity: Entity) -> Self {
        HierarchyBuilder { entity, query }
    }

    fn child(&mut self, child_entity: Entity) -> HierarchyBuilder<'a, 'b> {
        HierarchyBuilder {
            entity: child_entity,
            query: self.query,
        }
    }
}

fn main() {
    // First we create the world.
    let mut world = World::new();

    let e0 = world.spawn((true,));
    let e1 = world.spawn((true,));

    let query = world.query::<(&mut Node,)>().unwrap();
    {
        let mut hierarchy_builder = HierarchyBuilder::new(&query, e0);
        let child = hierarchy_builder.child(e1);
    }
}
