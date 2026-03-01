use gt_common::commands::Command;
use gt_common::types::EntityId;

/// Command category for rate limiting
pub(crate) enum CommandCategory {
    Build,      // BuildNode, BuildEdge, UpgradeNode, DecommissionNode
    Financial,  // TakeLoan, RepayLoan, SetBudget, PurchaseInsurance, etc.
    Research,   // StartResearch, CancelResearch
    Espionage,  // LaunchEspionage, LaunchSabotage
    General,    // Everything else
}

/// Extract the corporation EntityId that a command targets, if any.
/// Used for anti-cheat validation: ensures players can only issue commands
/// that affect their own corporation.
pub(crate) fn command_target_corp(command: &Command) -> Option<EntityId> {
    match command {
        Command::HireEmployee { corporation, .. }
        | Command::TakeLoan { corporation, .. }
        | Command::SetBudget { corporation, .. }
        | Command::StartResearch { corporation, .. }
        | Command::CancelResearch { corporation }
        | Command::SetPolicy { corporation, .. }
        | Command::DeclareBankruptcy { entity: corporation }
        | Command::RequestBailout { entity: corporation }
        | Command::AcceptBailout { entity: corporation } => Some(*corporation),
        Command::ProposeContract { from, .. } => Some(*from),
        Command::CreateSubsidiary { parent, .. } => Some(*parent),
        // Commands that operate on entities (nodes, edges, etc.) rather than
        // directly referencing a corp -- ownership is checked inside the
        // simulation engine, so we skip corp-level gating here.
        Command::BuildNode { .. }
        | Command::BuildEdge { .. }
        | Command::UpgradeNode { .. }
        | Command::DecommissionNode { .. }
        | Command::RepairNode { .. }
        | Command::RepairEdge { .. }
        | Command::EmergencyRepair { .. }
        | Command::FireEmployee { .. }
        | Command::AssignTeam { .. }
        | Command::RepayLoan { .. }
        | Command::AcceptContract { .. }
        | Command::RejectContract { .. }
        | Command::PurchaseInsurance { .. }
        | Command::CancelInsurance { .. }
        | Command::PlaceBid { .. }
        | Command::ProposeAcquisition { .. }
        | Command::RespondToAcquisition { .. }
        | Command::LaunchEspionage { .. }
        | Command::LaunchSabotage { .. }
        | Command::UpgradeSecurity { .. }
        | Command::StartLobbying { .. }
        | Command::CancelLobbying { .. }
        | Command::ProposeCoOwnership { .. }
        | Command::RespondCoOwnership { .. }
        | Command::ProposeBuyout { .. }
        | Command::VoteUpgrade { .. }
        | Command::UpdateEdgeWaypoints { .. }
        | Command::BidSpectrum { .. }
        | Command::AssignSpectrum { .. }
        | Command::UnassignSpectrum { .. }
        | Command::PurchaseCableShip
        | Command::SetSpeed(_)
        | Command::TogglePause
        | Command::LoadGame { .. }
        | Command::SaveGame { .. }
        | Command::ProposeAlliance { .. }
        | Command::AcceptAlliance { .. }
        | Command::DissolveAlliance { .. }
        | Command::FileLawsuit { .. }
        | Command::SettleLawsuit { .. }
        | Command::DefendLawsuit { .. }
        | Command::FilePatent { .. }
        | Command::RequestLicense { .. }
        | Command::SetLicensePrice { .. }
        | Command::RevokeLicense { .. }
        | Command::StartIndependentResearch { .. }
        | Command::BidForGrant { .. }
        | Command::CompleteGrant { .. }
        | Command::SetRegionPricing { .. }
        | Command::SetMaintenancePriority { .. }
        | Command::BuildConstellation { .. }
        | Command::OrderSatellites { .. }
        | Command::ScheduleLaunch { .. }
        | Command::ContractLaunch { .. }
        | Command::DeorbitSatellite { .. }
        | Command::OrderTerminals { .. }
        | Command::ShipTerminals { .. }
        | Command::SetSatellitePricing { .. }
        | Command::ServiceSatellite { .. } => None,
    }
}

/// Validate command parameters before forwarding to the simulation.
/// Returns an error message if validation fails.
pub(crate) fn validate_command(command: &Command) -> Result<(), &'static str> {
    match command {
        Command::BuildNode { lon, lat, .. } => {
            // Spatial validation: coordinates must be finite and within world bounds
            if !lon.is_finite() || !lat.is_finite() {
                return Err("Coordinates must be finite numbers");
            }
            if *lon < -180.0 || *lon > 180.0 {
                return Err("Longitude must be between -180 and 180");
            }
            if *lat < -90.0 || *lat > 90.0 {
                return Err("Latitude must be between -90 and 90");
            }
        }
        Command::TakeLoan { amount, .. } => {
            if *amount <= 0 {
                return Err("Loan amount must be positive");
            }
        }
        Command::HireEmployee { role, .. } => {
            if role.trim().is_empty() {
                return Err("Employee role cannot be empty");
            }
        }
        Command::ProposeContract { terms, .. } => {
            if terms.len() > 10_000 {
                return Err("Contract terms too long (max 10,000 chars)");
            }
        }
        Command::RepayLoan { amount, .. } => {
            if *amount <= 0 {
                return Err("Repayment amount must be positive");
            }
        }
        Command::PlaceBid { amount, .. } => {
            if *amount <= 0 {
                return Err("Bid amount must be positive");
            }
        }
        Command::ProposeAcquisition { offer, .. } => {
            if *offer <= 0 {
                return Err("Acquisition offer must be positive");
            }
        }
        Command::StartLobbying { budget, .. } => {
            if *budget <= 0 {
                return Err("Lobbying budget must be positive");
            }
        }
        Command::ProposeCoOwnership { share_pct, .. } => {
            if *share_pct <= 0.0 || *share_pct > 100.0 {
                return Err("Co-ownership share must be between 0 and 100");
            }
        }
        Command::ProposeBuyout { price, .. } => {
            if *price <= 0 {
                return Err("Buyout price must be positive");
            }
        }
        _ => {}
    }
    Ok(())
}

/// Categorize a command for per-type rate limiting.
pub(crate) fn categorize_command(command: &Command) -> CommandCategory {
    match command {
        Command::BuildNode { .. }
        | Command::BuildEdge { .. }
        | Command::UpgradeNode { .. }
        | Command::DecommissionNode { .. }
        | Command::RepairNode { .. }
        | Command::RepairEdge { .. }
        | Command::EmergencyRepair { .. }
        | Command::UpdateEdgeWaypoints { .. }
        | Command::BuildConstellation { .. }
        | Command::OrderSatellites { .. }
        | Command::ScheduleLaunch { .. }
        | Command::ContractLaunch { .. }
        | Command::DeorbitSatellite { .. }
        | Command::OrderTerminals { .. }
        | Command::ShipTerminals { .. }
        | Command::ServiceSatellite { .. } => CommandCategory::Build,

        Command::TakeLoan { .. }
        | Command::RepayLoan { .. }
        | Command::SetBudget { .. }
        | Command::PurchaseInsurance { .. }
        | Command::CancelInsurance { .. }
        | Command::PlaceBid { .. }
        | Command::ProposeAcquisition { .. }
        | Command::ProposeContract { .. }
        | Command::SetSatellitePricing { .. } => CommandCategory::Financial,

        Command::StartResearch { .. }
        | Command::CancelResearch { .. } => CommandCategory::Research,

        Command::LaunchEspionage { .. }
        | Command::LaunchSabotage { .. } => CommandCategory::Espionage,

        _ => CommandCategory::General,
    }
}
