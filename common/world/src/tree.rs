use std::fmt;
use std::num::NonZero;
use std::ops::Range;

/// A region of the world.
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
    pub const unsafe fn from_u8_unchecked(idx: u8) -> Self {
        unsafe { std::mem::transmute::<u8, Self>(idx) }
    }
    /// Convert `self` to a `usize`.
    pub const fn to_index(self) -> usize {
        self as usize
    }
    /// Flip between upper and lower halves.
    pub const fn flipped(self) -> Self {
        unsafe { Self::from_u8_unchecked((self as u8 + 5) % 10) }
    }
}
impl fmt::Display for WorldRegion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
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
    /// This converts the four bits of the world region, along with a sentinel bit in the fifth bit.
    pub const fn from_region(region: WorldRegion) -> Self {
        unsafe { Self(NonZero::new_unchecked(region as u8 as u64 | 0x10)) }
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
        let bits = 57 - self.0.leading_zeros();
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
    pub const fn is_root(self) -> Option<WorldRegion> {
        if self.0.get() < 0x1f {
            unsafe { Some(WorldRegion::from_u8_unchecked(self.0.get() as u8 & 0x0f)) }
        } else {
            None
        }
    }
    /// Get the containing region of this node.
    pub const fn region(self) -> WorldRegion {
        unsafe {
            let shift = 59 - self.0.leading_zeros();
            WorldRegion::from_u8_unchecked((self.0.get() >> shift) as u8 & 0x0f)
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
    /// This converts the four bits of the world region, along with a sentinel bit in the fifth bit.
    pub const fn from_region(region: WorldRegion) -> Self {
        unsafe { Self(NonZero::new_unchecked(region as u8 as u64 | 0x10)) }
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
    pub const fn is_root(self) -> Option<WorldRegion> {
        if self.0.get() < 0x1f {
            unsafe { Some(WorldRegion::from_u8_unchecked(self.0.get() as u8 & 0x0f)) }
        } else {
            None
        }
    }
    /// Get the containing region of this node.
    pub const fn region(self) -> WorldRegion {
        unsafe {
            let shift = 59 - self.0.leading_zeros();
            WorldRegion::from_u8_unchecked((self.0.get() >> shift) as u8 & 0x0f)
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
