import type {
	WorldInfo,
	CorporationData,
	Region,
	City,
	InfrastructureList,
	VisibleEntities,
	Parcel,
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
	TrafficFlows
} from './types';

let wasmModule: any = null;
let bridge: any = null;

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
	if (wasmModule) return;
	const wasm = await import('./pkg/gt_wasm');
	await wasm.default();
	wasmModule = wasm;
}

export function newGame(config?: {
	seed?: number;
	starting_era?: string;
	difficulty?: string;
	map_size?: string;
	ai_corporations?: number;
	use_real_earth?: boolean;
}): void {
	if (!wasmModule) throw new Error('WASM not initialized');
	if (config) {
		const configJson = JSON.stringify(config);
		bridge = wasmModule.WasmBridge.new_game(configJson);
	} else {
		bridge = new wasmModule.WasmBridge();
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

export function processCommand(command: object): void {
	try {
		bridge?.process_command(JSON.stringify(command));
	} catch (e) {
		onBridgeError(e, 'processCommand');
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

export function getParcelsInView(
	minX: number,
	minY: number,
	maxX: number,
	maxY: number
): Parcel[] {
	try {
		const json = bridge?.get_parcels_in_view(minX, minY, maxX, maxY) ?? '[]';
		return JSON.parse(json);
	} catch (e) {
		onBridgeError(e, 'getParcelsInView');
		return [];
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

export function getBuildableNodes(parcelId: number): BuildOption[] {
	try {
		const json = bridge?.get_buildable_nodes(BigInt(parcelId)) ?? '[]';
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
	return bridge.save_game();
}

export function loadGame(data: string): void {
	if (!wasmModule) throw new Error('WASM not initialized');
	bridge = wasmModule.WasmBridge.load_game(data);
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

export function isInitialized(): boolean {
	return bridge !== null;
}
