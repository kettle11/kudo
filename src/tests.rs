#[test]
fn async_query() {
    use crate::*;

    let mut world = World::new();
    world.spawn((3 as i32,));

    async fn f(_: Query<'_, (&f32,)>) -> f32 {
        10.
    }

    // This returns something that borrows world.
    let _ = (f).run(&world).unwrap();
}

#[test]
fn system_with_result() {
    use crate::*;

    let mut world = World::new();
    world.spawn((1 as i32,));
    world.spawn((2 as i32,));
    world.spawn((3 as i32,));

    // This returns something that borrows world.
    let result = (|q: Query<(&i32,)>| -> i32 { q.iter().sum() })
        .run(&world)
        .unwrap();

    assert!(result == 6);
}

#[test]
fn system_single_query() {
    use crate::*;
    let mut world = World::new();
    world.spawn((3 as i32,));

    (|i: &i32| assert!(*i == 3)).run(&world).unwrap()
}

#[test]
fn system_multiple_parameters() {
    use crate::*;
    let mut world = World::new();
    world.spawn((3 as i32,));
    world.spawn((4 as i32,));

    (|_i: &i32, _q: Query<(&i32,)>| {}).run(&world).unwrap()
}

/// This test intentionally creates overlapping borrows
#[test]
fn conflicting_queries() {
    use crate::*;
    let mut world = World::new();
    world.spawn((3 as i32,));
    world.spawn((4 as i32,));

    assert!((|_: &mut i32, _: &i32| {}).run(&world).is_err())
}

#[test]
fn mutable_closure() {
    use crate::*;
    let mut world = World::new();
    world.spawn((3 as i32,));

    let mut internal_data = 0;
    let mut closure = |num: &i32| {
        internal_data += num;
    };

    (&mut closure).run(&world).unwrap();
    (&mut closure).run(&world).unwrap();
}

#[test]
fn box_system() {
    use crate::*;
    let mut world = World::new();
    world.spawn((3 as i32,));

    let mut boxed_system = (|i: &i32| assert!(*i == 3)).box_system();
    (boxed_system)(&world).unwrap();
}

#[test]
fn three_parameters() {
    use crate::*;
    let mut world = World::new();
    world.spawn((3 as i32,));
    world.spawn((4 as i32,));

    (|_q: Query<(&i32, &bool, &f32)>| {}).run(&world).unwrap()
}

#[test]
fn simple_spawn() {
    use crate::*;

    let mut world = World::new();
    world.spawn((1 as i32,));
}

#[test]
fn add_component0() {
    use crate::*;

    println!("TYPE ID OF I32: {:?}", std::any::TypeId::of::<i32>());
    println!("TYPE ID OF BOOL: {:?}", std::any::TypeId::of::<bool>());

    let mut world = World::new();
    let entity = world.spawn((1 as i32,));
    println!("HERE");

    world.add_component(&entity, true);
    let result = (|q: Query<(&i32, &bool)>| -> bool { *q.iter().next().unwrap().1 })
        .run(&world)
        .unwrap();

    world.add_component(&entity, true);

    assert!(result == true);
}

#[test]
fn add_component1() {
    use crate::*;

    let mut world = World::new();

    let entity = world.spawn((true,));
    world.add_component(&entity, 10 as i32);
    let result = (|q: Query<(&i32, &bool)>| -> bool { *q.iter().next().unwrap().1 })
        .run(&world)
        .unwrap();

    world.add_component(&entity, true);

    assert!(result == true);
}

#[test]
fn remove_component0() {
    use crate::*;

    let mut world = World::new();
    let entity = world.spawn((1 as i32, true));
    assert!(world.remove_component::<bool>(&entity) == Some(true));
}

#[test]
fn iterate_entities() {
    use crate::*;
    let mut world = World::new();
    world.spawn((3 as i32,));
    world.spawn((4 as i32,));

    (|i: Query<(&i32,)>| {
        let entities: Vec<Entity> = i.entities().collect();
        assert!(entities[0].index() == 0);
        assert!(entities[1].index() == 1);
    })
    .run(&world)
    .unwrap()
}

#[test]
fn option_query() {
    use crate::*;
    let mut world = World::new();
    world.spawn((3 as i32, true));
    world.spawn((6 as i32, true));
    world.spawn((false,));

    (|values: Query<(Option<&i32>, &bool)>| {
        assert!(values.iter().count() == 3);
    })
    .run(&world)
    .unwrap()
}

#[test]
fn sum() {
    use crate::*;

    struct Position([f32; 3]);
    struct Velocity([f32; 3]);
    struct Rotation([f32; 3]);

    let mut world = World::new();

    for _ in 0..10 {
        world.spawn((
            Position([1., 0., 0.]),
            Rotation([1., 0., 0.]),
            Velocity([1., 0., 0.]),
        ));
    }

    let mut query = world
        .query::<(&Velocity, &mut Position, &Rotation)>()
        .unwrap();
    for (velocity, position, _rotation) in query.iter_mut() {
        position.0[0] += velocity.0[0];
        position.0[1] += velocity.0[1];
        position.0[2] += velocity.0[2];
    }

    for (_velocity, position, _rotation) in query.iter_mut() {
        assert!(position.0 == [2., 0., 0.]);
    }
}

#[test]
fn mutable_query() {
    use crate::*;

    let mut world = World::new();
    world.spawn((2 as i32,));

    (|q: &mut i32| {
        *q += 1;
    })
    .run(&world)
    .unwrap();
}

#[test]
fn get_component_mut() {
    use crate::*;
    let mut world = World::new();
    let entity = world.spawn((10 as f32, true));

    let mut query = world.query::<(&f32, &mut bool)>().unwrap();

    assert!(query.get_component_mut::<f32>(&entity).is_none());
    assert!(query.get_component_mut::<bool>(&entity).is_some());
}

#[test]
fn get_component_fail() {
    use crate::*;
    let mut world = World::new();
    let entity = world.spawn((10 as f32,));
    let query = world.query::<(&f32,)>().unwrap();
    assert!(query.get_component::<bool>(&entity).is_none());
}

#[test]
fn generic_system() {
    use crate::*;
    use std::fmt::Debug;

    let mut world = World::new();
    world.spawn((false,));

    fn test_system<T: Debug>(_data: &mut T) {}
    test_system::<bool>.run(&world).unwrap();
}

#[test]
fn generic_query() {
    use crate::*;

    let mut world = World::new();
    world.spawn((false,));

    fn test_system<WORLD: WorldTrait, Q: for<'a> QueryTrait<'a, WORLD>>(q: Q) {}
    test_system::<World, Query<(&bool,)>>.run(&world).unwrap();
}

#[test]
fn clone() {
    use crate::*;

    let mut world = World::new();
    world.register_clone_type::<bool>();
    let entity = world.spawn((false,));
    world.clone_entity(&entity).unwrap();
    assert!(world.query::<(&bool,)>().unwrap().iter().count() == 2);
}

#[test]
fn fail_to_clone() {
    use crate::*;
    let mut world = World::new();
    let entity = world.spawn((false,));
    assert!(world.clone_entity(&entity).is_none());
}

#[test]
fn hierarchy() {
    use crate::*;
    let mut world = World::new();
    let parent = world.spawn((0,));
    let child = world.spawn((1,));

    world.set_parent(Some(&parent), &child);
}

#[test]
fn despawn() {
    use crate::*;
    let mut world = World::new();
    let entity = world.spawn((0,));
    world.despawn(&entity).unwrap();
}

#[test]
fn hierarchy_despawn() {
    use crate::*;
    let mut world = World::new();
    let parent = world.spawn((0,));
    let child = world.spawn((1,));

    world.set_parent(Some(&parent), &child);
    world.despawn(&parent).unwrap();
}

#[test]
fn clone_world() {
    use crate::*;
    let mut world = CloneableWorld::new();
    world.spawn((true,));

    let world_cloned = world.clone();
    assert!(world_cloned.query::<(&bool,)>().unwrap().iter().count() == 1)
}
