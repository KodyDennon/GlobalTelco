// Infrastructure node icons
import centralOffice from './infrastructure/central-office.svg?raw';
import exchangePoint from './infrastructure/exchange-point.svg?raw';
import cellTower from './infrastructure/cell-tower.svg?raw';
import dataCenter from './infrastructure/data-center.svg?raw';
import satelliteGround from './infrastructure/satellite-ground.svg?raw';
import submarineLanding from './infrastructure/submarine-landing.svg?raw';
import wirelessRelay from './infrastructure/wireless-relay.svg?raw';
import backboneRouter from './infrastructure/backbone-router.svg?raw';

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
	// Infrastructure nodes — keyed to match Rust NodeType variants
	'central-office': centralOffice,
	'exchange-point': exchangePoint,
	'cell-tower': cellTower,
	'data-center': dataCenter,
	'satellite-ground': satelliteGround,
	'submarine-landing': submarineLanding,
	'wireless-relay': wirelessRelay,
	'backbone-router': backboneRouter,

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
	'central-office',
	'exchange-point',
	'cell-tower',
	'data-center',
	'satellite-ground',
	'submarine-landing',
	'wireless-relay',
	'backbone-router',
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
