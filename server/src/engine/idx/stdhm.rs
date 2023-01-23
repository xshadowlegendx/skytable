/*
 * Created on Mon Jan 23 2023
 *
 * This file is a part of Skytable
 * Skytable (formerly known as TerrabaseDB or Skybase) is a free and open-source
 * NoSQL database written by Sayan Nandan ("the Author") with the
 * vision to provide flexibility in data modelling without compromising
 * on performance, queryability or scalability.
 *
 * Copyright (c) 2023, Sayan Nandan <ohsayan@outlook.com>
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program. If not, see <https://www.gnu.org/licenses/>.
 *
*/

use super::{AsKey, AsKeyRef, AsValue, DummyMetrics, IndexBaseSpec, STIndex};
use std::{
    borrow::Borrow,
    collections::{
        hash_map::{Entry, Iter as StdMapIterKV, Keys as StdMapIterKey, Values as StdMapIterVal},
        HashMap as StdMap,
    },
    hash::BuildHasher,
    mem,
};

impl<K, V, S> IndexBaseSpec<K, V> for StdMap<K, V, S>
where
    S: BuildHasher + Default,
{
    const PREALLOC: bool = true;

    type Metrics = DummyMetrics;

    type IterKV<'a> = StdMapIterKV<'a, K, V>
    where
        Self: 'a,
        K: 'a,
        V: 'a;

    type IterKey<'a> = StdMapIterKey<'a, K, V>
    where
        Self: 'a,
        K: 'a;

    type IterValue<'a> = StdMapIterVal<'a, K, V>
    where
        Self: 'a,
        V: 'a;

    fn idx_init() -> Self {
        StdMap::with_hasher(S::default())
    }

    fn idx_init_with(s: Self) -> Self {
        s
    }

    fn idx_iter_kv<'a>(&'a self) -> Self::IterKV<'a> {
        self.iter()
    }

    fn idx_iter_key<'a>(&'a self) -> Self::IterKey<'a> {
        self.keys()
    }

    fn idx_iter_value<'a>(&'a self) -> Self::IterValue<'a> {
        self.values()
    }

    fn idx_metrics(&self) -> &Self::Metrics {
        &DummyMetrics
    }
}

impl<K, V, S> STIndex<K, V> for StdMap<K, V, S>
where
    K: AsKey,
    V: AsValue,
    S: BuildHasher + Default,
{
    fn st_compact(&mut self) {
        self.shrink_to_fit()
    }

    fn st_clear(&mut self) {
        self.clear()
    }

    fn st_insert(&mut self, key: K, val: V) -> bool {
        match self.entry(key) {
            Entry::Vacant(ve) => {
                ve.insert(val);
                true
            }
            _ => false,
        }
    }

    fn st_upsert(&mut self, key: K, val: V) {
        let _ = self.insert(key, val);
    }

    fn st_contains<Q>(&self, k: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: ?Sized + AsKeyRef,
    {
        self.contains_key(k)
    }

    fn st_get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: ?Sized + AsKeyRef,
    {
        self.get(key)
    }

    fn st_get_cloned<Q>(&self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: ?Sized + AsKeyRef,
    {
        self.get(key).cloned()
    }

    fn st_update<Q>(&mut self, key: &Q, val: V) -> bool
    where
        K: Borrow<Q>,
        Q: ?Sized + AsKeyRef,
    {
        self.get_mut(key).map(move |e| *e = val).is_some()
    }

    fn st_update_return<Q>(&mut self, key: &Q, val: V) -> Option<V>
    where
        K: Borrow<Q>,
        Q: ?Sized + AsKeyRef,
    {
        self.get_mut(key).map(move |e| {
            let mut new = val;
            mem::swap(&mut new, e);
            new
        })
    }

    fn st_delete<Q>(&mut self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: ?Sized + AsKeyRef,
    {
        self.remove(key).is_some()
    }

    fn st_delete_return<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: ?Sized + AsKeyRef,
    {
        self.remove(key)
    }
}
