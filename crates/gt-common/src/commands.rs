use crate::types::{EntityId, GameSpeed, Money};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    // Infrastructure
    BuildNode {
        node_type: crate::types::NodeType,
        lon: f64,
        lat: f64,
    },
    BuildEdge {
        edge_type: crate::types::EdgeType,
        from: EntityId,
        to: EntityId,
        #[serde(default)]
        waypoints: Vec<(f64, f64)>,
        #[serde(default)]
        deployment: Option<String>,
    },
    UpdateEdgeWaypoints {
        edge: EntityId,
        waypoints: Vec<(f64, f64)>,
        #[serde(default)]
        deployment: Option<String>,
    },
    UpgradeNode {
        entity: EntityId,
    },
    DecommissionNode {
        entity: EntityId,
    },
    RepairNode {
        entity: EntityId,
        #[serde(default)]
        emergency: bool,
    },
    RepairEdge {
        edge: EntityId,
        #[serde(default)]
        emergency: bool,
    },
    EmergencyRepair {
        entity: EntityId,
    },

    // Workforce
    HireEmployee {
        corporation: EntityId,
        role: String,
    },
    FireEmployee {
        entity: EntityId,
    },
    AssignTeam {
        team: EntityId,
        target: EntityId,
    },

    // Finance
    TakeLoan {
        corporation: EntityId,
        amount: Money,
    },
    RepayLoan {
        loan: EntityId,
        amount: Money,
    },
    SetBudget {
        corporation: EntityId,
        category: String,
        amount: Money,
    },

    // Contracts
    ProposeContract {
        from: EntityId,
        to: EntityId,
        terms: String,
    },
    AcceptContract {
        contract: EntityId,
    },
    RejectContract {
        contract: EntityId,
    },

    // Research
    StartResearch {
        corporation: EntityId,
        tech: String,
    },
    CancelResearch {
        corporation: EntityId,
    },

    // Policy
    SetPolicy {
        corporation: EntityId,
        policy: String,
        value: String,
    },

    // Subsidiary
    CreateSubsidiary {
        parent: EntityId,
        name: String,
    },

    // Insurance
    PurchaseInsurance {
        node: EntityId,
    },
    CancelInsurance {
        node: EntityId,
    },

    // Bankruptcy & Auctions
    DeclareBankruptcy {
        entity: EntityId,
    },
    RequestBailout {
        entity: EntityId,
    },
    AcceptBailout {
        entity: EntityId,
    },
    PlaceBid {
        auction: EntityId,
        amount: Money,
    },

    // Mergers & Acquisitions
    ProposeAcquisition {
        target: EntityId,
        offer: Money,
    },
    RespondToAcquisition {
        proposal: EntityId,
        accept: bool,
    },

    // Espionage & Sabotage
    LaunchEspionage {
        target: EntityId,
        region: EntityId,
    },
    LaunchSabotage {
        target: EntityId,
        node: EntityId,
    },
    UpgradeSecurity {
        level: u32,
    },

    // Lobbying
    StartLobbying {
        region: EntityId,
        policy: String,
        budget: Money,
    },
    CancelLobbying {
        lobby_id: EntityId,
    },

    // Alliance (Phase 5.1)
    ProposeAlliance {
        target_corp: EntityId,
        name: String,
        revenue_share: f64,
    },
    AcceptAlliance {
        alliance_id: EntityId,
    },
    DissolveAlliance {
        alliance_id: EntityId,
    },

    // Legal (Phase 5.2)
    FileLawsuit {
        defendant: EntityId,
        lawsuit_type: String,
        damages: Money,
    },
    SettleLawsuit {
        lawsuit_id: EntityId,
    },
    DefendLawsuit {
        lawsuit_id: EntityId,
    },

    // Patents & Licensing (Phase 5.3)
    FilePatent {
        tech_id: EntityId,
    },
    RequestLicense {
        patent_id: EntityId,
    },
    SetLicensePrice {
        patent_id: EntityId,
        price: Money,
        license_type: String, // "Permanent", "Royalty", "PerUnit", "Lease"
        /// Per-unit price for PerUnit licenses (charged per node built using the tech)
        #[serde(default)]
        per_unit_price: Money,
        /// Duration in ticks for Lease licenses (0 = use default 500 ticks)
        #[serde(default)]
        lease_duration: u64,
    },
    RevokeLicense {
        license_id: EntityId,
    },
    StartIndependentResearch {
        tech_id: EntityId,
        /// When true, research costs 200% (vs 150%) but grants +10% stat bonuses and patent rights
        #[serde(default)]
        premium: bool,
    },

    // Government Grants (Phase 5.4)
    BidForGrant {
        grant_id: EntityId,
    },
    CompleteGrant {
        grant_id: EntityId,
    },

    // Cooperative Infrastructure
    ProposeCoOwnership {
        node: EntityId,
        target_corp: EntityId,
        share_pct: f64,
    },
    RespondCoOwnership {
        proposal: EntityId,
        accept: bool,
    },
    ProposeBuyout {
        node: EntityId,
        target_corp: EntityId,
        price: Money,
    },
    VoteUpgrade {
        node: EntityId,
        approve: bool,
    },

    // Spectrum & Frequency Management (Phase 8)
    BidSpectrum {
        band: String,
        region: EntityId,
        bid: Money,
    },

    // Spectrum Assignment — assign a wireless node to a specific spectrum band
    AssignSpectrum {
        node: EntityId,
        band: String, // e.g. "Band700MHz", "Band3500MHz"
    },

    // Spectrum Unassignment — remove a spectrum band from a wireless node (carrier aggregation)
    UnassignSpectrum {
        node: EntityId,
        band: String,
    },

    // Cable Ship — purchase a cable ship for submarine cable construction
    PurchaseCableShip,

    // Regional Pricing (Phase 5.5)
    SetRegionPricing {
        region: EntityId,
        tier: String, // "Budget", "Standard", "Premium"
        price_per_unit: Money,
    },

    // Maintenance Priority (Phase 5.6)
    SetMaintenancePriority {
        entity: EntityId,
        priority: String, // "Critical", "Standard", "Low", "Deferred"
        auto_repair: bool,
    },

    // Satellite System
    /// Define a new constellation (creates the constellation entity, sats must be manufactured + launched)
    BuildConstellation {
        name: String,
        orbit_type: String,     // "LEO", "MEO", "GEO", "HEO"
        num_planes: u32,
        sats_per_plane: u32,
        altitude_km: f64,
        inclination_deg: f64,
    },
    /// Order satellites to be produced by a factory for a constellation
    OrderSatellites {
        factory: EntityId,
        constellation_id: EntityId,
        count: u32,
    },
    /// Schedule a launch from an owned launch pad
    ScheduleLaunch {
        launch_pad: EntityId,
        rocket_type: String,    // "Small", "Medium", "Heavy", "SuperHeavy"
        satellites: Vec<EntityId>,
    },
    /// Purchase a contract launch (no owned pad needed)
    ContractLaunch {
        rocket_type: String,
        satellites: Vec<EntityId>,
    },
    /// Command a satellite to deorbit
    DeorbitSatellite {
        satellite: EntityId,
    },
    /// Order customer terminals from a terminal factory
    OrderTerminals {
        factory: EntityId,
        count: u32,
    },
    /// Ship terminals from factory to a regional warehouse
    ShipTerminals {
        factory: EntityId,
        warehouse: EntityId,
        count: u32,
    },
    /// Set satellite service pricing for a region
    SetSatellitePricing {
        region: EntityId,
        monthly_rate: Money,
    },
    /// Service a satellite (refuel or repair)
    ServiceSatellite {
        satellite: EntityId,
        service_type: String,   // "Refuel", "Repair"
    },

    // Game control
    SetSpeed(GameSpeed),
    TogglePause,
    SaveGame {
        slot: u32,
    },
    LoadGame {
        slot: u32,
    },
}
