type ENTITY_INDEX = usize;
pub struct SparseSet<T> {
    indices: Vec<Option<usize>>,
    data: Vec<T>,
    data_index: Vec<usize>,
}

impl<T> SparseSet<T> {
    pub fn new() -> Self {
        Self {
            indices: Vec::new(),
            data: Vec::new(),
            data_index: Vec::new(),
        }
    }

    pub fn insert(&mut self, entity_index: ENTITY_INDEX, data: T) {
        // This line highlights the weakness of sparse sets.
        // They use a bunch of memory!
        if self.indices.len() <= entity_index {
            self.indices.resize(entity_index + 1, None);
        }
        let new_index = self.data.len();
        self.indices[entity_index] = Some(new_index);
        self.data.push(data);
        self.data_index.push(new_index);
    }

    pub fn remove(&mut self, entity_index: ENTITY_INDEX) -> Option<T> {
        let index_to_remove = self.indices[entity_index]?;
        let removed_data = self.data.swap_remove(index_to_remove);
        self.data_index.swap_remove(index_to_remove);

        // Update index of data swapped from the back.
        self.indices[self.data_index[index_to_remove]] = Some(index_to_remove);
        Some(removed_data)
    }

    pub fn get(&self, entity_index: ENTITY_INDEX) -> Option<&T> {
        Some(&self.data[self.indices.get(entity_index)?.clone()?])
    }

    pub fn get_mut(&mut self, entity_index: ENTITY_INDEX) -> Option<&T> {
        Some(&mut self.data[self.indices[entity_index]?])
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn data(&self) -> &Vec<T> {
        &self.data
    }
}
