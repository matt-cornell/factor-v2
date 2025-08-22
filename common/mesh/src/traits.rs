//! Core traits and default representation for tetrahedral meshes.

use bevy_math::Vec3;

/// A vertex index
///
/// Since we're working with tetrahedra, many vertices can
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum VertexIdx {
    V0 = 0,
    V1 = 1,
    V2 = 2,
    V3 = 3,
}
impl VertexIdx {
    pub const VALS: [Self; 4] = [Self::V0, Self::V1, Self::V2, Self::V3];

    /// Convert `self` to a `usize`, which is shorter as a method than `self as u8 as usize`.
    #[inline(always)]
    pub const fn to_usize(self) -> usize {
        self as u8 as usize
    }
    /// Get the value at this index in an array.
    ///
    /// This is infallible because the index can't be out of bounds, and is guaranteed to not bounds check unlike `arr[self.to_usize()]`.
    #[inline(always)]
    pub fn in_arr<T>(self, arr: &[T; 4]) -> &T {
        unsafe { arr.get_unchecked(self.to_usize()) }
    }
    /// Same as [`Self::in_arr`], but mutable.
    #[inline(always)]
    pub fn in_arr_mut<T>(self, arr: &mut [T; 4]) -> &mut T {
        unsafe { arr.get_unchecked_mut(self.to_usize()) }
    }
    /// Extract the value from an arry, and return the remaining three elements.
    ///
    /// The final elements will start after the index and wrap around—`VertexIdx::V1.extract([a, b, c, d]) == (b, [c, d, a])`.
    #[inline(always)]
    pub fn extract<T>(self, arr: [T; 4]) -> (T, [T; 3]) {
        let [a, b, c, d] = arr;
        match self {
            Self::V0 => (a, [b, c, d]),
            Self::V1 => (b, [c, d, a]),
            Self::V2 => (c, [d, a, b]),
            Self::V3 => (d, [a, b, c]),
        }
    }
}

/// A type that can act as a vertex.
///
/// Any type just needs to act like a `Vec3`, but they can have additional data.
pub trait VertexData {
    /// Get the current position for this vertex.
    fn as_vec3(&self) -> Vec3;
}
/// Vertex data that can be mutated.
///
/// [`VertexData`] could lazily compute its vertex positions, in which case mutation might not be possible.
pub trait VertexDataMut: VertexData {
    /// Set the current position for this vertex.
    fn set_vec3(&mut self, vec: Vec3);
}

/// Some kind of structure for face data.
///
/// This conceptually acts as a `Option<(K, VertexIdx)>`, and is implemented for those types, but more efficient layouts are possible.
/// It's implemented for all unsigned integer types (with that same type being the index), where the lower two bits are the
pub trait FaceData<K>: Sized {
    /// Pack a key and a vertex index into `Self`.
    fn pack(key: TetraId<K>, idx: VertexIdx) -> Self;
    /// Unpack the data into a key and vertex index.
    ///
    /// Behavior is unspecified if `self.is_none()`.
    fn unpack(self) -> (TetraId<K>, VertexIdx);
    /// Create a placeholder value.
    fn none() -> Self;
    /// Check if this was created from [`Self::pack`].
    fn is_some(&self) -> bool;
    /// Check if this was created from [`Self::none`].
    #[inline(always)]
    fn is_none(&self) -> bool {
        !self.is_some()
    }
    /// Convenience function to check if `self.is_some()` and if so, call `self.unpack()`.
    #[inline(always)]
    fn into_option(self) -> Option<(TetraId<K>, VertexIdx)> {
        self.is_some().then(|| self.unpack())
    }
    /// Convenience function to either call [`Self::pack`] or [`Self::none`] based on the value in an `Option`.
    #[inline(always)]
    fn from_option(opt: Option<(TetraId<K>, VertexIdx)>) -> Self {
        opt.map_or_else(Self::none, |(key, idx)| Self::pack(key, idx))
    }
}
impl<K> FaceData<K> for Option<(TetraId<K>, VertexIdx)> {
    #[inline(always)]
    fn pack(key: TetraId<K>, idx: VertexIdx) -> Self {
        Some((key, idx))
    }
    #[inline(always)]
    fn unpack(self) -> (TetraId<K>, VertexIdx) {
        self.unwrap()
    }
    #[inline(always)]
    fn none() -> Self {
        None
    }
    #[inline(always)]
    fn is_some(&self) -> bool {
        self.is_some()
    }
    #[inline(always)]
    fn into_option(self) -> Option<(TetraId<K>, VertexIdx)> {
        self
    }
    #[inline(always)]
    fn from_option(opt: Option<(TetraId<K>, VertexIdx)>) -> Self {
        opt
    }
}
macro_rules! for_int {
    ($($int:ty)*) => {
        $(
            #[allow(clippy::identity_op)]
            impl FaceData<$int> for PackedFace<$int> {
                #[inline(always)]
                fn pack(key: TetraId<$int>, idx: VertexIdx) -> Self {
                    Self(key.0 << 2 | idx as u8 as $int)
                }
                #[inline(always)]
                fn unpack(self) -> (TetraId<$int>, VertexIdx) {
                    let idx = unsafe { std::mem::transmute::<u8, VertexIdx>((self.0 & 3) as u8) };
                    let key = self.0 >> 2;
                    (TetraId(key), idx)
                }
                #[inline(always)]
                fn none() -> Self {
                    Self(<$int>::MAX)
                }
                #[inline(always)]
                fn is_some(&self) -> bool {
                    self.0 != <$int>::MAX
                }
            }
        )*
    };
}
for_int!(u8 u16 u32 u64 usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PackedFace<K>(pub K);

/// Tetrahedral data.
///
/// This expects
pub trait TetraData<K> {
    fn vertex(&self, vert: VertexIdx) -> VertexId<K>;
    fn face(&self, face: VertexIdx) -> Option<(TetraId<K>, VertexIdx)>;
}
pub trait TetraDataMut<K>: TetraData<K> {
    fn set_vertex(&mut self, vert: VertexIdx, val: VertexId<K>);
    fn set_face(&mut self, face: VertexIdx, val: Option<(TetraId<K>, VertexIdx)>);
}

impl VertexData for Vec3 {
    #[inline(always)]
    fn as_vec3(&self) -> Vec3 {
        *self
    }
}
impl VertexDataMut for Vec3 {
    #[inline(always)]
    fn set_vec3(&mut self, vec: Vec3) {
        *self = vec;
    }
}

/// A default vertex with associated data.
///
/// This is a basic implementation of [`VertexData`], and should be good enough for most use.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vertex<V = ()> {
    pub pos: Vec3,
    pub data: V,
}
impl<V> VertexData for Vertex<V> {
    #[inline(always)]
    fn as_vec3(&self) -> Vec3 {
        self.pos
    }
}
impl<V> VertexDataMut for Vertex<V> {
    #[inline(always)]
    fn set_vec3(&mut self, vec: Vec3) {
        self.pos = vec;
    }
}

pub type BasicFace<K> = Option<(TetraId<K>, VertexIdx)>;

/// A default tetrahedron with associated data.
///
/// Like [`Vertex`], this is a basic implementation that's general for most cases.
/// The `K` parameter is a key type, and the `F` parameter is the face data. [`BasicFace`] provides a basic implementation,
/// but using an integer with an integer key takes up half of the space, at the cost of only allowing 1/4 of the keys.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tetra<K, F = BasicFace<K>, T = ()> {
    pub conns: [(VertexId<K>, F); 4],
    pub data: T,
}
impl<K: Copy, F: FaceData<K> + Copy, T> TetraData<K> for Tetra<K, F, T> {
    fn vertex(&self, vert: VertexIdx) -> VertexId<K> {
        vert.in_arr(&self.conns).0
    }
    fn face(&self, face: VertexIdx) -> Option<(TetraId<K>, VertexIdx)> {
        face.in_arr(&self.conns).1.into_option()
    }
}
impl<K: Copy, F: FaceData<K> + Copy, T> TetraDataMut<K> for Tetra<K, F, T> {
    fn set_vertex(&mut self, vert: VertexIdx, val: VertexId<K>) {
        vert.in_arr_mut(&mut self.conns).0 = val;
    }
    fn set_face(&mut self, face: VertexIdx, val: Option<(TetraId<K>, VertexIdx)>) {
        face.in_arr_mut(&mut self.conns).1 = F::from_option(val);
    }
}

pub type PackedTetra<K, T = ()> = Tetra<K, PackedFace<K>, T>;

/// A vertex ID for a mesh.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct VertexId<K>(pub K);

/// A tetrahedron ID in a mesh.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct TetraId<K>(pub K);

/// A tetrahedral mesh.
pub trait TetraMesh {
    /// A key type to use for indices.
    type Key: Copy;
    /// The type to use for vertices.
    type Vertex: VertexData;
    /// The type to use for tetrahedra.
    type Tetra: TetraData<Self::Key>;
    /// Iterator type returned from [`Self::verts`].
    type VertsIter<'a>: Iterator<Item = (VertexId<Self::Key>, &'a Self::Vertex)>
    where
        Self: 'a;
    /// Iterator type returned from [`Self::Tetras`].
    type TetrasIter<'a>: Iterator<Item = (TetraId<Self::Key>, &'a Self::Tetra)>
    where
        Self: 'a;

    /// Get the vertex with the specified index.
    fn get_vertex(&self, id: VertexId<Self::Key>) -> Option<&Self::Vertex>;
    /// Get the tetrahedron with the specified index.
    fn get_tetra(&self, id: TetraId<Self::Key>) -> Option<&Self::Tetra>;

    /// Iterate over the vertices.
    fn verts(&self) -> Self::VertsIter<'_>;
    /// Iterate over the tetrahedra.
    fn tetras(&self) -> Self::TetrasIter<'_>;
    /// Quickly get the bounds for this mesh.
    ///
    /// The default value is `[Vec3::NEG_INFINITY, Vec3::INFINITY]`, but a more precise set of bounds should probably be used.
    fn bounds(&self) -> [Vec3; 2] {
        [Vec3::NEG_INFINITY, Vec3::INFINITY]
    }
}

/// A mutable tetrahedral mesh.
pub trait TetraMeshMut: TetraMesh
where
    Self::Vertex: VertexDataMut,
    Self::Tetra: TetraDataMut<Self::Key>,
{
    /// Mutably get the vertex with the specified index.
    fn get_vertex_mut(&mut self, id: VertexId<Self::Key>) -> Option<&mut Self::Vertex>;
    /// Mutably get the tetrahedron with the specified index.
    fn get_tetra_mut(&mut self, id: TetraId<Self::Key>) -> Option<&mut Self::Tetra>;
    /// Insert a vertex and get its ID.
    fn add_vertex(&mut self, vert: Self::Vertex) -> VertexId<Self::Key>;
    /// Insert a tetrahedron and get its ID.
    fn add_tetra(&mut self, tetra: Self::Tetra) -> TetraId<Self::Key>;
    /// Remove a vertex with a given ID.
    ///
    /// This is expected to be a stable operation, and the key may or may not be reused.
    fn remove_vertex(&mut self, id: VertexId<Self::Key>) -> Option<Self::Vertex>;
    /// Remove a tetrahedron with a given ID.
    ///
    /// This is expected to be a stable operation, and the key may or may not be reused.
    fn remove_tetra(&mut self, id: TetraId<Self::Key>) -> Option<Self::Tetra>;
}
