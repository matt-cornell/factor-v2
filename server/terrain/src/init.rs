use bevy_math::Vec3A;
use libnoise::prelude::*;

pub const NUM_DIMS: usize = 3;
pub type Seed = [u8; 32];

#[allow(clippy::excessive_precision)]
const INITIAL_POINTS: [Vec3A; 12] = [
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

pub fn noise_source(seed: Seed) -> impl Generator<3> {
    Source::simplex(seed).fbm(6, 1.0, 2.0, 0.5).blend(
        Source::constant(-1.0),
        Source::custom(|p| {
            let v = Vec3A::from(p.map(|c| c as f32 - 1.0));
            INITIAL_POINTS
                .iter()
                .map(|c| c.distance_squared(v))
                .min_by(f32::total_cmp)
                .unwrap()
                .powf(0.6)
                .mul_add(-1.2, 0.3)
                .max(-1.0) as f64
        }),
    )
}

pub fn to_coords(lon: f64, lat: f64) -> [f64; 3] {
    let (so, co) = lon.sin_cos();
    let (sa, ca) = lat.sin_cos();
    [co * ca + 1.0, so * ca + 1.0, sa + 1.0]
}
