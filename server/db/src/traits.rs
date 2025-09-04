//! Database-like traits.
//!
//! Some use cases might benefit from being done entirely in memory, without the overhead of a database,
//! while others need to persist. To handle both of these needs, the [`ReadableMap`] and [`WritableMap`]
//! traits can be used to be generic over the storage, allowing a [`BTreeMap`] to be used instead.

use std::borrow::Borrow;
use std::collections::BTreeMap;
use std::convert::Infallible;
use std::ops::RangeBounds;

/// A readable, sorted map, abstracted across [`BTreeMap`] and [`redb`]'s tables.
///
/// The API for this is more restrictive, than either of their APIs, but it should cover most use cases.
#[allow(clippy::type_complexity)]
pub trait ReadableMap {
    /// The error type for operations from this map.
    ///
    /// For BTreeMap, this is an uninhabited type, so the results should be transparent
    type Error: std::error::Error + Send + Sync + 'static;
    type Key;
    type Value;
    type Range<'a>: DoubleEndedIterator<
        Item = Result<(Self::KeyRef<'a>, Self::ValueRef<'a>), Self::Error>,
    >
    where
        Self: 'a;
    /// The reference type to the key that gets *returned* from some methods.
    ///
    /// A simple `&Self::Key` is used for inputs, since those don't need to be guarded.
    type KeyRef<'a>: ReadableValue<Self::Key, Self::Marker>
    where
        Self: 'a;
    /// The reference type to the value that gets returned from some methods.
    type ValueRef<'a>: ReadableValue<Self::Value, Self::Marker>
    where
        Self: 'a;
    /// The marker type for references.
    ///
    /// A marker type is needed because one could create an implementation of [`redb::Value`] for which `Self::SelfType<'_> == AccessGuard<'static, Self>`, which would
    /// create a conflicting implementation with the reflexive implementation.
    type Marker;

    fn get(&self, key: &Self::Key) -> Result<Option<Self::ValueRef<'_>>, Self::Error>;
    fn first(&self) -> Result<Option<(Self::KeyRef<'_>, Self::ValueRef<'_>)>, Self::Error>;
    fn last(&self) -> Result<Option<(Self::KeyRef<'_>, Self::ValueRef<'_>)>, Self::Error>;
    fn range(
        &self,
        range: impl RangeBounds<Self::Key> + 'static,
    ) -> Result<Self::Range<'_>, Self::Error>;
}

pub trait WritableMap: ReadableMap {
    type ValueMut<'a>: WritableValue<Self::Value, Self::Marker>
    where
        Self: 'a;
    type ValueOwned<'a>: ConsumableValue<Self::Value, Self::Marker>
    where
        Self: 'a;

    fn get_mut(&mut self, key: &Self::Key) -> Result<Option<Self::ValueMut<'_>>, Self::Error>;
    fn insert(
        &mut self,
        key: Self::Key,
        value: Self::Value,
    ) -> Result<Option<Self::ValueOwned<'_>>, Self::Error>;
    fn remove(&mut self, key: &Self::Key) -> Result<Option<Self::ValueOwned<'_>>, Self::Error>;
}

impl<K: Ord, V> ReadableMap for BTreeMap<K, V> {
    type Error = Infallible;
    type Key = K;
    type Value = V;
    type KeyRef<'a>
        = &'a K
    where
        Self: 'a;
    type ValueRef<'a>
        = &'a V
    where
        Self: 'a;
    type Range<'a>
        = std::iter::Map<
        std::collections::btree_map::Range<'a, K, V>,
        fn((&'a K, &'a V)) -> Result<(&'a K, &'a V), Infallible>,
    >
    where
        Self: 'a;
    type Marker = RegularMarker;

    fn get(&self, key: &Self::Key) -> Result<Option<Self::ValueRef<'_>>, Self::Error> {
        Ok(BTreeMap::get(self, key.borrow()))
    }
    fn first(&self) -> Result<Option<(Self::KeyRef<'_>, Self::ValueRef<'_>)>, Self::Error> {
        Ok(BTreeMap::first_key_value(self))
    }
    fn last(&self) -> Result<Option<(Self::KeyRef<'_>, Self::ValueRef<'_>)>, Self::Error> {
        Ok(BTreeMap::last_key_value(self))
    }
    fn range(&self, range: impl RangeBounds<K> + 'static) -> Result<Self::Range<'_>, Self::Error> {
        let [start, end]: [std::ops::Bound<&K>; 2] = [
            range.start_bound().map(Borrow::borrow),
            range.end_bound().map(Borrow::borrow),
        ];
        Ok(BTreeMap::range(self, (start, end)).map(Ok))
    }
}
impl<K: Ord, V> WritableMap for BTreeMap<K, V> {
    type ValueMut<'a>
        = &'a mut V
    where
        Self: 'a;
    type ValueOwned<'a>
        = V
    where
        Self: 'a;

    fn get_mut(&mut self, key: &Self::Key) -> Result<Option<Self::ValueMut<'_>>, Self::Error> {
        Ok(BTreeMap::get_mut(self, key))
    }
    fn insert(
        &mut self,
        key: Self::Key,
        value: Self::Value,
    ) -> Result<Option<Self::ValueOwned<'_>>, Self::Error> {
        Ok(BTreeMap::insert(self, key, value))
    }
    fn remove(&mut self, key: &Self::Key) -> Result<Option<Self::ValueOwned<'_>>, Self::Error> {
        Ok(BTreeMap::remove(self, key))
    }
}

impl<K: redb::Key + StaticValue, V: StaticValue> ReadableMap for redb::ReadOnlyTable<K, V> {
    type Error = redb::StorageError;
    type Key = K::SelfType<'static>;
    type Value = V::SelfType<'static>;
    type KeyRef<'a>
        = redb::AccessGuard<'a, K>
    where
        Self: 'a;
    type ValueRef<'a>
        = redb::AccessGuard<'a, V>
    where
        Self: 'a;
    type Range<'a>
        = redb::Range<'a, K, V>
    where
        Self: 'a;
    type Marker = AccessGuardMarker;

    fn get(&self, key: &Self::Key) -> Result<Option<Self::ValueRef<'_>>, Self::Error> {
        redb::ReadableTable::get(self, key.borrow())
    }
    fn first(&self) -> Result<Option<(Self::KeyRef<'_>, Self::ValueRef<'_>)>, Self::Error> {
        redb::ReadableTable::first(self)
    }
    fn last(&self) -> Result<Option<(Self::KeyRef<'_>, Self::ValueRef<'_>)>, Self::Error> {
        redb::ReadableTable::last(self)
    }
    fn range(
        &self,
        range: impl RangeBounds<Self::Key> + 'static,
    ) -> Result<Self::Range<'_>, Self::Error> {
        redb::ReadableTable::range(self, range)
    }
}
impl<K: redb::Key + StaticValue, V: StaticValue> ReadableMap for redb::Table<'_, K, V> {
    type Error = redb::StorageError;
    type Key = K::SelfType<'static>;
    type Value = V::SelfType<'static>;
    type KeyRef<'a>
        = redb::AccessGuard<'a, K>
    where
        Self: 'a;
    type ValueRef<'a>
        = redb::AccessGuard<'a, V>
    where
        Self: 'a;
    type Range<'a>
        = redb::Range<'a, K, V>
    where
        Self: 'a;
    type Marker = AccessGuardMarker;

    fn get(&self, key: &Self::Key) -> Result<Option<Self::ValueRef<'_>>, Self::Error> {
        redb::ReadableTable::get(self, key.borrow())
    }
    fn first(&self) -> Result<Option<(Self::KeyRef<'_>, Self::ValueRef<'_>)>, Self::Error> {
        redb::ReadableTable::first(self)
    }
    fn last(&self) -> Result<Option<(Self::KeyRef<'_>, Self::ValueRef<'_>)>, Self::Error> {
        redb::ReadableTable::last(self)
    }
    fn range(
        &self,
        range: impl RangeBounds<Self::Key> + 'static,
    ) -> Result<Self::Range<'_>, Self::Error> {
        redb::ReadableTable::range(self, range)
    }
}
impl<K: redb::Key + StaticValue, V: StaticValue> WritableMap for redb::Table<'_, K, V> {
    type ValueMut<'a>
        = redb::AccessGuardMut<'a, V>
    where
        Self: 'a;
    type ValueOwned<'a>
        = redb::AccessGuard<'a, V>
    where
        Self: 'a;

    fn get_mut(&mut self, key: &Self::Key) -> Result<Option<Self::ValueMut<'_>>, Self::Error> {
        redb::Table::get_mut(self, key)
    }
    fn insert(
        &mut self,
        key: Self::Key,
        value: Self::Value,
    ) -> Result<Option<Self::ValueOwned<'_>>, Self::Error> {
        redb::Table::insert(self, key, value)
    }
    fn remove(&mut self, key: &Self::Key) -> Result<Option<Self::ValueOwned<'_>>, Self::Error> {
        redb::Table::remove(self, key)
    }
}

/// A maybe-owned value that can be used as `T`.
///
/// This is implemented for all `T` and `&T`.
pub trait RefLike<T> {
    /// Get a reference to `self`.
    fn get_ref(&self) -> &T;
    /// Move out of `self`, cloning if necessary.
    fn maybe_cloned(self) -> T
    where
        T: Clone;
}
impl<T> RefLike<T> for T {
    fn get_ref(&self) -> &T {
        self
    }
    fn maybe_cloned(self) -> T
    where
        T: Clone,
    {
        self
    }
}
impl<T> RefLike<T> for &T {
    fn get_ref(&self) -> &T {
        self
    }
    fn maybe_cloned(self) -> T
    where
        T: Clone,
    {
        T::clone(self)
    }
}

/// Marker enum for regular (`T`, `&T`, `&mut T`) [`ReadableValue`] implementations
pub enum RegularMarker {}
/// Marker enum for [`ReadableValue`] implementations for [`AccessGuard`] and [`AccessGuardMut`]
pub enum AccessGuardMarker {}

/// A guard that can be read from.
pub trait ReadableValue<T, Marker> {
    /// The reference type returned.
    ///
    /// This may either be `T` or `&'a T`.
    type Ref<'a>: RefLike<T>
    where
        Self: 'a;
    /// Get the value behind this guard.
    fn value(&self) -> Self::Ref<'_>;
}
/// A guard that can be written to in addition to being read from.
///
/// Writing is potentially fallible, so this returns a `Result`.
pub trait WritableValue<T, Marker>: ReadableValue<T, Marker> {
    type Error: std::error::Error + Send + Sync + 'static;

    fn set_value(&mut self, value: T) -> Result<(), Self::Error>;
}
pub trait ConsumableValue<T, Marker>: ReadableValue<T, Marker> {
    fn consume(self) -> T;
}
impl<T> ReadableValue<T, RegularMarker> for T {
    type Ref<'a>
        = &'a T
    where
        Self: 'a;
    fn value(&self) -> Self::Ref<'_> {
        self
    }
}
impl<T> ReadableValue<T, RegularMarker> for &T {
    type Ref<'a>
        = &'a T
    where
        Self: 'a;
    fn value(&self) -> &T {
        self
    }
}
impl<T> ReadableValue<T, RegularMarker> for &mut T {
    type Ref<'a>
        = &'a T
    where
        Self: 'a;
    fn value(&self) -> &T {
        self
    }
}
impl<T> WritableValue<T, RegularMarker> for &mut T {
    type Error = Infallible;

    fn set_value(&mut self, value: T) -> Result<(), Self::Error> {
        **self = value;
        Ok(())
    }
}
impl<T> ConsumableValue<T, RegularMarker> for T {
    fn consume(self) -> T {
        self
    }
}

impl<T: StaticValue> ReadableValue<T::SelfType<'static>, AccessGuardMarker>
    for redb::AccessGuard<'_, T>
{
    type Ref<'b>
        = T::SelfType<'static>
    where
        Self: 'b;
    fn value(&self) -> Self::Ref<'_> {
        // SAFETY: T::SelfType is the same for all lifetimes, since no non-static lifetimes can appear in the type
        unsafe {
            std::mem::transmute::<T::SelfType<'_>, T::SelfType<'static>>(redb::AccessGuard::value(
                self,
            ))
        }
    }
}
impl<T: StaticValue> ReadableValue<T::SelfType<'static>, AccessGuardMarker>
    for redb::AccessGuardMut<'_, T>
{
    type Ref<'b>
        = T::SelfType<'static>
    where
        Self: 'b;
    fn value(&self) -> Self::Ref<'_> {
        // SAFETY: T::SelfType is the same for all lifetimes, since no non-static lifetimes can appear in the type
        unsafe {
            std::mem::transmute::<T::SelfType<'_>, T::SelfType<'static>>(
                redb::AccessGuardMut::value(self),
            )
        }
    }
}
impl<T: StaticValue> WritableValue<T::SelfType<'static>, AccessGuardMarker>
    for redb::AccessGuardMut<'_, T>
{
    type Error = redb::StorageError;
    fn set_value(&mut self, value: T::SelfType<'static>) -> Result<(), Self::Error> {
        self.insert(value)
    }
}
impl<T: StaticValue> ConsumableValue<T::SelfType<'static>, AccessGuardMarker>
    for redb::AccessGuard<'_, T>
{
    fn consume(self) -> T::SelfType<'static> {
        unsafe {
            std::mem::transmute::<&Self, &'static redb::AccessGuard<'static, T>>(&self).value()
        }
    }
}

/// A [`redb::Value`] for which `SelfType: 'static`
pub trait StaticValue: for<'a> redb::Value<SelfType<'a>: 'static> + 'static {
    /// A non-GAT `Self::SelfType`. This can't be directly substituted in some places because Rust can't prove that it's the same.
    type StaticSelf: 'static;
}
impl<T: redb::Value + 'static> StaticValue for T
where
    for<'a> T::SelfType<'a>: 'static,
{
    type StaticSelf = T::SelfType<'static>;
}
