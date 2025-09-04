//! Icosahedral utilities
//!
//! While the world is projected onto the surface of an icosahedron, edges purely along the latitude are removed,
//! leaving only *ten* regions. This reduces the number of regions to only ten, rather than twenty, with a positive
//! y-coordinate putting a point in the upper half and a negative coordinate putting it in the lower half.

use crate::tree::{RegionHalf, WorldRegion};
use bevy_math::DVec2;
use std::f64::consts::*;

const FRAC_2PI_5: f64 = TAU * 0.2;
const FRAC_PI_5: f64 = TAU * 0.1;
pub(crate) const TRANS_LAT: f64 = 0.5535743588970453; // atan(1/PHI)
const CENTER_SLOPE: f64 = TRANS_LAT * 2.0 / FRAC_PI_5;

/// Get the region and offset of a point.
///
/// Regions 0-4 are the northern ones, and 5-9 are the southern ones. Positive Y points north, and positive X is east.
pub fn region_offset(lon: f64, lat: f64) -> (WorldRegion, DVec2) {
    unsafe {
        let (region, offset) = region_offset_raw(lon, lat);
        (WorldRegion::from_u8_unchecked(region), offset)
    }
}
/// Get the region that contains a point as a `u8`, along with its offset.
///
/// See [`region_offset`] for more information.
pub fn region_offset_raw(lon: f64, lat: f64) -> (u8, DVec2) {
    let c = lat.cos();
    if lat > TRANS_LAT {
        let center = (lon.rem_euclid(TAU) / FRAC_2PI_5) as u8;
        let y = lat - TRANS_LAT;
        let x = (lon.rem_euclid(FRAC_2PI_5) - FRAC_PI_5) * c;
        (center, DVec2::new(x, y))
    } else if lat < -TRANS_LAT {
        let center = ((lon + FRAC_PI_5).rem_euclid(TAU) / FRAC_2PI_5) as u8 + 5;
        let y = lat + TRANS_LAT;
        let x = ((lon + FRAC_PI_5).rem_euclid(FRAC_2PI_5) - FRAC_PI_5) * c;
        (center, DVec2::new(x, y))
    } else {
        let mut center = (lon.rem_euclid(TAU) / FRAC_2PI_5) as u8;
        let mut y = lat - TRANS_LAT;
        let mut x = (lon + FRAC_2PI_5).rem_euclid(FRAC_2PI_5) - FRAC_PI_5;
        #[allow(clippy::collapsible_else_if)]
        if x > 0.0 {
            if y < x.mul_add(CENTER_SLOPE, -TRANS_LAT * 2.0) {
                if center == 4 {
                    center = 5;
                } else {
                    center += 6;
                }
                x -= FRAC_PI_5;
                y += TRANS_LAT * 2.0;
            }
        } else {
            if y < x.mul_add(-CENTER_SLOPE, -TRANS_LAT * 2.0) {
                center += 5;
                x += FRAC_PI_5;
                y += TRANS_LAT * 2.0;
            }
        }
        (center, DVec2::new(x * c, y))
    }
}

/// Get the region that contains a point.
///
/// See [`region_offset`] for more information on the regions.
#[inline(always)]
pub fn region(lon: f64, lat: f64) -> (WorldRegion, RegionHalf) {
    unsafe {
        let raw = region_raw(lon, lat);
        (
            WorldRegion::from_u8_unchecked(raw >> 1),
            RegionHalf::from_u8_unchecked(raw & 1),
        )
    }
}
/// Get the region that contains a point, as a `u8`.
///
/// See [`region`] for more information. Note that the result of this stores the region half as the lower bit and the region in the upper four bits.
pub fn region_raw(lon: f64, lat: f64) -> u8 {
    if lat > TRANS_LAT {
        ((lon.rem_euclid(TAU) / FRAC_2PI_5) as u8) << 1
    } else if lat < -TRANS_LAT {
        (((lon + FRAC_PI_5).rem_euclid(TAU) / FRAC_2PI_5) as u8 + 5) << 1
    } else {
        let mut center = (lon.rem_euclid(TAU) / FRAC_2PI_5) as u8;
        let x = (lon + FRAC_2PI_5).rem_euclid(FRAC_2PI_5) - FRAC_PI_5;
        #[allow(clippy::collapsible_else_if)]
        if x > 0.0 {
            if lat < x.mul_add(CENTER_SLOPE, -TRANS_LAT) {
                if center == 4 {
                    center = 5;
                } else {
                    center += 6;
                }
            }
        } else {
            if lat < x.mul_add(-CENTER_SLOPE, -TRANS_LAT) {
                center += 5;
            }
        }
        (center << 1) | 1
    }
}

/// Inverse operation for [`region_offset`].
#[inline(always)]
pub fn unproject(region: WorldRegion, offset: DVec2) -> (f64, f64) {
    unproject_raw(region as u8, offset)
}
/// Inverse operation for [`region_offset_raw`].
///
/// The result is unspecified if `region >= 10`.
pub fn unproject_raw(region: u8, offset: DVec2) -> (f64, f64) {
    let (clon, clat) = if region < 5 {
        (region as f64 * FRAC_2PI_5 + FRAC_PI_5, TRANS_LAT)
    } else {
        ((region - 5) as f64 * FRAC_2PI_5, -TRANS_LAT)
    };
    let lat = offset.y + clat;
    let c = lat.cos();
    let lon = clon + if c == 0.0 { 0.0 } else { offset.x / c };
    (lon.rem_euclid(TAU), lat)
}

#[cfg(test)]
mod tests {
    // monte carlo tests, these *could* be inaccurate but are probably correct
    mod monte_carlo {
        use crate::ico::{region, region_offset, unproject};
        use std::f64::consts::TAU;

        /// Generate a random longitude and latitude
        fn random_lonlat(rng: &mut impl rand::Rng) -> (f64, f64) {
            let [x, y] = rng.r#gen::<[f64; 2]>();
            (x * TAU, y.mul_add(2.0, -1.0).asin())
        }
        #[test]
        fn regions_equal() {
            let rng = &mut rand::thread_rng();
            for _ in 0..10000 {
                let (lon, lat) = random_lonlat(rng);
                assert_eq!(
                    region_offset(lon, lat).0,
                    region(lon, lat).0,
                    "regions for ({lon}, {lat}) don't match"
                );
            }
        }
        #[test]
        fn equal_area() {
            let rng = &mut rand::thread_rng();
            let mut bins = [0usize; 10];
            for _ in 0..1000000 {
                let (lon, lat) = random_lonlat(rng);
                bins[region(lon, lat).0 as usize] += 1;
            }
            for (i, count) in bins.iter().enumerate() {
                assert!(
                    (99000..101000).contains(count),
                    "More than 1% difference in region {i} (expected 10000, got {count})"
                );
            }
        }

        #[test]
        fn roundtrip() {
            const EPSILON: f64 = 0.0000000000001;
            let rng = &mut rand::thread_rng();
            for _ in 0..10000 {
                let (lon, lat) = random_lonlat(rng);
                let (region, offset) = region_offset(lon, lat);
                let (lo2, la2) = unproject(region, offset);
                assert!(
                    (lon - lo2).abs() < EPSILON,
                    "roundtrip failure: ({lon}, {lat}) => ({region}, {offset}) => ({lo2}, {la2})"
                );
                assert!(
                    (lat - la2).abs() < EPSILON,
                    "roundtrip failure: ({lon}, {lat}) => ({region}, {offset}) => ({lo2}, {la2})"
                );
            }
        }
    }
}
