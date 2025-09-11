use bevy_math::Vec3A;
use libnoise::prelude::*;

pub const NUM_DIMS: usize = 3;
pub type Seed = [u8; 32];

#[allow(clippy::excessive_precision)]
const ICO_POINTS: [Vec3A; 12] = [
    // North Pole
    Vec3A::Y,
    // Top Ring
    Vec3A::new(
        0.89442719099991585541,
        0.44721359549995792770,
        0.00000000000000000000,
    ),
    Vec3A::new(
        0.27639320225002106390,
        0.44721359549995792770,
        0.85065080835203987775,
    ),
    Vec3A::new(
        -0.72360679774997882507,
        0.44721359549995792770,
        0.52573111211913370333,
    ),
    Vec3A::new(
        -0.72360679774997904712,
        0.44721359549995792770,
        -0.52573111211913348129,
    ),
    Vec3A::new(
        0.27639320225002084186,
        0.44721359549995792770,
        -0.85065080835203998877,
    ),
    // Bottom Ring
    Vec3A::new(
        0.72360679774997871405,
        -0.44721359549995792770,
        -0.52573111211913392538,
    ),
    Vec3A::new(
        0.72360679774997904712,
        -0.44721359549995792770,
        0.52573111211913337026,
    ),
    Vec3A::new(
        -0.27639320225002073084,
        -0.44721359549995792770,
        0.85065080835203998877,
    ),
    Vec3A::new(
        -0.89442719099991585541,
        -0.44721359549995792770,
        0.00000000000000000000,
    ),
    Vec3A::new(
        -0.27639320225002139697,
        -0.44721359549995792770,
        -0.85065080835203976672,
    ),
    // South Pole
    Vec3A::NEG_Y,
];

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum Exclusion {
    #[default]
    None,
    Icosahedron,
    Healpix,
}

pub const HEALPIX_TRANSITION_X: f64 = 0.7453559924999299; // sqrt(5) / 3

/// Create a noise source that can be sampled in three dimensions.
///
/// The noise is expected to have its coordinates sampled on the surface of the unit sphere centered at (1, 1, 1).
/// Right now, the algorithm uses simplex noise with fractal Brownian motion, but this could change.
///
/// Points can also be "excluded," which forces them to be down low in the world, in a way that shouldn't be too
/// obvious. This is useful for ensuring that it's hard to naturally stumble upon points where geometry breaks.
pub fn noise_source(seed: Seed, exclusion: Exclusion) -> impl Generator<3> {
    // let base = Source::constant(1.0);
    let base = Source::simplex(seed).fbm(6, 1.0, 2.0, 0.5);
    base.blend(
        Source::constant(-1.0),
        Source::custom(move |p| match exclusion {
            Exclusion::None => -1.0,
            Exclusion::Icosahedron => {
                let v = Vec3A::from(p.map(|c| c as f32 - 1.0));
                ICO_POINTS
                    .iter()
                    .map(|c| c.distance_squared(v))
                    .min_by(f32::total_cmp)
                    .unwrap()
                    .powf(0.3)
                    .mul_add(-1.2, 0.5)
                    .max(-1.0) as f64
            }
            Exclusion::Healpix => {
                let [mut x, mut y, mut z] = p.map(|c| (c - 1.0).abs());
                if z > x {
                    std::mem::swap(&mut x, &mut z);
                }
                x -= HEALPIX_TRANSITION_X;
                y -= factor_healpix::TRANSITION_Z;
                (x * x + y * y + z * z)
                    .powf(0.3)
                    .mul_add(-1.2, 0.5)
                    .max(-1.0)
            }
        }),
    )
}

pub fn to_coords(lon: f64, lat: f64) -> [f64; 3] {
    let (so, co) = lon.sin_cos();
    let (sa, ca) = lat.sin_cos();
    [co * ca + 1.0, sa + 1.0, so * ca + 1.0]
}
