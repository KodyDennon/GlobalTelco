mod terrain;
mod network;
mod node;
mod edge;
mod spectrum;
mod satellite;
mod config;

pub use terrain::*;
pub use network::*;
pub use node::*;
pub use edge::*;
pub use spectrum::*;
pub use satellite::*;
pub use config::*;

use serde::{Deserialize, Serialize};

pub type EntityId = u64;
pub type Tick = u64;
pub type Money = i64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum NetworkLevel {
    Local,
    Regional,
    National,
    Continental,
    GlobalBackbone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CreditRating {
    AAA,
    AA,
    A,
    BBB,
    BB,
    B,
    CCC,
    D,
}
