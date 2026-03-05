use serde::{Deserialize, Serialize};

use super::{EntityId, Tick};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum NetworkTier {
    Access = 1,
    Aggregation = 2,
    Core = 3,
    Backbone = 4,
    Global = 5,
}

impl NetworkTier {
    /// Human-readable display name.
    pub fn display_name(&self) -> &'static str {
        match self {
            NetworkTier::Access => "Access",
            NetworkTier::Aggregation => "Aggregation",
            NetworkTier::Core => "Core",
            NetworkTier::Backbone => "Backbone",
            NetworkTier::Global => "Global",
        }
    }

    pub fn can_connect_to(&self, other: &NetworkTier) -> bool {
        let diff = (*self as i32 - *other as i32).abs();
        diff <= 1
    }

    pub fn value(&self) -> u8 {
        *self as u8
    }
}

/// Traffic demand between an origin-destination pair.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficDemand {
    pub source_city: EntityId,
    pub dest_city: EntityId,
    pub demand: f64,
}

/// Permission level for traffic crossing corporate boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TransitPermission {
    /// Same corporation — free routing, no penalty.
    OwnNetwork,
    /// Settlement-free peering contract — traffic allowed, no payment.
    PeeringContract,
    /// Paid transit contract — traffic allowed, originator pays per unit.
    TransitContract { price_per_unit: f64 },
    /// Allied corporations — reduced cost routing.
    Alliance { revenue_share_pct: f64 },
    /// Co-owned infrastructure — free routing.
    CoOwned,
    /// No agreement — traffic blocked.
    Blocked,
}

/// Attribution of a specific traffic flow to corporations based on network contribution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathAttribution {
    pub source_city: EntityId,
    pub dest_city: EntityId,
    pub traffic: f64,
    /// corp_id -> number of nodes contributed to this path
    pub corp_hops: std::collections::HashMap<EntityId, u32>,
}

/// Aggregated traffic flow data stored on GameWorld.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TrafficMatrix {
    /// OD pairs and their demand values
    pub od_pairs: Vec<TrafficDemand>,
    /// External (internet-bound) traffic per city
    pub external_traffic: Vec<(EntityId, f64)>,
    /// Total demand across all cities
    pub total_demand: f64,
    /// Tick when this matrix was last recomputed
    pub last_computed_tick: Tick,
    /// Per-node accumulated traffic load from OD routing
    pub node_traffic: std::collections::HashMap<EntityId, f64>,
    /// Per-edge accumulated traffic load from OD routing
    pub edge_traffic: std::collections::HashMap<EntityId, f64>,
    /// Total traffic successfully served
    pub total_served: f64,
    /// Total traffic dropped due to congestion or no path
    pub total_dropped: f64,
    /// Per-corporation traffic served
    pub corp_traffic_served: std::collections::HashMap<EntityId, f64>,
    /// Per-corporation traffic dropped
    pub corp_traffic_dropped: std::collections::HashMap<EntityId, f64>,
    /// Per-contract traffic flow (contract_id → traffic units routed through it)
    #[serde(default)]
    pub contract_traffic: std::collections::HashMap<EntityId, f64>,
    /// Detailed path attribution for alliance revenue sharing
    #[serde(default)]
    pub path_attribution: Vec<PathAttribution>,
}
