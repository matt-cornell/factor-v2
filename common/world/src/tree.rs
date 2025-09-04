use bevy_math::DVec2;
use std::fmt;
use std::num::NonZero;
use std::ops::{ControlFlow, Range};

/// A region of the world.
///
/// `R0..=R4` are northern regions and `R5..=R9` are southern regions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum WorldRegion {
    R0,
    R1,
    R2,
    R3,
    R4,
    R5,
    R6,
    R7,
    R8,
    R9,
}
impl WorldRegion {
    /// Transmuate a `u8` to `Self`.
    ///
    /// ## Safety
    /// `idx` must be in `0..10`.
    #[inline(always)]
    pub const unsafe fn from_u8_unchecked(idx: u8) -> Self {
        unsafe { std::mem::transmute::<u8, Self>(idx) }
    }
    /// Convert `self` to a `usize`.
    #[inline(always)]
    pub const fn to_index(self) -> usize {
        self as usize
    }
    /// Flip between upper and lower halves.
    #[inline(always)]
    pub const fn flipped(self) -> Self {
        unsafe { Self::from_u8_unchecked((self as u8 + 5) % 10) }
    }
    #[inline(always)]
    pub const fn is_north(&self) -> bool {
        matches!(self, Self::R0 | Self::R1 | Self::R2 | Self::R3 | Self::R4)
    }
    #[inline(always)]
    pub const fn is_south(&self) -> bool {
        matches!(self, Self::R5 | Self::R6 | Self::R7 | Self::R8 | Self::R9)
    }
    #[inline(always)]
    pub const fn hemisphere(&self) -> Hemisphere {
        match self {
            Self::R0 | Self::R1 | Self::R2 | Self::R3 | Self::R4 => Hemisphere::North,
            Self::R5 | Self::R6 | Self::R7 | Self::R8 | Self::R9 => Hemisphere::South,
        }
    }
}
impl fmt::Display for WorldRegion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hemisphere {
    North,
    South,
}

/// Which half of the region the tree is in.
///
/// The pyramid triangles are in the polar regions, while the antiprism triangles are in the middle region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum RegionHalf {
    Pyramid = 0,
    Antiprism = 1,
}
impl RegionHalf {
    /// Transmuate a `u8` to `Self`.
    ///
    /// ## Safety
    /// `idx` must be 0 or 1.
    pub const unsafe fn from_u8_unchecked(idx: u8) -> Self {
        unsafe { std::mem::transmute::<u8, Self>(idx) }
    }
}

/// Horizontal subdivision of a triangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum HorizontalSubdivision {
    NorthSouth = 0,
    Center = 1,
    West = 2,
    East = 3,
}
impl HorizontalSubdivision {
    /// Transmuate a `u8` to `Self`.
    ///
    /// ## Safety
    /// `idx` must be in `0..4`.
    pub const unsafe fn from_u8_unchecked(idx: u8) -> Self {
        unsafe { std::mem::transmute::<u8, Self>(idx) }
    }
    /// Get the subdivision for a point with north-to-west and north-to-east (or south-to west and south-to-east) barycentric coordinates.
    ///
    /// This returns the point in the subdivision, along with n2w and n2e coordinates that could be passed to this call again.
    pub fn subdivide_floats(n2w: f64, n2e: f64) -> (Self, f64, f64) {
        debug_assert!(
            n2w + n2e <= 1.0,
            "barycentric coordinates out of bounds for triangle subdivision"
        );
        if n2w > 0.5 {
            (Self::West, n2w.mul_add(2.0, -1.0), n2e * 2.0)
        } else if n2e > 0.5 {
            (Self::East, n2w * 2.0, n2e.mul_add(2.0, -1.0))
        } else if n2w + n2e >= 0.5 {
            (Self::Center, n2w.mul_add(-2.0, 1.0), n2e.mul_add(-2.0, 1.0))
        } else {
            (Self::NorthSouth, n2w * 2.0, n2e * 2.0)
        }
    }
    /// Subdive a point from north-to-west and north-to-east barycentric coordinates.
    ///
    /// See [`Self::subdivide_floats`] for more information.
    pub const fn subdivide(n2w: u32, n2e: u32) -> (Self, u32, u32) {
        const HALF: u32 = 1 << 28;
        const ONE: u32 = 1 << 29;
        const MASK: u32 = ONE - 1;
        debug_assert!(
            matches!(n2w.checked_add(n2e), Some(0..=ONE)),
            "barycentric coordinates out of bounds for triangle subdivision"
        );
        if n2w > HALF {
            (Self::West, (n2w << 1) & MASK, n2e << 1)
        } else if n2e > HALF {
            (Self::East, n2w << 1, (n2e << 1) & MASK)
        } else if n2w + n2e >= HALF {
            (Self::Center, ONE - (n2w << 1), ONE - (n2e << 1))
        } else {
            (Self::NorthSouth, n2w << 1, n2e << 1)
        }
    }
}

pub mod barycentric {
    use super::{Hemisphere, RegionHalf};
    use bevy_math::DMat2;
    /// Find the base transform matrix for a region to convert it to barycentric coordiantes in a triangle.
    ///
    /// For a point p, `base_transform() * (p + DVec2::Y * base_y_shift(hemisphere, half)` gives the north-to-west and north-to-east coordinataes.
    pub const fn base_transform(hemisphere: Hemisphere, half: RegionHalf) -> DMat2 {
        // generated with the following code:
        // fn main() {
        //     const MAX_X: f64 = 0.5344796660577975; // PI/5 * cos(TRANS_LAT)
        //     const MAX_PYR_Y: f64 = 1.0172219678978514; // PI/2 - TRANS_LAT
        //     const MAX_ANTI_Y: f64 = 1.1071487177940906; // 2 * TRANS_LAT
        //
        //     println!("NP: {:?}", DMat2::from_cols_array(&[-MAX_X, -MAX_PYR_Y,  MAX_X, -MAX_PYR_Y]).inverse().to_cols_array());
        //     println!("SP: {:?}", DMat2::from_cols_array(&[-MAX_X,  MAX_PYR_Y,  MAX_X,  MAX_PYR_Y]).inverse().to_cols_array());
        //     println!("NA: {:?}", DMat2::from_cols_array(&[-MAX_X,  MAX_ANTI_Y, MAX_X,  MAX_ANTI_Y]).inverse().to_cols_array());
        //     println!("SA: {:?}", DMat2::from_cols_array(&[-MAX_X, -MAX_ANTI_Y, MAX_X, -MAX_ANTI_Y]).inverse().to_cols_array());
        // }

        match (hemisphere, half) {
            (Hemisphere::North, RegionHalf::Pyramid) => DMat2::from_cols_array(&[
                -0.935489283788639,
                0.935489283788639,
                -0.4915348033952503,
                -0.4915348033952503,
            ]),
            (Hemisphere::South, RegionHalf::Pyramid) => DMat2::from_cols_array(&[
                -0.935489283788639,
                0.935489283788639,
                0.4915348033952503,
                0.4915348033952503,
            ]),
            (Hemisphere::North, RegionHalf::Antiprism) => DMat2::from_cols_array(&[
                -0.9354892837886393,
                0.9354892837886393,
                0.4516105126294252,
                0.4516105126294252,
            ]),
            (Hemisphere::South, RegionHalf::Antiprism) => DMat2::from_cols_array(&[
                -0.9354892837886393,
                0.9354892837886393,
                -0.4516105126294252,
                -0.4516105126294252,
            ]),
        }
    }
    /// Find the base Y shift to be applied before the transform matrix.
    ///
    /// See [`base_transform`] for more information on the transform.
    pub const fn base_y_shift(hemisphere: Hemisphere, half: RegionHalf) -> f64 {
        const MAX_PYR_Y: f64 = std::f64::consts::FRAC_PI_2 - crate::ico::TRANS_LAT;
        const MAX_ANTI_Y: f64 = 2.0 * crate::ico::TRANS_LAT;
        match (hemisphere, half) {
            (Hemisphere::North, RegionHalf::Pyramid) => -MAX_PYR_Y,
            (Hemisphere::South, RegionHalf::Pyramid) => MAX_PYR_Y,
            (Hemisphere::North, RegionHalf::Antiprism) => MAX_ANTI_Y,
            (Hemisphere::South, RegionHalf::Antiprism) => -MAX_ANTI_Y,
        }
    }
}

/// Vertical subdivision of a prism.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum VerticalSubdivision {
    Lower = 0b000,
    Upper = 0b100,
}
impl VerticalSubdivision {
    /// Transmuate a `u8` to `Self`.
    ///
    /// ## Safety
    /// `idx` must be 0 or 4.
    pub const unsafe fn from_u8_unchecked(idx: u8) -> Self {
        unsafe { std::mem::transmute::<u8, Self>(idx) }
    }
}

/// Error returned by octree subdividing methods if they'd exceed the maximum depth.
#[derive(Debug, Clone, Copy, thiserror::Error)]
#[error("Octree depth can't exceed 19 subdivisions")]
pub struct MaxOctreeDepth;

/// Error returned by quadtree subdividing methods if they'd exceed the maximum depth.
#[derive(Debug, Clone, Copy, thiserror::Error)]
#[error("Quadtree depth can't exceed 29 subdivisions")]
pub struct MaxQuadtreeDepth;

/// An octree index for three-dimensional space.
///
/// This stores both the depth and the subdivisions in a single `u64`. It has a maximum depth of 19 subdivisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OctreeIndex(pub NonZero<u64>);
impl OctreeIndex {
    /// Convert a [`WorldRegion`] to an `OctreeIndex`.
    ///
    /// The zero-depth representation uses six bits.
    pub const fn from_region(region: WorldRegion, triangle: RegionHalf) -> Self {
        unsafe {
            Self(NonZero::new_unchecked(
                ((region as u64) << 1) | triangle as u64 | 0x20,
            ))
        }
    }
    /// Convert a `u64` to an `OctreeIndex`.
    ///
    /// ## Safety
    /// `raw != 0`
    pub const unsafe fn from_u64_unchecked(raw: u64) -> Self {
        unsafe { Self(NonZero::new_unchecked(raw)) }
    }
    /// Get number of subdivisions in this index.
    pub const fn depth(&self) -> u32 {
        let bits = 58 - self.0.leading_zeros();
        debug_assert!(bits.is_multiple_of(3), "invalid octree index");
        bits / 3
    }
    /// Get the child indices of this node.
    pub const fn children(self) -> Result<Range<Self>, MaxOctreeDepth> {
        let Some(first) = self.0.get().checked_shl(3) else {
            return Err(MaxOctreeDepth);
        };
        let last = first + 8;
        unsafe { Ok(Self::from_u64_unchecked(first)..Self::from_u64_unchecked(last)) }
    }
    /// Get the child with the given horizontal and vertical subdivisions.
    pub const fn child(
        self,
        horiz: HorizontalSubdivision,
        vert: VerticalSubdivision,
    ) -> Result<Self, MaxOctreeDepth> {
        let Some(first) = self.0.get().checked_shl(3) else {
            return Err(MaxOctreeDepth);
        };
        unsafe { Ok(Self::from_u64_unchecked(first | horiz as u64 | vert as u64)) }
    }
    /// Get the parent index and subdivision if this isn't a root index.
    pub const fn parent(self) -> Option<(Self, HorizontalSubdivision, VerticalSubdivision)> {
        if self.0.get() < 0x80 {
            return None;
        }
        let raw = self.0.get();
        unsafe {
            Some((
                Self::from_u64_unchecked(raw >> 3),
                HorizontalSubdivision::from_u8_unchecked((raw & 0b011) as u8),
                VerticalSubdivision::from_u8_unchecked((raw & 0b100) as u8),
            ))
        }
    }
    /// If this is a root index, get the containing region.
    pub const fn is_root(self) -> Option<(WorldRegion, RegionHalf)> {
        let raw = self.0.get();
        if raw < 0x3f {
            unsafe {
                Some((
                    WorldRegion::from_u8_unchecked(raw as u8 >> 1),
                    RegionHalf::from_u8_unchecked(raw as u8 & 1),
                ))
            }
        } else {
            None
        }
    }
    /// Get the containing region of this node.
    pub const fn region(self) -> (WorldRegion, RegionHalf) {
        unsafe {
            let shift = 58 - self.0.leading_zeros();
            let raw = self.0.get() >> shift;
            (
                WorldRegion::from_u8_unchecked(raw as u8 >> 1),
                RegionHalf::from_u8_unchecked(raw as u8 & 1),
            )
        }
    }
    /// Find the index of a point given by longitude, latitude, and altitude.
    ///
    /// Altitude is expected to have an absolute value less than 32,768. The visitor function defines control
    /// flow to allow an early break if only a lower resolution is needed.
    pub fn hash<R, F: FnMut(Self) -> ControlFlow<R>>(
        lon: f64,
        lat: f64,
        alt: f64,
        mut visit: F,
    ) -> (Self, Option<R>) {
        let (base, off) = crate::ico::region_offset(lon, lat);
        let half = if base.is_south() ^ (off.y < 0.0) {
            RegionHalf::Antiprism
        } else {
            RegionHalf::Pyramid
        };
        let mut index = OctreeIndex::from_region(base, half);
        if let ControlFlow::Break(ret) = visit(index) {
            return (index, Some(ret));
        }
        let hemi = base.hemisphere();
        let [mut n2w, mut n2e] = (barycentric::base_transform(hemi, half)
            * (off + DVec2::new(0.0, barycentric::base_y_shift(hemi, half))))
        .to_array()
        .map(|c| (c * (1 << 29) as f64) as u32);
        const HALF: u32 = 1 << 18;
        const MASK: u32 = (HALF << 1) - 1;
        let mut a = alt.mul_add(1.0 / 16.0, HALF as _) as u32;
        let mut h;
        loop {
            (h, n2w, n2e) = HorizontalSubdivision::subdivide(n2w, n2e);
            let v = if a > HALF {
                VerticalSubdivision::Upper
            } else {
                VerticalSubdivision::Lower
            };
            a = (a << 1) & MASK;
            if let Ok(i) = index.child(h, v) {
                index = i;
            } else {
                return (index, None);
            }
            if let ControlFlow::Break(ret) = visit(index) {
                return (index, Some(ret));
            }
        }
    }
}

/// A quadtree index for two-dimensional space.
///
/// This stores both the depth and the subdivisions in a single `u64`. It has a maximum depth of 29 subdivisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QuadtreeIndex(pub NonZero<u64>);
impl QuadtreeIndex {
    /// Convert a [`WorldRegion`] to an `QuadtreeIndex`.
    ///
    /// The zero-depth representation uses six bits.
    pub const fn from_region(region: WorldRegion, triangle: RegionHalf) -> Self {
        unsafe {
            Self(NonZero::new_unchecked(
                ((region as u64) << 1) | triangle as u64 | 0x20,
            ))
        }
    }
    /// Convert a `u64` to an `QuadtreeIndex`.
    ///
    /// ## Safety
    /// `raw != 0`
    pub const unsafe fn from_u64_unchecked(raw: u64) -> Self {
        unsafe { Self(NonZero::new_unchecked(raw)) }
    }
    /// Get number of subdivisions in this index.
    pub const fn depth(&self) -> u32 {
        let bits = 58 - self.0.leading_zeros();
        debug_assert!(bits.is_multiple_of(2), "invalid quadtree index");
        bits / 2
    }
    /// Get the child indices of this node.
    pub const fn children(self) -> Result<Range<Self>, MaxQuadtreeDepth> {
        let Some(first) = self.0.get().checked_shl(2) else {
            return Err(MaxQuadtreeDepth);
        };
        let last = first + 4;
        unsafe { Ok(Self::from_u64_unchecked(first)..Self::from_u64_unchecked(last)) }
    }
    /// Get the child with the given horizontal and vertical subdivisions.
    pub const fn child(self, horiz: HorizontalSubdivision) -> Result<Self, MaxQuadtreeDepth> {
        let Some(first) = self.0.get().checked_shl(2) else {
            return Err(MaxQuadtreeDepth);
        };
        unsafe { Ok(Self::from_u64_unchecked(first | horiz as u64)) }
    }
    /// Get the parent index and subdivision if this isn't a root index.
    pub const fn parent(self) -> Option<(Self, HorizontalSubdivision)> {
        if self.0.get() < 0x80 {
            return None;
        }
        let raw = self.0.get();
        unsafe {
            Some((
                Self::from_u64_unchecked(raw >> 2),
                HorizontalSubdivision::from_u8_unchecked((raw & 0b11) as u8),
            ))
        }
    }
    /// If this is a root index, get the containing region.
    pub const fn is_root(self) -> Option<(WorldRegion, RegionHalf)> {
        let raw = self.0.get();
        if raw < 0x3f {
            unsafe {
                Some((
                    WorldRegion::from_u8_unchecked(raw as u8 >> 1),
                    RegionHalf::from_u8_unchecked(raw as u8 & 1),
                ))
            }
        } else {
            None
        }
    }
    /// Get the containing region of this node.
    pub const fn region(self) -> (WorldRegion, RegionHalf) {
        unsafe {
            let shift = 58 - self.0.leading_zeros();
            let raw = self.0.get() >> shift;
            (
                WorldRegion::from_u8_unchecked(raw as u8 >> 1),
                RegionHalf::from_u8_unchecked(raw as u8 & 1),
            )
        }
    }
    /// Find the index of a point given by longitude and latitude.
    ///
    /// The visitor function defines control flow to allow an early break if only a lower resolution is needed.
    pub fn hash<R, F: FnMut(QuadtreeIndex) -> ControlFlow<R>>(
        lon: f64,
        lat: f64,
        mut visit: F,
    ) -> (Self, Option<R>) {
        let (base, off) = crate::ico::region_offset(lon, lat);
        let half = if base.is_south() ^ (off.y < 0.0) {
            RegionHalf::Antiprism
        } else {
            RegionHalf::Pyramid
        };
        let mut index = Self::from_region(base, half);
        if let ControlFlow::Break(ret) = visit(index) {
            return (index, Some(ret));
        }
        let hemi = base.hemisphere();
        let [mut n2w, mut n2e] = (barycentric::base_transform(hemi, half)
            * (off + DVec2::new(0.0, barycentric::base_y_shift(hemi, half))))
        .to_array()
        .map(|c| (c * (1 << 29) as f64) as u32);
        let mut h;
        loop {
            (h, n2w, n2e) = HorizontalSubdivision::subdivide(n2w, n2e);
            if let Ok(i) = index.child(h) {
                index = i;
            } else {
                return (index, None);
            }
            if let ControlFlow::Break(ret) = visit(index) {
                return (index, Some(ret));
            }
        }
    }
}

#[cfg(feature = "redb")]
impl redb::Value for OctreeIndex {
    type AsBytes<'a>
        = [u8; 8]
    where
        Self: 'a;
    type SelfType<'a>
        = Self
    where
        Self: 'a;
    fn fixed_width() -> Option<usize> {
        Some(4)
    }
    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'b,
    {
        value.0.get().to_le_bytes()
    }
    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        let mut buf = [0u8; 8];
        buf.copy_from_slice(&data[..8]);
        Self(NonZero::new(u64::from_le_bytes(buf)).expect("Invalid key data"))
    }
    fn type_name() -> redb::TypeName {
        redb::TypeName::new("factor::OctreeIndex")
    }
}
#[cfg(feature = "redb")]
impl redb::Key for OctreeIndex {
    fn compare(data1: &[u8], data2: &[u8]) -> std::cmp::Ordering {
        let mut buf = [0u8; 8];
        buf.copy_from_slice(&data1[..8]);
        let a = u64::from_le_bytes(buf);
        buf.copy_from_slice(&data2[..8]);
        let b = u64::from_le_bytes(buf);
        a.cmp(&b)
    }
}

#[cfg(feature = "redb")]
impl redb::Value for QuadtreeIndex {
    type AsBytes<'a>
        = [u8; 8]
    where
        Self: 'a;
    type SelfType<'a>
        = Self
    where
        Self: 'a;
    fn fixed_width() -> Option<usize> {
        Some(4)
    }
    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'b,
    {
        value.0.get().to_le_bytes()
    }
    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        let mut buf = [0u8; 8];
        buf.copy_from_slice(&data[..8]);
        Self(NonZero::new(u64::from_le_bytes(buf)).expect("Invalid key data"))
    }
    fn type_name() -> redb::TypeName {
        redb::TypeName::new("factor::QuadtreeIndex")
    }
}
#[cfg(feature = "redb")]
impl redb::Key for QuadtreeIndex {
    fn compare(data1: &[u8], data2: &[u8]) -> std::cmp::Ordering {
        let mut buf = [0u8; 8];
        buf.copy_from_slice(&data1[..8]);
        let a = u64::from_le_bytes(buf);
        buf.copy_from_slice(&data2[..8]);
        let b = u64::from_le_bytes(buf);
        a.cmp(&b)
    }
}
