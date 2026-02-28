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

let wasmModule: any = null;
let bridge: any = null;

// Tauri desktop detection: provides native filesystem access for saves.
// The simulation still runs via WASM in the webview for API compatibility.
// Native sim commands exist in desktop/src-tauri for future async adoption.
const isTauri = typeof window !== 'undefined' && '__TAURI__' in window;
let tauriInvoke: ((cmd: string, args?: Record<string, unknown>) => Promise<unknown>) | null = null;

async function initTauri(): Promise<void> {
	if (!isTauri || tauriInvoke) return;
	try {
		// Dynamic import — @tauri-apps/api is only available in Tauri desktop builds.
		// Use string variable to avoid static analysis errors when package isn't installed.
		const tauriModulePath = '@tauri-apps/api/core';
		const tauri = await import(/* @vite-ignore */ tauriModulePath);
		tauriInvoke = tauri.invoke;
	} catch {
		// @tauri-apps/api not available — fall back to WASM-only
	}
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
	// Always init Tauri for native filesystem access (saves)
	if (isTauri) await initTauri();
	// Always load WASM — it runs the simulation in all environments
	if (wasmModule) return;
	const wasm = await import('./pkg/gt_wasm');
	await wasm.default();
	wasmModule = wasm;
}

export function newGame(config?: Partial<WorldConfig>): void {
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

export function tick(): void {
	try {
		bridge?.tick();
	} catch (e) {
		onBridgeError(e, 'tick');
	}
}

export function currentTick(): number {
	try {
		const val = bridge?.current_tick() ?? BigInt(0);
		return Number(val);
	} catch (e) {
		onBridgeError(e, 'currentTick');
		return 0;
	}
}

/** Returns true if a failure event (InsufficientFunds, etc.) was in the result. */
export function processCommand(command: object): boolean {
	try {
		const result = bridge?.process_command(JSON.stringify(command));
		if (result && result.length > 0) {
			try {
				const notifs = JSON.parse(result);
				if (Array.isArray(notifs) && notifs.length > 0) {
					onCommandNotifications(notifs);
					// Check for failure events
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

export function applyBatch(ops: unknown[]): void {
	try {
		bridge?.apply_batch(JSON.stringify(ops));
	} catch (e) {
		onBridgeError(e, 'applyBatch');
	}
}

export function getWorldInfo(): WorldInfo {
	try {
		const json = bridge?.get_world_info() ?? '{}';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getWorldInfo');
		return {} as WorldInfo;
	}
}

export function getCorporationData(corpId: number): CorporationData {
	try {
		const json = bridge?.get_corporation_data(BigInt(corpId)) ?? '{}';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getCorporationData');
		return {} as CorporationData;
	}
}

export function getRegions(): Region[] {
	try {
		const json = bridge?.get_regions() ?? '[]';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getRegions');
		return [];
	}
}

export function isRealEarth(): boolean {
	return bridge?.is_real_earth() ?? false;
}

export function getCities(): City[] {
	try {
		const json = bridge?.get_cities() ?? '[]';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getCities');
		return [];
	}
}

export function getInfrastructureList(corpId: number): InfrastructureList {
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
	try {
		const json = bridge?.get_visible_entities(minX, minY, maxX, maxY) ?? '{"nodes":[],"cities":[]}';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getVisibleEntities');
		return { nodes: [], cities: [] };
	}
}

export function getNotifications(): Notification[] {
	try {
		const json = bridge?.get_notifications() ?? '[]';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getNotifications');
		return [];
	}
}

export function getPlayerCorpId(): number {
	try {
		const val = bridge?.get_player_corp_id() ?? BigInt(0);
		return Number(val);
	} catch (e) {
		onBridgeError(e, 'getPlayerCorpId');
		return 0;
	}
}

export function getAllCorporations(): CorpSummary[] {
	try {
		const json = bridge?.get_all_corporations() ?? '[]';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getAllCorporations');
		return [];
	}
}

export function getCellCoverage(): CellCoverage[] {
	try {
		const json = bridge?.get_cell_coverage() ?? '[]';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getCellCoverage');
		return [];
	}
}

export function getAllInfrastructure(): AllInfrastructure {
	try {
		const json = bridge?.get_all_infrastructure() ?? '{"nodes":[],"edges":[]}';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getAllInfrastructure');
		return { nodes: [], edges: [] };
	}
}

export function getGridCells(): GridCell[] {
	try {
		const json = bridge?.get_grid_cells() ?? '[]';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getGridCells');
		return [];
	}
}

export function getContracts(corpId: number): ContractInfo[] {
	try {
		const json = bridge?.get_contracts(BigInt(corpId)) ?? '[]';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getContracts');
		return [];
	}
}

export function getDebtInstruments(corpId: number): DebtInfo[] {
	try {
		const json = bridge?.get_debt_instruments(BigInt(corpId)) ?? '[]';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getDebtInstruments');
		return [];
	}
}

export function getResearchState(): ResearchInfo[] {
	try {
		const json = bridge?.get_research_state() ?? '[]';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getResearchState');
		return [];
	}
}

export function getBuildableNodes(lon: number, lat: number): BuildOption[] {
	try {
		const json = bridge?.get_buildable_nodes(lon, lat) ?? '[]';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getBuildableNodes');
		return [];
	}
}

export function getBuildableEdges(sourceId: number): EdgeTarget[] {
	try {
		const json = bridge?.get_buildable_edges(BigInt(sourceId)) ?? '[]';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getBuildableEdges');
		return [];
	}
}

export function getDamagedNodes(corpId: number): DamagedNode[] {
	try {
		const json = bridge?.get_damaged_nodes(BigInt(corpId)) ?? '[]';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getDamagedNodes');
		return [];
	}
}

export function saveGame(): string {
	if (!bridge) throw new Error('No game to save');
	try {
		return bridge.save_game();
	} catch (e) {
		onBridgeError(e, 'saveGame');
		throw e;
	}
}

export function loadGame(data: string): void {
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
	try {
		const json = bridge?.get_auctions() ?? '[]';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getAuctions');
		return [];
	}
}

export function getAcquisitionProposals(): AcquisitionInfo[] {
	try {
		const json = bridge?.get_acquisition_proposals() ?? '[]';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getAcquisitionProposals');
		return [];
	}
}

export function getCovertOps(corpId: number): CovertOpsInfo {
	try {
		const json = bridge?.get_covert_ops(BigInt(corpId)) ?? '{}';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getCovertOps');
		return { security_level: 0, active_missions: 0, detection_count: 0 };
	}
}

export function getLobbyingCampaigns(corpId: number): LobbyingInfo[] {
	try {
		const json = bridge?.get_lobbying_campaigns(BigInt(corpId)) ?? '[]';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getLobbyingCampaigns');
		return [];
	}
}

export function getAchievements(corpId: number): AchievementsInfo {
	try {
		const json = bridge?.get_achievements(BigInt(corpId)) ?? '{"unlocked":[],"progress":{}}';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getAchievements');
		return { unlocked: [], progress: {} };
	}
}

export function getVictoryState(): VictoryInfo {
	try {
		const json = bridge?.get_victory_state() ?? '{}';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getVictoryState');
		return {} as VictoryInfo;
	}
}

export function getTrafficFlows(): TrafficFlows {
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
	try {
		if (bridge && typeof bridge.get_infra_nodes_typed === 'function') {
			return bridge.get_infra_nodes_typed() as InfraNodesTyped;
		}
	} catch (e) {
		onBridgeError(e, 'getInfraNodesTyped');
	}
	return { count: 0, ids: EMPTY_U32, owners: EMPTY_U32, positions: EMPTY_F64, stats: EMPTY_F64, node_types: EMPTY_U32, network_levels: EMPTY_U32, construction_flags: EMPTY_U8 };
}

export function getInfraEdgesTyped(): InfraEdgesTyped {
	try {
		if (bridge && typeof bridge.get_infra_edges_typed === 'function') {
			return bridge.get_infra_edges_typed() as InfraEdgesTyped;
		}
	} catch (e) {
		onBridgeError(e, 'getInfraEdgesTyped');
	}
	return { count: 0, ids: EMPTY_U32, owners: EMPTY_U32, endpoints: EMPTY_F64, stats: EMPTY_F64, edge_types: EMPTY_U32 };
}

export function getCorporationsTyped(): CorporationsTyped {
	try {
		if (bridge && typeof bridge.get_corporations_typed === 'function') {
			return bridge.get_corporations_typed() as CorporationsTyped;
		}
	} catch (e) {
		onBridgeError(e, 'getCorporationsTyped');
	}
	return { count: 0, ids: EMPTY_U32, financials: EMPTY_F64, name_offsets: EMPTY_U32, names_packed: EMPTY_U8 };
}

// ── Phase 8: Spectrum & Frequency Management ──────────────────────────

export function getSpectrumLicenses(): SpectrumLicense[] {
	try {
		const json = bridge?.get_spectrum_licenses() ?? '[]';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getSpectrumLicenses');
		return [];
	}
}

export function getSpectrumAuctions(): SpectrumAuction[] {
	try {
		const json = bridge?.get_spectrum_auctions() ?? '[]';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getSpectrumAuctions');
		return [];
	}
}

export function getAvailableSpectrum(regionId: number): AvailableSpectrum[] {
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
	try {
		return bridge?.road_fiber_route_cost(fromLon, fromLat, toLon, toLat) ?? 0;
	} catch (e) {
		onBridgeError(e, 'roadFiberRouteCost');
		return 0;
	}
}

/** Get all road segments for map rendering. */
export function getRoadSegments(): RoadSegmentInfo[] {
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
	try {
		const json = bridge?.get_constellation_data(BigInt(corpId)) ?? '[]';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getConstellationData');
		return [];
	}
}

export function getOrbitalView(): OrbitalSatellite[] {
	try {
		const json = bridge?.get_orbital_view() ?? '[]';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getOrbitalView');
		return [];
	}
}

export function getLaunchSchedule(corpId: number): LaunchPadInfo[] {
	try {
		const json = bridge?.get_launch_schedule(BigInt(corpId)) ?? '[]';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getLaunchSchedule');
		return [];
	}
}

export function getTerminalInventory(corpId: number): TerminalInventory {
	try {
		const json = bridge?.get_terminal_inventory(BigInt(corpId)) ?? '{"factories":[],"warehouses":[]}';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getTerminalInventory');
		return { factories: [], warehouses: [] };
	}
}

export function getDebrisStatus(): OrbitalShellStatus[] {
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
	return bridge !== null;
}
