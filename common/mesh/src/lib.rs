pub mod builder;
pub mod ecs;
pub mod generation;
pub mod slab_mesh;
pub mod traits;

pub mod prelude {
    pub use crate::ecs::{DynMesh, SurfaceSync, sync_meshes};
    pub use crate::slab_mesh::{DefaultPackedMesh, SlabMesh};
    pub use crate::traits::{
        Tetra, TetraData, TetraDataMut, TetraId, TetraMesh, TetraMeshMut, Vertex, VertexData,
        VertexDataMut, VertexId,
    };
}
