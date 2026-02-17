use gt_common::types::{EntityId, Money};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResearchCategory {
    OpticalNetworks,
    Wireless5G,
    Satellite,
    DataCenter,
    NetworkResilience,
    OperationalEfficiency,
}

impl ResearchCategory {
    pub fn base_cost(&self) -> Money {
        match self {
            ResearchCategory::OpticalNetworks => 5_000_000,
            ResearchCategory::Wireless5G => 8_000_000,
            ResearchCategory::Satellite => 10_000_000,
            ResearchCategory::DataCenter => 6_000_000,
            ResearchCategory::NetworkResilience => 3_000_000,
            ResearchCategory::OperationalEfficiency => 2_000_000,
        }
    }

    pub fn throughput_bonus(&self) -> f64 {
        match self {
            ResearchCategory::OpticalNetworks => 0.25,
            ResearchCategory::Wireless5G => 0.20,
            ResearchCategory::Satellite => 0.15,
            ResearchCategory::DataCenter => 0.30,
            ResearchCategory::NetworkResilience => 0.05,
            ResearchCategory::OperationalEfficiency => 0.0,
        }
    }

    pub fn cost_reduction(&self) -> f64 {
        match self {
            ResearchCategory::OperationalEfficiency => 0.15,
            ResearchCategory::NetworkResilience => 0.05,
            _ => 0.0,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            ResearchCategory::OpticalNetworks => "Optical Networks",
            ResearchCategory::Wireless5G => "Wireless & 5G",
            ResearchCategory::Satellite => "Satellite Systems",
            ResearchCategory::DataCenter => "Data Centers",
            ResearchCategory::NetworkResilience => "Network Resilience",
            ResearchCategory::OperationalEfficiency => "Operations",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PatentStatus {
    None,
    Patented,
    OpenSourced,
    Proprietary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechResearch {
    pub category: ResearchCategory,
    pub name: String,
    pub description: String,
    pub progress: f64,
    pub total_cost: Money,
    pub researcher: Option<EntityId>,
    pub completed: bool,
    pub patent_status: PatentStatus,
    pub patent_owner: Option<EntityId>,
    pub license_price: Money,
    pub licensed_to: Vec<EntityId>,
    pub prerequisites: Vec<String>,
    pub throughput_bonus: f64,
    pub cost_reduction: f64,
    pub reliability_bonus: f64,
}

impl TechResearch {
    pub fn new(category: ResearchCategory, name: impl Into<String>) -> Self {
        Self {
            total_cost: category.base_cost(),
            throughput_bonus: category.throughput_bonus(),
            cost_reduction: category.cost_reduction(),
            category,
            name: name.into(),
            description: String::new(),
            progress: 0.0,
            researcher: None,
            completed: false,
            patent_status: PatentStatus::None,
            patent_owner: None,
            license_price: 0,
            licensed_to: Vec::new(),
            prerequisites: Vec::new(),
            reliability_bonus: 0.0,
        }
    }

    pub fn with_details(
        category: ResearchCategory,
        name: impl Into<String>,
        description: impl Into<String>,
        cost: Money,
        throughput: f64,
        cost_red: f64,
        reliability: f64,
        prereqs: Vec<String>,
    ) -> Self {
        Self {
            category,
            name: name.into(),
            description: description.into(),
            total_cost: cost,
            throughput_bonus: throughput,
            cost_reduction: cost_red,
            reliability_bonus: reliability,
            prerequisites: prereqs,
            progress: 0.0,
            researcher: None,
            completed: false,
            patent_status: PatentStatus::None,
            patent_owner: None,
            license_price: 0,
            licensed_to: Vec::new(),
        }
    }

    pub fn advance(&mut self, investment: Money) -> bool {
        if self.completed {
            return true;
        }
        self.progress += investment as f64;
        if self.progress >= self.total_cost as f64 {
            self.completed = true;
        }
        self.completed
    }

    pub fn progress_pct(&self) -> f64 {
        if self.total_cost == 0 {
            1.0
        } else {
            (self.progress / self.total_cost as f64).clamp(0.0, 1.0)
        }
    }
}

/// Returns the full tech tree — 36 technologies across 6 categories
pub fn generate_tech_tree() -> Vec<TechResearch> {
    vec![
        // === Optical Networks (6 techs) ===
        TechResearch::with_details(
            ResearchCategory::OpticalNetworks, "Dense WDM", "Wavelength-division multiplexing for fiber",
            3_000_000, 0.15, 0.0, 0.0, vec![],
        ),
        TechResearch::with_details(
            ResearchCategory::OpticalNetworks, "Coherent Optics", "Advanced modulation for long-haul fiber",
            5_000_000, 0.20, 0.0, 0.02, vec!["Dense WDM".into()],
        ),
        TechResearch::with_details(
            ResearchCategory::OpticalNetworks, "Photonic Switching", "All-optical packet switching",
            8_000_000, 0.25, 0.05, 0.0, vec!["Coherent Optics".into()],
        ),
        TechResearch::with_details(
            ResearchCategory::OpticalNetworks, "Hollow-Core Fiber", "Next-gen fiber with lower latency",
            12_000_000, 0.15, 0.0, 0.05, vec!["Photonic Switching".into()],
        ),
        TechResearch::with_details(
            ResearchCategory::OpticalNetworks, "Quantum Key Distribution", "Quantum-secure optical channels",
            15_000_000, 0.05, 0.0, 0.10, vec!["Hollow-Core Fiber".into()],
        ),
        TechResearch::with_details(
            ResearchCategory::OpticalNetworks, "Optical Computing", "Optical signal processing at nodes",
            20_000_000, 0.30, 0.10, 0.0, vec!["Quantum Key Distribution".into()],
        ),
        // === Wireless & 5G (6 techs) ===
        TechResearch::with_details(
            ResearchCategory::Wireless5G, "Massive MIMO", "Multi-antenna beamforming",
            4_000_000, 0.20, 0.0, 0.0, vec![],
        ),
        TechResearch::with_details(
            ResearchCategory::Wireless5G, "mmWave Deployment", "Millimeter wave spectrum utilization",
            6_000_000, 0.25, 0.0, 0.0, vec!["Massive MIMO".into()],
        ),
        TechResearch::with_details(
            ResearchCategory::Wireless5G, "Network Slicing", "Virtual network partitioning",
            7_000_000, 0.10, 0.08, 0.0, vec!["mmWave Deployment".into()],
        ),
        TechResearch::with_details(
            ResearchCategory::Wireless5G, "O-RAN Architecture", "Open radio access network",
            9_000_000, 0.15, 0.12, 0.0, vec!["Network Slicing".into()],
        ),
        TechResearch::with_details(
            ResearchCategory::Wireless5G, "6G Research", "Sub-THz spectrum research",
            15_000_000, 0.30, 0.0, 0.05, vec!["O-RAN Architecture".into()],
        ),
        TechResearch::with_details(
            ResearchCategory::Wireless5G, "AI Radio Management", "ML-optimized spectrum allocation",
            12_000_000, 0.20, 0.15, 0.03, vec!["O-RAN Architecture".into()],
        ),
        // === Satellite Systems (6 techs) ===
        TechResearch::with_details(
            ResearchCategory::Satellite, "LEO Constellation", "Low-earth orbit satellite mesh",
            8_000_000, 0.15, 0.0, 0.0, vec![],
        ),
        TechResearch::with_details(
            ResearchCategory::Satellite, "Inter-Sat Links", "Laser links between satellites",
            10_000_000, 0.20, 0.0, 0.05, vec!["LEO Constellation".into()],
        ),
        TechResearch::with_details(
            ResearchCategory::Satellite, "Ground Station Auto", "Automated ground station management",
            6_000_000, 0.05, 0.10, 0.0, vec!["LEO Constellation".into()],
        ),
        TechResearch::with_details(
            ResearchCategory::Satellite, "V-Band Spectrum", "High-frequency satellite bandwidth",
            12_000_000, 0.25, 0.0, 0.0, vec!["Inter-Sat Links".into()],
        ),
        TechResearch::with_details(
            ResearchCategory::Satellite, "Satellite Edge Compute", "Processing at satellite level",
            14_000_000, 0.15, 0.05, 0.03, vec!["V-Band Spectrum".into()],
        ),
        TechResearch::with_details(
            ResearchCategory::Satellite, "Mega Constellation", "1000+ satellite deployment",
            25_000_000, 0.35, 0.0, 0.08, vec!["Satellite Edge Compute".into()],
        ),
        // === Data Centers (6 techs) ===
        TechResearch::with_details(
            ResearchCategory::DataCenter, "Liquid Cooling", "Immersion cooling for dense racks",
            4_000_000, 0.10, 0.08, 0.0, vec![],
        ),
        TechResearch::with_details(
            ResearchCategory::DataCenter, "GPU Clusters", "Accelerated compute infrastructure",
            6_000_000, 0.30, 0.0, 0.0, vec!["Liquid Cooling".into()],
        ),
        TechResearch::with_details(
            ResearchCategory::DataCenter, "Edge Computing", "Distributed micro data centers",
            8_000_000, 0.20, 0.05, 0.03, vec!["GPU Clusters".into()],
        ),
        TechResearch::with_details(
            ResearchCategory::DataCenter, "Modular DC Design", "Prefab container data centers",
            5_000_000, 0.05, 0.15, 0.0, vec!["Liquid Cooling".into()],
        ),
        TechResearch::with_details(
            ResearchCategory::DataCenter, "Green Energy DC", "Renewable-powered data centers",
            10_000_000, 0.0, 0.20, 0.0, vec!["Modular DC Design".into()],
        ),
        TechResearch::with_details(
            ResearchCategory::DataCenter, "Quantum Computing", "Quantum processing capability",
            30_000_000, 0.40, 0.0, 0.05, vec!["GPU Clusters".into(), "Edge Computing".into()],
        ),
        // === Network Resilience (6 techs) ===
        TechResearch::with_details(
            ResearchCategory::NetworkResilience, "Auto Failover", "Automatic traffic rerouting",
            3_000_000, 0.0, 0.0, 0.10, vec![],
        ),
        TechResearch::with_details(
            ResearchCategory::NetworkResilience, "Redundant Paths", "Multi-path routing algorithms",
            4_000_000, 0.05, 0.0, 0.12, vec!["Auto Failover".into()],
        ),
        TechResearch::with_details(
            ResearchCategory::NetworkResilience, "Disaster Hardening", "Ruggedized infrastructure",
            6_000_000, 0.0, 0.0, 0.15, vec!["Redundant Paths".into()],
        ),
        TechResearch::with_details(
            ResearchCategory::NetworkResilience, "Self-Healing Network", "AI-driven auto-repair",
            10_000_000, 0.05, 0.05, 0.20, vec!["Disaster Hardening".into()],
        ),
        TechResearch::with_details(
            ResearchCategory::NetworkResilience, "Cyber Defense Suite", "Advanced intrusion prevention",
            8_000_000, 0.0, 0.0, 0.15, vec!["Auto Failover".into()],
        ),
        TechResearch::with_details(
            ResearchCategory::NetworkResilience, "Zero-Trust Architecture", "Complete network security overhaul",
            12_000_000, 0.0, 0.03, 0.25, vec!["Cyber Defense Suite".into(), "Self-Healing Network".into()],
        ),
        // === Operational Efficiency (6 techs) ===
        TechResearch::with_details(
            ResearchCategory::OperationalEfficiency, "NOC Automation", "Network operations center AI",
            2_000_000, 0.0, 0.10, 0.0, vec![],
        ),
        TechResearch::with_details(
            ResearchCategory::OperationalEfficiency, "Predictive Maintenance", "ML-based failure prediction",
            4_000_000, 0.0, 0.08, 0.08, vec!["NOC Automation".into()],
        ),
        TechResearch::with_details(
            ResearchCategory::OperationalEfficiency, "Supply Chain Opt", "Optimized equipment procurement",
            3_000_000, 0.0, 0.15, 0.0, vec!["NOC Automation".into()],
        ),
        TechResearch::with_details(
            ResearchCategory::OperationalEfficiency, "Digital Twin", "Virtual network simulation",
            6_000_000, 0.05, 0.10, 0.05, vec!["Predictive Maintenance".into()],
        ),
        TechResearch::with_details(
            ResearchCategory::OperationalEfficiency, "Workforce AI", "AI-assisted field operations",
            5_000_000, 0.0, 0.12, 0.03, vec!["Supply Chain Opt".into()],
        ),
        TechResearch::with_details(
            ResearchCategory::OperationalEfficiency, "Full Autonomy", "Fully autonomous network operations",
            15_000_000, 0.10, 0.25, 0.10, vec!["Digital Twin".into(), "Workforce AI".into()],
        ),
    ]
}
