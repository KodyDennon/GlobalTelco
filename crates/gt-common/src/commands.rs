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

    // Cable Ship — purchase a cable ship for submarine cable construction
    PurchaseCableShip,

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
