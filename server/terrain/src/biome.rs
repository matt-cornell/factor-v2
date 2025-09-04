use bevy_color::Color;
use factor_db::traits::*;
use factor_world::tree::QuadtreeIndex;

pub trait Biome {
    fn kind(&self) -> BiomeKind;
    fn color(&self) -> Color;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BiomeKind {
    DeepOcean,
    ShallowOcean,
    Coastal,
    Inland,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BiomeInfo {}

#[derive(Debug, Clone, PartialEq)]
pub enum BiomeNode {
    Subdivided,
    Current(BiomeInfo),
}

pub fn init_biomes<M: WritableMap<Key = QuadtreeIndex, Value = BiomeNode>>(map: &mut M) {}
