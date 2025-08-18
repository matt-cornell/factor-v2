//! This is a re-export of the [`healpix`] crate for now, but it might gain additional features as needed.
//!
//! By doing this, I can prototype additional HEALPix features as needed without having to make changes to the upstream crate.

pub use healpix::*;

use bevy_math::DVec2;
use healpix::coords::LonLatT;
pub use healpix::geo::distance;

/// Wrapper around [`healpix::geo::absolute`] that uses [`DVec2`] instead of `[f32; 2]` for relative offsets.
#[inline(always)]
pub fn absolute(start: impl LonLatT, offset: DVec2) -> LonLat {
    healpix::geo::absolute(start, offset.into())
}

/// Wrapper around [`healpix::geo::absolute`] that uses [`DVec2`] instead of `[f32; 2]` for relative offsets.
#[inline(always)]
pub fn relative(start: impl LonLatT, end: impl LonLatT) -> DVec2 {
    healpix::geo::relative(start, end).into()
}
