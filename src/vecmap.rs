//! Implements a simple typesafe map for keys that are always consecutive.
//! Useful sometimes to avoid the overhead of a `HashMap`.

use std::marker::PhantomData;

pub struct VecMap<K, V> where K: IndexFor<V> {
    contents: Vec<V>,
    _key_type: PhantomData<K>,
}

pub trait IndexFor<V>: Copy + Into<usize> + CheckedFrom<usize> { }

// TODO: Remove this if TryFrom is ever stabilized
pub trait CheckedFrom<T>: Sized {
    fn checked_from(T) -> Option<Self>;
}

impl<K, V> VecMap<K, V> where K: IndexFor<V> {
    pub fn with_capacity(capacity: usize) -> Self {
        VecMap {
            contents: Vec::with_capacity(capacity),
            _key_type: PhantomData,
        }
    }

    pub fn get(&self, key: K) -> Result<&V, IndexErr<K>> {
        let i: usize = key.into();
        self.contents.get(i).ok_or(IndexErr::OutOfBounds(key))
    }

    pub fn get_mut(&mut self, key: K) -> Result<&mut V, IndexErr<K>> {
        let i: usize = key.into();
        self.contents.get_mut(i).ok_or(IndexErr::OutOfBounds(key))
    }

    pub fn push(&mut self, value: V) -> Result<K, IndexErr<K>> {
        let key: Option<K> = CheckedFrom::checked_from(self.contents.len());

        if let Some(key) = key {
            self.contents.push(value);
            Ok(key)
        } else {
            Err(IndexErr::ReprOverflow(self.contents.len()))
        }
    }

    pub fn len(&self) -> usize {
        self.contents.len()
    }

    pub fn iter(&self) -> IterPairs<K, V> {
        IterPairs(self, 0)
    }
}

pub struct IterPairs<'a, K: 'a, V: 'a>(&'a VecMap<K, V>, usize)
where K: IndexFor<V>;

impl<K, V> From<Vec<V>> for VecMap<K, V> where K: IndexFor<V> {
    fn from(vec: Vec<V>) -> Self {
        VecMap {
            contents: vec,
            _key_type: PhantomData,
        }
    }
}

/*
impl<K, V> From<VecMap<K, V>> for Vec<V> where K: IndexFor<V> {
    fn from(map: VecMap<K, V>) -> Self {
        map.contents
    }
}
*/

impl<K, V> AsRef<[V]> for VecMap<K, V> where K: IndexFor<V> {
    fn as_ref(&self) -> &[V] {
        self.contents.as_ref()
    }
}

#[derive(Debug)]
pub enum IndexErr<K> {
    OutOfBounds(K),
    ReprOverflow(usize),
}

impl<K, V> Clone for VecMap<K, V> where K: IndexFor<V>, V: Clone {
    fn clone(&self) -> Self {
        VecMap {
            contents: self.contents.clone(),
            _key_type: PhantomData,
        }
    }
}

impl<'a, K, V> Iterator for IterPairs<'a, K, V> where K: IndexFor<V> {
    type Item = (K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        let key = K::checked_from(self.1).unwrap();
        self.1 += 1;
        self.0.get(key).ok().map(|v| (key, v))
    }
}

use std::fmt;

impl<K, V> fmt::Debug for VecMap<K, V> where K: IndexFor<V>, V: fmt::Debug {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", &self.contents)
    }
}
