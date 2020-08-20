use std::any::Any;
use std::any::TypeId;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::iter::Zip;

pub struct World {
    archetypes: Vec<Archetype>,
    archetype_id_to_archetype: HashMap<u64, usize>,
}

impl World {
    pub fn new() -> Self {
        Self {
            archetypes: Vec::new(),
            archetype_id_to_archetype: HashMap::new(),
        }
    }

    pub fn query<'a, 'b: 'a, QUERY: Query<'a, 'b>>(&'b mut self) -> QUERY::I {
        QUERY::iterator(self)
    }

    pub fn get_archetype<'a, 'b: 'a, T: ComponentBundle>(
        &'b mut self,
    ) -> Option<&'a mut Archetype> {
        let archetype_id = T::archetype_id();
        let index = self.archetype_id_to_archetype.get(&archetype_id).copied();
        if let Some(index) = index {
            Some(&mut self.archetypes[index])
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
                self.archetypes.push(T::archetype());
                self.archetype_id_to_archetype.insert(archetype_id, index);
                &mut self.archetypes[index]
            };
        data.insert_into_archetype(archetype);
    }
}

pub trait ComponentBundle {
    fn archetype_id() -> u64;
    fn archetype() -> Archetype;
    fn insert_into_archetype(self, archetype: &mut Archetype);
}

impl<A: Sized + 'static> ComponentBundle for (A,) {
    fn archetype_id() -> u64 {
        let mut s = DefaultHasher::new();
        [TypeId::of::<A>()].hash(&mut s);
        s.finish()
    }

    fn archetype() -> Archetype {
        let mut archetype = Archetype::new();
        archetype.add_component::<A>();
        archetype
    }

    fn insert_into_archetype(self, archetype: &mut Archetype) {
        archetype.components[0]
            .downcast_mut::<Vec<A>>()
            .unwrap()
            .push(self.0)
    }
}

impl<A: Sized + 'static, B: Sized + 'static> ComponentBundle for (A, B) {
    fn archetype_id() -> u64 {
        let mut s = DefaultHasher::new();
        [TypeId::of::<A>(), TypeId::of::<B>()].sort().hash(&mut s);
        s.finish()
    }

    fn archetype() -> Archetype {
        let mut archetype = Archetype::new();
        // These need to be sorted before being inserted.
        archetype.add_component::<A>();
        archetype.add_component::<B>();
        archetype
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

impl<A: Sized + 'static, B: Sized + 'static, C: Sized + 'static, D: Sized + 'static> ComponentBundle
    for (A, B, C, D)
{
    fn archetype_id() -> u64 {
        let mut s = DefaultHasher::new();
        [
            TypeId::of::<A>(),
            TypeId::of::<B>(),
            TypeId::of::<C>(),
            TypeId::of::<D>(),
        ]
        .sort()
        .hash(&mut s);
        s.finish()
    }

    fn archetype() -> Archetype {
        let mut archetype = Archetype::new();
        // These need to be sorted before being inserted.
        archetype.add_component::<A>();
        archetype.add_component::<B>();
        archetype.add_component::<C>();
        archetype.add_component::<D>();
        archetype
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
        archetype.components[2]
            .downcast_mut::<Vec<C>>()
            .unwrap()
            .push(self.2);
        archetype.components[3]
            .downcast_mut::<Vec<D>>()
            .unwrap()
            .push(self.3);
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
    fn iterator(world: &'b mut World) -> Self::I;
}

impl<'a, 'b: 'a, A: ComponentReference<'a, 'b>> Query<'a, 'b> for (A,) {
    type Item = A;
    type I = A::I;

    fn iterator(world: &'b mut World) -> Self::I {
        let archetype = world.get_archetype::<(A::ReferenceType,)>().unwrap();
        <A>::get_component_iter(&mut archetype.components[0])
    }
}

impl<'a, 'b: 'a, A: ComponentReference<'a, 'b>, B: ComponentReference<'a, 'b>> Query<'a, 'b>
    for (A, B)
{
    type Item = (A, B);
    type I = Zip<A::I, B::I>;

    fn iterator(world: &'b mut World) -> Self::I {
        let archetype = world
            .get_archetype::<(A::ReferenceType, B::ReferenceType)>()
            .unwrap();

        let (a, b) = archetype.components.split_at_mut(1);
        let a = A::get_component_iter(&mut a[0]);
        let b = B::get_component_iter(&mut b[0]);
        a.zip(b)
    }
}

impl<
        'a,
        'b: 'a,
        A: ComponentReference<'a, 'b>,
        B: ComponentReference<'a, 'b>,
        C: ComponentReference<'a, 'b>,
        D: ComponentReference<'a, 'b>,
    > Query<'a, 'b> for (A, B, C, D)
{
    type Item = (A, B, C, D);
    type I = MultiIter4<A::I, B::I, C::I, D::I>;

    fn iterator(world: &'b mut World) -> Self::I {
        let archetype = world
            .get_archetype::<(A::ReferenceType, B::ReferenceType)>()
            .unwrap();

        let (a, tail) = archetype.components.split_at_mut(1);
        let (b, tail) = tail.split_at_mut(1);
        let (c, d) = tail.split_at_mut(1);

        let a = A::get_component_iter(&mut a[0]);
        let b = B::get_component_iter(&mut b[0]);
        let c = C::get_component_iter(&mut c[0]);
        let d = D::get_component_iter(&mut d[0]);

        MultiIter4::new(a, b, c, d)
    }
}
pub trait ComponentReference<'a, 'b: 'a>: Sized {
    type ReferenceType: 'static;
    type I: Iterator<Item = Self> + 'a;

    fn get_component_iter(archetype: &'b mut Box<dyn Any>) -> Self::I;
}

impl<'a, 'b: 'a, T: 'static> ComponentReference<'a, 'b> for &'a T {
    type ReferenceType = T;
    type I = std::slice::Iter<'a, T>;

    fn get_component_iter(components: &'b mut Box<dyn Any>) -> Self::I {
        components
            .downcast_mut::<Vec<Self::ReferenceType>>()
            .unwrap()
            .iter()
    }
}

impl<'a, 'b: 'a, T: 'static> ComponentReference<'a, 'b> for &'a mut T {
    type ReferenceType = T;
    type I = std::slice::IterMut<'a, T>;

    fn get_component_iter(components: &'b mut Box<dyn Any>) -> Self::I {
        components
            .downcast_mut::<Vec<Self::ReferenceType>>()
            .unwrap()
            .iter_mut()
    }
}

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
