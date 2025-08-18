use crate::volume::generation::*;
use crate::volume::traits::*;
use std::marker::PhantomData;

/// A [`TetraMesh`] that's implemented using slabs.
pub struct SlabMesh<K: SlabKey<GEN_BITS>, V, T, const GEN_BITS: usize = 0>
where
    BitMarker<GEN_BITS>: HasGeneration,
{
    pub verts: Slab<V, GEN_BITS>,
    pub tetras: Slab<T, GEN_BITS>,
    pub _marker: PhantomData<K>,
}
impl<K: SlabKey<GEN_BITS>, V: VertexData, T: TetraData<K>, const GEN_BITS: usize> TetraMesh
    for SlabMesh<K, V, T, GEN_BITS>
where
    BitMarker<GEN_BITS>: HasGeneration,
{
    type Key = K;
    type Vertex = V;
    type Tetra = T;

    fn get_vertex(&self, id: VertexId<Self::Key>) -> Option<&Self::Vertex> {
        let (i, g) = id.0.unpack();
        self.verts.get(GenerationIndex::new(i, g))
    }
    fn get_tetra(&self, id: TetraId<Self::Key>) -> Option<&Self::Tetra> {
        let (i, g) = id.0.unpack();
        self.tetras.get(GenerationIndex::new(i, g))
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
        let idx = self.verts.insert(vert);
        VertexId(K::pack(idx.index, idx.generation))
    }
    fn add_tetra(&mut self, tetra: Self::Tetra) -> TetraId<Self::Key> {
        let idx = self.tetras.insert(tetra);
        TetraId(K::pack(idx.index, idx.generation))
    }
    fn remove_vertex(&mut self, id: VertexId<Self::Key>) -> Option<Self::Vertex> {
        let (i, g) = id.0.unpack();
        self.verts.remove(GenerationIndex::new(i, g))
    }
    fn remove_tetra(&mut self, id: TetraId<Self::Key>) -> Option<Self::Tetra> {
        let (i, g) = id.0.unpack();
        self.tetras.remove(GenerationIndex::new(i, g))
    }
}

/// A packed form of an index and generation count.
///
/// This is implemented for all integers, with no generation being specified, and `(integer, G)` tuples with the generation
/// being the second element.
pub trait SlabKey<const BITS: usize>: Copy
where
    BitMarker<BITS>: HasGeneration,
{
    fn pack(index: usize, generation: <BitMarker<BITS> as HasGeneration>::Generation) -> Self;
    fn unpack(self) -> (usize, <BitMarker<BITS> as HasGeneration>::Generation);
}
macro_rules! impl_slab_key {
    ($($int:ty)*) => {
        $(
            impl SlabKey<0> for $int {
                fn pack(index: usize, _generation: ()) -> Self {
                    index as _
                }
                fn unpack(self) -> (usize, ()) {
                    (self as _, ())
                }
            }
            impl<const BITS: usize> SlabKey<BITS> for $int where BitMarker<BITS>: HasIntegerSize<Integer: TryFrom<Self> + Into<Self>> {
                fn pack(index: usize, generation: <BitMarker<BITS> as HasIntegerSize>::Integer) -> Self {
                    ((index as $int) << BITS) | generation.into()
                }
                fn unpack(self) -> (usize, <BitMarker<BITS> as HasIntegerSize>::Integer) {
                    ((self >> BITS) as _, (self & ((1 << BITS) - 1)).try_into().unwrap_or_else(|_| unreachable!()))
                }
            }
            impl<const BITS: usize> SlabKey<BITS> for ($int, <BitMarker<BITS> as HasGeneration>::Generation) where
            BitMarker<BITS>: HasGeneration,{
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

pub type DefaultPackedMesh<K, V = (), T = (), const GEN_BITS: usize = 0> =
    SlabMesh<K, Vertex<V>, Tetra<K, PackedFace<K>, T>, GEN_BITS>;

#[allow(dead_code)]
mod tests {
    type AssertValidPacked = super::DefaultPackedMesh<u32, i32, String, 4>;
}
