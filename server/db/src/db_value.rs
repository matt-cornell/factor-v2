use std::borrow::Cow;
use std::convert::Infallible;
use std::fmt;
use std::marker::PhantomData;

/// Type-erased data for heterogenous fields.
///
/// This uses Postcard as its data format to store the data.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ErasedData<'a>(pub Cow<'a, [u8]>);
#[cfg(feature = "postcard")]
impl ErasedData<'_> {
    pub fn new<T: serde::Serialize>(value: &T) -> postcard::Result<Self> {
        Ok(Self(Cow::Owned(postcard::to_stdvec(value)?)))
    }
    pub fn get<'a, T: serde::Deserialize<'a>>(&'a self) -> postcard::Result<T> {
        postcard::from_bytes(&self.0)
    }
    pub fn overwrite<T: serde::Serialize>(&mut self, value: &T) -> postcard::Result<()> {
        let vec = self.0.to_mut();
        vec.clear();
        postcard::to_io(value, vec).map(drop)
    }
}
impl ErasedData<'_> {
    pub fn into_owned(self) -> ErasedData<'static> {
        ErasedData(Cow::Owned(self.0.into_owned()))
    }
}
impl redb::Value for ErasedData<'_> {
    type AsBytes<'a>
        = &'a [u8]
    where
        Self: 'a;
    type SelfType<'a>
        = ErasedData<'a>
    where
        Self: 'a;
    fn fixed_width() -> Option<usize> {
        None
    }
    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'b,
    {
        &value.0
    }
    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        ErasedData(Cow::Borrowed(data))
    }
    fn type_name() -> redb::TypeName {
        redb::TypeName::new("factor::ErasedData")
    }
}

/// A marker type that implmenets [`redb::Value`] for an element type of `T` in terms of using Postcard to de/serialize the values.
pub struct PostcardValue<T> {
    _marker: PhantomData<T>,
    uninhabited: Infallible,
}
impl<T> fmt::Debug for PostcardValue<T> {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.uninhabited {}
    }
}
impl<T> Clone for PostcardValue<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for PostcardValue<T> {}
#[cfg(feature = "postcard")]
impl<T: fmt::Debug + serde::Serialize + serde::de::DeserializeOwned> redb::Value
    for PostcardValue<T>
{
    type AsBytes<'a>
        = Vec<u8>
    where
        Self: 'a;
    type SelfType<'a>
        = T
    where
        Self: 'a;
    fn fixed_width() -> Option<usize> {
        None
    }
    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'b,
    {
        postcard::to_stdvec(value).expect("serialization failed")
    }
    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        postcard::from_bytes(data).expect("deserialization failed")
    }
    fn type_name() -> redb::TypeName {
        redb::TypeName::new(&format!(
            "factor::PostcardValue<{}>",
            std::any::type_name::<T>()
        ))
    }
}
#[cfg(feature = "postcard")]
impl<T: Ord + fmt::Debug + serde::Serialize + serde::de::DeserializeOwned> redb::Key
    for PostcardValue<T>
{
    fn compare(data1: &[u8], data2: &[u8]) -> std::cmp::Ordering {
        let lhs = postcard::from_bytes::<T>(data1).expect("deserialization failed");
        let rhs = postcard::from_bytes::<T>(data2).expect("deserialization failed");
        lhs.cmp(&rhs)
    }
}
