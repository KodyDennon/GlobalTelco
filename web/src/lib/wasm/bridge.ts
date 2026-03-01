import type {
	WorldInfo,
	CorporationData,
	Region,
	City,
	InfrastructureList,
	VisibleEntities,
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
	AvailableSpectrum
} from './types';

import * as tauriBridge from './tauriBridge';

let wasmModule: any = null;
let bridge: any = null;

// Tauri desktop detection
const isTauri = typeof window !== 'undefined' && '__TAURI__' in window;
let tauriInvoke: ((cmd: string, args?: Record<string, unknown>) => Promise<unknown>) | null = null;

// When true, all sim queries route to native Rust via Tauri IPC (no WASM loaded).
let useNativeSim = false;

async function initTauri(): Promise<void> {
	if (!isTauri || tauriInvoke) return;
	try {
		const tauriModulePath = '@tauri-apps/api/core';
		const tauri = await import(/* @vite-ignore */ tauriModulePath);
		tauriInvoke = tauri.invoke;
	} catch {
		// @tauri-apps/api not available — fall back to WASM-only
	}
}

/** Whether the simulation is running natively via Tauri (no WASM). */
export function isNativeSim(): boolean {
	return useNativeSim;
}

type ErrorHandler = (error: string, context: string) => void;
let errorHandler: ErrorHandler | null = null;

export function setErrorHandler(handler: ErrorHandler): void {
	errorHandler = handler;
}

function onBridgeError(error: unknown, context: string): void {
	const message = error instanceof Error ? error.message : String(error);
	console.error(`[bridge:${context}]`, message);
	if (errorHandler) {
		errorHandler(message, context);
	}
}

export async function initWasm(): Promise<void> {
	if (isTauri) {
		await initTauri();
		// Desktop: use native Rust simulation — no WASM needed
		await tauriBridge.init();
		useNativeSim = true;
		return;
	}
	// Browser: load WASM as before
	if (wasmModule) return;
	const wasm = await import('./pkg/gt_wasm');
	await wasm.default();
	wasmModule = wasm;
}

export async function newGame(config?: Partial<WorldConfig>): Promise<void> {
	if (useNativeSim) {
		await tauriBridge.newGame(config);
		return;
	}
	if (!wasmModule) throw new Error('WASM not initialized');
	try {
		if (config) {
			const configJson = JSON.stringify(config);
			bridge = wasmModule.WasmBridge.new_game(configJson);
		} else {
			bridge = new wasmModule.WasmBridge();
		}
	} catch (e) {
		onBridgeError(e, 'newGame');
		throw e;
	}
}

export async function tick(): Promise<void> {
	if (useNativeSim) {
		await tauriBridge.tick();
		return;
	}
	try {
		bridge?.tick();
	} catch (e) {
		onBridgeError(e, 'tick');
	}
}

export function currentTick(): number {
	if (useNativeSim) return tauriBridge.getCachedWorldInfo().tick ?? 0;
	try {
		const val = bridge?.current_tick() ?? BigInt(0);
		return Number(val);
	} catch (e) {
		onBridgeError(e, 'currentTick');
		return 0;
	}
}

/** Returns true if a failure event (InsufficientFunds, etc.) was in the result. */
export async function processCommand(command: object): Promise<boolean> {
	if (useNativeSim) {
		try {
			const result = await tauriBridge.processCommand(JSON.stringify(command));
			if (result && result.length > 0) {
				try {
					const notifs = JSON.parse(result);
					if (Array.isArray(notifs) && notifs.length > 0) {
						onCommandNotifications(notifs);
						const hasFailure = notifs.some((n: any) => {
							const evt = n.event;
							if (!evt || typeof evt !== 'object') return false;
							return 'InsufficientFunds' in evt || 'CommandFailed' in evt || 'InvalidPlacement' in evt;
						});
						return hasFailure;
					}
				} catch { /* ignore */ }
			}
			return false;
		} catch (e) {
			onBridgeError(e, 'processCommand');
			return true;
		}
	}
	try {
		const result = bridge?.process_command(JSON.stringify(command));
		if (result && result.length > 0) {
			try {
				const notifs = JSON.parse(result);
				if (Array.isArray(notifs) && notifs.length > 0) {
					onCommandNotifications(notifs);
					const hasFailure = notifs.some((n: any) => {
						const evt = n.event;
						if (!evt || typeof evt !== 'object') return false;
						return 'InsufficientFunds' in evt || 'CommandFailed' in evt || 'InvalidPlacement' in evt;
					});
					return hasFailure;
				}
			} catch {
				// Not valid JSON, ignore
			}
		}
		return false;
	} catch (e) {
		onBridgeError(e, 'processCommand');
		return true;
	}
}


type CommandNotificationHandler = (notifications: Notification[]) => void;
let commandNotificationHandler: CommandNotificationHandler | null = null;

export function setCommandNotificationHandler(handler: CommandNotificationHandler): void {
	commandNotificationHandler = handler;
}

function onCommandNotifications(notifs: Notification[]): void {
	if (commandNotificationHandler) {
		commandNotificationHandler(notifs);
	}
}

export async function applyBatch(ops: unknown[]): Promise<void> {
	if (useNativeSim) {
		await tauriBridge.applyBatch(JSON.stringify(ops));
		return;
	}
	try {
		bridge?.apply_batch(JSON.stringify(ops));
	} catch (e) {
		onBridgeError(e, 'applyBatch');
	}
}

export function getWorldInfo(): WorldInfo {
	if (useNativeSim) return tauriBridge.getCachedWorldInfo();
	try {
		const json = bridge?.get_world_info() ?? '{}';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getWorldInfo');
		return {} as WorldInfo;
	}
}

export function getCorporationData(corpId: number): CorporationData {
	if (useNativeSim) return tauriBridge.getCachedCorporationData(corpId);
	try {
		const json = bridge?.get_corporation_data(BigInt(corpId)) ?? '{}';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getCorporationData');
		return {} as CorporationData;
	}
}

export function getRegions(): Region[] {
	if (useNativeSim) return tauriBridge.getCachedRegions();
	try {
		const json = bridge?.get_regions() ?? '[]';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getRegions');
		return [];
	}
}

export function isRealEarth(): boolean {
	if (useNativeSim) return tauriBridge.getCachedIsRealEarth();
	return bridge?.is_real_earth() ?? false;
}

export function getCities(): City[] {
	if (useNativeSim) return tauriBridge.getCachedCities();
	try {
		const json = bridge?.get_cities() ?? '[]';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getCities');
		return [];
	}
}

export function getInfrastructureList(corpId: number): InfrastructureList {
	if (useNativeSim) return tauriBridge.getCachedInfrastructureList(corpId);
	try {
		const json = bridge?.get_infrastructure_list(BigInt(corpId)) ?? '{"nodes":[],"edges":[]}';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getInfrastructureList');
		return { nodes: [], edges: [] };
	}
}

export function getVisibleEntities(
	minX: number,
	minY: number,
	maxX: number,
	maxY: number
): VisibleEntities {
	if (useNativeSim) return { nodes: [], cities: [] };
	try {
		const json = bridge?.get_visible_entities(minX, minY, maxX, maxY) ?? '{"nodes":[],"cities":[]}';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getVisibleEntities');
		return { nodes: [], cities: [] };
	}
}

export function getNotifications(): Notification[] {
	if (useNativeSim) return tauriBridge.getCachedNotifications();
	try {
		const json = bridge?.get_notifications() ?? '[]';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getNotifications');
		return [];
	}
}

export function getPlayerCorpId(): number {
	if (useNativeSim) return tauriBridge.getCachedPlayerCorpId();
	try {
		const val = bridge?.get_player_corp_id() ?? BigInt(0);
		return Number(val);
	} catch (e) {
		onBridgeError(e, 'getPlayerCorpId');
		return 0;
	}
}

export function getAllCorporations(): CorpSummary[] {
	if (useNativeSim) return tauriBridge.getCachedAllCorporations();
	try {
		const json = bridge?.get_all_corporations() ?? '[]';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getAllCorporations');
		return [];
	}
}

export function getCellCoverage(): CellCoverage[] {
	if (useNativeSim) return tauriBridge.getCachedCellCoverage();
	try {
		const json = bridge?.get_cell_coverage() ?? '[]';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getCellCoverage');
		return [];
	}
}

export function getAllInfrastructure(): AllInfrastructure {
	if (useNativeSim) return tauriBridge.getCachedAllInfrastructure();
	try {
		const json = bridge?.get_all_infrastructure() ?? '{"nodes":[],"edges":[]}';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getAllInfrastructure');
		return { nodes: [], edges: [] };
	}
}

export function getGridCells(): GridCell[] {
	if (useNativeSim) return tauriBridge.getCachedGridCells();
	try {
		const json = bridge?.get_grid_cells() ?? '[]';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getGridCells');
		return [];
	}
}

export function getContracts(corpId: number): ContractInfo[] {
	if (useNativeSim) return tauriBridge.getCachedContracts(corpId);
	try {
		const json = bridge?.get_contracts(BigInt(corpId)) ?? '[]';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getContracts');
		return [];
	}
}

export function getDebtInstruments(corpId: number): DebtInfo[] {
	if (useNativeSim) return tauriBridge.getCachedDebtInstruments(corpId);
	try {
		const json = bridge?.get_debt_instruments(BigInt(corpId)) ?? '[]';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getDebtInstruments');
		return [];
	}
}

export function getResearchState(): ResearchInfo[] {
	if (useNativeSim) return tauriBridge.getCachedResearchState();
	try {
		const json = bridge?.get_research_state() ?? '[]';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getResearchState');
		return [];
	}
}

export function getBuildableNodes(lon: number, lat: number): BuildOption[] {
	if (useNativeSim) return tauriBridge.getCachedBuildableNodes(lon, lat);
	try {
		const json = bridge?.get_buildable_nodes(lon, lat) ?? '[]';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getBuildableNodes');
		return [];
	}
}

export function getBuildableEdges(sourceId: number): EdgeTarget[] {
	if (useNativeSim) return tauriBridge.getCachedBuildableEdges(sourceId);
	try {
		const json = bridge?.get_buildable_edges(BigInt(sourceId)) ?? '[]';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getBuildableEdges');
		return [];
	}
}

export function getDamagedNodes(corpId: number): DamagedNode[] {
	if (useNativeSim) return tauriBridge.getCachedDamagedNodes(corpId);
	try {
		const json = bridge?.get_damaged_nodes(BigInt(corpId)) ?? '[]';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getDamagedNodes');
		return [];
	}
}

export async function saveGame(): Promise<string> {
	if (useNativeSim) return tauriBridge.saveGame();
	if (!bridge) throw new Error('No game to save');
	try {
		return bridge.save_game();
	} catch (e) {
		onBridgeError(e, 'saveGame');
		throw e;
	}
}

export async function loadGame(data: string): Promise<void> {
	if (useNativeSim) {
		await tauriBridge.loadGame(data);
		return;
	}
	if (!wasmModule) throw new Error('WASM not initialized');
	try {
		bridge = wasmModule.WasmBridge.load_game(data);
	} catch (e) {
		onBridgeError(e, 'loadGame');
		throw e;
	}
}

// Phase 10 queries

export function getAuctions(): AuctionInfo[] {
	if (useNativeSim) return tauriBridge.getCachedAuctions();
	try {
		const json = bridge?.get_auctions() ?? '[]';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getAuctions');
		return [];
	}
}

export function getAcquisitionProposals(): AcquisitionInfo[] {
	if (useNativeSim) return tauriBridge.getCachedAcquisitionProposals();
	try {
		const json = bridge?.get_acquisition_proposals() ?? '[]';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getAcquisitionProposals');
		return [];
	}
}

export function getCovertOps(corpId: number): CovertOpsInfo {
	if (useNativeSim) return tauriBridge.getCachedCovertOps(corpId);
	try {
		const json = bridge?.get_covert_ops(BigInt(corpId)) ?? '{}';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getCovertOps');
		return { security_level: 0, active_missions: 0, detection_count: 0 };
	}
}

export function getLobbyingCampaigns(corpId: number): LobbyingInfo[] {
	if (useNativeSim) return tauriBridge.getCachedLobbyingCampaigns(corpId);
	try {
		const json = bridge?.get_lobbying_campaigns(BigInt(corpId)) ?? '[]';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getLobbyingCampaigns');
		return [];
	}
}

export function getAchievements(corpId: number): AchievementsInfo {
	if (useNativeSim) return tauriBridge.getCachedAchievements(corpId);
	try {
		const json = bridge?.get_achievements(BigInt(corpId)) ?? '{"unlocked":[],"progress":{}}';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getAchievements');
		return { unlocked: [], progress: {} };
	}
}

export function getVictoryState(): VictoryInfo {
	if (useNativeSim) return tauriBridge.getCachedVictoryState();
	try {
		const json = bridge?.get_victory_state() ?? '{}';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getVictoryState');
		return {} as VictoryInfo;
	}
}

export function getTrafficFlows(): TrafficFlows {
	if (useNativeSim) return tauriBridge.getCachedTrafficFlows();
	try {
		const json = bridge?.get_traffic_flows() ?? '{"edge_flows":[],"node_flows":[],"total_served":0,"total_dropped":0,"total_demand":0,"player_served":0,"player_dropped":0,"top_congested":[]}';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getTrafficFlows');
		return { edge_flows: [], node_flows: [], total_served: 0, total_dropped: 0, total_demand: 0, player_served: 0, player_dropped: 0, top_congested: [] };
	}
}

// World preview and GeoJSON generation for procgen worlds

export function createWorldPreview(config: Partial<WorldConfig>): WorldPreviewData | null {
	if (useNativeSim) return tauriBridge.getCachedWorldPreview(config);
	if (!wasmModule) return null;
	try {
		if (typeof wasmModule.WasmBridge.create_world_preview === 'function') {
			const configJson = JSON.stringify(config);
			const json = wasmModule.WasmBridge.create_world_preview(configJson);
			return JSON.parse(json);
		}
	} catch (e) {
		onBridgeError(e, 'createWorldPreview');
	}
	return null;
}

export function getWorldGeoJSON(): any {
	if (useNativeSim) return tauriBridge.getCachedWorldGeoJSON();
	try {
		if (bridge && typeof bridge.get_world_geojson === 'function') {
			const json = bridge.get_world_geojson();
			return JSON.parse(json);
		}
	} catch (e) {
		onBridgeError(e, 'getWorldGeoJSON');
	}
	return null;
}

// ── Typed Array Queries (Hot-Path Rendering) ──────────────────────────

const EMPTY_F64 = new Float64Array(0);
const EMPTY_U32 = new Uint32Array(0);
const EMPTY_U8 = new Uint8Array(0);

export function getInfraNodesTyped(): InfraNodesTyped {
	if (useNativeSim) return tauriBridge.getCachedInfraNodesTyped();
	try {
		if (bridge && typeof bridge.get_infra_nodes_typed === 'function') {
			const arr = bridge.get_infra_nodes_typed();
			if (arr && arr.length >= 8) {
				return {
					count: arr[0] as number,
					ids: arr[1] as Uint32Array,
					owners: arr[2] as Uint32Array,
					positions: arr[3] as Float64Array,
					stats: arr[4] as Float64Array,
					node_types: arr[5] as Uint32Array,
					network_levels: arr[6] as Uint32Array,
					construction_flags: arr[7] as Uint8Array,
				};
			}
		}
	} catch (e) {
		onBridgeError(e, 'getInfraNodesTyped');
	}
	return { count: 0, ids: EMPTY_U32, owners: EMPTY_U32, positions: EMPTY_F64, stats: EMPTY_F64, node_types: EMPTY_U32, network_levels: EMPTY_U32, construction_flags: EMPTY_U8 };
}

export function getInfraEdgesTyped(): InfraEdgesTyped {
	if (useNativeSim) return tauriBridge.getCachedInfraEdgesTyped();
	try {
		if (bridge && typeof bridge.get_infra_edges_typed === 'function') {
			const arr = bridge.get_infra_edges_typed();
			if (arr && arr.length >= 6) {
				return {
					count: arr[0] as number,
					ids: arr[1] as Uint32Array,
					owners: arr[2] as Uint32Array,
					endpoints: arr[3] as Float64Array,
					stats: arr[4] as Float64Array,
					edge_types: arr[5] as Uint32Array,
				};
			}
		}
	} catch (e) {
		onBridgeError(e, 'getInfraEdgesTyped');
	}
	return { count: 0, ids: EMPTY_U32, owners: EMPTY_U32, endpoints: EMPTY_F64, stats: EMPTY_F64, edge_types: EMPTY_U32 };
}

export function getCorporationsTyped(): CorporationsTyped {
	if (useNativeSim) return tauriBridge.getCachedCorporationsTyped();
	try {
		if (bridge && typeof bridge.get_corporations_typed === 'function') {
			const arr = bridge.get_corporations_typed();
			if (arr && arr.length >= 5) {
				return {
					count: arr[0] as number,
					ids: arr[1] as Uint32Array,
					financials: arr[2] as Float64Array,
					name_offsets: arr[3] as Uint32Array,
					names_packed: arr[4] as Uint8Array,
				};
			}
		}
	} catch (e) {
		onBridgeError(e, 'getCorporationsTyped');
	}
	return { count: 0, ids: EMPTY_U32, financials: EMPTY_F64, name_offsets: EMPTY_U32, names_packed: EMPTY_U8 };
}

// ── Phase 8: Spectrum & Frequency Management ──────────────────────────

export function getSpectrumLicenses(): SpectrumLicense[] {
	if (useNativeSim) return tauriBridge.getCachedSpectrumLicenses();
	try {
		const json = bridge?.get_spectrum_licenses() ?? '[]';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getSpectrumLicenses');
		return [];
	}
}

export function getSpectrumAuctions(): SpectrumAuction[] {
	if (useNativeSim) return tauriBridge.getCachedSpectrumAuctions();
	try {
		const json = bridge?.get_spectrum_auctions() ?? '[]';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getSpectrumAuctions');
		return [];
	}
}

export function getAvailableSpectrum(regionId: number): AvailableSpectrum[] {
	if (useNativeSim) return tauriBridge.getCachedAvailableSpectrum(regionId);
	try {
		const json = bridge?.get_available_spectrum(BigInt(regionId)) ?? '[]';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getAvailableSpectrum');
		return [];
	}
}

// ── Phase 9: Disaster Forecasts ──────────────────────────────────────

export interface DisasterForecast {
	region_id: number;
	region_name: string;
	predicted_tick: number;
	probability: number;
	disaster_type: string;
}

export function getDisasterForecasts(): DisasterForecast[] {
	if (useNativeSim) return tauriBridge.getCachedDisasterForecasts();
	try {
		const json = bridge?.get_disaster_forecasts() ?? '[]';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getDisasterForecasts');
		return [];
	}
}

// ── Phase 7.5: Weather Forecasts ─────────────────────────────────────

export interface WeatherForecast {
	region_id: number;
	region_name: string;
	predicted_type: string;
	probability: number;
	eta_ticks: number;
	predicted_severity: number;
}

export function getWeatherForecasts(): WeatherForecast[] {
	if (useNativeSim) return tauriBridge.getCachedWeatherForecasts();
	try {
		const json = bridge?.get_weather_forecasts() ?? '[]';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getWeatherForecasts');
		return [];
	}
}

// ── Road Network Queries (Fiber Auto-Routing) ────────────────────────

export interface RoadSegmentInfo {
	id: number;
	from: [number, number];
	to: [number, number];
	road_class: string;
	length_km: number;
	region_id: number;
}

/** A* pathfinding along the road network. Returns waypoints as [lon, lat] pairs. */
export function roadPathfind(fromLon: number, fromLat: number, toLon: number, toLat: number): [number, number][] {
	if (useNativeSim) return tauriBridge.getCachedRoadPathfind(fromLon, fromLat, toLon, toLat);
	try {
		const json = bridge?.road_pathfind(fromLon, fromLat, toLon, toLat) ?? '[]';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'roadPathfind');
		return [];
	}
}

/** Cost of routing fiber along roads between two points (weighted km). */
export function roadFiberRouteCost(fromLon: number, fromLat: number, toLon: number, toLat: number): number {
	if (useNativeSim) return tauriBridge.getCachedRoadFiberRouteCost(fromLon, fromLat, toLon, toLat);
	try {
		return bridge?.road_fiber_route_cost(fromLon, fromLat, toLon, toLat) ?? 0;
	} catch (e) {
		onBridgeError(e, 'roadFiberRouteCost');
		return 0;
	}
}

/** Get all road segments for map rendering. */
export function getRoadSegments(): RoadSegmentInfo[] {
	if (useNativeSim) return tauriBridge.getCachedRoadSegments();
	try {
		const json = bridge?.get_road_segments() ?? '[]';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getRoadSegments');
		return [];
	}
}

// ── Satellite Queries ────────────────────────────────────────────────

export interface ConstellationData {
	id: number;
	name: string;
	orbit_type: string;
	target_altitude_km: number;
	target_inclination_deg: number;
	num_planes: number;
	sats_per_plane: number;
	total_target: number;
	operational_count: number;
	satellite_ids: number[];
}

export interface OrbitalSatellite {
	id: number;
	owner: number;
	lon: number;
	lat: number;
	altitude_km: number;
	orbit_type: string;
	status: string;
	fuel_remaining: number;
	fuel_capacity: number;
	constellation_id: number;
}

export interface LaunchPadInfo {
	launch_pad_id: number;
	cooldown_remaining: number;
	reusable: boolean;
	queue: { rocket_type: string; satellite_count: number }[];
}

export interface TerminalInventory {
	factories: { factory_id: number; tier: string; produced_stored: number; production_progress: number }[];
	warehouses: { warehouse_id: number; region_id: number; terminal_inventory: number; distribution_rate: number }[];
}

export interface OrbitalShellStatus {
	index: number;
	min_altitude_km: number;
	max_altitude_km: number;
	debris_count: number;
	collision_probability: number;
	kessler_threshold: number;
	cascade_active: boolean;
}

export function getConstellationData(corpId: number): ConstellationData[] {
	if (useNativeSim) return tauriBridge.getCachedConstellationData(corpId);
	try {
		const json = bridge?.get_constellation_data(BigInt(corpId)) ?? '[]';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getConstellationData');
		return [];
	}
}

export function getOrbitalView(): OrbitalSatellite[] {
	if (useNativeSim) return tauriBridge.getCachedOrbitalView();
	try {
		const json = bridge?.get_orbital_view() ?? '[]';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getOrbitalView');
		return [];
	}
}

export function getLaunchSchedule(corpId: number): LaunchPadInfo[] {
	if (useNativeSim) return tauriBridge.getCachedLaunchSchedule(corpId);
	try {
		const json = bridge?.get_launch_schedule(BigInt(corpId)) ?? '[]';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getLaunchSchedule');
		return [];
	}
}

export function getTerminalInventory(corpId: number): TerminalInventory {
	if (useNativeSim) return tauriBridge.getCachedTerminalInventory(corpId);
	try {
		const json = bridge?.get_terminal_inventory(BigInt(corpId)) ?? '{"factories":[],"warehouses":[]}';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getTerminalInventory');
		return { factories: [], warehouses: [] };
	}
}

export function getDebrisStatus(): OrbitalShellStatus[] {
	if (useNativeSim) return tauriBridge.getCachedDebrisStatus();
	try {
		const json = bridge?.get_debris_status() ?? '[]';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getDebrisStatus');
		return [];
	}
}

export interface SatelliteArrays {
	ids: Uint32Array;
	owners: Uint32Array;
	positions: Float64Array;
	altitudes: Float64Array;
	orbitTypes: Uint32Array;
	statuses: Uint32Array;
	fuelLevels: Float64Array;
}

export function getSatelliteArrays(): SatelliteArrays | null {
	if (useNativeSim) return tauriBridge.getCachedSatelliteArrays();
	try {
		const result = bridge?.get_satellite_arrays();
		if (!result || result.length < 7) return null;
		return {
			ids: result[0] as Uint32Array,
			owners: result[1] as Uint32Array,
			positions: result[2] as Float64Array,
			altitudes: result[3] as Float64Array,
			orbitTypes: result[4] as Uint32Array,
			statuses: result[5] as Uint32Array,
			fuelLevels: result[6] as Float64Array,
		};
	} catch (e) {
		onBridgeError(e, 'getSatelliteArrays');
		return null;
	}
}

// ── Alliance, Legal, Stock Market, Pricing, Maintenance Queries ───────

export interface AllianceInfo {
	id: number;
	name: string;
	member_corp_ids: number[];
	member_names: string[];
	trust_scores: Record<string, number>;
	revenue_share_pct: number;
	formed_tick: number;
}

export function getAlliances(corpId: number): AllianceInfo[] {
	if (useNativeSim) return tauriBridge.getCachedAlliances(corpId);
	try {
		const json = bridge?.get_alliances(BigInt(corpId)) ?? '[]';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getAlliances');
		return [];
	}
}

export interface LawsuitInfo {
	id: number;
	plaintiff: number;
	plaintiff_name: string;
	defendant: number;
	defendant_name: string;
	lawsuit_type: string;
	damages_claimed: number;
	filing_cost: number;
	filed_tick: number;
	resolution_tick: number | null;
	status: string;
	outcome: string | null;
}

export function getLawsuits(corpId: number): LawsuitInfo[] {
	if (useNativeSim) return tauriBridge.getCachedLawsuits(corpId);
	try {
		const json = bridge?.get_lawsuits(BigInt(corpId)) ?? '[]';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getLawsuits');
		return [];
	}
}

export interface StockMarketInfo {
	public: boolean;
	total_shares: number;
	share_price: number;
	dividends_per_share: number;
	ipo_tick: number | null;
	shareholder_satisfaction: number;
	board_votes: { proposal: string; votes_for: number; votes_against: number; deadline_tick: number }[];
}

export function getStockMarket(corpId: number): StockMarketInfo {
	if (useNativeSim) return tauriBridge.getCachedStockMarket(corpId);
	try {
		const json = bridge?.get_stock_market(BigInt(corpId)) ?? '{}';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getStockMarket');
		return { public: false, total_shares: 0, share_price: 0, dividends_per_share: 0, ipo_tick: null, shareholder_satisfaction: 0, board_votes: [] };
	}
}

export interface RegionPricingInfo {
	region_id: number;
	region_name: string;
	tier: string;
	price_per_unit: number;
}

export function getRegionPricing(corpId: number): RegionPricingInfo[] {
	if (useNativeSim) return tauriBridge.getCachedRegionPricing(corpId);
	try {
		const json = bridge?.get_region_pricing(BigInt(corpId)) ?? '[]';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getRegionPricing');
		return [];
	}
}

export interface MaintenancePriorityInfo {
	node_id: number;
	priority: string;
	auto_repair: boolean;
}

export function getMaintenancePriorities(corpId: number): MaintenancePriorityInfo[] {
	if (useNativeSim) return tauriBridge.getCachedMaintenancePriorities(corpId);
	try {
		const json = bridge?.get_maintenance_priorities(BigInt(corpId)) ?? '[]';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getMaintenancePriorities');
		return [];
	}
}

// ── Tauri Native Filesystem ───────────────────────────────────────────

export async function saveGameNative(slot: number, data: string): Promise<string | null> {
	if (!tauriInvoke) return null;
	try {
		return (await tauriInvoke('save_game_native', { slot, data })) as string;
	} catch (e) {
		onBridgeError(e, 'saveGameNative');
		return null;
	}
}

export async function loadGameNative(slot: number): Promise<string | null> {
	if (!tauriInvoke) return null;
	try {
		return (await tauriInvoke('load_game_native', { slot })) as string | null;
	} catch (e) {
		onBridgeError(e, 'loadGameNative');
		return null;
	}
}

export interface NativeSaveEntry {
	name: string;
	path: string;
	size: number;
	modified: number;
}

export async function listSavesNative(): Promise<NativeSaveEntry[]> {
	if (!tauriInvoke) return [];
	try {
		return (await tauriInvoke('list_saves')) as NativeSaveEntry[];
	} catch (e) {
		onBridgeError(e, 'listSavesNative');
		return [];
	}
}

export function isTauriDesktop(): boolean {
	return isTauri && tauriInvoke !== null;
}

export function isInitialized(): boolean {
	return useNativeSim || bridge !== null;
}
