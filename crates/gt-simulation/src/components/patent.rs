use gt_common::types::{EntityId, Money, Tick};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LicenseType {
    /// One-time payment, permanent access
    Permanent,
    /// Ongoing per-tick royalty payment
    Royalty,
    /// Per-unit charge (per node built using the tech)
    PerUnit,
    /// Time-limited access that expires
    Lease { expires_tick: Tick },
}

/// A patent grants exclusive rights over a completed technology.
/// Only the patent holder (or licensees) can use the patented tech.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Patent {
    pub tech_id: EntityId,
    pub holder_corp: EntityId,
    pub filed_tick: Tick,
    pub license_price: Money,
    pub license_type: LicenseType,
    /// Per-unit charge for PerUnit licenses (charged each time a node is built using the tech)
    pub per_unit_price: Money,
    /// Duration in ticks for Lease licenses
    pub lease_duration: u64,
}

impl Patent {
    pub fn new(tech_id: EntityId, holder_corp: EntityId, filed_tick: Tick) -> Self {
        Self {
            tech_id,
            holder_corp,
            filed_tick,
            license_price: 1_000_000, // default $1M
            license_type: LicenseType::Royalty,
            per_unit_price: 50_000, // default $50k per node
            lease_duration: 500, // default 500 ticks
        }
    }

    /// Per-tick royalty cost for Royalty-type licenses.
    pub fn royalty_per_tick(&self) -> Money {
        match self.license_type {
            LicenseType::Royalty => self.license_price / 100, // 1% of price per tick
            _ => 0,
        }
    }

    /// Per-unit charge for PerUnit-type licenses.
    pub fn per_unit_charge(&self) -> Money {
        match self.license_type {
            LicenseType::PerUnit => self.per_unit_price,
            _ => 0,
        }
    }
}

/// A license grants a corporation the right to use a patented technology.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct License {
    pub patent_id: EntityId,
    pub licensee_corp: EntityId,
    pub license_type: LicenseType,
    pub price_paid: Money,
    pub granted_tick: Tick,
    /// Number of times this license has been used (for PerUnit tracking)
    pub uses: u32,
    /// Total amount paid in royalties/per-unit fees over the lifetime of this license
    pub total_fees_paid: Money,
}

impl License {
    pub fn new(
        patent_id: EntityId,
        licensee_corp: EntityId,
        license_type: LicenseType,
        price_paid: Money,
        granted_tick: Tick,
    ) -> Self {
        Self {
            patent_id,
            licensee_corp,
            license_type,
            price_paid,
            granted_tick,
            uses: 0,
            total_fees_paid: 0,
        }
    }

    /// Whether this license is still active at the given tick.
    pub fn is_active(&self, tick: Tick) -> bool {
        match self.license_type {
            LicenseType::Permanent => true,
            LicenseType::Royalty => true, // active as long as royalties are paid
            LicenseType::PerUnit => true, // active indefinitely
            LicenseType::Lease { expires_tick } => tick < expires_tick,
        }
    }

    /// Record a per-unit use of the license. Returns the charge amount.
    pub fn record_use(&mut self, per_unit_price: Money) -> Money {
        self.uses += 1;
        self.total_fees_paid += per_unit_price;
        per_unit_price
    }
}
