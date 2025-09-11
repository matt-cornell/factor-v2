/// Parameters for the planet's orbit
#[derive(Debug, Clone, Copy)]
pub struct OrbitParams {
    /// Length of a year, in seconds
    pub year_length: f64,
    /// Length of a day, in seconds
    pub day_length: f64,
    /// Obliquity of the orbit, in radians
    pub obliquity: f64,
    /// Energy received by the surface (perpendicular to the sun), in W/m²
    pub irradiance: f64,
}
impl OrbitParams {
    pub const EARTH: Self = Self {
        year_length: 31556736.0,
        day_length: 86400.0,
        obliquity: 0.4101523742186675,
        irradiance: 1548.0,
    };
}
