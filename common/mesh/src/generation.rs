use std::fmt::{self, Debug, Display, Formatter};
use std::mem::MaybeUninit;
use std::ops::{Index, IndexMut};

/// A type that acts like `Option<T>`, but with a generation count.
pub trait OptWithGeneration: Default {
    type Generation: Generation;
    type Value;

    fn has_value(&self) -> bool;
    fn has_gen(&self, generation: Self::Generation) -> bool;
    fn drop_value_unchecked(&mut self) -> Option<Self::Value>;
    fn insert_value(&mut self, val: Self::Value) -> Self::Generation;
    fn get_unchecked(&self) -> Option<&Self::Value>;
    fn get_unchecked_mut(&mut self) -> Option<&mut Self::Value>;
    fn get(&self, generation: Self::Generation) -> Option<&Self::Value>;
    fn get_mut(&mut self, generation: Self::Generation) -> Option<&mut Self::Value>;
    fn generation(&self) -> Option<Self::Generation>;
    fn drop_value_checked(&mut self, generation: Self::Generation) -> Option<Self::Value> {
        self.has_gen(generation)
            .then(|| self.drop_value_unchecked())
            .flatten()
    }
    fn with_value(val: Self::Value) -> (Self, Self::Generation) {
        let mut this = Self::default();
        let g = this.insert_value(val);
        (this, g)
    }
}
impl<T> OptWithGeneration for Option<T> {
    type Generation = ();
    type Value = T;

    fn has_value(&self) -> bool {
        self.is_some()
    }
    fn has_gen(&self, _generation: Self::Generation) -> bool {
        self.is_some()
    }
    fn drop_value_unchecked(&mut self) -> Option<Self::Value> {
        self.take()
    }
    fn drop_value_checked(&mut self, _generation: Self::Generation) -> Option<Self::Value> {
        self.take()
    }
    fn insert_value(&mut self, val: Self::Value) {
        *self = Some(val);
    }
    fn get_unchecked(&self) -> Option<&Self::Value> {
        self.as_ref()
    }
    fn get_unchecked_mut(&mut self) -> Option<&mut Self::Value> {
        self.as_mut()
    }
    fn get(&self, _generation: Self::Generation) -> Option<&Self::Value> {
        self.as_ref()
    }
    fn get_mut(&mut self, _generation: Self::Generation) -> Option<&mut Self::Value> {
        self.as_mut()
    }
    fn generation(&self) -> Option<Self::Generation> {
        self.as_ref().map(drop)
    }
    fn with_value(val: Self::Value) -> (Self, Self::Generation) {
        (Some(val), ())
    }
}

/// An integer type, with some basic operations.
///
/// # Safety
/// The methods must do what they're documented to
pub unsafe trait Integer: PartialEq + Clone + Generation {
    /// 0
    const ZERO: Self;
    /// 1
    const ONE: Self;
    /// self & 1 == 1
    fn is_odd(&self) -> bool;
    /// *self &= !1
    fn make_even(&mut self);
    /// *self |= 1
    fn make_odd(&mut self);
    /// *self = self.wrapping_add(2) & mask
    fn add_2_mask(&mut self, mask: Self);
}

pub struct BitMarker<const N: usize>;
pub trait HasIntegerSize {
    type Integer: Integer;
    const MASK: Self::Integer;
}
macro_rules! impl_bit_marker {
    ($($int:ty, $($val:expr)*;)*) => {
        $(
            $(
                impl HasIntegerSize for BitMarker<$val> {
                    type Integer = $int;
                    const MASK: $int = <$int>::unbounded_shl(1, $val).wrapping_sub(1);
                }
            )*
        )*
    };
}
impl_bit_marker!(
    u8, 1 2 3 4 5 6 7 8;
    u16, 9 10 11 12 13 14 15 16;
    u32, 17 18 19 20 21 22 23 24 25 26 27 28 29 30 31 32;
    u64, 33 34 35 36 37 38 39 40 41 42 43 44 45 46 47 48 49 50 51 52 53 54 55 56 57 58 59 60 61 62 63 64;
);

pub trait HasGeneration {
    type Generation: Generation;
    type Type<T>: OptWithGeneration<Value = T, Generation = Self::Generation>;
}
#[diagnostic::do_not_recommend]
impl<const BITS: usize> HasGeneration for BitMarker<BITS>
where
    Self: HasIntegerSize,
{
    type Generation = <Self as HasIntegerSize>::Integer;
    type Type<T> = Generational<T, BITS>;
}
impl HasGeneration for BitMarker<0> {
    type Generation = ();
    type Type<T> = Option<T>;
}

pub struct Generational<T, const BITS: usize>
where
    BitMarker<BITS>: HasIntegerSize,
{
    generation: <BitMarker<BITS> as HasIntegerSize>::Integer,
    value: MaybeUninit<T>,
}
impl<T, const BITS: usize> Generational<T, BITS>
where
    BitMarker<BITS>: HasIntegerSize,
{
    pub const fn new() -> Self {
        Self {
            generation: <BitMarker<BITS> as HasIntegerSize>::Integer::ZERO,
            value: MaybeUninit::uninit(),
        }
    }
}
impl<T, const BITS: usize> Drop for Generational<T, BITS>
where
    BitMarker<BITS>: HasIntegerSize,
{
    fn drop(&mut self) {
        if self.has_value() {
            unsafe {
                self.value.assume_init_drop();
            }
        }
    }
}
impl<T, const BITS: usize> Default for Generational<T, BITS>
where
    BitMarker<BITS>: HasIntegerSize,
{
    fn default() -> Self {
        Self::new()
    }
}
impl<T: Debug, const BITS: usize> Debug for Generational<T, BITS>
where
    BitMarker<BITS>: HasIntegerSize,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut s = f.debug_struct("Generational");
        let has_value = self.has_value();
        s.field("has_value", &has_value);
        if has_value {
            unsafe {
                s.field("value", self.value.assume_init_ref());
            }
        }
        s.finish_non_exhaustive()
    }
}
impl<T: Clone, const BITS: usize> Clone for Generational<T, BITS>
where
    BitMarker<BITS>: HasIntegerSize,
{
    fn clone(&self) -> Self {
        Self {
            generation: self.generation,
            value: if self.has_value() {
                unsafe { MaybeUninit::new(self.value.assume_init_ref().clone()) }
            } else {
                MaybeUninit::uninit()
            },
        }
    }
}
impl<T, const BITS: usize> OptWithGeneration for Generational<T, BITS>
where
    BitMarker<BITS>: HasIntegerSize,
{
    type Generation = <BitMarker<BITS> as HasIntegerSize>::Integer;
    type Value = T;

    fn has_value(&self) -> bool {
        self.generation.is_odd()
    }
    fn has_gen(&self, generation: Self::Generation) -> bool {
        self.generation == generation
    }
    fn drop_value_unchecked(&mut self) -> Option<Self::Value> {
        self.has_value().then(|| unsafe {
            self.generation.make_even();
            self.value.assume_init_read()
        })
    }
    fn drop_value_checked(&mut self, generation: Self::Generation) -> Option<Self::Value> {
        self.has_gen(generation).then(|| unsafe {
            self.generation.make_even();
            self.value.assume_init_read()
        })
    }
    fn insert_value(&mut self, val: Self::Value) -> Self::Generation {
        self.generation
            .add_2_mask(<BitMarker<BITS> as HasIntegerSize>::MASK);
        let g = self.generation;
        if self.generation.is_odd() {
            unsafe {
                self.value.assume_init_drop();
            }
            self.value.write(val);
        } else {
            self.generation.make_odd();
        }
        g
    }
    fn get_unchecked(&self) -> Option<&Self::Value> {
        self.has_value()
            .then(|| unsafe { self.value.assume_init_ref() })
    }
    fn get_unchecked_mut(&mut self) -> Option<&mut Self::Value> {
        self.has_value()
            .then(|| unsafe { self.value.assume_init_mut() })
    }
    fn get(&self, generation: Self::Generation) -> Option<&Self::Value> {
        self.has_gen(generation)
            .then(|| unsafe { self.value.assume_init_ref() })
    }
    fn get_mut(&mut self, generation: Self::Generation) -> Option<&mut Self::Value> {
        self.has_gen(generation)
            .then(|| unsafe { self.value.assume_init_mut() })
    }
    fn generation(&self) -> Option<Self::Generation> {
        self.generation.is_odd().then_some(self.generation)
    }
    fn with_value(val: Self::Value) -> (Self, Self::Generation) {
        (
            Self {
                generation: <BitMarker<BITS> as HasIntegerSize>::Integer::ONE,
                value: MaybeUninit::new(val),
            },
            <BitMarker<BITS> as HasIntegerSize>::Integer::ONE,
        )
    }
}
macro_rules! impl_for {
    ($($int:ty)*) => {
        $(
            unsafe impl Integer for $int {
                const ZERO: Self = 0;
                const ONE: Self = 1;
                fn is_odd(&self) -> bool {
                    self & 1 != 0
                }
                fn make_even(&mut self) {
                    *self &= !1;
                }
                fn make_odd(&mut self) {
                    *self |= 1;
                }
                fn add_2_mask(&mut self, mask: Self) {
                    *self = self.wrapping_add(2);
                    *self &= mask;
                }
            }
            impl Generation for $int {
                fn fmt_gen_idx(&self, index: usize, f: &mut Formatter) -> fmt::Result {
                    write!(f, "{index}!{self}")
                }
            }
        )*
    };
}
impl_for!(u8 u16 u32 u64 usize);

pub trait Generation: Copy {
    fn fmt_gen_idx(&self, index: usize, f: &mut Formatter) -> fmt::Result;
}
impl Generation for () {
    fn fmt_gen_idx(&self, index: usize, f: &mut Formatter) -> fmt::Result {
        Display::fmt(&index, f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GenerationIndex<M> {
    pub index: usize,
    pub generation: M,
}
impl<M> GenerationIndex<M> {
    pub const fn new(index: usize, generation: M) -> Self {
        Self { index, generation }
    }
}
impl<M: Generation> Display for GenerationIndex<M> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.generation.fmt_gen_idx(self.index, f)
    }
}

#[derive(Default)]
pub struct Slab<T, const BITS: usize>
where
    BitMarker<BITS>: HasGeneration,
{
    elems: Vec<<BitMarker<BITS> as HasGeneration>::Type<T>>,
    first_free: usize,
}
impl<T, const BITS: usize> Slab<T, BITS>
where
    BitMarker<BITS>: HasGeneration,
{
    pub const fn new() -> Self {
        Self {
            elems: Vec::new(),
            first_free: 0,
        }
    }
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            elems: Vec::with_capacity(capacity),
            first_free: 0,
        }
    }
    pub fn insert(
        &mut self,
        val: T,
    ) -> GenerationIndex<<BitMarker<BITS> as HasGeneration>::Generation> {
        let index = self.first_free;
        let g;
        if self.first_free == self.elems.len() {
            self.first_free += 1;
            let v;
            (v, g) = <BitMarker<BITS> as HasGeneration>::Type::<T>::with_value(val);
            self.elems.push(v);
        } else {
            let mut slice = &mut self.elems[self.first_free..];
            let head = slice.split_off_first_mut().unwrap();
            g = head.insert_value(val);
            self.first_free += slice
                .iter()
                .position(|v| !v.has_value())
                .unwrap_or(slice.len());
        }
        GenerationIndex {
            index,
            generation: g,
        }
    }
    pub fn remove(
        &mut self,
        index: GenerationIndex<<BitMarker<BITS> as HasGeneration>::Generation>,
    ) -> Option<T> {
        let res = self
            .elems
            .get_mut(index.index)?
            .drop_value_checked(index.generation);
        if res.is_some() && index.index < self.first_free {
            self.first_free = index.index;
        }
        res
    }
    pub fn contains(
        &self,
        index: GenerationIndex<<BitMarker<BITS> as HasGeneration>::Generation>,
    ) -> bool {
        self.elems
            .get(index.index)
            .is_some_and(|v| v.has_gen(index.generation))
    }
    pub fn get(
        &self,
        index: GenerationIndex<<BitMarker<BITS> as HasGeneration>::Generation>,
    ) -> Option<&T> {
        self.elems.get(index.index)?.get(index.generation)
    }
    pub fn get_mut(
        &mut self,
        index: GenerationIndex<<BitMarker<BITS> as HasGeneration>::Generation>,
    ) -> Option<&mut T> {
        self.elems.get_mut(index.index)?.get_mut(index.generation)
    }
    /// Get the maximum index for the slab.
    pub const fn max_idx(&self) -> usize {
        self.elems.len()
    }
    pub fn iter(&self) -> Iter<'_, <BitMarker<BITS> as HasGeneration>::Type<T>> {
        Iter {
            iter: self.elems.iter(),
            index: 0,
        }
    }
    pub fn iter_mut(&mut self) -> IterMut<'_, <BitMarker<BITS> as HasGeneration>::Type<T>> {
        IterMut {
            iter: self.elems.iter_mut(),
            index: 0,
        }
    }
    pub fn values(&self) -> Values<'_, <BitMarker<BITS> as HasGeneration>::Type<T>> {
        Values {
            iter: self.elems.iter(),
        }
    }
    pub fn values_mut(&mut self) -> ValuesMut<'_, <BitMarker<BITS> as HasGeneration>::Type<T>> {
        ValuesMut {
            iter: self.elems.iter_mut(),
        }
    }
}
impl<T: Debug, const BITS: usize> Debug for Slab<T, BITS>
where
    BitMarker<BITS>: HasGeneration<Generation: Debug>,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish_non_exhaustive()
    }
}
impl<T: Clone, const BITS: usize> Clone for Slab<T, BITS>
where
    BitMarker<BITS>: HasGeneration<Type<T>: Clone>,
{
    fn clone(&self) -> Self {
        Self {
            elems: self.elems.clone(),
            first_free: self.first_free,
        }
    }
    fn clone_from(&mut self, source: &Self) {
        self.elems.clone_from(&source.elems);
        self.first_free = source.first_free;
    }
}
impl<T, const BITS: usize> Index<GenerationIndex<<BitMarker<BITS> as HasGeneration>::Generation>>
    for Slab<T, BITS>
where
    BitMarker<BITS>: HasGeneration,
{
    type Output = T;

    fn index(
        &self,
        index: GenerationIndex<<BitMarker<BITS> as HasGeneration>::Generation>,
    ) -> &Self::Output {
        self.get(index)
            .unwrap_or_else(|| panic!("index {index} not present in slab"))
    }
}
impl<T, const BITS: usize> IndexMut<GenerationIndex<<BitMarker<BITS> as HasGeneration>::Generation>>
    for Slab<T, BITS>
where
    BitMarker<BITS>: HasGeneration,
{
    fn index_mut(
        &mut self,
        index: GenerationIndex<<BitMarker<BITS> as HasGeneration>::Generation>,
    ) -> &mut Self::Output {
        self.get_mut(index)
            .unwrap_or_else(|| panic!("index {index} not present in slab"))
    }
}

#[derive(Debug, Clone)]
pub struct Iter<'a, G> {
    iter: std::slice::Iter<'a, G>,
    index: usize,
}
impl<'a, G: OptWithGeneration> Iterator for Iter<'a, G> {
    type Item = (GenerationIndex<G::Generation>, &'a G::Value);
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.find_map(|v| {
            let index = self.index;
            self.index += 1;
            let generation = v.generation()?;
            Some((
                GenerationIndex::new(index, generation),
                v.get_unchecked().unwrap(),
            ))
        })
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.iter.len()))
    }
}

#[derive(Debug)]
pub struct IterMut<'a, G> {
    iter: std::slice::IterMut<'a, G>,
    index: usize,
}
impl<'a, G: OptWithGeneration> Iterator for IterMut<'a, G> {
    type Item = (GenerationIndex<G::Generation>, &'a mut G::Value);
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.find_map(|v| {
            let index = self.index;
            self.index += 1;
            let generation = v.generation()?;
            Some((
                GenerationIndex::new(index, generation),
                v.get_unchecked_mut().unwrap(),
            ))
        })
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.iter.len()))
    }
}

#[derive(Debug, Clone)]
pub struct Values<'a, G> {
    iter: std::slice::Iter<'a, G>,
}
impl<'a, G: OptWithGeneration> Iterator for Values<'a, G> {
    type Item = &'a G::Value;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.find_map(G::get_unchecked)
    }
}

#[derive(Debug)]
pub struct ValuesMut<'a, G> {
    iter: std::slice::IterMut<'a, G>,
}
impl<'a, G: OptWithGeneration> Iterator for ValuesMut<'a, G> {
    type Item = &'a mut G::Value;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.find_map(G::get_unchecked_mut)
    }
}
