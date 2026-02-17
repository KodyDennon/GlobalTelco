use crate::types::{EntityId, Money, Tick};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameEvent {
    // Construction
    ConstructionStarted {
        entity: EntityId,
        tick: Tick,
    },
    ConstructionCompleted {
        entity: EntityId,
        tick: Tick,
    },

    // Infrastructure
    NodeBuilt {
        entity: EntityId,
        owner: EntityId,
    },
    EdgeBuilt {
        entity: EntityId,
        from: EntityId,
        to: EntityId,
    },
    NodeDestroyed {
        entity: EntityId,
    },

    // Finance
    RevenueEarned {
        corporation: EntityId,
        amount: Money,
    },
    CostIncurred {
        corporation: EntityId,
        amount: Money,
    },
    LoanTaken {
        corporation: EntityId,
        amount: Money,
    },
    Bankruptcy {
        corporation: EntityId,
    },

    // Corporation
    CorporationFounded {
        entity: EntityId,
        name: String,
    },
    CorporationMerged {
        absorbed: EntityId,
        absorber: EntityId,
    },

    // Contract
    ContractProposed {
        entity: EntityId,
        from: EntityId,
        to: EntityId,
    },
    ContractAccepted {
        entity: EntityId,
    },
    ContractExpired {
        entity: EntityId,
    },

    // Research
    ResearchStarted {
        corporation: EntityId,
        tech: String,
    },
    ResearchCompleted {
        corporation: EntityId,
        tech: String,
    },

    // Disaster
    DisasterStruck {
        region: EntityId,
        severity: f64,
        disaster_type: String,
        affected_nodes: u32,
    },
    InfrastructureDamaged {
        entity: EntityId,
        damage: f64,
    },
    RepairStarted {
        entity: EntityId,
        cost: Money,
    },
    RepairCompleted {
        entity: EntityId,
    },

    // Subsidiary
    SubsidiaryCreated {
        parent: EntityId,
        subsidiary: EntityId,
        name: String,
    },

    // Insurance
    InsurancePurchased {
        entity: EntityId,
        premium: Money,
    },
    InsurancePayout {
        entity: EntityId,
        amount: Money,
    },

    // Regulation
    RegulationChanged {
        region: EntityId,
        description: String,
    },

    // Market
    MarketShiftOccurred {
        description: String,
    },
    MarketUpdate {
        economic_health: f64,
        active_corporations: u32,
    },

    // Bankruptcy & Auctions
    InsolvencyWarning {
        corporation: EntityId,
    },
    BailoutTaken {
        corporation: EntityId,
        amount: Money,
        interest_rate: f64,
    },
    BankruptcyDeclared {
        corporation: EntityId,
    },
    AuctionStarted {
        auction: EntityId,
        seller: EntityId,
        asset_count: u32,
    },
    AuctionBidPlaced {
        auction: EntityId,
        bidder: EntityId,
        amount: Money,
    },
    AuctionWon {
        auction: EntityId,
        asset: EntityId,
        winner: EntityId,
        price: Money,
    },
    AuctionCancelled {
        auction: EntityId,
    },

    // Mergers & Acquisitions
    AcquisitionProposed {
        acquirer: EntityId,
        target: EntityId,
        offer: Money,
    },
    AcquisitionAccepted {
        acquirer: EntityId,
        target: EntityId,
    },
    AcquisitionRejected {
        acquirer: EntityId,
        target: EntityId,
    },
    MergerCompleted {
        absorbed: EntityId,
        absorber: EntityId,
    },

    // Espionage & Sabotage
    EspionageCompleted {
        spy: EntityId,
        target: EntityId,
    },
    SabotageCompleted {
        saboteur: EntityId,
        target: EntityId,
        damage: f64,
    },
    EspionageDetected {
        spy: EntityId,
        target: EntityId,
        penalty: Money,
    },
    SabotageDetected {
        saboteur: EntityId,
        target: EntityId,
        penalty: Money,
    },
    SecurityUpgraded {
        corporation: EntityId,
        level: u32,
    },

    // Lobbying & Political Influence
    LobbyingStarted {
        corporation: EntityId,
        region: EntityId,
        policy: String,
    },
    LobbyingSucceeded {
        corporation: EntityId,
        region: EntityId,
        effect: String,
    },
    LobbyingFailed {
        corporation: EntityId,
        region: EntityId,
    },
    ScandalOccurred {
        corporation: EntityId,
        reputation_loss: f64,
    },

    // Cooperative Infrastructure
    CoOwnershipEstablished {
        node: EntityId,
        partner: EntityId,
        share_pct: f64,
    },
    UpgradeVotePassed {
        node: EntityId,
    },
    UpgradeVoteRejected {
        node: EntityId,
    },
    BuyoutCompleted {
        node: EntityId,
        buyer: EntityId,
        seller: EntityId,
        price: Money,
    },

    // Achievements & Victory
    AchievementUnlocked {
        corporation: EntityId,
        achievement: String,
    },
    VictoryAchieved {
        corporation: EntityId,
        victory_type: String,
        score: f64,
    },
}
