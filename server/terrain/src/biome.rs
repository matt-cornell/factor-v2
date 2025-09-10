#![expect(dead_code, unused_variables, reason = "WIP")]

use bevy_color::Color;
use factor_db::traits::*;
use factor_world::tree::QuadtreeIndex;
use std::fmt;
use std::sync::Arc;

pub trait Biome: Send + Sync {
    fn kind(&self) -> BiomeKind;
    fn color(&self) -> Color;

    fn debug(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&disqualified::ShortName::of::<Self>(), f)
    }
}
impl fmt::Debug for dyn Biome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.debug(f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BiomeKind {
    DeepOcean,
    ShallowOcean,
    Coastal,
    Inland,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BiomeInfo {
    biome: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BiomeNode {
    Subdivided,
    Current(BiomeInfo),
}

#[derive(Debug, Default, Clone)]
pub struct BiomeRegistry {
    biomes: Vec<Arc<dyn Biome>>,
}

pub fn init_biomes<M: WritableMap<Key = QuadtreeIndex, Value = BiomeNode>>(
    map: &mut M,
    registry: &BiomeRegistry,
) {
}
