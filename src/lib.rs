//! Ordered iterators.

#![cfg_attr(test, feature(core, test))]

#[cfg(test)]
extern crate test;

use std::cmp::Ordering::*;
use std::iter::Peekable;
use std::collections::{
    btree_map, btree_set,
    vec_map,
    bit_set
};

/// Allows an iterator to be do an inner join with another
/// iterator to combine their values or filter based on their keys.
/// this trait is applied to an iterator over a map like structure
pub trait OrderedMapIterator: Iterator<Item=(<Self as OrderedMapIterator>::Key, <Self as OrderedMapIterator>::Val)> + Sized {
    type Key;
    type Val;
    /// join two ordered maps together
    fn inner_join_map<I>(self, map: I) -> InnerJoinMapIterator<Self, I>
    where I: OrderedMapIterator<Key=Self::Key> {
        InnerJoinMapIterator {
            a: self,
            b: map
        }
    }

    /// filter an ordered map with an ordered set
    fn inner_join_set<I>(self, set: I) -> InnerJoinMapSetIterator<Self, I>
    where I: OrderedSetIterator<Item=Self::Key> {
        InnerJoinMapSetIterator {
            map: self,
            set: set
        }
    }

    /// Join an ordered iterator with the right ordered iterator. The
    /// new iterator will return a key value pair for every key in
    /// either iterator. If a key is present in both iterators they
    /// will be returned together (two values). If a value is in the Right,
    /// but not the left iterator it will be return without the value in the
    /// left iterator. If the value is in the left iterator by not the right
    /// that will be return without the value from the left iterator.
    fn outer_join<I>(self, right: I) -> OuterJoinIterator<Self, I>
    where I: OrderedMapIterator<Key=Self::Key> {
        OuterJoinIterator {
            left: self.peekable(),
            right: right.peekable()
        }
    }
}

/// Allows an iterator to be do an inner join with another
/// iterator to combine their values or filter based on their keys.
/// this trait is applied to an iterator over a set like structure
pub trait OrderedSetIterator: Iterator + Sized {
    /// join two ordered maps together
    fn inner_join_map<I>(self, map: I) -> InnerJoinMapSetIterator<I, Self>
    where I: OrderedMapIterator<Key=Self::Item> {
        InnerJoinMapSetIterator {
            map: map,
            set: self
        }
    }

    /// filter an ordered map with an ordered set
    fn inner_join_set<I>(self, map: I) -> InnerJoinSetIterator<Self, I>
    where I: OrderedSetIterator<Item=Self::Item> {
        InnerJoinSetIterator {
            a: self,
            b: map
        }
    }
}

pub struct InnerJoinMapIterator<A, B> {a: A, b: B}
pub struct InnerJoinMapSetIterator<A, B> {map: A, set: B}
pub struct InnerJoinSetIterator<A, B> {a: A, b: B}
pub struct OuterJoinIterator<A: Iterator, B: Iterator> {
    left: Peekable<A>,
    right: Peekable<B>,
}

impl<A, B> Iterator for InnerJoinMapIterator<A, B>
where A: OrderedMapIterator,
      B: OrderedMapIterator<Key=A::Key>,
      A::Key: Ord,
{

    type Item = (A::Key, (A::Val, B::Val));

    fn next(&mut self) -> Option<(A::Key, (A::Val, B::Val))> {
        let (mut key_a, mut data_a) = match self.a.next() {
            None => return None,
            Some((key, data)) => (key, data)
        };

        let (mut key_b, mut data_b) = match self.b.next() {
            None => return None,
            Some((key, data)) => (key, data)
        };

        loop {
            match key_a.cmp(&key_b) {
                Less => {
                    match self.a.next() {
                        None => return None,
                        Some((key, data)) => {
                            key_a = key;
                            data_a = data;
                        }
                    };
                },
                Equal => return Some((key_a, (data_a, data_b))),
                Greater => {
                    match self.b.next() {
                        None => return None,
                        Some((key, data)) => {
                            key_b = key;
                            data_b = data;
                        }
                    };
                }
            }
        }
    }
}


impl<A, B> Iterator for InnerJoinSetIterator<A, B>
where A: OrderedSetIterator,
      B: OrderedSetIterator<Item=A::Item>,
      A::Item: Ord,
{

    type Item = A::Item;

    fn next(&mut self) -> Option<A::Item> {
        let mut key_a = match self.a.next() {
            None => return None,
            Some(key) => key
        };

        let mut key_b = match self.b.next() {
            None => return None,
            Some(key) => key
        };

        loop {
            match key_a.cmp(&key_b) {
                Less => {
                    match self.a.next() {
                        None => return None,
                        Some(key) => { key_a = key; }
                    };
                },
                Equal => return Some(key_a),
                Greater => {
                    match self.b.next() {
                        None => return None,
                        Some(key) => { key_b = key; }
                    };
                }
            }
        }
    }
}

impl<MapIter, SetIter> Iterator for InnerJoinMapSetIterator<MapIter, SetIter>
where SetIter: OrderedSetIterator,
      MapIter: OrderedMapIterator<Key=SetIter::Item>,
      MapIter::Key: Ord,
{

    type Item = (MapIter::Key, MapIter::Val);

    fn next(&mut self) -> Option<(MapIter::Key, MapIter::Val)> {
        let mut key_set = match self.set.next() {
            None => return None,
            Some(key) => key
        };

        let (mut key_map, mut data) = match self.map.next() {
            None => return None,
            Some((key, data)) => (key, data)
        };

        loop {
            match key_set.cmp(&key_map) {
                Less => {
                    match self.set.next() {
                        None => return None,
                        Some(key) => { key_set = key; }
                    };
                },
                Equal => return Some((key_set, data)),
                Greater => {
                    match self.map.next() {
                        None => return None,
                        Some((key, d)) => {
                            key_map = key;
                            data = d;
                        }
                    };
                }
            }
        }
    }
}

impl<A, B> Iterator for OuterJoinIterator<A, B>
where A: OrderedMapIterator,
      B: OrderedMapIterator<Key=A::Key>,
      A::Key: Ord + Eq,
{

    type Item = (A::Key, (Option<A::Val>, Option<B::Val>));

    fn next(&mut self) -> Option<(A::Key, (Option<A::Val>, Option<B::Val>))> {
        let which = match (self.left.peek(), self.right.peek()) {
            (Some(&(ref ka, _)), Some(&(ref kb, _))) => kb.cmp(ka),
            (None, Some(_)) => Less,
            (Some(_), None) => Greater,
            (None, None) => return None
        };

        match which {
            Equal => {
                let ((k, a), (_, b)) =
                    (self.left.next().expect("no value found"),
                     self.right.next().expect("no value found"));

                Some((k, (Some(a), Some(b))))
            }
            Less => {
                let (k, v) = self.right.next().expect("no value found");
                Some((k, (None, Some(v))))
            }
            Greater => {
                let (k, v) = self.left.next().expect("no value found");
                Some((k, (Some(v), None)))
            }
        }
    }
}

impl<'a, K: Ord> OrderedSetIterator for btree_set::Iter<'a, K> {}
impl<'a, K: Ord, V> OrderedMapIterator for btree_map::Iter<'a, K, V> {
    type Key = &'a K;
    type Val = &'a V;
}

impl<K: Ord, V> OrderedMapIterator for btree_map::IntoIter<K, V> {
    type Key = K;
    type Val = V;
}

impl<'a, K: Ord, V> OrderedMapIterator for btree_map::IterMut<'a, K, V> {
    type Key = &'a K;
    type Val = &'a mut V;
}

impl<'a, K: Ord, V> OrderedSetIterator for btree_map::Keys<'a, K, V> {}

impl<'a, V> OrderedMapIterator for vec_map::Iter<'a, V> {
    type Key = usize;
    type Val = &'a V;
}

impl<'a> OrderedSetIterator for bit_set::Iter<'a> {}

impl<A, B> OrderedMapIterator for InnerJoinMapIterator<A, B>
where A: OrderedMapIterator,
      B: OrderedMapIterator<Key=A::Key>,
      A::Key: Ord,
{
    type Key = A::Key;
    type Val = (A::Val, B::Val);
}

impl<A, B> OrderedMapIterator for InnerJoinMapSetIterator<A, B>
where A: OrderedMapIterator,
      B: OrderedSetIterator<Item=A::Key>,
      A::Key: Ord,
{
    type Key = A::Key;
    type Val = A::Val;
}

impl<A, B> OrderedSetIterator for InnerJoinSetIterator<A, B>
where A: OrderedSetIterator,
      B: OrderedSetIterator<Item=A::Item>,
      A::Item: Ord,
{}

#[cfg(test)]
mod tests {
    use test::Bencher;
    use test;

    use super::{OrderedSetIterator, OrderedMapIterator};

    #[test]
    fn join_two_sets() {
        use std::collections::BTreeSet;

        let powers_of_two: BTreeSet<i32> = range(1, 10).map(|x| x * 2).collect();
        let powers_of_three: BTreeSet<i32> = range(1, 10).map(|x| x * 3).collect();

        let expected = vec![6, 12, 18];

        let powers_of_two_and_three: Vec<i32> =
            powers_of_two.iter()
            .inner_join_set(powers_of_three.iter())
            .map(|&x| x)
            .collect();

        assert_eq!(expected, powers_of_two_and_three);
    }

    #[test]
    fn join_three_sets() {
        use std::collections::BTreeSet;

        let powers_of_two: BTreeSet<i32> = range(1, 100).map(|x| x * 2).collect();
        let powers_of_three: BTreeSet<i32> = range(1, 100).map(|x| x * 3).collect();
        let powers_of_five: BTreeSet<i32> = range(1, 100).map(|x| x * 5).collect();

        let expected = vec![30, 60, 90, 120, 150, 180];

        let powers_of_two_and_three: Vec<i32> =
            powers_of_two.iter()
            .inner_join_set(powers_of_three.iter())
            .inner_join_set(powers_of_five.iter())
            .map(|&x| x)
            .collect();

        assert_eq!(expected, powers_of_two_and_three);
    }

    #[test]
    fn join_two_maps() {
        use std::collections::BTreeMap;

        let powers_of_two: BTreeMap<i32, i32> = range(1, 10).map(|x| (x * 2, x)).collect();
        let powers_of_three: BTreeMap<i32, i32> = range(1, 10).map(|x| (x * 3, x)).collect();

        let mut powers_of_two_and_three =
            powers_of_two.iter().inner_join_map(powers_of_three.iter())
            .map(|(&k, (&a, &b))| (k, a, b));

        assert_eq!(Some((6, 3, 2)), powers_of_two_and_three.next());
        assert_eq!(Some((12, 6, 4)), powers_of_two_and_three.next());
        assert_eq!(Some((18, 9, 6)), powers_of_two_and_three.next());
        assert_eq!(None, powers_of_two_and_three.next());
    }

    #[test]
    fn join_two_maps_to_set() {
        use std::collections::{BTreeMap, BTreeSet};

        let powers_of_two: BTreeSet<i32> = range(1, 10).map(|x| x * 2).collect();
        let powers_of_three: BTreeMap<i32, i32> = range(1, 10).map(|x| (x * 3, x)).collect();

        let mut powers_of_two_and_three =
            powers_of_two.iter().inner_join_map(powers_of_three.iter())
            .map(|(&k, &a)| (k, a));

        assert_eq!(Some((6, 2)), powers_of_two_and_three.next());
        assert_eq!(Some((12, 4)), powers_of_two_and_three.next());
        assert_eq!(Some((18, 6)), powers_of_two_and_three.next());
        assert_eq!(None, powers_of_two_and_three.next());
    }

    #[test]
    fn outer_join_fizz_buzz() {
        use std::collections::BTreeMap;

        let mul_of_three: BTreeMap<i32, i32> = range(0, 100).map(|x| (x*3, x)).collect();
        let mul_of_five: BTreeMap<i32, i32> = range(0, 100).map(|x| (x*5, x)).collect();

        let mut fizz_buzz = BTreeMap::new();

        for (key, (three, five)) in mul_of_three.iter()
                                                .outer_join(mul_of_five.iter()) {
            fizz_buzz.insert(key, (three.is_some(), five.is_some()));
        }

        let res: BTreeMap<i32, String> = range(1, 100).map(|i|
            (i, match fizz_buzz.get(&i) {
                None => format!("{}", i),
                Some(&(true, false)) => format!("Fizz"),
                Some(&(false, true)) => format!("Buzz"),
                Some(&(true, true)) => format!("FizzBuzz"),
                Some(&(false, false)) => panic!("Outer join failed...")
            })).collect();

        for i in range(1, 100) {
            match (i % 3, i % 5) {
                (0, 0) => assert_eq!("FizzBuzz", res[i].as_slice()),
                (0, _) => assert_eq!("Fizz", res[i].as_slice()),
                (_, 0) => assert_eq!("Buzz", res[i].as_slice()),
                _ => assert_eq!(format!("{}", i).as_slice(), res[i].as_slice())
            }
        }
    }


    #[bench]
    pub fn inner_join_map(b: &mut test::Bencher) {
        use std::collections::BTreeSet;

        let powers_of_two: BTreeSet<u32> = range(1, 1000000).map(|x| x * 2).collect();
        let powers_of_three: BTreeSet<u32> = range(1, 1000000).map(|x| x * 3).collect();

        b.iter(||{
            for x in powers_of_two.iter()
                .inner_join_set(powers_of_three.iter()) {

                test::black_box(x);
            }
        })
    }
}
