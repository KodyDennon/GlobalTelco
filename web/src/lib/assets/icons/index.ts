// Infrastructure node icons — Original 8
import centralOffice from './infrastructure/central-office.svg?raw';
import exchangePoint from './infrastructure/exchange-point.svg?raw';
import cellTower from './infrastructure/cell-tower.svg?raw';
import dataCenter from './infrastructure/data-center.svg?raw';
import satelliteGround from './infrastructure/satellite-ground.svg?raw';
import submarineLanding from './infrastructure/submarine-landing.svg?raw';
import wirelessRelay from './infrastructure/wireless-relay.svg?raw';
import backboneRouter from './infrastructure/backbone-router.svg?raw';

// Era 1: Telegraph (~1850s)
import telegraphOffice from './infrastructure/telegraph-office.svg?raw';
import telegraphRelay from './infrastructure/telegraph-relay.svg?raw';
import cableHut from './infrastructure/cable-hut.svg?raw';

// Era 2: Telephone (~1900s)
import manualExchange from './infrastructure/manual-exchange.svg?raw';
import automaticExchange from './infrastructure/automatic-exchange.svg?raw';
import telephonePole from './infrastructure/telephone-pole.svg?raw';
import longDistanceRelay from './infrastructure/long-distance-relay.svg?raw';

// Era 3: Early Digital (~1970s)
import digitalSwitch from './infrastructure/digital-switch.svg?raw';
import microwaveTower from './infrastructure/microwave-tower.svg?raw';
import coaxHub from './infrastructure/coax-hub.svg?raw';
import earlyDataCenter from './infrastructure/early-data-center.svg?raw';
import satelliteGroundStation from './infrastructure/satellite-ground-station.svg?raw';

// Era 4: Internet (~1990s)
import fiberPop from './infrastructure/fiber-pop.svg?raw';
import internetExchangePoint from './infrastructure/internet-exchange-point.svg?raw';
import subseaLandingStation from './infrastructure/subsea-landing-station.svg?raw';
import colocationFacility from './infrastructure/colocation-facility.svg?raw';
import ispGateway from './infrastructure/isp-gateway.svg?raw';

// Era 5: Modern (~2010s)
import macroCell from './infrastructure/macro-cell.svg?raw';
import smallCell from './infrastructure/small-cell.svg?raw';
import edgeDataCenter from './infrastructure/edge-data-center.svg?raw';
import hyperscaleDataCenter from './infrastructure/hyperscale-data-center.svg?raw';
import cloudOnRamp from './infrastructure/cloud-on-ramp.svg?raw';
import contentDeliveryNode from './infrastructure/content-delivery-node.svg?raw';
import fiberSplicePoint from './infrastructure/fiber-splice-point.svg?raw';
import dwdmTerminal from './infrastructure/dwdm-terminal.svg?raw';
import fiberDistributionHub from './infrastructure/fiber-distribution-hub.svg?raw';
import networkAccessPoint from './infrastructure/network-access-point.svg?raw';

// Era 6: Near Future (~2030s)
import leoSatelliteGateway from './infrastructure/leo-satellite-gateway.svg?raw';
import quantumRepeater from './infrastructure/quantum-repeater.svg?raw';
import meshDroneRelay from './infrastructure/mesh-drone-relay.svg?raw';
import underwaterDataCenter from './infrastructure/underwater-data-center.svg?raw';
import neuromorphicEdgeNode from './infrastructure/neuromorphic-edge-node.svg?raw';
import terahertzRelay from './infrastructure/terahertz-relay.svg?raw';

// Satellite System
import leoSatellite from './infrastructure/leo-satellite.svg?raw';
import meoSatellite from './infrastructure/meo-satellite.svg?raw';
import geoSatellite from './infrastructure/geo-satellite.svg?raw';
import heoSatellite from './infrastructure/heo-satellite.svg?raw';
import leoGroundStation from './infrastructure/leo-ground-station.svg?raw';
import meoGroundStation from './infrastructure/meo-ground-station.svg?raw';
import satelliteFactory from './infrastructure/satellite-factory.svg?raw';
import terminalFactory from './infrastructure/terminal-factory.svg?raw';
import satelliteWarehouse from './infrastructure/satellite-warehouse.svg?raw';
import launchPad from './infrastructure/launch-pad.svg?raw';

// City tier icons
import hamlet from './cities/hamlet.svg?raw';
import town from './cities/town.svg?raw';
import city from './cities/city.svg?raw';
import metropolis from './cities/metropolis.svg?raw';
import megalopolis from './cities/megalopolis.svg?raw';

// Edge type icons
import fiberOptic from './edges/fiber-optic.svg?raw';
import copper from './edges/copper.svg?raw';
import microwave from './edges/microwave.svg?raw';
import satellite from './edges/satellite.svg?raw';
import submarine from './edges/submarine.svg?raw';

// UI icons
import pause from './ui/pause.svg?raw';
import play from './ui/play.svg?raw';
import fastForward from './ui/fast-forward.svg?raw';
import ultraSpeed from './ui/ultra-speed.svg?raw';
import save from './ui/save.svg?raw';
import money from './ui/money.svg?raw';
import research from './ui/research.svg?raw';
import workforce from './ui/workforce.svg?raw';
import contract from './ui/contract.svg?raw';
import settings from './ui/settings.svg?raw';
import warning from './ui/warning.svg?raw';
import dashboard from './ui/dashboard.svg?raw';
import infrastructure from './ui/infrastructure.svg?raw';
import region from './ui/region.svg?raw';
import advisor from './ui/advisor.svg?raw';
import auction from './ui/auction.svg?raw';
import merger from './ui/merger.svg?raw';
import intel from './ui/intel.svg?raw';
import achievement from './ui/achievement.svg?raw';

export const icons = {
	// Infrastructure nodes — keyed to match Rust NodeType kebab-case names
	// Original 8
	'central-office': centralOffice,
	'exchange-point': exchangePoint,
	'cell-tower': cellTower,
	'data-center': dataCenter,
	'satellite-ground': satelliteGround,
	'submarine-landing': submarineLanding,
	'wireless-relay': wirelessRelay,
	'backbone-router': backboneRouter,
	// Era 1: Telegraph
	'telegraph-office': telegraphOffice,
	'telegraph-relay': telegraphRelay,
	'cable-hut': cableHut,
	// Era 2: Telephone
	'manual-exchange': manualExchange,
	'automatic-exchange': automaticExchange,
	'telephone-pole': telephonePole,
	'long-distance-relay': longDistanceRelay,
	// Era 3: Early Digital
	'digital-switch': digitalSwitch,
	'microwave-tower': microwaveTower,
	'coax-hub': coaxHub,
	'early-data-center': earlyDataCenter,
	'satellite-ground-station': satelliteGroundStation,
	// Era 4: Internet
	'fiber-pop': fiberPop,
	'internet-exchange-point': internetExchangePoint,
	'subsea-landing-station': subseaLandingStation,
	'colocation-facility': colocationFacility,
	'isp-gateway': ispGateway,
	// Era 5: Modern
	'macro-cell': macroCell,
	'small-cell': smallCell,
	'edge-data-center': edgeDataCenter,
	'hyperscale-data-center': hyperscaleDataCenter,
	'cloud-on-ramp': cloudOnRamp,
	'content-delivery-node': contentDeliveryNode,
	'fiber-splice-point': fiberSplicePoint,
	'dwdm-terminal': dwdmTerminal,
	'fiber-distribution-hub': fiberDistributionHub,
	'network-access-point': networkAccessPoint,
	// Era 6: Near Future
	'leo-satellite-gateway': leoSatelliteGateway,
	'quantum-repeater': quantumRepeater,
	'mesh-drone-relay': meshDroneRelay,
	'underwater-data-center': underwaterDataCenter,
	'neuromorphic-edge-node': neuromorphicEdgeNode,
	'terahertz-relay': terahertzRelay,
	// Satellite System
	'leo-satellite': leoSatellite,
	'meo-satellite': meoSatellite,
	'geo-satellite': geoSatellite,
	'heo-satellite': heoSatellite,
	'leo-ground-station': leoGroundStation,
	'meo-ground-station': meoGroundStation,
	'satellite-factory': satelliteFactory,
	'terminal-factory': terminalFactory,
	'satellite-warehouse': satelliteWarehouse,
	'launch-pad': launchPad,

	// City tiers — keyed by population bracket
	hamlet,
	town,
	city,
	metropolis,
	megalopolis,

	// Edge types — keyed to match Rust EdgeType variants
	'fiber-optic': fiberOptic,
	copper,
	microwave,
	satellite,
	submarine,

	// UI
	pause,
	play,
	'fast-forward': fastForward,
	'ultra-speed': ultraSpeed,
	save,
	money,
	research,
	workforce,
	contract,
	settings,
	warning,
	dashboard,
	infrastructure,
	region,
	advisor,
	auction,
	merger,
	intel,
	achievement,
} as const;

export type IconName = keyof typeof icons;

/** All infrastructure node icon names */
export const infrastructureIcons: IconName[] = [
	// Original 8
	'central-office',
	'exchange-point',
	'cell-tower',
	'data-center',
	'satellite-ground',
	'submarine-landing',
	'wireless-relay',
	'backbone-router',
	// Era 1: Telegraph
	'telegraph-office',
	'telegraph-relay',
	'cable-hut',
	// Era 2: Telephone
	'manual-exchange',
	'automatic-exchange',
	'telephone-pole',
	'long-distance-relay',
	// Era 3: Early Digital
	'digital-switch',
	'microwave-tower',
	'coax-hub',
	'early-data-center',
	'satellite-ground-station',
	// Era 4: Internet
	'fiber-pop',
	'internet-exchange-point',
	'subsea-landing-station',
	'colocation-facility',
	'isp-gateway',
	// Era 5: Modern
	'macro-cell',
	'small-cell',
	'edge-data-center',
	'hyperscale-data-center',
	'cloud-on-ramp',
	'content-delivery-node',
	'fiber-splice-point',
	'dwdm-terminal',
	'fiber-distribution-hub',
	'network-access-point',
	// Era 6: Near Future
	'leo-satellite-gateway',
	'quantum-repeater',
	'mesh-drone-relay',
	'underwater-data-center',
	'neuromorphic-edge-node',
	'terahertz-relay',
	// Satellite System
	'leo-satellite',
	'meo-satellite',
	'geo-satellite',
	'heo-satellite',
	'leo-ground-station',
	'meo-ground-station',
	'satellite-factory',
	'terminal-factory',
	'satellite-warehouse',
	'launch-pad',
];

/** All city tier icon names */
export const cityIcons: IconName[] = [
	'hamlet',
	'town',
	'city',
	'metropolis',
	'megalopolis',
];

/** All edge type icon names */
export const edgeIcons: IconName[] = [
	'fiber-optic',
	'copper',
	'microwave',
	'satellite',
	'submarine',
];

/** All UI icon names */
export const uiIcons: IconName[] = [
	'pause',
	'play',
	'fast-forward',
	'ultra-speed',
	'save',
	'money',
	'research',
	'workforce',
	'contract',
	'settings',
	'warning',
	'dashboard',
	'infrastructure',
	'region',
	'advisor',
	'auction',
	'merger',
	'intel',
	'achievement',
];
