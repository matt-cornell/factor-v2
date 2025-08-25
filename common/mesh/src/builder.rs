use crate::traits::*;
use bevy_math::{Quat, Vec2, Vec3};
use std::hash::Hash;

#[doc(hidden)]
pub use bevy_math::Affine3A;

/// A buildable mesh.
///
/// This is implemented for any [`TetraMeshMut`] whose vertices and tetrahedra can be constructed from `Vec3` and `TetraPrimitive<Self::Key>`, respectively.
pub trait BuildMesh {
    type Key: Hash + Eq + Copy;

    fn add_vertex(&mut self, vert: Vec3) -> VertexId<Self::Key>;
    fn add_tetra(&mut self, tetra: TetraPrimitive<Self::Key>) -> TetraId<Self::Key>;
}
impl<M: TetraMeshMut> BuildMesh for &mut M
where
    M::Vertex: From<Vec3>,
    M::Tetra: From<TetraPrimitive<M::Key>>,
{
    type Key = <M as TetraMesh>::Key;

    fn add_vertex(&mut self, vert: Vec3) -> VertexId<Self::Key> {
        TetraMeshMut::add_vertex(*self, vert.into())
    }
    fn add_tetra(&mut self, tetra: TetraPrimitive<Self::Key>) -> TetraId<Self::Key> {
        TetraMeshMut::add_tetra(*self, tetra.into())
    }
}

pub trait MeshBuilder {
    type Transformed: MeshBuilder
    where
        Self: Sized;

    fn append_to<M: BuildMesh>(&self, mesh: M);
    fn build<M: Default>(&self) -> M
    where
        for<'a> &'a mut M: BuildMesh,
    {
        let mut mesh = M::default();
        self.append_to(&mut mesh);
        mesh
    }
    fn transform(self, transform: Affine3A) -> Self::Transformed
    where
        Self: Sized;
    fn translate(self, translation: Vec3) -> Self::Transformed
    where
        Self: Sized,
    {
        self.transform(Affine3A::from_translation(translation))
    }
    fn rotate(self, rotation: Quat) -> Self::Transformed
    where
        Self: Sized,
    {
        self.transform(Affine3A::from_quat(rotation))
    }
}

/// A type that applies a transformation to either a builder or a mesh.
///
/// This can be used either as `Transformed::new(builder, transform).append_to(mesh)` or
/// `builder.append_to(Transformed::new(mesh))`. Note that the [`MeshBuilder::transform`] method can be more efficient
/// in many cases by eagerly transforming all of the points.
#[derive(Debug, Clone, Copy)]
pub struct Transformed<B> {
    pub base: B,
    pub transform: Affine3A,
}
impl<B> Transformed<B> {
    #[inline(always)]
    pub const fn new(base: B, transform: Affine3A) -> Self {
        Self { base, transform }
    }
}
impl<B: MeshBuilder> MeshBuilder for Transformed<B> {
    type Transformed = Self;
    fn append_to<M: BuildMesh>(&self, mesh: M) {
        self.base.append_to(Transformed::new(mesh, self.transform))
    }
    fn transform(self, transform: Affine3A) -> Self::Transformed {
        Self::new(self.base, transform * self.transform)
    }
}
impl<M: BuildMesh> BuildMesh for Transformed<M> {
    type Key = M::Key;

    fn add_vertex(&mut self, vert: Vec3) -> VertexId<Self::Key> {
        self.base.add_vertex(self.transform.transform_point3(vert))
    }
    #[inline(always)]
    fn add_tetra(&mut self, tetra: TetraPrimitive<Self::Key>) -> TetraId<Self::Key> {
        self.base.add_tetra(tetra)
    }
}

/// Helper macro for [`MeshBuilder`] implementation.
///
/// This creates a lazy transform by using [`Transformed`] as the transformed builder.
#[macro_export]
macro_rules! impl_builder_via_transformed {
    () => {
        type Transformed = $crate::builder::Transformed<Self>;

        fn transform(self, transform: $crate::builder::Affine3A) -> Self::Transformed {
            $crate::builder::Transformed::new(self, transform)
        }
    };
}

/// A hexahedron, or a "deformed cube".
///
/// Points are expected to form a convex hexahedron and be given in a U-shaped order, such as:
/// ```text
/// (0, 0, 0)
/// (1, 0, 0)
/// (1, 1, 0)
/// (0, 1, 0)
/// (0, 0, 1)
/// (1, 0, 1)
/// (1, 1, 1)
/// (0, 1, 1)
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hexahedron {
    pub points: [Vec3; 8],
}
impl Hexahedron {
    pub const UNIT_CUBE: Self = Self::cuboid(Vec3::ZERO, Vec3::ONE);
    pub const CENTERED_CUBE: Self = Self::cuboid(Vec3::NEG_ONE, Vec3::ONE);
    #[inline(always)]
    pub const fn new(points: [Vec3; 8]) -> Self {
        Self { points }
    }
    #[inline(always)]
    pub const fn cuboid(min: Vec3, max: Vec3) -> Self {
        let Vec3 {
            x: x1,
            y: y1,
            z: z1,
        } = min;
        let Vec3 {
            x: x2,
            y: y2,
            z: z2,
        } = max;
        Self::new([
            Vec3::new(x1, y1, z1),
            Vec3::new(x2, y1, z1),
            Vec3::new(x2, y2, z1),
            Vec3::new(x1, y2, z1),
            Vec3::new(x1, y1, z2),
            Vec3::new(x2, y1, z2),
            Vec3::new(x2, y2, z2),
            Vec3::new(x1, y2, z2),
        ])
    }
}
impl From<Cuboid> for Hexahedron {
    fn from(value: Cuboid) -> Self {
        Self::cuboid(value.min, value.max)
    }
}
impl MeshBuilder for Hexahedron {
    type Transformed = Self;

    fn append_to<M: BuildMesh>(&self, mut mesh: M) {
        let [a, b, c, d, e, f, g, h] = self.points.map(|p| mesh.add_vertex(p));
        let center = mesh.add_tetra([(b, None), (e, None), (d, None), (g, None)]);
        mesh.add_tetra([
            (a, Some((center, VertexIdx::V3))),
            (b, None),
            (e, None),
            (d, None),
        ]);
        mesh.add_tetra([
            (f, Some((center, VertexIdx::V2))),
            (b, None),
            (g, None),
            (e, None),
        ]);
        mesh.add_tetra([
            (c, Some((center, VertexIdx::V1))),
            (b, None),
            (d, None),
            (g, None),
        ]);
        mesh.add_tetra([
            (h, Some((center, VertexIdx::V0))),
            (g, None),
            (d, None),
            (e, None),
        ]);
    }
    fn transform(self, transform: Affine3A) -> Self::Transformed {
        Self::new(self.points.map(|p| transform.transform_point3(p)))
    }
}

/// A simple, axis-aligned cuboid.
///
/// This delegates to [`Hexahedron`] to actually build the mesh.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cuboid {
    pub min: Vec3,
    pub max: Vec3,
}
impl Cuboid {
    pub const UNIT_CUBE: Self = Self::new(Vec3::ZERO, Vec3::ONE);
    pub const CENTERED_CUBE: Self = Self::new(Vec3::NEG_ONE, Vec3::ONE);
    pub const fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }
}
impl MeshBuilder for Cuboid {
    type Transformed = Hexahedron;

    fn append_to<M: BuildMesh>(&self, mesh: M) {
        Hexahedron::cuboid(self.min, self.max).append_to(mesh);
    }
    fn transform(self, transform: Affine3A) -> Self::Transformed {
        Hexahedron::cuboid(self.min, self.max).transform(transform)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bipyramid<B> {
    pub apexes: [Vec3; 2],
    pub base: B,
}
impl<B> Bipyramid<B> {
    pub const fn new(apexes: [Vec3; 2], base: B) -> Self {
        Self { apexes, base }
    }
}
impl<const N: usize> Bipyramid<[Vec3; N]> {
    const fn evenly_spaced_base() -> [Vec3; N] {
        let mut out = [Vec3::ZERO; N];
        let f = const_soft_float::soft_f32::SoftF32::from_f32(std::f32::consts::TAU / N as f32);
        let rot = Vec2::new(f.cos().to_f32(), f.sin().to_f32());
        let mut i = 0;
        let mut v = Vec2::X;
        while i < N {
            out[i] = v.extend(0.0);
            v = Vec2::new(v.x * rot.x - v.y * rot.y, v.x * rot.y + v.y * rot.x);
            i += 1;
        }
        out
    }
    pub const EVENLY_SPACED_BASE: [Vec3; N] = Self::evenly_spaced_base();
    pub const CENTERED: Self = Self {
        apexes: [Vec3::NEG_Z, Vec3::Z],
        base: Self::EVENLY_SPACED_BASE,
    };
}
impl Bipyramid<Vec<Vec3>> {
    pub fn evenly_spaced_base(points: usize, start: Vec2) -> Vec<Vec3> {
        let mut out = Vec::with_capacity(points);
        let rot = Vec2::from_angle(std::f32::consts::TAU / points as f32);
        let mut v = start;
        for _ in 0..points {
            out.push(v.extend(0.0));
            v = v.rotate(rot);
        }
        out
    }
    pub fn centered(points: usize) -> Self {
        Self {
            apexes: [Vec3::NEG_Z, Vec3::Z],
            base: Self::evenly_spaced_base(points, Vec2::X),
        }
    }
}
impl<B: AsRef<[Vec3]> + AsMut<[Vec3]>> MeshBuilder for Bipyramid<B> {
    type Transformed = Self;
    fn append_to<M: BuildMesh>(&self, mut mesh: M) {
        let mut it = self.base.as_ref().iter();
        let Some(&v1) = it.next() else { return };
        let Some(&v2) = it.next() else { return };
        let [a, b] = self.apexes.map(|p| mesh.add_vertex(p));
        let first = mesh.add_vertex(v1);
        let mut last = mesh.add_vertex(v2);
        let first_tet = mesh.add_tetra([(a, None), (b, None), (first, None), (last, None)]);
        if let Some(&v3) = it.next() {
            let mut old = std::mem::replace(&mut last, mesh.add_vertex(v3));
            let mut last_tet = mesh.add_tetra([
                (a, None),
                (b, None),
                (old, None),
                (last, Some((first_tet, VertexIdx::V2))),
            ]);
            for &v in it {
                old = std::mem::replace(&mut last, mesh.add_vertex(v));
                last_tet = mesh.add_tetra([
                    (a, None),
                    (b, None),
                    (old, None),
                    (last, Some((last_tet, VertexIdx::V2))),
                ]);
            }
            mesh.add_tetra([
                (a, None),
                (b, None),
                (last, Some((first_tet, VertexIdx::V3))),
                (first, Some((last_tet, VertexIdx::V2))),
            ]);
        }
    }
    fn transform(mut self, transform: Affine3A) -> Self::Transformed {
        let [a, b] = &mut self.apexes;
        *a = transform.transform_point3(*a);
        *b = transform.transform_point3(*b);
        for pt in self.base.as_mut() {
            *pt = transform.transform_point3(*pt);
        }
        self
    }
}
pub type Octahedron = Bipyramid<[Vec3; 4]>;
