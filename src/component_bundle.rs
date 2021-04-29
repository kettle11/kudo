use crate::{Archetype, ComponentChannelStorage, ComponentTrait, Entity, EntityLocation, World};
use std::any::TypeId;
// A DynamicBundle struct could be implemented that could spawn things non-statically.
pub trait ComponentBundle: ComponentTrait {
    fn spawn_in_world(self, world: &mut World) -> Entity;
}

// This macro is a little funky because it needs to reorder insert based on type ids.
// This code takes a ComponentBundle and finds an appropriate archetype, creating one if necessary.
// It then inserts the tuple members in order based on their TypeIds.
macro_rules! component_bundle_impl {
    ($count: expr, $(($name: ident, $index: tt)),*) => {
        impl< $($name: ComponentTrait),*> ComponentBundle for ($($name,)*) {
            fn spawn_in_world(self, world: &mut World) -> Entity {
                let new_entity = world.entities.new_entity_handle();
                let mut type_ids_and_order = [$(($index, TypeId::of::<$name>())), *];

                debug_assert!(
                    type_ids_and_order.windows(2).all(|x| x[0].1 != x[1].1),
                    "`ComponentBundles cannot have duplicate types!"
                );

                type_ids_and_order.sort_unstable_by_key(|a| a.1);
                let type_ids = [$(type_ids_and_order[$index].1), *];

                // Find the archetype in the world
                let archetype_index = match world.storage_lookup.get_archetype_with_components(&type_ids) {
                    Some(index) => index,
                    None => {
                        let mut new_archetype = Archetype::new();
                        // Insert each channel
                        $(new_archetype.push_channel(ComponentChannelStorage::new::<$name>());)*
                        // Sort the channels
                        new_archetype.sort_channels();

                        let new_archetype_index = world.archetypes.len();
                        world.archetypes.push(new_archetype);
                        world.storage_lookup
                            .new_archetype(new_archetype_index, &type_ids);
                        new_archetype_index
                    }
                };
                let archetype = &mut world.archetypes[archetype_index];

                // Is there a better way to map the original ordering to the sorted ordering?
                let mut order = [0; $count];
                for i in 0..order.len() {
                    order[type_ids_and_order[i].0] = i;
                }

                $(archetype.get_channel_mut(order[$index]).push(self.$index);)*

                let index_within_archetype = archetype.entities.get_mut().unwrap().len();
                archetype.entities.get_mut().unwrap().push(new_entity);

                *world.entities.get_at_index_mut(new_entity.index) = Some(EntityLocation {
                    archetype_index,
                    index_within_archetype
                });
                new_entity
            }
        }
    };
}

// I don't like this macro. It is tedious to edit the below definitions.
component_bundle_impl! {1, (A, 0)}
component_bundle_impl! {2, (A, 0), (B, 1)}
component_bundle_impl! {3, (A, 0), (B, 1), (C, 2)}
component_bundle_impl! {4, (A, 0), (B, 1), (C, 2), (D, 3)}
component_bundle_impl! {5, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4)}
component_bundle_impl! {6, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5)}
component_bundle_impl! {7, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6)}
component_bundle_impl! {8, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6), (H, 7)}
component_bundle_impl! {9, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6), (H, 7), (I, 8)}
component_bundle_impl! {10, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6), (H, 7), (I, 8), (J, 9)}
component_bundle_impl! {11, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6), (H, 7), (I, 8), (J, 9), (K, 10)}
component_bundle_impl! {12, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6), (H, 7), (I, 8), (J, 9), (K, 10), (L, 11)}
