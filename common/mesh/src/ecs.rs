//! Utilities for using meshes with Bevy's ECS.

use crate::traits::*;
use bevy_asset::Assets;
use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::entity::Entity;
#[cfg(feature = "render")]
use bevy_ecs::system::ResMut;
use bevy_ecs::system::{Commands, Query};
use bevy_ecs::world::Ref;
use bevy_math::Vec3;
#[cfg(feature = "render")]
use bevy_render::mesh::{Mesh, Mesh3d};
use std::any::Any;
use std::fmt::{self, Debug, Formatter};

/// A dyn-compatible tetrahedral mesh.
pub trait TetraMeshDyn {
    fn append_all(&self, verts: &mut Vec<Vec3>, faces: &mut Vec<[u32; 3]>);
    /// See [`TetraMesh::append_primitive_surface`].
    fn append_primitive_surface(&self, verts: &mut Vec<Vec3>, faces: &mut Vec<[u32; 3]>);
    /// See [`TetraMesh::sync_primitve_surface`].
    ///
    /// The state can be initialized with `None` for the first run.
    fn sync_primitive_surface(
        &self,
        verts: &mut Vec<Vec3>,
        faces: &mut Vec<[u32; 3]>,
        state: &mut Option<Box<dyn Any + Send + Sync>>,
    );
    /// See [`TetraMesh::append_external_points`].
    fn append_external_points(&self, points: &mut Vec<Vec3>);
}
impl<T: TetraMesh> TetraMeshDyn for T {
    fn append_all(&self, verts: &mut Vec<Vec3>, faces: &mut Vec<[u32; 3]>) {
        let mut lookup = std::collections::HashMap::new();
        let mut i = verts.len() as u32;
        verts.extend(self.verts().map(|(k, v)| {
            lookup.insert(k, i);
            i += 1;
            v.as_vec3()
        }));
        faces.extend(self.tetras().flat_map(|(_, t)| {
            let [Some(&a), Some(&b), Some(&c), Some(&d)] =
                VertexIdx::VALS.map(|v| lookup.get(&t.vertex(v)))
            else {
                return None.into_iter().flatten();
            };
            Some([[a, b, c], [b, c, d], [c, d, a], [b, c, d]])
                .into_iter()
                .flatten()
        }));
    }
    fn append_primitive_surface(&self, verts: &mut Vec<Vec3>, faces: &mut Vec<[u32; 3]>) {
        TetraMesh::append_primitive_surface(self, verts, faces);
    }
    fn sync_primitive_surface(
        &self,
        verts: &mut Vec<Vec3>,
        faces: &mut Vec<[u32; 3]>,
        state: &mut Option<Box<dyn Any + Send + Sync>>,
    ) {
        let r: &mut T::SurfaceSyncState = if let Some(state) = state {
            if let Some(r) = state.downcast_mut() {
                r
            } else {
                *state = Box::new(T::SurfaceSyncState::default());
                state.downcast_mut().unwrap()
            }
        } else {
            *state = Some(Box::new(T::SurfaceSyncState::default()));
            state.as_mut().unwrap().downcast_mut().unwrap()
        };
        TetraMesh::sync_primitive_surface(self, verts, faces, r);
    }
    fn append_external_points(&self, verts: &mut Vec<Vec3>) {
        TetraMesh::append_external_points(self, verts);
    }
}

/// An ECS component for a tetrahedral mesh.
#[derive(bevy_ecs_macros::Component)]
pub struct DynMesh(pub Box<dyn TetraMeshDyn + Send + Sync>);
impl Debug for DynMesh {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("DynMesh(..)")
    }
}
impl<M: TetraMeshDyn + Send + Sync + 'static> From<M> for DynMesh {
    fn from(value: M) -> Self {
        Self(Box::new(value))
    }
}

/// An ECS component for the surface synch
#[derive(Default, bevy_ecs_macros::Component)]
pub struct SurfaceSync {
    pub state: Option<Box<dyn Any + Send + Sync>>,
    pub internal: bool,
}
impl Debug for SurfaceSync {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("SurfaceSync")
            .field("state", &self.state.as_ref().map(|_| ..))
            .field("internal", &self.internal)
            .finish()
    }
}

/// Query data for [`sync_meshes`].
#[derive(bevy_ecs_macros::QueryData)]
#[query_data(mutable)]
pub struct MeshQuery {
    entity: Entity,
    mesh: Ref<'static, DynMesh>,
    sync: &'static mut SurfaceSync,
    #[cfg(feature = "render")]
    render: Option<&'static Mesh3d>,
}

/// This event gets triggered for every mesh that gets updated by [`sync_meshes`].
#[derive(Debug, Clone, Copy, bevy_ecs_macros::Event)]
pub struct UpdatedMesh;

/// Synchronize the surface of a tetrahedral mesh with a rendered mesh.
#[cfg(feature = "render")]
pub fn sync_meshes(
    mut commands: Commands,
    mut query: Query<MeshQuery>,
    #[cfg(feature = "render")] mut meshes: ResMut<Assets<Mesh>>,
) {
    use bevy_asset::RenderAssetUsages;
    use bevy_render::mesh::{Indices, VertexAttributeValues};

    for mut item in query.iter_mut() {
        if !(item.mesh.is_changed() || item.sync.is_changed()) {
            continue;
        }
        #[cfg(feature = "render")]
        if let Some(mesh) = item.render.and_then(|h| meshes.get_mut(h)) {
            let mut faces = mesh.remove_indices().map_or(Vec::new(), |v| {
                if let Indices::U32(v) = v {
                    bytemuck::try_cast_vec(v).unwrap_or_default()
                } else {
                    Vec::new()
                }
            });
            let mut verts = if let Some(VertexAttributeValues::Float32x3(v)) =
                mesh.remove_attribute(Mesh::ATTRIBUTE_POSITION)
            {
                bytemuck::cast_vec(v)
            } else {
                Vec::new()
            };

            if item.sync.internal {
                verts.clear();
                faces.clear();
                item.mesh.0.append_all(&mut verts, &mut faces);
            } else {
                item.mesh
                    .0
                    .sync_primitive_surface(&mut verts, &mut faces, &mut item.sync.state);
            }

            mesh.insert_indices(Indices::U32(bytemuck::cast_vec(faces)));
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, verts);
        } else {
            let mut verts = Vec::new();
            let mut faces = Vec::new();
            if item.sync.internal {
                item.mesh.0.append_all(&mut verts, &mut faces);
            } else {
                item.mesh
                    .0
                    .sync_primitive_surface(&mut verts, &mut faces, &mut item.sync.state);
            }
            let handle = meshes.add(
                Mesh::new(
                    bevy_render::mesh::PrimitiveTopology::TriangleList,
                    RenderAssetUsages::RENDER_WORLD,
                )
                .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, verts)
                .with_inserted_indices(Indices::U32(bytemuck::cast_vec(faces))),
            );
            commands.entity(item.entity).insert(Mesh3d(handle));
            commands.trigger_targets(UpdatedMesh, item.entity);
        }
    }
}

#[cfg(not(feature = "render"))]
pub fn sync_meshes() {}
