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

    // Disaster (DEPRECATED — disaster system removed; kept for save file backwards compatibility)
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

    // Weather (DEPRECATED — weather system removed; kept for save file backwards compatibility)
    WeatherStarted {
        region: EntityId,
        weather_type: String,
        severity: f64,
        duration_ticks: u32,
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

    // Alliance (Phase 5.1)
    AllianceFormed {
        alliance_id: EntityId,
        members: Vec<EntityId>,
    },
    AllianceDissolved {
        alliance_id: EntityId,
        reason: String,
    },

    // Legal (Phase 5.2)
    LawsuitFiled {
        lawsuit_id: EntityId,
        plaintiff: EntityId,
        defendant: EntityId,
    },
    LawsuitResolved {
        lawsuit_id: EntityId,
        plaintiff: EntityId,
        defendant: EntityId,
        outcome: String,
    },
    SettlementReached {
        lawsuit_id: EntityId,
        plaintiff: EntityId,
        defendant: EntityId,
        amount: Money,
    },

    // Patents & Licensing (Phase 5.3)
    PatentFiled {
        patent_id: EntityId,
        tech_id: EntityId,
        holder: EntityId,
    },
    LicenseGranted {
        license_id: EntityId,
        patent_id: EntityId,
        licensee: EntityId,
        price: Money,
    },
    LicenseRevoked {
        license_id: EntityId,
        patent_id: EntityId,
        licensee: EntityId,
    },
    IndependentResearchStarted {
        corporation: EntityId,
        tech_id: EntityId,
        cost_multiplier: f64,
    },

    // Government Grants (Phase 5.4)
    GrantAvailable {
        grant_id: EntityId,
        region: EntityId,
        reward: Money,
    },
    GrantAwarded {
        grant_id: EntityId,
        corporation: EntityId,
        region: EntityId,
    },
    GrantCompleted {
        grant_id: EntityId,
        corporation: EntityId,
        reward: Money,
    },
    GrantExpired {
        grant_id: EntityId,
        region: EntityId,
    },

    // Spectrum & Frequency Management (Phase 8)
    SpectrumAuctionStarted {
        band: String,
        region: EntityId,
    },
    SpectrumBidPlaced {
        band: String,
        region: EntityId,
        bidder: EntityId,
        amount: Money,
    },
    SpectrumAuctionWon {
        band: String,
        region: EntityId,
        winner: EntityId,
        price: Money,
    },
    SpectrumLicenseExpired {
        band: String,
        region: EntityId,
        owner: EntityId,
    },

    // Spectrum Assignment
    SpectrumAssigned {
        node: EntityId,
        band: String,
        corp: EntityId,
    },

    // Spectrum Unassignment (carrier aggregation)
    SpectrumUnassigned {
        node: EntityId,
        band: String,
        corp: EntityId,
    },

    // Cable Ship
    CableShipPurchased {
        corp: EntityId,
    },

    // Regional Pricing
    PricingChanged {
        corporation: EntityId,
        region: EntityId,
        tier: String,
    },

    // Transit & Interconnection
    TransitPayment {
        provider: EntityId,
        consumer: EntityId,
        contract: EntityId,
        amount: Money,
    },
    SLAPenaltyPaid {
        provider: EntityId,
        consumer: EntityId,
        contract: EntityId,
        amount: Money,
    },

    // Maintenance Priority
    MaintenancePrioritySet {
        entity: EntityId,
        priority: String,
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

    // Satellite System
    ConstellationCreated {
        constellation_id: EntityId,
        owner: EntityId,
        name: String,
    },
    SatelliteManufactured {
        satellite_id: EntityId,
        owner: EntityId,
        constellation_id: EntityId,
    },
    LaunchAttempted {
        owner: EntityId,
        rocket_type: String,
        satellite_count: u32,
    },
    LaunchSucceeded {
        owner: EntityId,
        satellite_count: u32,
    },
    LaunchFailed {
        owner: EntityId,
        satellite_count: u32,
        debris_created: u32,
    },
    SatelliteOperational {
        satellite_id: EntityId,
        owner: EntityId,
    },
    SatelliteDecaying {
        satellite_id: EntityId,
        owner: EntityId,
    },
    SatelliteDeorbited {
        satellite_id: EntityId,
        owner: EntityId,
    },
    SatelliteDead {
        satellite_id: EntityId,
        owner: EntityId,
    },
    DebrisCollision {
        satellite_id: EntityId,
        owner: EntityId,
        debris_created: u32,
    },
    KesslerCascadeStarted {
        shell_min_km: f64,
        shell_max_km: f64,
        debris_count: u32,
    },
    SatelliteServiced {
        satellite_id: EntityId,
        service_type: String,
        cost: Money,
    },
    TerminalsDeployed {
        city_id: EntityId,
        count: u32,
        owner: EntityId,
    },
    SatelliteSubscribersGained {
        city_id: EntityId,
        owner: EntityId,
        new_subscribers: u32,
    },

    // UI & System
    GlobalNotification {
        message: String,
        level: String, // "info", "warning", "error", "success"
    },
}

impl GameEvent {
    /// Returns the corporation IDs that this event is relevant to.
    /// Events with no specific corp (global/market events) return an empty vec,
    /// meaning they should be sent to all players.
    pub fn related_corps(&self) -> Vec<EntityId> {
        match self {
            // Infrastructure — visible to all (real-world: infra is publicly visible)
            GameEvent::NodeBuilt { .. }
            | GameEvent::EdgeBuilt { .. }
            | GameEvent::NodeDestroyed { .. }
            | GameEvent::ConstructionStarted { .. }
            | GameEvent::ConstructionCompleted { .. } => vec![],

            // Finance — private to the corporation
            GameEvent::RevenueEarned { corporation, .. }
            | GameEvent::CostIncurred { corporation, .. }
            | GameEvent::LoanTaken { corporation, .. }
            | GameEvent::Bankruptcy { corporation } => vec![*corporation],

            // Corporation lifecycle — global
            GameEvent::CorporationFounded { .. } | GameEvent::CorporationMerged { .. } => vec![],

            // Contracts — relevant to both parties
            GameEvent::ContractProposed { from, to, .. } => vec![*from, *to],
            GameEvent::ContractAccepted { entity } | GameEvent::ContractExpired { entity } => {
                vec![*entity]
            }

            // Research — private
            GameEvent::ResearchStarted { corporation, .. }
            | GameEvent::ResearchCompleted { corporation, .. } => vec![*corporation],

            // Disasters & Weather — global
            GameEvent::DisasterStruck { .. }
            | GameEvent::WeatherStarted { .. } => vec![],
            GameEvent::InfrastructureDamaged { .. }
            | GameEvent::RepairStarted { .. }
            | GameEvent::RepairCompleted { .. } => vec![],

            // Subsidiaries — private
            GameEvent::SubsidiaryCreated { parent, .. } => vec![*parent],

            // Insurance — private
            GameEvent::InsurancePurchased { entity, .. }
            | GameEvent::InsurancePayout { entity, .. } => vec![*entity],

            // Regulation & Market — global
            GameEvent::RegulationChanged { .. }
            | GameEvent::MarketShiftOccurred { .. }
            | GameEvent::MarketUpdate { .. } => vec![],

            // Bankruptcy & Auctions — global
            GameEvent::InsolvencyWarning { corporation } => vec![*corporation],
            GameEvent::BailoutTaken { corporation, .. } => vec![*corporation],
            GameEvent::BankruptcyDeclared { .. }
            | GameEvent::AuctionStarted { .. }
            | GameEvent::AuctionBidPlaced { .. }
            | GameEvent::AuctionWon { .. }
            | GameEvent::AuctionCancelled { .. } => vec![],

            // M&A — relevant to both parties
            GameEvent::AcquisitionProposed {
                acquirer, target, ..
            }
            | GameEvent::AcquisitionAccepted { acquirer, target }
            | GameEvent::AcquisitionRejected { acquirer, target } => vec![*acquirer, *target],
            GameEvent::MergerCompleted { absorbed, absorber } => vec![*absorbed, *absorber],

            // Espionage — relevant to both spy and target
            GameEvent::EspionageCompleted { spy, target }
            | GameEvent::SabotageCompleted {
                saboteur: spy,
                target,
                ..
            }
            | GameEvent::EspionageDetected { spy, target, .. }
            | GameEvent::SabotageDetected {
                saboteur: spy,
                target,
                ..
            } => vec![*spy, *target],
            GameEvent::SecurityUpgraded { corporation, .. } => vec![*corporation],

            // Lobbying — private
            GameEvent::LobbyingStarted { corporation, .. }
            | GameEvent::LobbyingSucceeded { corporation, .. }
            | GameEvent::LobbyingFailed { corporation, .. }
            | GameEvent::ScandalOccurred { corporation, .. } => vec![*corporation],

            // Co-ownership — global (infra visibility)
            GameEvent::CoOwnershipEstablished { .. }
            | GameEvent::UpgradeVotePassed { .. }
            | GameEvent::UpgradeVoteRejected { .. }
            | GameEvent::BuyoutCompleted { .. } => vec![],

            // Alliance — relevant to all members
            GameEvent::AllianceFormed { members, .. } => members.clone(),
            GameEvent::AllianceDissolved { .. } => vec![],

            // Legal — relevant to both parties
            GameEvent::LawsuitFiled {
                plaintiff,
                defendant,
                ..
            }
            | GameEvent::LawsuitResolved {
                plaintiff,
                defendant,
                ..
            }
            | GameEvent::SettlementReached {
                plaintiff,
                defendant,
                ..
            } => vec![*plaintiff, *defendant],

            // Spectrum — global (auctions are public)
            GameEvent::SpectrumAuctionStarted { .. } | GameEvent::SpectrumAuctionWon { .. } => {
                vec![]
            }
            GameEvent::SpectrumBidPlaced { bidder, .. } => vec![*bidder],
            GameEvent::SpectrumLicenseExpired { owner, .. } => vec![*owner],

            // Spectrum Assignment/Unassignment — private to the corp
            GameEvent::SpectrumAssigned { corp, .. }
            | GameEvent::SpectrumUnassigned { corp, .. } => vec![*corp],

            // Cable Ship — private to the corp
            GameEvent::CableShipPurchased { corp } => vec![*corp],

            // Pricing — private
            GameEvent::PricingChanged { corporation, .. } => vec![*corporation],

            // Transit & Interconnection — relevant to both parties
            GameEvent::TransitPayment {
                provider, consumer, ..
            }
            | GameEvent::SLAPenaltyPaid {
                provider, consumer, ..
            } => vec![*provider, *consumer],

            // Maintenance — private
            GameEvent::MaintenancePrioritySet { .. } => vec![],

            // Achievements — private
            GameEvent::AchievementUnlocked { corporation, .. }
            | GameEvent::VictoryAchieved { corporation, .. } => vec![*corporation],

            // Patents & Licensing (Phase 5.3)
            GameEvent::PatentFiled { holder, .. } => vec![*holder],
            GameEvent::LicenseGranted { licensee, .. }
            | GameEvent::LicenseRevoked { licensee, .. } => vec![*licensee],
            GameEvent::IndependentResearchStarted { corporation, .. } => vec![*corporation],

            // Government Grants (Phase 5.4)
            GameEvent::GrantAvailable { .. } | GameEvent::GrantExpired { .. } => vec![],
            GameEvent::GrantAwarded { corporation, .. }
            | GameEvent::GrantCompleted { corporation, .. } => vec![*corporation],

            // Satellite System
            GameEvent::ConstellationCreated { owner, .. } => vec![*owner],
            GameEvent::SatelliteManufactured { owner, .. } => vec![*owner],
            GameEvent::LaunchAttempted { owner, .. }
            | GameEvent::LaunchSucceeded { owner, .. }
            | GameEvent::LaunchFailed { owner, .. } => vec![*owner],
            GameEvent::SatelliteOperational { owner, .. }
            | GameEvent::SatelliteDecaying { owner, .. }
            | GameEvent::SatelliteDeorbited { owner, .. }
            | GameEvent::SatelliteDead { owner, .. } => vec![*owner],
            GameEvent::DebrisCollision { owner, .. } => vec![*owner],
            GameEvent::KesslerCascadeStarted { .. } => vec![],  // global event
            GameEvent::SatelliteServiced { .. } => vec![],
            GameEvent::TerminalsDeployed { owner, .. } => vec![*owner],
            GameEvent::SatelliteSubscribersGained { owner, .. } => vec![*owner],

            // Global notifications — all players
            GameEvent::GlobalNotification { .. } => vec![],
        }
    }
}
