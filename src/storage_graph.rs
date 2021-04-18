use std::any::TypeId;
#[derive(Debug, Clone)]
struct Node {
    // usize is another node index
    edges: Vec<(TypeId, usize)>,
    // usize is storage index
    storage: Option<usize>,
}
impl Node {
    pub fn new() -> Self {
        Self {
            edges: Vec::new(),
            storage: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StorageGraph {
    nodes: Vec<Node>,
}

#[derive(Debug, Clone, Copy)]

pub struct Requirement {
    pub(crate) type_id: TypeId,
    pub(crate) requirement_type: RequirementType,
    pub(crate) original_index: usize,
}

// Some constructors to save a little typing elsewhere.
impl Requirement {
    pub fn with_(original_index: usize, type_id: TypeId) -> Self {
        Self {
            type_id,
            requirement_type: RequirementType::With,
            original_index,
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[allow(unused)]
pub enum RequirementType {
    With,
    Without,
    Optional,
}

impl StorageGraph {
    pub fn new() -> Self {
        Self {
            // The first node is the empty node
            nodes: vec![Node::new()],
        }
    }

    /// Finds the storage index or an insert handle for where it could go.
    pub(crate) fn find_storage<'a>(
        &self,
        looking_for: &'a [TypeId],
    ) -> Result<usize, InsertHandle<'a>> {
        self.recurse_find_storage(0, looking_for)
    }

    pub(crate) fn insert_storage(&mut self, insert_handle: InsertHandle, storage_index: usize) {
        if let Some((first, remainder)) = insert_handle.type_ids.split_first() {
            let new_node_index = self.nodes.len();
            self.nodes.push(Node::new());
            self.nodes[insert_handle.node_index]
                .edges
                .insert(insert_handle.index_in_node, (*first, new_node_index));
            self.recurse_insert(new_node_index, remainder, storage_index);
        } else {
            self.nodes[insert_handle.node_index].storage = Some(storage_index);
        }
    }

    // Insert nodes for the remaining type_ids
    fn recurse_insert(&mut self, node_index: usize, type_ids: &[TypeId], storage_index: usize) {
        if let Some((first, remainder)) = type_ids.split_first() {
            let new_node_index = self.nodes.len();
            self.nodes.push(Node::new());
            self.nodes[node_index].edges.push((*first, new_node_index));
            self.recurse_insert(new_node_index, remainder, storage_index);
        } else {
            self.nodes[node_index].storage = Some(storage_index);
        }
    }

    fn recurse_find_storage<'a>(
        &self,
        node_index: usize,
        type_ids: &'a [TypeId],
    ) -> Result<usize, InsertHandle<'a>> {
        if type_ids.len() == 0 {
            if let Some(index) = self.nodes[node_index].storage {
                Ok(index)
            } else {
                Err(InsertHandle {
                    node_index,
                    index_in_node: 0,
                    type_ids,
                })
            }
        } else {
            let next_node = self.nodes[node_index]
                .edges
                .binary_search_by_key(&type_ids[0], |v| v.0);
            let next_node = match next_node {
                Result::Ok(i) => self.nodes[node_index].edges[i].1,
                Result::Err(i) => {
                    return Err(InsertHandle {
                        node_index,
                        index_in_node: i,
                        type_ids,
                    })
                }
            };
            self.recurse_find_storage(next_node, &type_ids[1..])
        }
    }

    /// Iterate storage channel_indices that contain at least a given set of Types.
    /// If the function passed in returns an error iteration will stop and the error
    /// will be returned.
    pub(crate) fn iterate_matching_storage<
        ERROR,
        F: FnMut(usize, &[usize]) -> Result<(), ERROR>,
    >(
        &self,
        types: &[Requirement],
        mut f: F,
    ) -> Result<(), ERROR> {
        let mut channel_indices = Vec::with_capacity(types.len());
        let current_index = 0;
        self.recurse_matching_storage(0, types, &mut channel_indices, current_index, &mut f)?;
        Ok(())
    }

    fn recurse_matching_storage<ERROR, F: FnMut(usize, &[usize]) -> Result<(), ERROR>>(
        &self,
        node: usize,
        types: &[Requirement],
        channel_indices: &mut Vec<usize>,
        channel_index: usize,
        f: &mut F,
    ) -> Result<(), ERROR> {
        use std::cmp::Ordering;
        if let Some((head, tail)) = types.split_first() {
            match head.requirement_type {
                RequirementType::With => {
                    for edge in self.nodes[node].edges.iter() {
                        match edge.0.cmp(&head.type_id) {
                            Ordering::Equal => {
                                channel_indices.push(channel_index);
                                let result = self.recurse_matching_storage(
                                    edge.1,
                                    tail,
                                    channel_indices,
                                    channel_index + 1,
                                    f,
                                );
                                channel_indices.pop();
                                result
                            }
                            Ordering::Less => self.recurse_matching_storage(
                                edge.1,
                                types,
                                channel_indices,
                                channel_index + 1,
                                f,
                            ),
                            // It is no longer possible that nodes will match.
                            Ordering::Greater => Ok(()),
                        }? // Note the '?' tha: We should return if there's an error
                    }
                }
                RequirementType::Without => {
                    for edge in self.nodes[node].edges.iter() {
                        match edge.0.cmp(&head.type_id) {
                            // Skip archetypes that contain something this thing.
                            Ordering::Equal => continue,
                            Ordering::Less => self.recurse_matching_storage(
                                edge.1,
                                types,
                                channel_indices,
                                channel_index + 1,
                                f,
                            ),
                            // We want to skip `Without` channels, so continue here.
                            Ordering::Greater => self.recurse_matching_storage(
                                edge.1,
                                tail,
                                channel_indices,
                                channel_index + 1,
                                f,
                            ),
                        }? // Note the '?' tha: We should return if there's an error
                    }
                }
                RequirementType::Optional => unimplemented!(),
            }
        } else {
            if let Some(storage) = self.nodes[node].storage {
                f(storage, channel_indices)?;
            }
            // At this point all further archetypes with more components will be a superset of the search, so they match.
            for edge in self.nodes[node].edges.iter() {
                self.recurse_matching_storage(edge.1, types, channel_indices, channel_index + 1, f)?
            }
        }
        Ok(())
    }

    // For debugging purposes
    #[allow(unused)]
    pub(crate) fn print_node(&self, node: usize, indentation: usize) {
        println!(
            "{:indent$}Node archetype: {:?}",
            "",
            self.nodes[node].storage,
            indent = indentation,
        );
        for node in self.nodes[node].edges.iter() {
            println!(
                "{:indent$}TypeID: {:?}",
                "",
                node.0,
                indent = indentation + 2,
            );
            self.print_node(node.1, indentation + 2);
        }
    }
}

#[derive(Debug)]
pub struct InsertHandle<'a> {
    node_index: usize,
    index_in_node: usize,
    type_ids: &'a [TypeId],
}

/*
#[cfg(test)]
mod tests {
    use std::any::TypeId;

    use super::{Requirement, StorageGraph};

    // Creates a fake type ID we can use for tests.
    fn fake_type_id(id: u64) -> TypeId {
        unsafe { std::mem::transmute(id) }
    }

    fn insert_fake_archetype(storage_graph: &mut StorageGraph, ids: &[u64], storage_index: usize) {
        let ids = unsafe { std::mem::transmute(ids) };
        let insert_handle = storage_graph.find_storage(ids).err().unwrap();
        storage_graph.insert_storage(insert_handle, storage_index);
    }

    #[test]
    fn storage_graph() {
        let mut storage_graph = StorageGraph::new();

        insert_fake_archetype(&mut storage_graph, &[0, 1, 2], 0);
        insert_fake_archetype(&mut storage_graph, &[1, 2], 1);

        let ids = [
            Requirement::with_(0, fake_type_id(1)),
            Requirement::with_(1, fake_type_id(2)),
        ];
        let mut matching = Vec::new();
        let _ = storage_graph.iterate_matching_storage::<(), _>(&ids, |index, _indices| {
            matching.push(index);
            Ok(())
        });

        assert!(matching.contains(&0));
        assert!(matching.contains(&1));
    }

    #[test]
    fn storage_graph_without0() {
        let mut storage_graph = StorageGraph::new();

        insert_fake_archetype(&mut storage_graph, &[0, 1, 2], 0);
        insert_fake_archetype(&mut storage_graph, &[1, 2], 1);

        let ids = [
            Requirement::without(0, fake_type_id(0)),
            Requirement::with_(1, fake_type_id(2)),
        ];
        let mut matching = Vec::new();
        let _ = storage_graph.iterate_matching_storage::<(), _>(&ids, |index, _indices| {
            matching.push(index);
            Ok(())
        });

        assert!(!matching.contains(&0));
        assert!(matching.contains(&1));
    }

    #[test]
    fn storage_graph_without1() {
        let mut storage_graph = StorageGraph::new();

        insert_fake_archetype(&mut storage_graph, &[0, 1, 2, 3, 4], 0);
        insert_fake_archetype(&mut storage_graph, &[0, 1, 2, 4], 1);

        let ids = [
            Requirement::with_(0, fake_type_id(2)),
            Requirement::without(1, fake_type_id(3)),
        ];
        let mut matching = Vec::new();
        let _ = storage_graph.iterate_matching_storage::<(), _>(&ids, |index, _indices| {
            matching.push(index);
            Ok(())
        });

        assert!(!matching.contains(&0));
        assert!(matching.contains(&1));
    }
}
*/
