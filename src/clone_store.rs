use crate::*;
use std::collections::HashMap;
use std::sync::Arc;
use std::{any::TypeId, borrow::BorrowMut};

#[derive(Clone)]
pub struct CloneStore {
    data: Arc<HashMap<TypeId, Box<dyn ClonerTrait>>>,
}

pub struct CloneStoreBuilder {
    data: HashMap<TypeId, Box<dyn ClonerTrait>>,
}

impl CloneStoreBuilder {
    pub fn build(self) -> CloneStore {
        CloneStore {
            data: Arc::new(self.data),
        }
    }

    pub fn register_type<T: Clone + Sync + Send + 'static>(&mut self) {
        let type_id = TypeId::of::<T>();

        self.data.borrow_mut().insert(
            type_id,
            Box::new(Cloner {
                phantom: std::marker::PhantomData::<T>,
            }),
        );
    }
}

impl CloneStore {
    pub fn new() -> CloneStoreBuilder {
        CloneStoreBuilder {
            data: HashMap::new(),
        }
    }

    pub(crate) fn get(&self, type_id: TypeId) -> Option<&dyn ClonerTrait> {
        self.data.get(&type_id).map(|b| b.as_ref())
    }
}

pub(crate) trait ClonerTrait: Sync + Send
where
    (dyn ClonerTrait + 'static): Sync + Send,
{
    fn clone_component(
        &self,
        origin_index: usize,
        origin_channel: &mut ComponentStore,
        destination_channel: &mut ComponentStore,
    );

    fn clone_component_into_self(&self, entity_index: usize, channel: &mut ComponentStore);
}

struct Cloner<T> {
    phantom: std::marker::PhantomData<T>,
}

impl<T: Clone + Sync + Send + 'static> ClonerTrait for Cloner<T> {
    fn clone_component(
        &self,
        origin_index: usize,
        origin_channel: &mut ComponentStore,
        destination_channel: &mut ComponentStore,
    ) {
        let data: T = component_vec_to_mut::<T>(origin_channel.data.as_mut())[origin_index].clone();
        component_vec_to_mut(destination_channel.data.as_mut()).push(data)
    }

    fn clone_component_into_self(&self, entity_index: usize, channel: &mut ComponentStore) {
        let v: &mut Vec<T> = component_vec_to_mut::<T>(channel.data.as_mut());
        v.push(v[entity_index].clone())
    }
}
