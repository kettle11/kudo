use std::any::Any;
use std::any::TypeId;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::iter::Zip;

pub struct World {
    archetypes: Vec<Archetype>,
    archetype_id_to_archetype: HashMap<u64, usize>,
    component_archetypes: HashMap<TypeId, Vec<(usize, usize)>>,
}

impl World {
    pub fn new() -> Self {
        Self {
            archetypes: Vec::new(),
            archetype_id_to_archetype: HashMap::new(),
            component_archetypes: HashMap::new(),
        }
    }

    pub fn query<'a, 'b: 'a, QUERY: Query<'a, 'b>>(&'b mut self) -> QUERY::I {
        QUERY::iterator(self)
    }

    pub fn get_archetype<'a, 'b: 'a, T: ComponentBundle>(&'b self) -> Option<&'a Archetype> {
        let archetype_id = T::archetype_id();
        let index = self.archetype_id_to_archetype.get(&archetype_id).copied();
        if let Some(index) = index {
            Some(&self.archetypes[index])
        } else {
            None
        }
    }

    pub fn spawn<T: ComponentBundle>(&mut self, data: T) {
        let archetype_id = T::archetype_id();

        let archetype =
            if let Some(archetype_index) = self.archetype_id_to_archetype.get(&archetype_id) {
                &mut self.archetypes[*archetype_index]
            } else {
                let index = self.archetypes.len();
                let (archetype, types) = T::archetype();
                self.archetypes.push(archetype);
                self.archetype_id_to_archetype.insert(archetype_id, index);
                // Keep track of which archetypes store a component.
                for (i, t) in types.iter().enumerate() {
                    //println!("Adding: {:?} to archetype {:?} at index {:?}", t, index, i);
                    if let Some(s) = self.component_archetypes.get_mut(t) {
                        s.push((index, i));
                    } else {
                        let v = vec![(index, i)];
                        self.component_archetypes.insert(*t, v);
                    }
                }
                &mut self.archetypes[index]
            };
        data.insert_into_archetype(archetype);
    }

    pub fn get_component_iter<'a, 'b: 'a, T: 'static + Sized>(&'b self) -> ComponentIter<'a, T> {
        let t = TypeId::of::<T>();
        let component_archetypes = &self.component_archetypes[&t];
        let data: Vec<&'a Vec<T>> = component_archetypes
            .iter()
            .map(|(archetype, index)| {
                /*
                println!(
                    "Looking up t: {:?} at archetype: {:?} and index: {:?}",
                    t, archetype, index
                );*/
                let archetype = &self.archetypes[*archetype];
                let components = &archetype.components[*index];
                components.downcast_ref::<Vec<T>>().unwrap()
            })
            .collect();
        ComponentIter::new(data)
    }
}

// Is there a way to make this use an outer iter and inner iter.
pub struct ComponentIter<'a, T> {
    data: Vec<&'a Vec<T>>,
    current_data: usize,
    current_iter: std::slice::Iter<'a, T>,
}
impl<'a, T> ComponentIter<'a, T> {
    pub fn new(data: Vec<&'a Vec<T>>) -> Self {
        let current_iter = data[0].iter();
        Self {
            data,
            current_data: 0,
            current_iter,
        }
    }
}

impl<'a, T: 'static + Sized> Iterator for ComponentIter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        self.current_iter.next().or_else(|| {
            // If we've reached the end of the current iter then advance to the next one.
            self.current_data += 1;
            if let Some(d) = self.data.get(self.current_data) {
                self.current_iter = d.iter();
                self.current_iter.next()
            } else {
                None
            }
        })
    }
}

// Clarity is needed around when and where TypeId is requested, and where it's stored.
pub trait ComponentBundle {
    fn types() -> Vec<TypeId>;
    fn archetype_id() -> u64;
    fn archetype() -> (Archetype, Vec<TypeId>);
    fn insert_into_archetype(self, archetype: &mut Archetype);
}

impl<A: Sized + 'static> ComponentBundle for (A,) {
    fn types() -> Vec<TypeId> {
        vec![TypeId::of::<A>()]
    }

    fn archetype_id() -> u64 {
        let mut s = DefaultHasher::new();
        [TypeId::of::<A>()].hash(&mut s);
        s.finish()
    }

    fn archetype() -> (Archetype, Vec<TypeId>) {
        let mut archetype = Archetype::new();
        archetype.add_component::<A>();
        (archetype, vec![TypeId::of::<A>()])
    }

    fn insert_into_archetype(self, archetype: &mut Archetype) {
        archetype.components[0]
            .downcast_mut::<Vec<A>>()
            .unwrap()
            .push(self.0)
    }
}

impl<A: Sized + 'static, B: Sized + 'static> ComponentBundle for (A, B) {
    fn types() -> Vec<TypeId> {
        let mut v = vec![TypeId::of::<A>(), TypeId::of::<B>()];
        v.sort();
        v
    }

    fn archetype_id() -> u64 {
        let mut s = DefaultHasher::new();
        [TypeId::of::<A>(), TypeId::of::<B>()].sort().hash(&mut s);
        s.finish()
    }

    fn archetype() -> (Archetype, Vec<TypeId>) {
        let mut archetype = Archetype::new();
        // These need to be sorted before being inserted.
        archetype.add_component::<A>();
        archetype.add_component::<B>();
        (archetype, vec![TypeId::of::<A>(), TypeId::of::<B>()])
    }

    fn insert_into_archetype(self, archetype: &mut Archetype) {
        // These need to be sorted before being inserted.
        archetype.components[0]
            .downcast_mut::<Vec<A>>()
            .unwrap()
            .push(self.0);
        archetype.components[1]
            .downcast_mut::<Vec<B>>()
            .unwrap()
            .push(self.1);
    }
}

/// A storage for the components of entities that share the same component.
pub struct Archetype {
    /// An array of Vecs that store components.
    pub(crate) components: Vec<Box<dyn Any>>,
}

impl Archetype {
    pub fn new() -> Self {
        Self {
            components: Vec::new(),
        }
    }

    pub fn add_component<T: 'static>(&mut self) {
        self.components.push(Box::new(Vec::<T>::new()));
    }
}

pub trait Query<'a, 'b: 'a> {
    type Item;
    type I: Iterator<Item = Self::Item> + 'a;
    fn iterator(world: &'b World) -> Self::I;
}

impl<'a, 'b: 'a, A: ComponentReference<'a, 'b>> Query<'a, 'b> for (A,) {
    type Item = A;
    type I = A::I;

    fn iterator(world: &'b World) -> Self::I {
        let archetype = world.get_archetype::<(A::ReferenceType,)>().unwrap();
        <A>::get_component_iter(world)
    }
}

impl<'a, 'b: 'a, A: ComponentReference<'a, 'b>, B: ComponentReference<'a, 'b>> Query<'a, 'b>
    for (A, B)
{
    type Item = (A, B);
    type I = Zip<A::I, B::I>;

    fn iterator(world: &'b World) -> Self::I {
        let archetype = world
            .get_archetype::<(A::ReferenceType, B::ReferenceType)>()
            .unwrap();

        let a = A::get_component_iter(world);
        let b = B::get_component_iter(world);
        a.zip(b)
    }
}

pub trait ComponentReference<'a, 'b: 'a>: Sized {
    type ReferenceType: 'static;
    type I: Iterator<Item = Self> + 'a;

    fn get_component_iter(world: &'b World) -> Self::I;
}

impl<'a, 'b: 'a, T: 'static> ComponentReference<'a, 'b> for &'a T {
    type ReferenceType = T;
    type I = ComponentIter<'a, T>;

    fn get_component_iter(world: &'b World) -> Self::I {
        world.get_component_iter()
    }
}

/*
impl<'a, 'b: 'a, T: 'static> ComponentReference<'a, 'b> for &'a mut T {
    type ReferenceType = T;
    type I = std::slice::IterMut<'a, T>;

    fn get_component_iter(world: &mut World) -> Self::I {
        components
            .downcast_mut::<Vec<Self::ReferenceType>>()
            .unwrap()
            .iter_mut()
    }
}*/

pub struct MultiIter4<I0: Iterator, I1: Iterator, I2: Iterator, I3: Iterator> {
    iterator: Zip<Zip<Zip<I0, I1>, I2>, I3>,
}

impl<I0: Iterator, I1: Iterator, I2: Iterator, I3: Iterator> MultiIter4<I0, I1, I2, I3> {
    pub fn new(first: I0, second: I1, third: I2, fourth: I3) -> Self {
        Self {
            iterator: first.zip(second).zip(third).zip(fourth),
        }
    }
}

impl<'a, I0: Iterator, I1: Iterator, I2: Iterator, I3: Iterator> Iterator
    for MultiIter4<I0, I1, I2, I3>
{
    type Item = (I0::Item, I1::Item, I2::Item, I3::Item);

    fn next(&mut self) -> Option<Self::Item> {
        self.iterator.next().map(|(((a, b), c), d)| (a, b, c, d))
    }
}
