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
let cachedInfraNodes: InfraNodesTyped = { count: 0, ids: new Uint32Array(0), owners: new Uint32Array(0), positions: new Float64Array(0), stats: new Float64Array(0), node_types: new Uint32Array(0), network_levels: new Uint32Array(0), construction_flags: new Uint8Array(0) };
let cachedInfraEdges: InfraEdgesTyped = { count: 0, ids: new Uint32Array(0), owners: new Uint32Array(0), endpoints: new Float64Array(0), stats: new Float64Array(0), edge_types: new Uint32Array(0) };
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
		const [infraJson, contractsJson, debtJson, damagedJson, covertJson, lobbyJson, achieveJson, constJson, launchJson, termJson] = await Promise.all([
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
	return cachedPlayerCorpData;
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
