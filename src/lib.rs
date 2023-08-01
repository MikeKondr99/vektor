#![feature(allocator_api)]

use std::{
    alloc::{realloc, Allocator, Global, Layout},
    ops::{Deref, DerefMut, Index, IndexMut},
    ptr::NonNull,
    slice::{from_raw_parts, from_raw_parts_mut, Iter, IterMut},
};

pub struct Vek<T, A = Global>
where
    A: Allocator,
{
    ptr: NonNull<T>,
    len: usize,
    capacity: usize,
    alloc: A,
}

impl<T> Vek<T, Global> {
    pub fn new() -> Vek<T> {
        Vek {
            ptr: NonNull::dangling(),
            len: 0,
            capacity: 0,
            alloc: Global,
        }
    }
}

impl<T, A> Vek<T, A>
where
    A: Allocator,
{
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn push(&mut self, value: T) {
        self.grow(self.len + 1);
        unsafe {
            self.ptr.as_ptr().add(self.len).write(value);
            self.len += 1;
        }
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.len > 0 {
            self.len -= 1;
            unsafe {
                return Some(self.ptr.as_ptr().add(self.len).read());
            }
        }
        None
    }

    fn grow(&mut self, need: usize) {
        if need > self.capacity {
            let new_cap = usize::max(self.capacity * 2, 16);
            self.realloc(new_cap);
        }
    }

    fn realloc(&mut self, new_cap: usize) {
        let new_ptr = self
            .alloc
            .allocate(Layout::array::<T>(new_cap).expect("layout error"))
            .expect("alloc error");
        if self.capacity > 0 {
            unsafe {
                std::ptr::copy_nonoverlapping(
                    self.ptr.as_ptr(),
                    new_ptr.cast::<T>().as_mut(),
                    self.len,
                );
                self.alloc.deallocate(
                    self.ptr.cast(),
                    Layout::array::<T>(self.capacity).expect("layout error"),
                );
            }
        }
        self.capacity = new_cap;
        self.ptr = new_ptr.cast();
    }

    pub fn as_slice(&self) -> &[T] {
        self
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        self
    }

    pub fn reserve(&mut self, n: usize) {
        self.realloc(n);
    }
}

impl<T> Default for Vek<T> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T, A> Index<usize> for Vek<T, A>
where
    A: Allocator,
{
    type Output = T;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        if index < self.len {
            unsafe {
                return self
                    .ptr
                    .as_ptr()
                    .add(index)
                    .as_ref()
                    .expect("nonNull was null");
            }
        }
        panic!();
    }
}

impl<T, A> IndexMut<usize> for Vek<T, A>
where
    A: Allocator,
{
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        if index < self.len {
            unsafe {
                return self
                    .ptr
                    .as_ptr()
                    .add(index)
                    .as_mut()
                    .expect("nonNull was null");
            }
        }
        panic!();
    }
}

impl<T, A> Drop for Vek<T, A>
where
    A: Allocator,
{
    #[inline]
    fn drop(&mut self) {
        if self.capacity > 0 {
            unsafe {
                self.alloc.deallocate(
                    self.ptr.cast(),
                    Layout::array::<T>(self.capacity).expect("layout error"),
                )
            }
        }
    }
}

impl<T, A> Deref for Vek<T, A>
where
    A: Allocator,
{
    type Target = [T];

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { from_raw_parts(self.ptr.as_ptr(), self.len) }
    }
}

impl<T, A> DerefMut for Vek<T, A>
where
    A: Allocator,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { from_raw_parts_mut(self.ptr.as_ptr(), self.len) }
    }
}

impl<'v, T, A: Allocator> IntoIterator for &'v Vek<T, A> {
    type Item = &'v T;

    type IntoIter = Iter<'v, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.as_slice().iter()
    }
}

impl<'v, T, A: Allocator> IntoIterator for &'v mut Vek<T, A> {
    type Item = &'v mut T;

    type IntoIter = IterMut<'v, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.as_mut_slice().iter_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::Vek;

    #[test]
    #[should_panic]
    fn index_out_of_bounds() {
        let v = Vek::<u32>::new();
        let _b = v[0];
    }

    #[test]
    fn push_should_allocate_16() {
        let mut v = Vek::<u32>::new();
        v.push(3);
        assert_eq!(v.capacity, 16);
        assert_eq!(v.len, 1);
    }

    #[test]
    fn push_should_work() {
        let mut v = Vek::<u32>::new();
        for i in 0..16 {
            v.push(i * i);
        }
        for i in 0..16 {
            assert_eq!(v[i], (i * i) as u32);
        }
    }

    #[test]
    fn should_reallocate_after_16() {
        let mut v = Vek::<u32>::new();
        for i in 0..16 {
            v.push(i * i);
        }
        assert_eq!(v.len, 16);
        assert_eq!(v.capacity, 16);
        let old = v.ptr;
        v.push(16 * 16);
        assert_eq!(v.len, 17);
        assert_eq!(v.capacity, 32);
        assert_ne!(old, v.ptr);
        for i in 0..17 {
            dbg!(v[i]);
            assert_eq!(v[i], (i * i) as u32);
        }
    }

    #[test]
    fn mut_index_should_work() {
        let mut v = Vek::<u32>::new();
        for i in 0..16 {
            v.push(i);
        }
        for i in 0..16 {
            assert_eq!(v[i], i as u32)
        }
        for i in 0..16 {
            v[i] *= 2;
        }
        for i in 0..16 {
            assert_eq!(v[i], 2 * i as u32)
        }
    }

    #[test]
    fn pop_should_work() {
        let mut v = Vek::<u32>::new();
        assert_eq!(v.capacity,0);
        assert_eq!(v.len(),0);

        v.push(1);
        assert_eq!(v.len(),1);

        v.push(2);
        assert_eq!(v.len(),2);

        assert_eq!(v.pop(),Some(2));
        assert_eq!(v.len(),1);

        assert_eq!(v.pop(),Some(1));
        assert_eq!(v.len(),0);

        assert_eq!(v.pop(),None);
        assert_eq!(v.len(),0);

    }

    #[test]
    fn as_slice_to_iters() {
        let mut v = Vek::<u32>::new();
        for i in 0..16 {
            v.push(i);
        }
        let mut count = 0;
        v.as_slice().iter().for_each(|x| {
            assert_eq!(*x, count);
            count += 1;
        });
    }

    #[test]
    fn into_iter() {
        let mut v = Vek::<u32>::new();
        for i in 0..16 {
            v.push(i);
        }
        let mut count = 0;
        v.as_slice().iter().for_each(|x| {
            assert_eq!(*x, count);
            count += 1;
        });
    }

    #[test]
    fn iterate_by_for() {
        let mut v = Vek::<u32>::new();
        for _ in 0..16 {
            v.push(4);
        }
        for item in &v {
            assert_eq!(*item, 4);
        }
    }

    #[test]
    fn iterate_mutably_for() {
        let mut v = Vek::<u32>::new();
        for _ in 0..16 {
            v.push(1);
        }
        for item in &mut v {
            *item = item.pow(2);
        }
    }
}
