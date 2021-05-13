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

    pub fn clone_hierarchy(&self) -> Self {
        Self {
            parent: self.parent.clone_entity(),
            last_child: self.last_child.clone_entity(),
            next_sibling: self.next_sibling.clone_entity(),
            previous_sibling: self.previous_sibling.clone_entity(),
        }
    }
}

impl World {
    pub fn set_parent(&mut self, parent: Option<&Entity>, child: &Entity) {
        let mut add_hierarchy_to_parent = false;
        let mut add_hierarchy_to_child = false;

        let previous_last_child = {
            if let Some(parent) = &parent {
                // Check if a HierarchyNode exists on the parent, otherwise create one.
                if let Some(parent_hierarchy) = self.get_component_mut::<HierarchyNode>(parent) {
                    let previous_last_child = parent_hierarchy.last_child.clone_entity();
                    parent_hierarchy.last_child = Some(child.clone_entity());
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
        if let Some(previous_last_child) = previous_last_child.as_ref() {
            let previous_last_child = self
                .get_component_mut::<HierarchyNode>(previous_last_child)
                .unwrap();

            previous_last_child.next_sibling = Some(child.clone_entity());
        }

        let mut old_parent = None;

        // Connect the child with its new siblings
        // Create a HierarchyComponent if the child doesn't have one.
        if let Some(child) = self.get_component_mut::<HierarchyNode>(child) {
            old_parent = child.parent.clone_entity();

            child.parent = parent.map(|e| e.clone_entity()).clone_entity();
            child.previous_sibling = previous_last_child.clone_entity();
            child.next_sibling = None;
        } else {
            add_hierarchy_to_child = true;
        }

        if add_hierarchy_to_parent {
            let parent = parent.clone().unwrap();

            self.add_component(
                &parent,
                HierarchyNode {
                    parent: None,
                    last_child: Some(child.clone_entity()),
                    next_sibling: None,
                    previous_sibling: None,
                },
            )
            .unwrap()
        }

        if add_hierarchy_to_child {
            self.add_component(
                child,
                HierarchyNode {
                    parent: parent.map(|e| e.clone_entity()),
                    previous_sibling: previous_last_child,
                    next_sibling: None,
                    last_child: None,
                },
            );
        }

        // Remove the entity from its old parent, if it has one.
        if let Some(old_parent) = old_parent {
            self.remove_child(&old_parent, &child).unwrap()
        }
    }

    fn remove_child(&mut self, parent: &Entity, child: &Entity) -> Result<(), ()> {
        let (previous, next) = {
            let child = self.get_component_mut::<HierarchyNode>(child).ok_or(())?;

            if child.parent != Some(parent.clone_entity()) {
                return Err(());
            }

            let (previous, next) = (
                child.previous_sibling.clone_entity(),
                child.next_sibling.clone_entity(),
            );
            child.previous_sibling = None;
            child.next_sibling = None;
            child.parent = None;
            (previous, next)
        };

        if let Some(previous) = previous.as_ref() {
            let previous = self.get_component_mut::<HierarchyNode>(previous).unwrap();
            previous.next_sibling = next.clone_entity();
        }

        if let Some(next) = next.as_ref() {
            let next = self.get_component_mut::<HierarchyNode>(next).unwrap();
            next.previous_sibling = previous;
        } else {
            // We're removing the last child, so update the parent.
            let parent = self.get_component_mut::<HierarchyNode>(parent).unwrap();

            parent.last_child = previous.clone_entity();
        }

        Ok(())
    }
}
