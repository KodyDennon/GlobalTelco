import { writable, derived } from 'svelte/store';

export type PanelType = 'none' | 'info' | 'dashboard' | 'infrastructure' | 'research' | 'contracts' | 'region' | 'workforce' | 'advisor' | 'auctions' | 'mergers' | 'intel' | 'achievements';
export type OverlayType = 'none' | 'terrain' | 'ownership' | 'population' | 'demand' | 'disaster' | 'coverage' | 'congestion' | 'traffic';
export type PanelGroupType = 'finance' | 'operations' | 'diplomacy' | 'research' | 'market' | 'info';

// Panel group → tab definitions
export const PANEL_GROUP_TABS: Record<PanelGroupType, Array<{ key: string; label: string; component?: string; comingSoon?: { feature: string; phase: string; description: string } }>> = {
	finance: [
		{ key: 'dashboard', label: 'Dashboard', component: 'dashboard' },
		{ key: 'pricing', label: 'Pricing', comingSoon: { feature: 'Regional Pricing', phase: 'Phase 5', description: 'Set per-region pricing strategies and dynamic rate adjustments.' } },
		{ key: 'insurance', label: 'Insurance', comingSoon: { feature: 'Insurance', phase: 'Phase 8', description: 'Insure infrastructure against disasters and market crashes.' } },
	],
	operations: [
		{ key: 'infrastructure', label: 'Infrastructure', component: 'infrastructure' },
		{ key: 'workforce', label: 'Workforce', component: 'workforce' },
		{ key: 'maintenance', label: 'Maintenance', comingSoon: { feature: 'Maintenance Priority', phase: 'Phase 6', description: 'Set maintenance schedules and priority levels for your infrastructure.' } },
		{ key: 'repair', label: 'Repair', comingSoon: { feature: 'Repair Queue', phase: 'Phase 6', description: 'Manage post-disaster repair queues and emergency response.' } },
	],
	diplomacy: [
		{ key: 'intel', label: 'Intel', component: 'intel' },
		{ key: 'alliance', label: 'Alliance', comingSoon: { feature: 'Alliances', phase: 'Phase 9', description: 'Form strategic alliances for shared routing and infrastructure.' } },
		{ key: 'legal', label: 'Legal', comingSoon: { feature: 'Legal Actions', phase: 'Phase 10', description: 'File lawsuits, defend patents, and handle regulatory disputes.' } },
		{ key: 'coownership', label: 'Co-ownership', comingSoon: { feature: 'Co-ownership', phase: 'Phase 9', description: 'Jointly own infrastructure with shared revenue and voting.' } },
	],
	research: [
		{ key: 'research', label: 'Research', component: 'research' },
		{ key: 'patents', label: 'Patents', comingSoon: { feature: 'Patents', phase: 'Phase 7', description: 'File patents on technologies and license them to competitors.' } },
	],
	market: [
		{ key: 'contracts', label: 'Contracts', component: 'contracts' },
		{ key: 'auctions', label: 'Auctions', component: 'auctions' },
		{ key: 'mergers', label: 'M&A', component: 'mergers' },
		{ key: 'grants', label: 'Grants', comingSoon: { feature: 'Government Grants', phase: 'Phase 11', description: 'Apply for government infrastructure subsidies and grants.' } },
	],
	info: [
		{ key: 'region', label: 'Region', component: 'region' },
		{ key: 'advisor', label: 'Advisor', component: 'advisor' },
		{ key: 'achievements', label: 'Achievements', component: 'achievements' },
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
export const hoveredEntityId = writable<number | null>(null);
export const buildMode = writable<string | null>(null); // null | 'node' | 'edge'
export const buildEdgeSource = writable<number | null>(null); // source node ID for edge building
export const buildMenuLocation = writable<{ lon: number; lat: number } | null>(null);
export const zoomLevel = writable<number>(1);
export const viewport = writable({ minX: -180, minY: -90, maxX: 180, maxY: 90 });
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

/** 9 pinnable hotbar slots (keys 1-9) */
export const hotbarSlots = writable<HotbarSlot[]>([
	{ itemType: 'CellTower', category: 'node' },
	{ itemType: 'CentralOffice', category: 'node' },
	{ itemType: 'FiberLocal', category: 'edge' },
	{ itemType: 'DataCenter', category: 'node' },
	{ itemType: null, category: null },
	{ itemType: null, category: null },
	{ itemType: null, category: null },
	{ itemType: null, category: null },
	{ itemType: null, category: null },
]);

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
	CellTower: 1, WirelessRelay: 1,
	CentralOffice: 2, ExchangePoint: 2,
	DataCenter: 3,
	BackboneRouter: 4,
	SatelliteGround: 5, SubmarineLanding: 5,
};

const EDGE_ALLOWED_TIERS: Record<string, [number, number][]> = {
	Copper:         [[1,1],[1,2]],
	FiberLocal:     [[1,1],[1,2],[2,2]],
	Microwave:      [[1,1],[1,2],[2,2],[2,3]],
	FiberRegional:  [[2,2],[2,3],[3,3]],
	FiberNational:  [[3,3],[3,4],[4,4]],
	Satellite:      [[3,5],[4,5],[5,5]],
	Submarine:      [[5,5]],
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
}

// Helper to close active panel group
export function closePanelGroup() {
	activePanelGroup.set('none');
	activeGroupTab.set('');
}

// Helper to show confirmation dialog
export function showConfirm(message: string, onConfirm: () => void) {
	confirmDialog.set({ visible: true, message, onConfirm });
}
