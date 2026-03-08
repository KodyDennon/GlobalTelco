/**
 * Native Tauri bridge — all sim queries route through Tauri IPC
 * to the background SimThread instead of WASM.
 *
 * All public getters are **synchronous** — they return cached data.
 * Async IPC fetches happen internally during tick() and refreshAll().
 * This preserves the synchronous API contract that the rest of the
 * codebase depends on (MapRenderer, panels, GameLoop, etc.).
 */

import type {
	WorldInfo,
	CorporationData,
	Region,
	City,
	InfrastructureList,
	Notification,
	GridCell,
	CellCoverage,
	AllInfrastructure,
	CorpSummary,
	ContractInfo,
	DebtInfo,
	ResearchInfo,
	BuildOption,
	EdgeTarget,
	DamagedNode,
	AuctionInfo,
	AcquisitionInfo,
	CovertOpsInfo,
	LobbyingInfo,
	AchievementsInfo,
	VictoryInfo,
	TrafficFlows,
	WorldConfig,
	WorldPreviewData,
	InfraNodesTyped,
	InfraEdgesTyped,
	CorporationsTyped,
	SpectrumLicense,
	SpectrumAuction,
	AvailableSpectrum,
	GrantInfo,
	SatelliteInventoryItem,
	CoOwnershipProposal,
	UpgradeVote
} from './types';

import type {
	DisasterForecast,
	WeatherForecast,
	RoadSegmentInfo,
	ConstellationData,
	OrbitalSatellite,
	LaunchPadInfo,
	TerminalInventory,
	OrbitalShellStatus,
	SatelliteArrays,
	AllianceInfo,
	LawsuitInfo,
	StockMarketInfo,
	RegionPricingInfo,
	MaintenancePriorityInfo,
} from './bridge';

import {
	unpackInfraNodes,
	unpackInfraEdges,
	unpackSatellites,
	unpackCorporations,
} from './binaryUnpack';

let invoke: (cmd: string, args?: Record<string, unknown>) => Promise<unknown>;

// ── Cached state ──────────────────────────────────────────────────────────
// All caches are populated asynchronously and read synchronously.

// Hot-path caches (refreshed every tick)
let cachedWorldInfo: WorldInfo = {} as WorldInfo;
let cachedNotifications: Notification[] = [];
let cachedPlayerCorpData: CorporationData = {} as CorporationData;
let cachedCorporationMetadata: Map<number, CorporationData> = new Map();
let cachedInfraNodes: InfraNodesTyped = { count: 0, ids: new Uint32Array(0), owners: new Uint32Array(0), positions: new Float64Array(0), stats: new Float64Array(0), node_types: new Uint8Array(0), network_levels: new Uint32Array(0), construction_flags: new Uint8Array(0), cell_indices: new Uint32Array(0) };
let cachedInfraEdges: InfraEdgesTyped = { 
    count: 0, 
    ids: new Uint32Array(0), 
    owners: new Uint32Array(0), 
    endpoints: new Float64Array(0), 
    stats: new Float64Array(0), 
    edge_types: new Uint8Array(0),
    deployment_types: new Uint8Array(0),
    waypoints_data: new Float64Array(0),
    waypoint_offsets: new Uint32Array(0),
    waypoint_lengths: new Uint8Array(0)
};
let cachedCorporationsTyped: CorporationsTyped = { count: 0, ids: new Uint32Array(0), financials: new Float64Array(0), name_offsets: new Uint32Array(0), names_packed: new Uint8Array(0) };
let cachedSatelliteArrays: SatelliteArrays | null = null;

// Full refresh caches (refreshed every 5 ticks)
let cachedRegions: Region[] = [];
let cachedCities: City[] = [];
let cachedAllCorporations: CorpSummary[] = [];
let cachedTrafficFlows: TrafficFlows = { edge_flows: [], node_flows: [], total_served: 0, total_dropped: 0, total_demand: 0, player_served: 0, player_dropped: 0, top_congested: [] };
let cachedPlayerInfraList: InfrastructureList = { nodes: [], edges: [] };
let cachedCellCoverage: CellCoverage[] = [];
let cachedAllInfrastructure: AllInfrastructure = { nodes: [], edges: [] };
let cachedSpectrumLicenses: SpectrumLicense[] = [];

// Stable caches (refreshed on newGame/loadGame)
let cachedGridCells: GridCell[] = [];
let cachedIsRealEarth = false;
let cachedPlayerCorpId = 0;
let cachedStaticDefinitions: any = null;

// Panel data caches (fetch-behind: return cached, async refresh in background)
let cachedContracts: ContractInfo[] = [];
let cachedContractsCorpId = -1;
let cachedDebtInstruments: DebtInfo[] = [];
let cachedDebtCorpId = -1;
let cachedResearchState: ResearchInfo[] = [];
let cachedBuildableNodes: BuildOption[] = [];
let cachedBuildableNodesKey = '';
let cachedBuildableEdges: EdgeTarget[] = [];
let cachedBuildableEdgesId = -1;
let cachedDamagedNodes: DamagedNode[] = [];
let cachedDamagedCorpId = -1;
let cachedAuctions: AuctionInfo[] = [];
let cachedAcquisitionProposals: AcquisitionInfo[] = [];
let cachedCovertOps: CovertOpsInfo = { security_level: 0, active_missions: 0, detection_count: 0 };
let cachedCovertOpsCorpId = -1;
let cachedLobbyingCampaigns: LobbyingInfo[] = [];
let cachedLobbyingCorpId = -1;
let cachedAchievements: AchievementsInfo = { unlocked: [], progress: {} };
let cachedAchievementsCorpId = -1;
let cachedVictoryState: VictoryInfo = {} as VictoryInfo;
let cachedSpectrumAuctions: SpectrumAuction[] = [];
let cachedAvailableSpectrum: AvailableSpectrum[] = [];
let cachedAvailableSpectrumRegionId = -1;
let cachedDisasterForecasts: DisasterForecast[] = [];
let cachedWeatherForecasts: WeatherForecast[] = [];
let cachedRoadSegments: RoadSegmentInfo[] = [];
let cachedConstellationData: ConstellationData[] = [];
let cachedConstellationCorpId = -1;
let cachedOrbitalView: OrbitalSatellite[] = [];
let cachedLaunchSchedule: LaunchPadInfo[] = [];
let cachedLaunchCorpId = -1;
let cachedTerminalInventory: TerminalInventory = { factories: [], warehouses: [] };
let cachedTerminalCorpId = -1;
let cachedDebrisStatus: OrbitalShellStatus[] = [];
let cachedWorldPreview: WorldPreviewData | null = null;
let cachedWorldGeoJSON: unknown = null;
let cachedAlliances: AllianceInfo[] = [];
let cachedAlliancesCorpId = -1;
let cachedLawsuits: LawsuitInfo[] = [];
let cachedLawsuitsCorpId = -1;
let cachedGrants: GrantInfo[] = [];
let cachedGrantsCorpId = -1;
let cachedSatelliteInventory: SatelliteInventoryItem[] = [];
let cachedSatelliteInventoryCorpId = -1;
let cachedCoOwnershipProposals: CoOwnershipProposal[] = [];
let cachedCoOwnershipCorpId = -1;
let cachedPendingUpgradeVotes: UpgradeVote[] = [];
let cachedUpgradeVotesCorpId = -1;
let cachedStockMarket: StockMarketInfo = { public: false, total_shares: 0, share_price: 0, dividends_per_share: 0, ipo_tick: null, shareholder_satisfaction: 0, board_votes: [], shareholders: {} };
let cachedStockMarketCorpId = -1;
let cachedRegionPricing: RegionPricingInfo[] = [];
let cachedRegionPricingCorpId = -1;
let cachedMaintenancePrioritiesList: MaintenancePriorityInfo[] = [];
let cachedMaintenancePrioritiesCorpId = -1;

// Targeted metadata caches
let cachedNodeMetadata: Map<number, any> = new Map();
let cachedEdgeMetadata: Map<number, any> = new Map();

let lastFullRefreshTick = -1;

// ── Init ──────────────────────────────────────────────────────────────────

export async function init(): Promise<void> {
	const tauriModulePath = '@tauri-apps/api/core';
	const tauri = await import(/* @vite-ignore */ tauriModulePath);
	invoke = tauri.invoke;
}

// ── Lifecycle ─────────────────────────────────────────────────────────────

export async function newGame(config?: Partial<WorldConfig>): Promise<void> {
	const configJson = config ? JSON.stringify(config) : '{}';
	await invoke('sim_new_game', { configJson });
	lastFullRefreshTick = -1;
	await refreshAll();
}

export async function loadGame(data: string): Promise<void> {
	await invoke('sim_load_game', { data });
	lastFullRefreshTick = -1;
	await refreshAll();
}

export async function tick(): Promise<void> {
	await invoke('sim_tick');
	await refreshPostTick();
}

export async function saveGame(): Promise<string> {
	return (await invoke('sim_save_game')) as string;
}

export async function processCommand(commandJson: string): Promise<string> {
	return (await invoke('sim_process_command', { commandJson })) as string;
}

export async function applyBatch(opsJson: string): Promise<void> {
	await invoke('sim_apply_batch', { opsJson });
}

// ── Post-tick refresh ─────────────────────────────────────────────────────

async function refreshPostTick(): Promise<void> {
	const [infoJson, notifsJson, infraBuf, edgeBuf, corpBuf, satBuf] = await Promise.all([
		invoke('sim_get_world_info') as Promise<string>,
		invoke('sim_get_notifications') as Promise<string>,
		invoke('sim_get_infra_binary') as Promise<ArrayBuffer>,
		invoke('sim_get_edges_binary') as Promise<ArrayBuffer>,
		invoke('sim_get_corporations_binary') as Promise<ArrayBuffer>,
		invoke('sim_get_satellites_binary') as Promise<ArrayBuffer>,
	]);
	cachedWorldInfo = JSON.parse(infoJson);
	cachedNotifications = JSON.parse(notifsJson);
	cachedInfraNodes = unpackInfraNodes(infraBuf);
	cachedInfraEdges = unpackInfraEdges(edgeBuf);
	cachedCorporationsTyped = unpackCorporations(corpBuf);
	cachedSatelliteArrays = unpackSatellites(satBuf);

	if (cachedWorldInfo.player_corp_id > 0) {
		const corpJson = (await invoke('sim_get_corporation_data', { id: cachedWorldInfo.player_corp_id })) as string;
		cachedPlayerCorpData = JSON.parse(corpJson);
		cachedPlayerCorpId = cachedWorldInfo.player_corp_id;
	}

	// Full refresh every 5 ticks
	if (cachedWorldInfo.tick - lastFullRefreshTick >= 5) {
		lastFullRefreshTick = cachedWorldInfo.tick;
		await refreshFull();
	}
}

async function refreshFull(): Promise<void> {
	const [regionsJson, citiesJson, corpsJson, coverageJson, allInfraJson, spectrumJson, trafficJson, auctionsJson, acqJson, specAuctJson, disasterJson, weatherJson, victoryJson, researchJson] = await Promise.all([
		invoke('sim_get_regions') as Promise<string>,
		invoke('sim_get_cities') as Promise<string>,
		invoke('sim_get_all_corporations') as Promise<string>,
		invoke('sim_get_cell_coverage') as Promise<string>,
		invoke('sim_get_all_infrastructure') as Promise<string>,
		invoke('sim_get_spectrum_licenses') as Promise<string>,
		invoke('sim_get_traffic_flows') as Promise<string>,
		invoke('sim_get_auctions') as Promise<string>,
		invoke('sim_get_acquisition_proposals') as Promise<string>,
		invoke('sim_get_spectrum_auctions') as Promise<string>,
		invoke('sim_get_disaster_forecasts') as Promise<string>,
		invoke('sim_get_weather_forecasts') as Promise<string>,
		invoke('sim_get_victory_state') as Promise<string>,
		invoke('sim_get_research_state') as Promise<string>,
	]);
	cachedRegions = JSON.parse(regionsJson);
	cachedCities = JSON.parse(citiesJson);
	cachedAllCorporations = JSON.parse(corpsJson);
	cachedCellCoverage = JSON.parse(coverageJson);
	cachedAllInfrastructure = JSON.parse(allInfraJson);
	cachedSpectrumLicenses = JSON.parse(spectrumJson);
	cachedTrafficFlows = JSON.parse(trafficJson);
	cachedAuctions = JSON.parse(auctionsJson);
	cachedAcquisitionProposals = JSON.parse(acqJson);
	cachedSpectrumAuctions = JSON.parse(specAuctJson);
	cachedDisasterForecasts = JSON.parse(disasterJson);
	cachedWeatherForecasts = JSON.parse(weatherJson);
	cachedVictoryState = JSON.parse(victoryJson);
	cachedResearchState = JSON.parse(researchJson);

	// Refresh player-specific data
	if (cachedPlayerCorpId > 0) {
		const [infraJson, contractsJson, debtJson, damagedJson, covertJson, lobbyJson, achieveJson, constJson, launchJson, termJson, alliancesJson, lawsuitsJson, stockJson, pricingJson, maintJson, grantsJson, inventoryJson, coOwnershipJson, votesJson] = await Promise.all([
			invoke('sim_get_infrastructure_list', { id: cachedPlayerCorpId }) as Promise<string>,
			invoke('sim_get_contracts', { id: cachedPlayerCorpId }) as Promise<string>,
			invoke('sim_get_debt_instruments', { id: cachedPlayerCorpId }) as Promise<string>,
			invoke('sim_get_damaged_nodes', { id: cachedPlayerCorpId }) as Promise<string>,
			invoke('sim_get_covert_ops', { id: cachedPlayerCorpId }) as Promise<string>,
			invoke('sim_get_lobbying_campaigns', { id: cachedPlayerCorpId }) as Promise<string>,
			invoke('sim_get_achievements', { id: cachedPlayerCorpId }) as Promise<string>,
			invoke('sim_get_constellation_data', { id: cachedPlayerCorpId }) as Promise<string>,
			invoke('sim_get_launch_schedule', { id: cachedPlayerCorpId }) as Promise<string>,
			invoke('sim_get_terminal_inventory', { id: cachedPlayerCorpId }) as Promise<string>,
			invoke('sim_get_alliances', { id: cachedPlayerCorpId }) as Promise<string>,
			invoke('sim_get_lawsuits', { id: cachedPlayerCorpId }) as Promise<string>,
			invoke('sim_get_stock_market', { id: cachedPlayerCorpId }) as Promise<string>,
			invoke('sim_get_region_pricing', { id: cachedPlayerCorpId }) as Promise<string>,
			invoke('sim_get_maintenance_priorities', { id: cachedPlayerCorpId }) as Promise<string>,
			invoke('sim_get_grants', { id: cachedPlayerCorpId }) as Promise<string>,
			invoke('sim_get_satellite_inventory', { id: cachedPlayerCorpId }) as Promise<string>,
			invoke('sim_get_co_ownership_proposals', { id: cachedPlayerCorpId }) as Promise<string>,
			invoke('sim_get_pending_upgrade_votes', { id: cachedPlayerCorpId }) as Promise<string>,
		]);
		cachedPlayerInfraList = JSON.parse(infraJson);
		cachedContracts = JSON.parse(contractsJson);
		cachedContractsCorpId = cachedPlayerCorpId;
		cachedDebtInstruments = JSON.parse(debtJson);
		cachedDebtCorpId = cachedPlayerCorpId;
		cachedDamagedNodes = JSON.parse(damagedJson);
		cachedDamagedCorpId = cachedPlayerCorpId;
		cachedCovertOps = JSON.parse(covertJson);
		cachedCovertOpsCorpId = cachedPlayerCorpId;
		cachedLobbyingCampaigns = JSON.parse(lobbyJson);
		cachedLobbyingCorpId = cachedPlayerCorpId;
		cachedAchievements = JSON.parse(achieveJson);
		cachedAchievementsCorpId = cachedPlayerCorpId;
		cachedConstellationData = JSON.parse(constJson);
		cachedConstellationCorpId = cachedPlayerCorpId;
		cachedLaunchSchedule = JSON.parse(launchJson);
		cachedLaunchCorpId = cachedPlayerCorpId;
		cachedTerminalInventory = JSON.parse(termJson);
		cachedTerminalCorpId = cachedPlayerCorpId;
		cachedAlliances = JSON.parse(alliancesJson);
		cachedAlliancesCorpId = cachedPlayerCorpId;
		cachedLawsuits = JSON.parse(lawsuitsJson);
		cachedLawsuitsCorpId = cachedPlayerCorpId;
		cachedStockMarket = JSON.parse(stockJson);
		cachedStockMarketCorpId = cachedPlayerCorpId;
		cachedRegionPricing = JSON.parse(pricingJson);
		cachedRegionPricingCorpId = cachedPlayerCorpId;
		cachedMaintenancePrioritiesList = JSON.parse(maintJson);
		cachedMaintenancePrioritiesCorpId = cachedPlayerCorpId;
		cachedGrants = JSON.parse(grantsJson);
		cachedGrantsCorpId = cachedPlayerCorpId;
		cachedSatelliteInventory = JSON.parse(inventoryJson);
		cachedSatelliteInventoryCorpId = cachedPlayerCorpId;
		cachedCoOwnershipProposals = JSON.parse(coOwnershipJson);
		cachedCoOwnershipCorpId = cachedPlayerCorpId;
		cachedPendingUpgradeVotes = JSON.parse(votesJson);
		cachedUpgradeVotesCorpId = cachedPlayerCorpId;
	}

	// Debris + orbital view (not corp-specific)
	const [debrisJson, orbitalJson] = await Promise.all([
		invoke('sim_get_debris_status') as Promise<string>,
		invoke('sim_get_orbital_view') as Promise<string>,
	]);
	cachedDebrisStatus = JSON.parse(debrisJson);
	cachedOrbitalView = JSON.parse(orbitalJson);
}

async function refreshAll(): Promise<void> {
	const [infoJson, notifsJson, regionsJson, citiesJson, corpsJson, gridJson, realEarthJson, playerCorpIdJson, coverageJson, allInfraJson, spectrumJson] = await Promise.all([
		invoke('sim_get_world_info') as Promise<string>,
		invoke('sim_get_notifications') as Promise<string>,
		invoke('sim_get_regions') as Promise<string>,
		invoke('sim_get_cities') as Promise<string>,
		invoke('sim_get_all_corporations') as Promise<string>,
		invoke('sim_get_grid_cells') as Promise<string>,
		invoke('sim_get_is_real_earth') as Promise<string>,
		invoke('sim_get_player_corp_id') as Promise<string>,
		invoke('sim_get_cell_coverage') as Promise<string>,
		invoke('sim_get_all_infrastructure') as Promise<string>,
		invoke('sim_get_spectrum_licenses') as Promise<string>,
	]);
	cachedWorldInfo = JSON.parse(infoJson);
	cachedNotifications = JSON.parse(notifsJson);
	cachedRegions = JSON.parse(regionsJson);
	cachedCities = JSON.parse(citiesJson);
	cachedAllCorporations = JSON.parse(corpsJson);
	cachedGridCells = JSON.parse(gridJson);
	cachedIsRealEarth = JSON.parse(realEarthJson);
	cachedPlayerCorpId = JSON.parse(playerCorpIdJson);
	cachedCellCoverage = JSON.parse(coverageJson);
	cachedAllInfrastructure = JSON.parse(allInfraJson);
	cachedSpectrumLicenses = JSON.parse(spectrumJson);
	lastFullRefreshTick = cachedWorldInfo.tick;

	// Fetch typed arrays
	const [infraBuf, edgeBuf, corpBuf, satBuf] = await Promise.all([
		invoke('sim_get_infra_binary') as Promise<ArrayBuffer>,
		invoke('sim_get_edges_binary') as Promise<ArrayBuffer>,
		invoke('sim_get_corporations_binary') as Promise<ArrayBuffer>,
		invoke('sim_get_satellites_binary') as Promise<ArrayBuffer>,
	]);
	cachedInfraNodes = unpackInfraNodes(infraBuf);
	cachedInfraEdges = unpackInfraEdges(edgeBuf);
	cachedCorporationsTyped = unpackCorporations(corpBuf);
	cachedSatelliteArrays = unpackSatellites(satBuf);

	if (cachedPlayerCorpId > 0) {
		const [corpJson, infraJson, trafficJson] = await Promise.all([
			invoke('sim_get_corporation_data', { id: cachedPlayerCorpId }) as Promise<string>,
			invoke('sim_get_infrastructure_list', { id: cachedPlayerCorpId }) as Promise<string>,
			invoke('sim_get_traffic_flows') as Promise<string>,
		]);
		cachedPlayerCorpData = JSON.parse(corpJson);
		cachedPlayerInfraList = JSON.parse(infraJson);
		cachedTrafficFlows = JSON.parse(trafficJson);
	}

	// Trigger full panel data refresh
	await refreshFull();
}

// ── Synchronous cached getters ────────────────────────────────────────────
// These are called by bridge.ts in the `useNativeSim` path.
// All return cached data synchronously. Data is refreshed asynchronously
// during tick() (every tick or every 5 ticks).

// Hot-path (every tick)
export function getCachedWorldInfo(): WorldInfo { return cachedWorldInfo; }
export function getCachedNotifications(): Notification[] {
	const result = cachedNotifications;
	cachedNotifications = [];
	return result;
}
export function getCachedInfraNodesTyped(): InfraNodesTyped { return cachedInfraNodes; }
export function getCachedInfraEdgesTyped(): InfraEdgesTyped { return cachedInfraEdges; }
export function getCachedCorporationsTyped(): CorporationsTyped { return cachedCorporationsTyped; }
export function getCachedSatelliteArrays(): SatelliteArrays | null { return cachedSatelliteArrays; }
export function getCachedCorporationData(corpId: number): CorporationData {
	if (corpId === cachedPlayerCorpId) return cachedPlayerCorpData;
	
	const cached = cachedCorporationMetadata.get(corpId);
	if (cached) return cached;

	// Trigger async fetch for background/competitor corp
	invoke('sim_get_corporation_data', { id: corpId }).then((json) => {
		cachedCorporationMetadata.set(corpId, JSON.parse(json as string));
	}).catch(() => {});
	
	return {} as CorporationData;
}

// Full refresh (every 5 ticks)
export function getCachedRegions(): Region[] { return cachedRegions; }
export function getCachedCities(): City[] { return cachedCities; }
export function getCachedAllCorporations(): CorpSummary[] { return cachedAllCorporations; }
export function getCachedTrafficFlows(): TrafficFlows { return cachedTrafficFlows; }
export function getCachedInfrastructureList(corpId: number): InfrastructureList {
	if (corpId === cachedPlayerCorpId) return cachedPlayerInfraList;
	return { nodes: [], edges: [] };
}
export function getCachedCellCoverage(): CellCoverage[] { return cachedCellCoverage; }
export function getCachedAllInfrastructure(): AllInfrastructure { return cachedAllInfrastructure; }
export function getCachedSpectrumLicenses(): SpectrumLicense[] { return cachedSpectrumLicenses; }

// Stable (refreshed on newGame/loadGame)
export function getCachedGridCells(): GridCell[] { return cachedGridCells; }
export function getCachedPlayerCorpId(): number { return cachedPlayerCorpId; }
export function getCachedIsRealEarth(): boolean { return cachedIsRealEarth; }

export function getCachedStaticDefinitions(): any {
	if (cachedStaticDefinitions === null) {
		invoke('sim_get_static_definitions').then((json) => {
			cachedStaticDefinitions = JSON.parse(json as string);
		}).catch(() => {});
	}
	return cachedStaticDefinitions;
}

export function getCachedInfraNodesTypedViewport(west: number, south: number, east: number, north: number, minLevel: number): InfraNodesTyped {
	// Tauri doesn't currently support partial viewport sync via IPC; return full cached set
	// This maintains API compatibility with the worker bridge
	return cachedInfraNodes;
}

export function getCachedInfraEdgesTypedViewport(west: number, south: number, east: number, north: number, minLevel: number): InfraEdgesTyped {
	return cachedInfraEdges;
}

export function getCachedGrants(corpId: number): GrantInfo[] {
	if (corpId !== cachedGrantsCorpId) {
		cachedGrantsCorpId = corpId;
		invoke('sim_get_grants', { id: corpId }).then((json) => { cachedGrants = JSON.parse(json as string); }).catch(() => {});
	}
	return cachedGrants;
}

export function getCachedSatelliteInventory(corpId: number): SatelliteInventoryItem[] {
	if (corpId !== cachedSatelliteInventoryCorpId) {
		cachedSatelliteInventoryCorpId = corpId;
		invoke('sim_get_satellite_inventory', { id: corpId }).then((json) => { cachedSatelliteInventory = JSON.parse(json as string); }).catch(() => {});
	}
	return cachedSatelliteInventory;
}

export function getCachedCoOwnershipProposals(corpId: number): CoOwnershipProposal[] {
	if (corpId !== cachedCoOwnershipCorpId) {
		cachedCoOwnershipCorpId = corpId;
		invoke('sim_get_co_ownership_proposals', { id: corpId }).then((json) => { cachedCoOwnershipProposals = JSON.parse(json as string); }).catch(() => {});
	}
	return cachedCoOwnershipProposals;
}

export function getCachedPendingUpgradeVotes(corpId: number): UpgradeVote[] {
	if (corpId !== cachedUpgradeVotesCorpId) {
		cachedUpgradeVotesCorpId = corpId;
		invoke('sim_get_pending_upgrade_votes', { id: corpId }).then((json) => { cachedPendingUpgradeVotes = JSON.parse(json as string); }).catch(() => {});
	}
	return cachedPendingUpgradeVotes;
}

// Panel data (refreshed every 5 ticks via refreshFull, fetch-behind for different corpIds)

export function getCachedContracts(corpId: number): ContractInfo[] {
	if (corpId !== cachedContractsCorpId) {
		cachedContractsCorpId = corpId;
		invoke('sim_get_contracts', { id: corpId }).then((json) => { cachedContracts = JSON.parse(json as string); }).catch(() => {});
	}
	return cachedContracts;
}

export function getCachedDebtInstruments(corpId: number): DebtInfo[] {
	if (corpId !== cachedDebtCorpId) {
		cachedDebtCorpId = corpId;
		invoke('sim_get_debt_instruments', { id: corpId }).then((json) => { cachedDebtInstruments = JSON.parse(json as string); }).catch(() => {});
	}
	return cachedDebtInstruments;
}

export function getCachedResearchState(): ResearchInfo[] { return cachedResearchState; }

export function getCachedBuildableNodes(lon: number, lat: number): BuildOption[] {
	const key = `${lon},${lat}`;
	if (key !== cachedBuildableNodesKey) {
		cachedBuildableNodesKey = key;
		invoke('sim_get_buildable_nodes', { lon, lat }).then((json) => { cachedBuildableNodes = JSON.parse(json as string); }).catch(() => {});
	}
	return cachedBuildableNodes;
}

export function getCachedBuildableEdges(sourceId: number): EdgeTarget[] {
	if (sourceId !== cachedBuildableEdgesId) {
		cachedBuildableEdgesId = sourceId;
		invoke('sim_get_buildable_edges', { id: sourceId }).then((json) => { cachedBuildableEdges = JSON.parse(json as string); }).catch(() => {});
	}
	return cachedBuildableEdges;
}

export function getCachedDamagedNodes(corpId: number): DamagedNode[] {
	if (corpId !== cachedDamagedCorpId) {
		cachedDamagedCorpId = corpId;
		invoke('sim_get_damaged_nodes', { id: corpId }).then((json) => { cachedDamagedNodes = JSON.parse(json as string); }).catch(() => {});
	}
	return cachedDamagedNodes;
}

export function getCachedAuctions(): AuctionInfo[] { return cachedAuctions; }
export function getCachedAcquisitionProposals(): AcquisitionInfo[] { return cachedAcquisitionProposals; }

export function getCachedCovertOps(corpId: number): CovertOpsInfo {
	if (corpId !== cachedCovertOpsCorpId) {
		cachedCovertOpsCorpId = corpId;
		invoke('sim_get_covert_ops', { id: corpId }).then((json) => { cachedCovertOps = JSON.parse(json as string); }).catch(() => {});
	}
	return cachedCovertOps;
}

export function getCachedLobbyingCampaigns(corpId: number): LobbyingInfo[] {
	if (corpId !== cachedLobbyingCorpId) {
		cachedLobbyingCorpId = corpId;
		invoke('sim_get_lobbying_campaigns', { id: corpId }).then((json) => { cachedLobbyingCampaigns = JSON.parse(json as string); }).catch(() => {});
	}
	return cachedLobbyingCampaigns;
}

export function getCachedAchievements(corpId: number): AchievementsInfo {
	if (corpId !== cachedAchievementsCorpId) {
		cachedAchievementsCorpId = corpId;
		invoke('sim_get_achievements', { id: corpId }).then((json) => { cachedAchievements = JSON.parse(json as string); }).catch(() => {});
	}
	return cachedAchievements;
}

export function getCachedVictoryState(): VictoryInfo { return cachedVictoryState; }
export function getCachedSpectrumAuctions(): SpectrumAuction[] { return cachedSpectrumAuctions; }

export function getCachedAvailableSpectrum(regionId: number): AvailableSpectrum[] {
	if (regionId !== cachedAvailableSpectrumRegionId) {
		cachedAvailableSpectrumRegionId = regionId;
		invoke('sim_get_available_spectrum', { id: regionId }).then((json) => { cachedAvailableSpectrum = JSON.parse(json as string); }).catch(() => {});
	}
	return cachedAvailableSpectrum;
}

export function getCachedDisasterForecasts(): DisasterForecast[] { return cachedDisasterForecasts; }
export function getCachedWeatherForecasts(): WeatherForecast[] { return cachedWeatherForecasts; }

export function getCachedRoadPathfind(fromLon: number, fromLat: number, toLon: number, toLat: number): [number, number][] {
	// Road pathfinding is on-demand; fire-and-forget won't help since result is location-specific.
	// Return empty and let the caller retry next frame.
	invoke('sim_road_pathfind', { fromLon, fromLat, toLon, toLat }).catch(() => {});
	return [];
}

export function getCachedRoadFiberRouteCost(fromLon: number, fromLat: number, toLon: number, toLat: number): number {
	invoke('sim_road_fiber_route_cost', { fromLon, fromLat, toLon, toLat }).catch(() => {});
	return 0;
}

export function getCachedRoadSegments(): RoadSegmentInfo[] {
	if (cachedRoadSegments.length === 0) {
		invoke('sim_get_road_segments').then((json) => { cachedRoadSegments = JSON.parse(json as string); }).catch(() => {});
	}
	return cachedRoadSegments;
}

export function getCachedConstellationData(corpId: number): ConstellationData[] {
	if (corpId !== cachedConstellationCorpId) {
		cachedConstellationCorpId = corpId;
		invoke('sim_get_constellation_data', { id: corpId }).then((json) => { cachedConstellationData = JSON.parse(json as string); }).catch(() => {});
	}
	return cachedConstellationData;
}

export function getCachedOrbitalView(): OrbitalSatellite[] { return cachedOrbitalView; }

export function getCachedLaunchSchedule(corpId: number): LaunchPadInfo[] {
	if (corpId !== cachedLaunchCorpId) {
		cachedLaunchCorpId = corpId;
		invoke('sim_get_launch_schedule', { id: corpId }).then((json) => { cachedLaunchSchedule = JSON.parse(json as string); }).catch(() => {});
	}
	return cachedLaunchSchedule;
}

export function getCachedTerminalInventory(corpId: number): TerminalInventory {
	if (corpId !== cachedTerminalCorpId) {
		cachedTerminalCorpId = corpId;
		invoke('sim_get_terminal_inventory', { id: corpId }).then((json) => { cachedTerminalInventory = JSON.parse(json as string); }).catch(() => {});
	}
	return cachedTerminalInventory;
}

export function getCachedDebrisStatus(): OrbitalShellStatus[] { return cachedDebrisStatus; }

export function getCachedAlliances(corpId: number): AllianceInfo[] {
	if (corpId !== cachedAlliancesCorpId) {
		cachedAlliancesCorpId = corpId;
		invoke('sim_get_alliances', { id: corpId }).then((json) => { cachedAlliances = JSON.parse(json as string); }).catch(() => {});
	}
	return cachedAlliances;
}

export function getCachedLawsuits(corpId: number): LawsuitInfo[] {
	if (corpId !== cachedLawsuitsCorpId) {
		cachedLawsuitsCorpId = corpId;
		invoke('sim_get_lawsuits', { id: corpId }).then((json) => { cachedLawsuits = JSON.parse(json as string); }).catch(() => {});
	}
	return cachedLawsuits;
}

export function getCachedStockMarket(corpId: number): StockMarketInfo {
	if (corpId !== cachedStockMarketCorpId) {
		cachedStockMarketCorpId = corpId;
		invoke('sim_get_stock_market', { id: corpId }).then((json) => { cachedStockMarket = JSON.parse(json as string); }).catch(() => {});
	}
	return cachedStockMarket;
}

export function getCachedRegionPricing(corpId: number): RegionPricingInfo[] {
	if (corpId !== cachedRegionPricingCorpId) {
		cachedRegionPricingCorpId = corpId;
		invoke('sim_get_region_pricing', { id: corpId }).then((json) => { cachedRegionPricing = JSON.parse(json as string); }).catch(() => {});
	}
	return cachedRegionPricing;
}

export function getCachedMaintenancePriorities(corpId: number): MaintenancePriorityInfo[] {
	if (corpId !== cachedMaintenancePrioritiesCorpId) {
		cachedMaintenancePrioritiesCorpId = corpId;
		invoke('sim_get_maintenance_priorities', { id: corpId }).then((json) => { cachedMaintenancePrioritiesList = JSON.parse(json as string); }).catch(() => {});
	}
	return cachedMaintenancePrioritiesList;
}

export function getCachedTerrainAt(lon: number, lat: number): string | null {
	// For Tauri, we'll just do a one-shot async fetch for now as terrain lookups are low frequency
	// (only when mouse moves in build mode)
	return null; // Return null until we implement a better sync cache for this
}

export function getCachedNodeMetadata(id: number): any {
	if (!cachedNodeMetadata.has(id)) {
		invoke('sim_get_node_metadata', { id }).then((json) => {
			cachedNodeMetadata.set(id, JSON.parse(json as string));
		}).catch(() => {});
		return {};
	}
	return cachedNodeMetadata.get(id);
}

export function getCachedNodesMetadata(ids: number[]): any[] {
	// Simple implementation for now: fetch each or return empty
	return ids.map(id => getCachedNodeMetadata(id));
}

export function getCachedEdgeMetadata(id: number): any {
	if (!cachedEdgeMetadata.has(id)) {
		invoke('sim_get_edge_metadata', { id }).then((json) => {
			cachedEdgeMetadata.set(id, JSON.parse(json as string));
		}).catch(() => {});
		return {};
	}
	return cachedEdgeMetadata.get(id);
}

export function getCachedWorldPreview(config: Partial<WorldConfig>): WorldPreviewData | null {
	// Fire async fetch; return cached (possibly null on first call)
	invoke('sim_create_world_preview', { configJson: JSON.stringify(config) })
		.then((json) => { cachedWorldPreview = JSON.parse(json as string); })
		.catch(() => {});
	return cachedWorldPreview;
}

export function getCachedWorldGeoJSON(): unknown {
	if (cachedWorldGeoJSON === null) {
		invoke('sim_get_world_geojson').then((json) => { cachedWorldGeoJSON = JSON.parse(json as string); }).catch(() => {});
	}
	return cachedWorldGeoJSON;
}
