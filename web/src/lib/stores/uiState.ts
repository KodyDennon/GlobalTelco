import { writable, derived, get } from 'svelte/store';
import { playerCorp } from './gameState';
import { audioManager } from '$lib/audio/AudioManager';

export type PanelType = 'none' | 'info' | 'dashboard' | 'infrastructure' | 'network' | 'research' | 'contracts' | 'region' | 'workforce' | 'advisor' | 'auctions' | 'mergers' | 'intel' | 'achievements' | 'spectrum';
export type OverlayType = 'none' | 'terrain' | 'ownership' | 'population' | 'demand' | 'disaster' | 'coverage' | 'congestion' | 'traffic' | 'market_share' | 'ocean_depth' | 'spectrum' | 'elevation_contour' | 'submarine_reference' | 'coverage_overlap' | 'density' | 'interconnection' | 'satellite';
export type PanelGroupType = 'finance' | 'operations' | 'diplomacy' | 'research' | 'market' | 'info' | 'satellite';

// ── Company Size Tier (Management Scaling) ───────────────────────────────────
// Small: 1-10 nodes (hands-on), Medium: 11-100 (teams/budgets), Large: 101+ (policies/departments)
export type CompanyTier = 'small' | 'medium' | 'large';

export const companyTier = derived(playerCorp, ($corp): CompanyTier => {
	const count = $corp?.infrastructure_count ?? 0;
	if (count > 100) return 'large';
	if (count > 10) return 'medium';
	return 'small';
});

export const companyTierLabel = derived(companyTier, ($tier): string => {
	switch ($tier) {
		case 'small': return 'Startup';
		case 'medium': return 'Regional Operator';
		case 'large': return 'Enterprise';
	}
});

// Panel group → tab definitions
export const PANEL_GROUP_TABS: Record<PanelGroupType, Array<{ key: string; label: string; component?: string; comingSoon?: { feature: string; phase: string; description: string } }>> = {
	finance: [
		{ key: 'dashboard', label: 'Dashboard', component: 'dashboard' },
		{ key: 'pricing', label: 'Pricing', component: 'pricing' },
		{ key: 'insurance', label: 'Insurance', component: 'insurance' },
		{ key: 'stockmarket', label: 'Stock Market', component: 'stockmarket' },
	],
	operations: [
		{ key: 'infrastructure', label: 'Infrastructure', component: 'infrastructure' },
		{ key: 'network', label: 'Network', component: 'network' },
		{ key: 'workforce', label: 'Workforce', component: 'workforce' },
		{ key: 'spectrum', label: 'Spectrum', component: 'spectrum' },
		{ key: 'maintenance', label: 'Maintenance', component: 'maintenance' },
		{ key: 'repair', label: 'Repair', component: 'repair' },
	],
	diplomacy: [
		{ key: 'intel', label: 'Intel', component: 'intel' },
		{ key: 'alliance', label: 'Alliance', component: 'alliance' },
		{ key: 'legal', label: 'Legal', component: 'legal' },
		{ key: 'coownership', label: 'Co-ownership', component: 'coownership' },
	],
	research: [
		{ key: 'research', label: 'Research', component: 'research' },
		{ key: 'patents', label: 'Patents', component: 'patents' },
	],
	market: [
		{ key: 'contracts', label: 'Contracts', component: 'contracts' },
		{ key: 'auctions', label: 'Auctions', component: 'auctions' },
		{ key: 'mergers', label: 'M&A', component: 'mergers' },
		{ key: 'grants', label: 'Grants', component: 'grants' },
	],
	info: [
		{ key: 'region', label: 'Region', component: 'region' },
		{ key: 'advisor', label: 'Advisor', component: 'advisor' },
		{ key: 'achievements', label: 'Achievements', component: 'achievements' },
	],
	satellite: [
		{ key: 'satellites', label: 'Fleet', component: 'satellites' },
		{ key: 'launch', label: 'Launch', component: 'launch' },
		{ key: 'terminals', label: 'Terminals', component: 'terminals' },
		{ key: 'debris', label: 'Debris', component: 'debris' },
	],
};

// Panel group display names
export const PANEL_GROUP_NAMES: Record<PanelGroupType, string> = {
	finance: 'Finance',
	operations: 'Operations',
	diplomacy: 'Diplomacy',
	research: 'Research',
	market: 'Market',
	info: 'Info',
	satellite: 'Satellite',
};

// New panel group system
export const activePanelGroup = writable<PanelGroupType | 'none'>('none');
export const activeGroupTab = writable<string>('');

// Legacy activePanel — derived from group system for backwards compatibility
export const activePanel = derived(
	[activePanelGroup, activeGroupTab],
	([$group, $tab]): PanelType => {
		if ($group === 'none') return 'none';
		const tabs = PANEL_GROUP_TABS[$group];
		const tabDef = tabs?.find(t => t.key === $tab);
		if (tabDef?.component) return tabDef.component as PanelType;
		return 'none';
	}
);

export const selectedEntityId = writable<number | null>(null);
export const selectedEntityType = writable<string | null>(null);
export const buildMode = writable<string | null>(null); // null | 'node' | 'edge'
export const buildEdgeSource = writable<number | null>(null); // source node ID for edge building
export const buildMenuLocation = writable<{ lon: number; lat: number } | null>(null);
export const zoomLevel = writable<number>(1);
export const viewport = writable({ minX: -180, minY: -90, maxX: 180, maxY: 90, zoom: 1 });
export const activeOverlay = writable<OverlayType>('none');
export const tooltipData = writable<{ x: number; y: number; content: string } | null>(null);
export const selectedEdgeType = writable<string>('FiberLocal');

// ── Radial Build Menu + Hotbar state ────────────────────────────────────────

/** Currently selected build item type string (e.g. 'CellTower', 'FiberLocal') or null */
export const selectedBuildItem = writable<string | null>(null);

/** Whether the selected item is a node or edge */
export const buildCategory = writable<'node' | 'edge' | null>(null);

/** Whether the radial menu is open */
export const radialMenuOpen = writable<boolean>(false);

/** Screen position where the radial menu should appear */
export const radialMenuPosition = writable<{ x: number; y: number }>({ x: 0, y: 0 });

/** Geo position (lon/lat) where the radial menu was opened */
export const radialMenuGeoPosition = writable<{ lon: number; lat: number } | null>(null);

/** Hotbar slot definition */
export interface HotbarSlot {
	itemType: string | null;
	category: 'node' | 'edge' | null;
}

const HOTBAR_STORAGE_KEY = 'globaltelco-hotbar';

const DEFAULT_HOTBAR_SLOTS: HotbarSlot[] = [
	{ itemType: 'CellTower', category: 'node' },
	{ itemType: 'CentralOffice', category: 'node' },
	{ itemType: 'FiberLocal', category: 'edge' },
	{ itemType: 'DataCenter', category: 'node' },
	{ itemType: null, category: null },
	{ itemType: null, category: null },
	{ itemType: null, category: null },
	{ itemType: null, category: null },
	{ itemType: null, category: null },
];

/** Load hotbar config from localStorage, falling back to defaults. */
function loadHotbarSlots(): HotbarSlot[] {
	try {
		if (typeof window === 'undefined' || !window.localStorage) return [...DEFAULT_HOTBAR_SLOTS];
		const stored = localStorage.getItem(HOTBAR_STORAGE_KEY);
		if (!stored) return [...DEFAULT_HOTBAR_SLOTS];
		const parsed = JSON.parse(stored) as unknown;
		if (!Array.isArray(parsed) || parsed.length !== 9) return [...DEFAULT_HOTBAR_SLOTS];
		// Validate each slot has the expected shape
		for (const slot of parsed) {
			if (typeof slot !== 'object' || slot === null) return [...DEFAULT_HOTBAR_SLOTS];
			if (!('itemType' in slot) || !('category' in slot)) return [...DEFAULT_HOTBAR_SLOTS];
		}
		return parsed as HotbarSlot[];
	} catch {
		return [...DEFAULT_HOTBAR_SLOTS];
	}
}

/** Save hotbar config to localStorage. */
function saveHotbarSlots(slots: HotbarSlot[]): void {
	try {
		if (typeof window === 'undefined' || !window.localStorage) return;
		localStorage.setItem(HOTBAR_STORAGE_KEY, JSON.stringify(slots));
	} catch {
		// Silently ignore storage errors (quota, security, etc.)
	}
}

/** 9 pinnable hotbar slots (keys 1-9) — persisted in localStorage */
export const hotbarSlots = writable<HotbarSlot[]>(loadHotbarSlots());

// Subscribe to changes and persist to localStorage
hotbarSlots.subscribe(saveHotbarSlots);

/** Enter placement mode for a specific item */
export function enterPlacementMode(itemType: string, category: 'node' | 'edge'): void {
	selectedBuildItem.set(itemType);
	buildCategory.set(category);
	if (category === 'node') {
		buildMode.set('node');
		buildEdgeSource.set(null);
	} else {
		buildMode.set('edge');
		selectedEdgeType.set(itemType);
		buildMenuLocation.set(null);
	}
	radialMenuOpen.set(false);
}

/** Exit placement mode */
export function exitPlacementMode(): void {
	selectedBuildItem.set(null);
	buildCategory.set(null);
	buildMode.set(null);
	buildMenuLocation.set(null);
	buildEdgeSource.set(null);
	radialMenuOpen.set(false);
}

// Edge target data when source is selected in edge build mode
export const edgeTargets = writable<Array<{
	target_id: number;
	target_type: string;
	x: number;
	y: number;
	distance_km: number;
	cost: number;
	affordable: boolean;
}>>([]);

// Tier compatibility matrix — matches Rust EdgeType::allowed_tier_connections()
// Keys: "T{from}-T{to}" where from <= to (sorted by tier value)
const TIER_MAP: Record<string, number> = {
	// Access (T1) — 9 types
	CellTower: 1, WirelessRelay: 1, TelegraphOffice: 1, TelegraphRelay: 1,
	TelephonePole: 1, SmallCell: 1, NetworkAccessPoint: 1, MeshDroneRelay: 1, TerahertzRelay: 1,
	// Aggregation (T2) — 13 types
	CentralOffice: 2, ExchangePoint: 2, ManualExchange: 2, AutomaticExchange: 2,
	LongDistanceRelay: 2, DigitalSwitch: 2, CoaxHub: 2, FiberPOP: 2, ISPGateway: 2,
	MacroCell: 2, FiberSplicePoint: 2, FiberDistributionHub: 2, QuantumRepeater: 2,
	// Core (T3) — 10 types
	DataCenter: 3, MicrowaveTower: 3, EarlyDataCenter: 3, InternetExchangePoint: 3,
	ColocationFacility: 3, EdgeDataCenter: 3, ContentDeliveryNode: 3, DWDM_Terminal: 3,
	CloudOnRamp: 3, NeuromorphicEdgeNode: 3,
	// Backbone (T4) — 3 types
	BackboneRouter: 4, HyperscaleDataCenter: 4, SatelliteGroundStation: 4,
	// Global (T5) — 6 types
	SatelliteGround: 5, SubmarineLanding: 5, CableHut: 5, SubseaLandingStation: 5,
	LEO_SatelliteGateway: 5, UnderwaterDataCenter: 5,
};

const EDGE_ALLOWED_TIERS: Record<string, [number, number][]> = {
	// Original 7
	Copper:         [[1,1],[1,2]],
	FiberLocal:     [[1,1],[1,2],[2,2]],
	FiberRegional:  [[2,2],[2,3],[3,3]],
	FiberNational:  [[3,3],[3,4],[4,4]],
	Microwave:      [[1,1],[1,2],[2,2],[2,3]],
	Satellite:      [[3,5],[4,5],[5,5]],
	Submarine:      [[5,5]],
	// Era 1: Telegraph
	TelegraphWire:          [[1,1],[1,2],[2,2]],
	SubseaTelegraphCable:   [[5,5],[2,5]],
	// Era 2: Telephone
	CopperTrunkLine:        [[1,2],[2,2]],
	LongDistanceCopper:     [[2,2],[2,3],[3,3]],
	// Era 3: Early Digital
	CoaxialCable:           [[1,1],[1,2],[2,2]],
	MicrowaveLink:          [[2,2],[2,3],[3,3],[3,4]],
	EarlySatelliteLink:     [[3,5],[4,5],[5,5]],
	// Era 4: Internet
	SubseaFiberCable:       [[4,5],[5,5]],
	// Era 5: Modern
	FiberMetro:             [[2,2],[2,3],[3,3]],
	FiberLongHaul:          [[3,3],[3,4],[4,4]],
	DWDM_Backbone:          [[3,4],[4,4],[4,5]],
	SatelliteLEOLink:       [[1,5],[3,5],[4,5],[5,5]],
	FeederFiber:            [[2,2],[2,3]],
	DistributionFiber:      [[1,2],[2,2]],
	DropCable:              [[1,1],[1,2]],
	// Era 6: Near Future
	QuantumFiberLink:       [[3,3],[3,4],[4,4],[4,5]],
	TerahertzBeam:          [[1,1],[1,2],[2,2]],
	LaserInterSatelliteLink:[[5,5],[4,5]],
};

/** Check if an edge type can connect two node types. */
export function canEdgeConnect(edgeType: string, fromType: string, toType: string): boolean {
	const tFrom = TIER_MAP[fromType];
	const tTo = TIER_MAP[toType];
	if (tFrom === undefined || tTo === undefined) return false;
	const lo = Math.min(tFrom, tTo);
	const hi = Math.max(tFrom, tTo);
	const allowed = EDGE_ALLOWED_TIERS[edgeType];
	if (!allowed) return false;
	return allowed.some(([a, b]) => a === lo && b === hi);
}

/** Get all edge types that can connect two node types. */
export function getCompatibleEdgeTypes(fromType: string, toType: string): string[] {
	return Object.keys(EDGE_ALLOWED_TIERS).filter(et => canEdgeConnect(et, fromType, toType));
}

/** Get all edge types compatible with a source node type. */
export function getEdgeTypesForSource(sourceType: string): string[] {
	const sTier = TIER_MAP[sourceType];
	if (sTier === undefined) return [];
	const result = new Set<string>();
	for (const [edgeType, pairs] of Object.entries(EDGE_ALLOWED_TIERS)) {
		for (const [lo, hi] of pairs) {
			if (lo === sTier || hi === sTier) {
				result.add(edgeType);
				break;
			}
		}
	}
	return [...result];
}

/** Get the tier number for a node type (1-5). */
export function getNodeTier(nodeType: string): number {
	return TIER_MAP[nodeType] ?? 0;
}

// ── Diplomatic Context Menu State ─────────────────────────────────────────────
export interface DiploMenuState {
	visible: boolean;
	x: number;
	y: number;
	corpId: number;
	corpName: string;
	nodeId: number;
	nodeType: string;
	entityType: 'node' | 'edge';
}
export const diploMenu = writable<DiploMenuState | null>(null);

// ── Build Mode Ghost Preview Data ─────────────────────────────────────────────
// Exposed by MapRenderer during node placement so the HUD can display
// terrain type, construction cost, terrain cost multiplier, and validity.

export interface GhostPreviewInfo {
	terrainType: string | null;
	cost: number | null;
	valid: boolean;
	costMultiplier: number;
}

export const ghostPreviewInfo = writable<GhostPreviewInfo>({
	terrainType: null,
	cost: null,
	valid: true,
	costMultiplier: 1.0,
});

// Terrain cost multiplier table (mirrors Rust TerrainType::construction_cost_multiplier)
export const TERRAIN_COST_MULTIPLIERS: Record<string, number> = {
	Urban: 2.0,
	Suburban: 1.2,
	Rural: 1.0,
	Mountainous: 3.0,
	Desert: 1.8,
	Coastal: 1.5,
	OceanShallow: 5.0,
	OceanDeep: 10.0,
	OceanTrench: 15.0,
	Tundra: 2.5,
	Frozen: 4.0,
};

// ── Pinned Dashboard Widgets ──────────────────────────────────────────────────
// Persist which NetworkDashboard widget sections are pinned as floating overlays.

const PINNED_WIDGETS_STORAGE_KEY = 'globaltelco-pinned-widgets';

function loadPinnedWidgets(): string[] {
	try {
		if (typeof window === 'undefined' || !window.localStorage) return [];
		const stored = localStorage.getItem(PINNED_WIDGETS_STORAGE_KEY);
		if (!stored) return [];
		const parsed = JSON.parse(stored);
		if (!Array.isArray(parsed)) return [];
		return parsed.filter((s: unknown) => typeof s === 'string');
	} catch {
		return [];
	}
}

function savePinnedWidgets(widgets: string[]): void {
	try {
		if (typeof window === 'undefined' || !window.localStorage) return;
		localStorage.setItem(PINNED_WIDGETS_STORAGE_KEY, JSON.stringify(widgets));
	} catch {
		// Silently ignore storage errors
	}
}

export const pinnedWidgets = writable<string[]>(loadPinnedWidgets());

// Subscribe to persist pinned widgets
pinnedWidgets.subscribe(savePinnedWidgets);

export function togglePinnedWidget(widgetId: string): void {
	pinnedWidgets.update(current => {
		if (current.includes(widgetId)) {
			return current.filter(w => w !== widgetId);
		}
		return [...current, widgetId];
	});
}

// Confirmation dialog state
export const confirmDialog = writable<{
	visible: boolean;
	message: string;
	onConfirm: (() => void) | null;
}>({ visible: false, message: '', onConfirm: null });

// Helper to open a panel group at a specific tab
export function openPanelGroup(group: PanelGroupType, tab?: string) {
	const tabs = PANEL_GROUP_TABS[group];
	activePanelGroup.set(group);
	activeGroupTab.set(tab ?? tabs[0]?.key ?? '');
	audioManager.playSfx('ui_open');
}

// Helper to close active panel group
export function closePanelGroup() {
	activePanelGroup.set('none');
	activeGroupTab.set('');
	audioManager.playSfx('ui_close');
}

// Helper to show confirmation dialog
export function showConfirm(message: string, onConfirm: () => void) {
	confirmDialog.set({ visible: true, message, onConfirm });
}

// ── Node Functional Supersession ──────────────────────────────────────────────
// Each chain is ordered old-to-new. When a newer node in the chain is buildable,
// older nodes in that chain are hidden from the build menu.
export const NODE_SUPERSESSION_CHAINS: string[][] = [
	// Local switching/exchange
	['TelegraphOffice', 'ManualExchange', 'AutomaticExchange', 'DigitalSwitch', 'CentralOffice'],
	// Access tower/pole
	['TelephonePole', 'CellTower', 'MacroCell'],
	// Relay/repeater
	['TelegraphRelay', 'LongDistanceRelay', 'WirelessRelay', 'MicrowaveTower'],
	// Data center evolution
	['EarlyDataCenter', 'DataCenter', 'ColocationFacility'],
	// Submarine landing
	['CableHut', 'SubmarineLanding', 'SubseaLandingStation'],
	// Satellite ground (GEO)
	['SatelliteGround', 'SatelliteGroundStation'],
	// Hub/POP
	['CoaxHub', 'FiberPOP'],
	// Exchange point
	['ExchangePoint', 'InternetExchangePoint'],
];

/**
 * Given a set of buildable node type names, filter out superseded nodes.
 * For each chain, finds the latest buildable node and hides all earlier ones.
 */
export function getVisibleNodes(buildableNodes: Set<string>): Set<string> {
	const hidden = new Set<string>();
	for (const chain of NODE_SUPERSESSION_CHAINS) {
		// Find the latest buildable node in this chain
		let latestIdx = -1;
		for (let i = chain.length - 1; i >= 0; i--) {
			if (buildableNodes.has(chain[i])) {
				latestIdx = i;
				break;
			}
		}
		// Hide all nodes before the latest
		if (latestIdx > 0) {
			for (let i = 0; i < latestIdx; i++) {
				hidden.add(chain[i]);
			}
		}
	}
	const visible = new Set<string>();
	for (const node of buildableNodes) {
		if (!hidden.has(node)) {
			visible.add(node);
		}
	}
	return visible;
}

// ── Edge Function Groups ──────────────────────────────────────────────────────
// Ordered old-to-new within each group. The build menu shows only the best
// available (latest buildable) per group, with expand to see older.
export const EDGE_FUNCTION_GROUPS: Record<string, string[]> = {
	'Local Access': ['TelegraphWire', 'CopperTrunkLine', 'CoaxialCable', 'Copper', 'FiberLocal'],
	'Metro/Regional': ['LongDistanceCopper', 'FiberRegional', 'FiberMetro'],
	'National/Backbone': ['FiberNational', 'FiberLongHaul', 'DWDM_Backbone', 'QuantumFiberLink'],
	'Submarine': ['SubseaTelegraphCable', 'Submarine', 'SubseaFiberCable'],
	'Terrestrial Wireless': ['Microwave', 'MicrowaveLink', 'TerahertzBeam'],
	'Satellite Link': ['EarlySatelliteLink', 'Satellite', 'SatelliteLEOLink', 'LaserInterSatelliteLink'],
};

// FTTH edges excluded from the radial menu (only in guided builder)
export const FTTH_EDGE_TYPES = new Set(['FeederFiber', 'DistributionFiber', 'DropCable']);

// FTTH nodes excluded from the radial menu (only in guided builder)
export const FTTH_NODE_TYPES = new Set(['FiberSplicePoint', 'FiberDistributionHub', 'NetworkAccessPoint']);

// Edge type → icon key mapping (5 available icons: fiber-optic, copper, microwave, satellite, submarine)
export const EDGE_ICON_MAP: Record<string, string> = {
	TelegraphWire: 'copper',
	CopperTrunkLine: 'copper',
	CoaxialCable: 'copper',
	Copper: 'copper',
	LongDistanceCopper: 'copper',
	FiberLocal: 'fiber-optic',
	FiberRegional: 'fiber-optic',
	FiberMetro: 'fiber-optic',
	FiberNational: 'fiber-optic',
	FiberLongHaul: 'fiber-optic',
	DWDM_Backbone: 'fiber-optic',
	QuantumFiberLink: 'fiber-optic',
	FeederFiber: 'fiber-optic',
	DistributionFiber: 'fiber-optic',
	DropCable: 'fiber-optic',
	Microwave: 'microwave',
	MicrowaveLink: 'microwave',
	TerahertzBeam: 'microwave',
	SubseaTelegraphCable: 'submarine',
	Submarine: 'submarine',
	SubseaFiberCable: 'submarine',
	EarlySatelliteLink: 'satellite',
	Satellite: 'satellite',
	SatelliteLEOLink: 'satellite',
	LaserInterSatelliteLink: 'satellite',
};

// ── FTTH Builder State ────────────────────────────────────────────────────────
export const ftthBuilderActive = writable<boolean>(false);

// ── Hotbar pinning helper ─────────────────────────────────────────────────────
/** Add an item to the first empty hotbar slot. Returns true if successful. */
export function addToHotbar(itemType: string, category: 'node' | 'edge'): boolean {
	let success = false;
	hotbarSlots.update(slots => {
		const emptyIdx = slots.findIndex(s => s.itemType === null);
		if (emptyIdx === -1) return slots;
		const updated = [...slots];
		updated[emptyIdx] = { itemType, category };
		success = true;
		return updated;
	});
	return success;
}

/** Check if an item is already in the hotbar */
export function isInHotbar(itemType: string): boolean {
	return get(hotbarSlots).some(s => s.itemType === itemType);
}
