use gt_common::types::*;

use crate::components::*;

use super::GameWorld;

impl GameWorld {
    // === Phase 8: Spectrum & Frequency Management ===

    pub(super) fn cmd_bid_spectrum(&mut self, band_name: &str, region: EntityId, bid: Money) {
        use gt_common::types::FrequencyBand;

        let corp_id = match self.player_corp_id {
            Some(id) => id,
            None => return,
        };

        let band = match FrequencyBand::from_name(band_name) {
            Some(b) => b,
            None => {
                self.event_queue.push(
                    self.tick,
                    gt_common::events::GameEvent::GlobalNotification {
                        message: format!("Invalid frequency band: {}", band_name),
                        level: "error".to_string(),
                    },
                );
                return;
            }
        };

        // Validate region exists
        if !self.regions.contains_key(&region) {
            self.event_queue.push(
                self.tick,
                gt_common::events::GameEvent::GlobalNotification {
                    message: "Invalid region for spectrum bid".to_string(),
                    level: "error".to_string(),
                },
            );
            return;
        }

        // Check player has funds
        if let Some(fin) = self.financials.get(&corp_id) {
            if fin.cash < bid {
                self.event_queue.push(
                    self.tick,
                    gt_common::events::GameEvent::GlobalNotification {
                        message: "Insufficient funds for spectrum bid".to_string(),
                        level: "warning".to_string(),
                    },
                );
                return;
            }
        } else {
            return;
        }

        // Minimum bid = base cost for the band's max bandwidth
        let min_bid = band.cost_per_mhz() * band.max_bandwidth_mhz() as Money;
        if bid < min_bid {
            self.event_queue.push(
                self.tick,
                gt_common::events::GameEvent::GlobalNotification {
                    message: format!(
                        "Bid too low. Minimum: ${} for {} {}",
                        min_bid,
                        band.display_name(),
                        band.max_bandwidth_mhz()
                    ),
                    level: "warning".to_string(),
                },
            );
            return;
        }

        // Check if there's already an active auction for this band+region
        let existing_auction = self
            .spectrum_auctions
            .iter()
            .find(|(_, a)| a.band == band && a.region_id == region && !a.is_ended(self.tick))
            .map(|(&id, _)| id);

        if let Some(auction_id) = existing_auction {
            // Update bid on existing auction
            let auction = self.spectrum_auctions.get_mut(&auction_id).unwrap();
            let current_highest = auction.highest_bid().map(|(_, amt)| amt).unwrap_or(0);
            if bid <= current_highest {
                self.event_queue.push(
                    self.tick,
                    gt_common::events::GameEvent::GlobalNotification {
                        message: format!("Bid must exceed current highest: ${}", current_highest),
                        level: "warning".to_string(),
                    },
                );
                return;
            }
            auction.place_bid(corp_id, bid);
            self.event_queue.push(
                self.tick,
                gt_common::events::GameEvent::SpectrumBidPlaced {
                    band: band_name.to_string(),
                    region,
                    bidder: corp_id,
                    amount: bid,
                },
            );
        } else {
            // Check band isn't already licensed in this region
            let already_licensed = self
                .spectrum_licenses
                .values()
                .any(|l| l.band == band && l.region_id == region && l.is_active(self.tick));

            if already_licensed {
                self.event_queue.push(
                    self.tick,
                    gt_common::events::GameEvent::GlobalNotification {
                        message: format!(
                            "{} is already licensed in this region",
                            band.display_name()
                        ),
                        level: "warning".to_string(),
                    },
                );
                return;
            }

            // Create new auction — resolves after 10 ticks
            let auction_id = self.allocate_entity();
            let mut auction = SpectrumAuction::new(
                band,
                region,
                band.max_bandwidth_mhz(),
                self.tick,
                10, // 10 tick auction duration
            );
            auction.place_bid(corp_id, bid);
            self.spectrum_auctions.insert(auction_id, auction);

            self.event_queue.push(
                self.tick,
                gt_common::events::GameEvent::SpectrumAuctionStarted {
                    band: band_name.to_string(),
                    region,
                },
            );
            self.event_queue.push(
                self.tick,
                gt_common::events::GameEvent::SpectrumBidPlaced {
                    band: band_name.to_string(),
                    region,
                    bidder: corp_id,
                    amount: bid,
                },
            );
        }
    }

    /// Resolve completed spectrum auctions and expire old licenses.
    /// Called at the end of each tick.
    pub fn resolve_spectrum_auctions(&mut self) {
        // Find auctions that have ended this tick
        let ended: Vec<EntityId> = self
            .spectrum_auctions
            .iter()
            .filter(|(_, a)| a.is_ended(self.tick))
            .map(|(&id, _)| id)
            .collect();

        for auction_id in ended {
            let auction = match self.spectrum_auctions.remove(&auction_id) {
                Some(a) => a,
                None => continue,
            };

            if let Some((winner, price)) = auction.highest_bid() {
                // Deduct funds from winner
                if let Some(fin) = self.financials.get_mut(&winner) {
                    fin.cash -= price;
                }

                // Create spectrum license (lasts 200 ticks)
                let license_id = self.allocate_entity();
                let license = SpectrumLicense::new(
                    auction.band,
                    auction.region_id,
                    winner,
                    auction.bandwidth_mhz,
                    self.tick,
                    200, // license duration: 200 ticks
                    price,
                );
                self.spectrum_licenses.insert(license_id, license);

                self.event_queue.push(
                    self.tick,
                    gt_common::events::GameEvent::SpectrumAuctionWon {
                        band: format!("{:?}", auction.band),
                        region: auction.region_id,
                        winner,
                        price,
                    },
                );
            }
        }

        // Expire old licenses
        let expired: Vec<EntityId> = self
            .spectrum_licenses
            .iter()
            .filter(|(_, l)| !l.is_active(self.tick) && l.end_tick() <= self.tick)
            .map(|(&id, _)| id)
            .collect();

        for license_id in expired {
            if let Some(license) = self.spectrum_licenses.remove(&license_id) {
                self.event_queue.push(
                    self.tick,
                    gt_common::events::GameEvent::SpectrumLicenseExpired {
                        band: format!("{:?}", license.band),
                        region: license.region_id,
                        owner: license.owner,
                    },
                );
            }
        }
    }

    /// Assign a spectrum band to a wireless infrastructure node.
    /// The owning corporation must hold an active license for the band in the node's region.
    /// Wireless nodes without an assigned band operate at 50% throughput.
    pub(super) fn cmd_assign_spectrum(&mut self, node_id: EntityId, band_name: &str) {
        use gt_common::types::FrequencyBand;

        let corp_id = match self.player_corp_id {
            Some(id) => id,
            None => return,
        };

        let band = match FrequencyBand::from_name(band_name) {
            Some(b) => b,
            None => {
                self.event_queue.push(
                    self.tick,
                    gt_common::events::GameEvent::GlobalNotification {
                        message: format!("Invalid frequency band: {}", band_name),
                        level: "error".to_string(),
                    },
                );
                return;
            }
        };

        // Validate node exists and is owned by the player
        let node = match self.infra_nodes.get(&node_id) {
            Some(n) => n,
            None => {
                self.event_queue.push(
                    self.tick,
                    gt_common::events::GameEvent::GlobalNotification {
                        message: "Node not found".to_string(),
                        level: "error".to_string(),
                    },
                );
                return;
            }
        };

        if node.owner != corp_id {
            self.event_queue.push(
                self.tick,
                gt_common::events::GameEvent::GlobalNotification {
                    message: "You do not own this node".to_string(),
                    level: "error".to_string(),
                },
            );
            return;
        }

        // Only wireless node types can have a spectrum band
        let is_wireless = matches!(
            node.node_type,
            NodeType::CellTower
                | NodeType::MacroCell
                | NodeType::SmallCell
                | NodeType::WirelessRelay
                | NodeType::MicrowaveTower
                | NodeType::SatelliteGroundStation
                | NodeType::MeshDroneRelay
                | NodeType::TerahertzRelay
        );

        if !is_wireless {
            self.event_queue.push(
                self.tick,
                gt_common::events::GameEvent::GlobalNotification {
                    message: "Only wireless nodes can be assigned a spectrum band".to_string(),
                    level: "error".to_string(),
                },
            );
            return;
        }

        // Check the corp holds an active license for this band in the node's region
        let node_pos = self.positions.get(&node_id);
        let node_region = node_pos.and_then(|pos| {
            self.regions
                .iter()
                .find(|(_, r)| {
                    // Simple distance-based check: node in the nearest region
                    let dx = pos.x - r.center_lon;
                    let dy = pos.y - r.center_lat;
                    dx * dx + dy * dy < 100.0 // within ~10 degrees
                })
                .map(|(&id, _)| id)
        });

        let has_license = self.spectrum_licenses.values().any(|l| {
            l.owner == corp_id
                && l.band == band
                && l.is_active(self.tick)
                && (node_region == Some(l.region_id))
        });

        if !has_license {
            self.event_queue.push(
                self.tick,
                gt_common::events::GameEvent::GlobalNotification {
                    message: format!(
                        "No active {} license in this region. Win a spectrum auction first.",
                        band.display_name()
                    ),
                    level: "warning".to_string(),
                },
            );
            return;
        }

        // Assign the band (carrier aggregation: push to Vec, skip if already assigned)
        if let Some(n) = self.infra_nodes.get_mut(&node_id) {
            if n.assigned_bands.contains(&band_name.to_string()) {
                self.event_queue.push(
                    self.tick,
                    gt_common::events::GameEvent::GlobalNotification {
                        message: format!(
                            "Band {} is already assigned to this node",
                            band.display_name()
                        ),
                        level: "warning".to_string(),
                    },
                );
                return;
            }
            n.assigned_bands.push(band_name.to_string());
        }

        self.event_queue.push(
            self.tick,
            gt_common::events::GameEvent::SpectrumAssigned {
                node: node_id,
                band: band_name.to_string(),
                corp: corp_id,
            },
        );
    }

    /// Remove a spectrum band from a wireless infrastructure node (carrier aggregation).
    pub(super) fn cmd_unassign_spectrum(&mut self, node_id: EntityId, band_name: &str) {
        use gt_common::types::FrequencyBand;

        let corp_id = match self.player_corp_id {
            Some(id) => id,
            None => return,
        };

        let band = match FrequencyBand::from_name(band_name) {
            Some(b) => b,
            None => {
                self.event_queue.push(
                    self.tick,
                    gt_common::events::GameEvent::GlobalNotification {
                        message: format!("Invalid frequency band: {}", band_name),
                        level: "error".to_string(),
                    },
                );
                return;
            }
        };

        // Validate node exists and is owned by the player
        let node = match self.infra_nodes.get(&node_id) {
            Some(n) => n,
            None => {
                self.event_queue.push(
                    self.tick,
                    gt_common::events::GameEvent::GlobalNotification {
                        message: "Node not found".to_string(),
                        level: "error".to_string(),
                    },
                );
                return;
            }
        };

        if node.owner != corp_id {
            self.event_queue.push(
                self.tick,
                gt_common::events::GameEvent::GlobalNotification {
                    message: "You do not own this node".to_string(),
                    level: "error".to_string(),
                },
            );
            return;
        }

        let band_str = band_name.to_string();
        if !node.assigned_bands.contains(&band_str) {
            self.event_queue.push(
                self.tick,
                gt_common::events::GameEvent::GlobalNotification {
                    message: format!("Band {} is not assigned to this node", band.display_name()),
                    level: "warning".to_string(),
                },
            );
            return;
        }

        // Remove the band
        if let Some(n) = self.infra_nodes.get_mut(&node_id) {
            n.assigned_bands.retain(|b| b != &band_str);
        }

        self.event_queue.push(
            self.tick,
            gt_common::events::GameEvent::SpectrumUnassigned {
                node: node_id,
                band: band_name.to_string(),
                corp: corp_id,
            },
        );
    }

    // === Satellite System ===

    pub(super) fn cmd_build_constellation(
        &mut self,
        name: String,
        orbit_type_str: &str,
        num_planes: u32,
        sats_per_plane: u32,
        altitude_km: f64,
        inclination_deg: f64,
    ) -> gt_common::protocol::CommandResult {
        use gt_common::protocol::CommandResult;

        let corp_id = match self.player_corp_id {
            Some(id) => id,
            None => return CommandResult::fail("No player corporation"),
        };

        let orbit_type = match orbit_type_str {
            "LEO" => gt_common::types::OrbitType::LEO,
            "MEO" => gt_common::types::OrbitType::MEO,
            "GEO" => gt_common::types::OrbitType::GEO,
            "HEO" => gt_common::types::OrbitType::HEO,
            _ => return CommandResult::fail("Invalid orbit type"),
        };

        // Validate altitude
        let (min_alt, max_alt) = orbit_type.altitude_range_km();
        if altitude_km < min_alt || altitude_km > max_alt {
            return CommandResult::fail(format!(
                "Altitude {:.0}km outside range for {:?}: {:.0}-{:.0}km",
                altitude_km, orbit_type, min_alt, max_alt
            ));
        }

        let constellation_id = self.allocate_entity();
        let constellation = crate::components::Constellation {
            name: name.clone(),
            owner: corp_id,
            orbit_type,
            target_altitude_km: altitude_km,
            target_inclination_deg: inclination_deg,
            num_planes,
            sats_per_plane,
            satellite_ids: Vec::new(),
            operational_count: 0,
        };

        self.constellations.insert(constellation_id, constellation);

        self.event_queue.push(
            self.tick,
            gt_common::events::GameEvent::ConstellationCreated {
                constellation_id,
                owner: corp_id,
                name,
            },
        );

        CommandResult::ok_with_entity(constellation_id)
    }
}
