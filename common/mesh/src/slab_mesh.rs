use crate::generation::*;
use crate::traits::*;
use bevy_math::Vec3;
use std::fmt::{self, Debug, Formatter};
use std::hash::Hash;
use std::marker::PhantomData;

fn add_point([lower, upper]: &mut [Vec3; 2], point: Vec3) {
    *lower = lower.min(point);
    *upper = upper.max(point);
}

/// A [`TetraMesh`] that's implemented using slabs.
pub struct SlabMesh<K, V, T, const GEN_BITS: usize = 0>
where
    BitMarker<GEN_BITS>: HasGeneration,
{
    pub verts: Slab<V, GEN_BITS>,
    pub tetras: Slab<T, GEN_BITS>,
    pub bounds: [Vec3; 2],
    pub _marker: PhantomData<K>,
}
impl<K, V, T, const GEN_BITS: usize> SlabMesh<K, V, T, GEN_BITS>
where
    BitMarker<GEN_BITS>: HasGeneration,
{
    /// Create a new, empty mesh.
    pub const fn new() -> Self {
        Self {
            verts: Slab::new(),
            tetras: Slab::new(),
            bounds: [Vec3::INFINITY, Vec3::NEG_INFINITY],
            _marker: PhantomData,
        }
    }
}
impl<K, V: VertexData, T, const GEN_BITS: usize> SlabMesh<K, V, T, GEN_BITS>
where
    BitMarker<GEN_BITS>: HasGeneration,
{
    /// Recompute the bounds of this mesh.
    ///
    /// This is only needed after points have been removed.
    pub fn shrink_bounds(&mut self) {
        self.bounds =
            self.verts
                .values()
                .fold([Vec3::INFINITY, Vec3::NEG_INFINITY], |[min, max], p| {
                    let v = p.as_vec3();
                    [v.min(min), v.max(max)]
                });
    }
}
impl<K, V, T, const GEN_BITS: usize> Default for SlabMesh<K, V, T, GEN_BITS>
where
    BitMarker<GEN_BITS>: HasGeneration,
{
    fn default() -> Self {
        Self::new()
    }
}
impl<K, V: Debug, T: Debug, const GEN_BITS: usize> Debug for SlabMesh<K, V, T, GEN_BITS>
where
    BitMarker<GEN_BITS>: HasGeneration<Generation: Debug>,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("SlabMesh")
            .field("verts", &self.verts)
            .field("tetras", &self.tetras)
            .finish()
    }
}
impl<K: SlabKey<GEN_BITS>, V: VertexData, T: TetraData<K>, const GEN_BITS: usize> TetraMesh
    for SlabMesh<K, V, T, GEN_BITS>
where
    BitMarker<GEN_BITS>: HasGeneration,
{
    type Key = K;
    type Vertex = V;
    type Tetra = T;
    type VertsIter<'a>
        = VertsIter<'a, K, V, GEN_BITS>
    where
        Self: 'a;
    type TetrasIter<'a>
        = TetrasIter<'a, K, T, GEN_BITS>
    where
        Self: 'a;
    type SurfaceSyncState = ();

    fn get_vertex(&self, id: VertexId<Self::Key>) -> Option<&Self::Vertex> {
        let (i, g) = id.0.unpack();
        self.verts.get(GenerationIndex::new(i, g))
    }
    fn get_tetra(&self, id: TetraId<Self::Key>) -> Option<&Self::Tetra> {
        let (i, g) = id.0.unpack();
        self.tetras.get(GenerationIndex::new(i, g))
    }
    #[inline(always)]
    fn verts(&self) -> Self::VertsIter<'_> {
        VertsIter {
            inner: self.verts.iter(),
            _marker: PhantomData,
        }
    }
    #[inline(always)]
    fn tetras(&self) -> Self::TetrasIter<'_> {
        TetrasIter {
            inner: self.tetras.iter(),
            _marker: PhantomData,
        }
    }
    #[inline(always)]
    fn bounds(&self) -> [Vec3; 2] {
        self.bounds
    }
    fn append_external_points<C: Extend<Vec3>>(&self, verts: &mut C) {
        let mut seen = fixedbitset::FixedBitSet::with_capacity(self.verts.max_idx());
        for (_, tet) in self.tetras() {
            let mut count = 0;
            for idx in VertexIdx::VALS {
                if tet.face(idx).is_none() {
                    for v in idx.others() {
                        let id = tet.vertex(v);
                        if !seen.put(id.0.unpack().0) {
                            verts.extend(self.get_vertex(id).map(VertexData::as_vec3));
                        }
                    }
                    count += 1;
                    if count == 2 {
                        break;
                    }
                }
            }
        }
    }
}
impl<K: SlabKey<GEN_BITS>, V: VertexDataMut, T: TetraDataMut<K>, const GEN_BITS: usize> TetraMeshMut
    for SlabMesh<K, V, T, GEN_BITS>
where
    BitMarker<GEN_BITS>: HasGeneration,
{
    fn get_vertex_mut(&mut self, id: VertexId<Self::Key>) -> Option<&mut Self::Vertex> {
        let (i, g) = id.0.unpack();
        self.verts.get_mut(GenerationIndex::new(i, g))
    }
    fn get_tetra_mut(&mut self, id: TetraId<Self::Key>) -> Option<&mut Self::Tetra> {
        let (i, g) = id.0.unpack();
        self.tetras.get_mut(GenerationIndex::new(i, g))
    }
    fn add_vertex(&mut self, vert: Self::Vertex) -> VertexId<Self::Key> {
        add_point(&mut self.bounds, vert.as_vec3());
        let idx = self.verts.insert(vert);
        VertexId(K::pack(idx.index, idx.generation))
    }
    fn add_tetra(&mut self, tetra: Self::Tetra) -> TetraId<Self::Key> {
        let adjs = VertexIdx::VALS.map(|v| (v, tetra.face(v)));
        let idx = self.tetras.insert(tetra);
        let tet = TetraId(K::pack(idx.index, idx.generation));
        for (v, adj) in adjs {
            if let Some((n, i)) = adj
                && let Some(t) = self.get_tetra_mut(n)
            {
                t.set_face(i, Some((tet, v)));
            }
        }
        tet
    }
    fn remove_vertex(&mut self, id: VertexId<Self::Key>) -> Option<Self::Vertex> {
        let (i, g) = id.0.unpack();
        self.verts.remove(GenerationIndex::new(i, g))
    }
    fn remove_tetra(&mut self, id: TetraId<Self::Key>) -> Option<Self::Tetra> {
        let (i, g) = id.0.unpack();
        let tet = self.tetras.remove(GenerationIndex::new(i, g));
        if let Some(tet) = &tet {
            for adj in VertexIdx::VALS.map(|v| tet.face(v)) {
                if let Some((n, i)) = adj
                    && let Some(t) = self.get_tetra_mut(n)
                {
                    t.set_face(i, None);
                }
            }
        }
        tet
    }
}

/// Iterator type for vertices in a [`SlabMesh`]
///
/// This just maps the values, but it saves eight bytes by inlining the function, in comparison to `std::iter::Map`.
pub struct VertsIter<'a, K, V, const BITS: usize>
where
    BitMarker<BITS>: HasGeneration,
{
    inner: super::generation::Iter<'a, <BitMarker<BITS> as HasGeneration>::Type<V>>,
    _marker: PhantomData<K>,
}
impl<'a, K: SlabKey<BITS>, V: 'a, const BITS: usize> Iterator for VertsIter<'a, K, V, BITS>
where
    BitMarker<BITS>: HasGeneration,
{
    type Item = (VertexId<K>, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|(k, v)| (VertexId(K::pack(k.index, k.generation)), v))
    }
}

/// Iterator type for tetrahedra in a [`SlabMesh`]
///
/// This just maps the values, but it saves eight bytes by inlining the function, in comparison to `std::iter::Map`.
pub struct TetrasIter<'a, K, T, const BITS: usize>
where
    BitMarker<BITS>: HasGeneration,
{
    inner: super::generation::Iter<'a, <BitMarker<BITS> as HasGeneration>::Type<T>>,
    _marker: PhantomData<K>,
}
impl<'a, K: SlabKey<BITS>, T: 'a, const BITS: usize> Iterator for TetrasIter<'a, K, T, BITS>
where
    BitMarker<BITS>: HasGeneration,
{
    type Item = (TetraId<K>, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|(k, v)| (TetraId(K::pack(k.index, k.generation)), v))
    }
}

/// A packed form of an index and generation count.
///
/// This is implemented for all integers, with no generation being specified, and `(integer, G)` tuples with the generation
/// being the second element.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a valid key for a {BITS}-bit generation counter"
)]
pub trait SlabKey<const BITS: usize>: Copy + Hash + Eq + Debug
where
    BitMarker<BITS>: HasGeneration,
{
    fn pack(index: usize, generation: <BitMarker<BITS> as HasGeneration>::Generation) -> Self;
    fn unpack(self) -> (usize, <BitMarker<BITS> as HasGeneration>::Generation);
}
macro_rules! impl_slab_key {
    ($($int:ty)*) => {
        $(
            #[diagnostic::do_not_recommend]
            impl SlabKey<0> for $int {
                fn pack(index: usize, _generation: ()) -> Self {
                    index as _
                }
                fn unpack(self) -> (usize, ()) {
                    (self as _, ())
                }
            }
            #[diagnostic::do_not_recommend]
            impl<const BITS: usize> SlabKey<BITS> for $int where BitMarker<BITS>: HasIntegerSize<Integer: TryFrom<Self> + Into<Self>> {
                fn pack(index: usize, generation: <BitMarker<BITS> as HasIntegerSize>::Integer) -> Self {
                    ((index as $int) << BITS) | generation.into()
                }
                fn unpack(self) -> (usize, <BitMarker<BITS> as HasIntegerSize>::Integer) {
                    ((self >> BITS) as _, (self & ((1 << BITS) - 1)).try_into().unwrap_or_else(|_| unreachable!()))
                }
            }
            impl<const BITS: usize> SlabKey<BITS> for ($int, <BitMarker<BITS> as HasGeneration>::Generation)
            where
                BitMarker<BITS>: HasGeneration<Generation: Hash + Eq + Debug>,
            {
                fn pack(index: usize, generation: <BitMarker<BITS> as HasGeneration>::Generation) -> Self {
                    (index as _, generation)
                }
                fn unpack(self) -> (usize, <BitMarker<BITS> as HasGeneration>::Generation) {
                    (self.0 as _, self.1)
                }
            }
        )*
    };
}
impl_slab_key!(u8 u16 u32 u64 usize);

/// A mesh that uses a packed layout.
///
/// The maximum number of vertices is `K::MAX >> GEN_BITS` and the maximum number of tetrahedra is `K::MAX >> (GEN_BITS + 2)`
pub type DefaultPackedMesh<K, V = (), T = (), const GEN_BITS: usize = 0> =
    SlabMesh<K, Vertex<V>, Tetra<K, PackedFace<K>, T>, GEN_BITS>;

// all of these are tests to make sure that what we call a mesh is valid.
#[allow(dead_code)]
mod tests {
    use super::*;

    struct AssertMesh<T: TetraMesh>(T);

    struct AssertValidPacked1(AssertMesh<DefaultPackedMesh<u32, i32, String, 4>>);
    struct AssertValidPacked2(AssertMesh<DefaultPackedMesh<u16, (), (), 9>>);
    // more generation bits than can fit in a u8, this is a compile error
    // struct AssertInvliadPacked(AssertMesh<DefaultPackedMesh<u8, (), (), 20>>);
}
