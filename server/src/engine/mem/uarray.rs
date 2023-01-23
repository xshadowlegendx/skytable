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

use core::{
    fmt,
    iter::FusedIterator,
    mem::MaybeUninit,
    ops::{Deref, DerefMut},
    ptr, slice,
};

pub struct UArray<const N: usize, T> {
    a: [MaybeUninit<T>; N],
    l: usize,
}

impl<const N: usize, T> UArray<N, T> {
    const NULL: MaybeUninit<T> = MaybeUninit::uninit();
    const NULLED_ARRAY: [MaybeUninit<T>; N] = [Self::NULL; N];
    #[inline(always)]
    pub const fn new() -> Self {
        Self {
            a: Self::NULLED_ARRAY,
            l: 0,
        }
    }
    #[inline(always)]
    pub const fn len(&self) -> usize {
        self.l
    }
    #[inline(always)]
    pub const fn capacity(&self) -> usize {
        N
    }
    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }
    #[inline(always)]
    pub fn push(&mut self, v: T) {
        if self.l == N {
            panic!("stack,capof");
        }
        unsafe {
            // UNSAFE(@ohsayan): verified length is smaller
            self.push_unchecked(v);
        }
    }
    pub fn remove(&mut self, idx: usize) -> T {
        if idx >= self.len() {
            panic!("out of range. idx is `{idx}` but len is `{}`", self.len());
        }
        unsafe {
            // UNSAFE(@ohsayan): verified idx < l
            self.remove_unchecked(idx)
        }
    }
    /// SAFETY: idx < self.l
    unsafe fn remove_unchecked(&mut self, idx: usize) -> T {
        // UNSAFE(@ohsayan): Verified idx
        let target = self.a.as_mut_ptr().add(idx).cast::<T>();
        // UNSAFE(@ohsayan): Verified idx
        let ret = ptr::read(target);
        // UNSAFE(@ohsayan): ov; not-null; correct len
        ptr::copy(target.add(1), target, self.len() - idx - 1);
        ret
    }
    #[inline(always)]
    /// SAFETY: self.l < N
    unsafe fn push_unchecked(&mut self, v: T) {
        // UNSAFE(@ohsayan): verified correct offsets (N)
        self.a.as_mut_ptr().add(self.l).write(MaybeUninit::new(v));
        // UNSAFE(@ohsayan): all G since l =< N
        self.incr_len();
    }
    pub fn as_slice(&self) -> &[T] {
        unsafe {
            // UNSAFE(@ohsayan): ptr is always valid and len is correct, due to push impl
            slice::from_raw_parts(self.a.as_ptr() as *const T, self.l)
        }
    }
    pub fn as_slice_mut(&mut self) -> &mut [T] {
        unsafe {
            // UNSAFE(@ohsayan): ptr is always valid and len is correct, due to push impl
            slice::from_raw_parts_mut(self.a.as_mut_ptr() as *mut T, self.l)
        }
    }
    #[inline(always)]
    unsafe fn set_len(&mut self, l: usize) {
        self.l = l;
    }
    #[inline(always)]
    unsafe fn incr_len(&mut self) {
        self.set_len(self.len() + 1)
    }
}

impl<const N: usize, T: Clone> Clone for UArray<N, T> {
    fn clone(&self) -> Self {
        self.iter().cloned().collect()
    }
}

impl<const M: usize, const N: usize, T: PartialEq> PartialEq<UArray<M, T>> for UArray<N, T> {
    fn eq(&self, other: &UArray<M, T>) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl<const N: usize, T> Drop for UArray<N, T> {
    fn drop(&mut self) {
        if !self.is_empty() {
            unsafe {
                // UNSAFE(@ohsayan): as_slice_mut returns a correct offset
                ptr::drop_in_place(self.as_slice_mut())
            }
        }
    }
}

impl<const N: usize, T> Deref for UArray<N, T> {
    type Target = [T];
    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<const N: usize, T> DerefMut for UArray<N, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_slice_mut()
    }
}

impl<const N: usize, T> FromIterator<T> for UArray<N, T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut slf = Self::new();
        iter.into_iter().for_each(|v| slf.push(v));
        slf
    }
}

impl<const N: usize, T> Extend<T> for UArray<N, T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        iter.into_iter().for_each(|v| self.push(v))
    }
}

impl<const N: usize, T: fmt::Debug> fmt::Debug for UArray<N, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

pub struct IntoIter<const N: usize, T> {
    i: usize,
    l: usize,
    d: UArray<N, T>,
}

impl<const N: usize, T> IntoIter<N, T> {
    fn _next(&mut self) -> Option<T> {
        if self.i == self.l {
            return None;
        }
        unsafe {
            // UNSAFE(@ohsayan): Below length, so this is legal
            let target = self.d.a.as_ptr().add(self.i) as *mut T;
            // UNSAFE(@ohsayan): Again, non-null and part of our stack
            let ret = ptr::read(target);
            self.i += 1;
            Some(ret)
        }
    }
}

impl<const N: usize, T> Drop for IntoIter<N, T> {
    fn drop(&mut self) {
        if self.i < self.l {
            unsafe {
                // UNSAFE(@ohsayan): Len is verified, due to intoiter init
                let ptr = self.d.a.as_mut_ptr().add(self.i) as *mut T;
                let len = self.l - self.i;
                // UNSAFE(@ohsayan): we know the segment to drop
                ptr::drop_in_place(ptr::slice_from_raw_parts_mut(ptr, len))
            }
        }
    }
}

impl<const N: usize, T> Iterator for IntoIter<N, T> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        self._next()
    }
}
impl<const N: usize, T> ExactSizeIterator for IntoIter<N, T> {}
impl<const N: usize, T> FusedIterator for IntoIter<N, T> {}

impl<const N: usize, T> IntoIterator for UArray<N, T> {
    type Item = T;

    type IntoIter = IntoIter<N, T>;

    fn into_iter(mut self) -> Self::IntoIter {
        let l = self.len();
        unsafe {
            // UNSAFE(@ohsayan): Leave drop to intoiter
            // HACK(@ohsayan): sneaky trick to let drop be handled by intoiter
            self.set_len(0);
        }
        Self::IntoIter { d: self, i: 0, l }
    }
}
