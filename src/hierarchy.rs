use crate::*;

#[derive(Debug)]
pub struct HierarchyNode {
    pub(crate) parent: Option<Entity>,
    pub(crate) last_child: Option<Entity>,
    pub(crate) next_sibling: Option<Entity>,
    pub(crate) previous_sibling: Option<Entity>,
}

impl HierarchyNode {
    pub fn parent(&self) -> &Option<Entity> {
        &self.parent
    }
    pub fn last_child(&self) -> &Option<Entity> {
        &self.last_child
    }
    pub fn next_sibling(&self) -> &Option<Entity> {
        &self.next_sibling
    }
    pub fn previous_sibling(&self) -> &Option<Entity> {
        &self.previous_sibling
    }
}

pub fn despawn_hierachy(world: &mut World, entity: Entity) -> Result<(), ()> {
    // This Vec could be probably be avoided
    let mut to_despawn = Vec::new();
    {
        let mut query = world.query::<(&mut HierarchyNode,)>().unwrap();
        query.get_component_mut::<HierarchyNode>(entity).ok_or(())?;

        iterate_descendents(&query, entity, &mut |entity| to_despawn.push(entity));
    }

    for entity in to_despawn {
        world.despawn(entity).unwrap()
    }

    Ok(())
}

pub fn remove_child(world: &mut World, parent: Entity, child: Entity) -> Result<(), ()> {
    let mut query = world.query::<(&mut HierarchyNode,)>().unwrap();

    let (previous, next) = {
        let child = query.get_component_mut::<HierarchyNode>(child).ok_or(())?;

        if child.parent != Some(parent) {
            return Err(());
        }

        let (previous, next) = (child.previous_sibling, child.next_sibling);
        child.previous_sibling = None;
        child.next_sibling = None;
        child.parent = None;
        (previous, next)
    };

    if let Some(previous) = previous {
        let previous = query.get_component_mut::<HierarchyNode>(previous).unwrap();
        previous.next_sibling = next;
    }

    if let Some(next) = next {
        let next = query.get_component_mut::<HierarchyNode>(next).unwrap();
        next.previous_sibling = previous;
    } else {
        // We're removing the last child, so update the parent.
        let parent = query.get_component_mut::<HierarchyNode>(parent).unwrap();

        parent.last_child = previous;
    }

    Ok(())
}

pub fn set_parent(world: &mut World, parent: Option<Entity>, child: Entity) -> Result<(), ()> {
    let mut query = world.query::<(&mut HierarchyNode,)>().unwrap();
    let mut add_hierarchy_to_parent = false;
    let mut add_hierarchy_to_child = false;

    let previous_last_child = {
        if let Some(parent) = parent {
            // Check if a HierarchyNode exists on the parent, otherwise create one.
            if let Some(parent_hierarchy) = query.get_component_mut::<HierarchyNode>(parent) {
                let previous_last_child = parent_hierarchy.last_child;
                parent_hierarchy.last_child = Some(child);
                previous_last_child
            } else {
                add_hierarchy_to_parent = true;
                None
            }
        } else {
            None
        }
    };

    // Connect the previous child to the new child.
    if let Some(previous_last_child) = previous_last_child {
        let previous_last_child = query
            .get_component_mut::<HierarchyNode>(previous_last_child)
            .unwrap();

        previous_last_child.next_sibling = Some(child);
    }

    let mut old_parent = None;

    // Connect the child with its new siblings
    // Create a HierarchyComponent if the child doesn't have one.
    if let Some(child) = query.get_component_mut::<HierarchyNode>(child) {
        old_parent = child.parent;

        child.parent = parent;
        child.previous_sibling = previous_last_child;
        child.next_sibling = None;
    } else {
        add_hierarchy_to_child = true;
    }

    std::mem::drop(query);

    if add_hierarchy_to_parent {
        let parent = parent.unwrap();
        world
            .add_component(
                parent,
                HierarchyNode {
                    parent: None,
                    last_child: Some(child),
                    next_sibling: None,
                    previous_sibling: None,
                },
            )
            .unwrap()
    }

    if add_hierarchy_to_child {
        world.add_component(
            child,
            HierarchyNode {
                parent,
                previous_sibling: previous_last_child,
                next_sibling: None,
                last_child: None,
            },
        );
    }

    // Remove the entity from its old parent, if it has one.
    if let Some(old_parent) = old_parent {
        remove_child(world, old_parent, child).unwrap()
    }

    Ok(())
}

// Pray that a cycle is never made.
// In the future kudo should be amended to allow a mutable query to be downgraded to an immutable query.
// That would allow this to accept &Query<(&HierarchyNode,)> instead.
// This is far too messy to pass in a generic query that implements GetComponent.
pub fn iterate_descendents<'a, T: QueryParameters>(
    query: &Query<'a, T>, //&Query<(&mut HierarchyNode,)>,
    entity: Entity,
    function: &mut impl FnMut(Entity),
) where
    <T as QueryParametersBorrow<'a>>::ComponentBorrows: GetComponent,
{
    function(entity);

    if let Some(node) = query.get_component::<HierarchyNode>(entity) {
        if let Some(child) = node.last_child {
            iterate_descendents(query, child, function);
        }
        if let Some(previous_sibling) = node.previous_sibling {
            iterate_descendents(query, previous_sibling, function);
        }
    }
}
