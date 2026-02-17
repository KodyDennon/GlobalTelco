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
	CorpSummary
} from './types';

let wasmModule: any = null;
let bridge: any = null;

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
	bridge?.tick();
}

export function currentTick(): number {
	const val = bridge?.current_tick() ?? BigInt(0);
	return Number(val);
}

export function processCommand(command: object): void {
	bridge?.process_command(JSON.stringify(command));
}

export function getWorldInfo(): WorldInfo {
	const json = bridge?.get_world_info() ?? '{}';
	return JSON.parse(json);
}

export function getCorporationData(corpId: number): CorporationData {
	const json = bridge?.get_corporation_data(BigInt(corpId)) ?? '{}';
	return JSON.parse(json);
}

export function getRegions(): Region[] {
	const json = bridge?.get_regions() ?? '[]';
	return JSON.parse(json);
}

export function getCities(): City[] {
	const json = bridge?.get_cities() ?? '[]';
	return JSON.parse(json);
}

export function getInfrastructureList(corpId: number): InfrastructureList {
	const json = bridge?.get_infrastructure_list(BigInt(corpId)) ?? '{"nodes":[],"edges":[]}';
	return JSON.parse(json);
}

export function getVisibleEntities(
	minX: number,
	minY: number,
	maxX: number,
	maxY: number
): VisibleEntities {
	const json = bridge?.get_visible_entities(minX, minY, maxX, maxY) ?? '{"nodes":[],"cities":[]}';
	return JSON.parse(json);
}

export function getParcelsInView(
	minX: number,
	minY: number,
	maxX: number,
	maxY: number
): Parcel[] {
	const json = bridge?.get_parcels_in_view(minX, minY, maxX, maxY) ?? '[]';
	return JSON.parse(json);
}

export function getNotifications(): Notification[] {
	const json = bridge?.get_notifications() ?? '[]';
	return JSON.parse(json);
}

export function getPlayerCorpId(): number {
	const val = bridge?.get_player_corp_id() ?? BigInt(0);
	return Number(val);
}

export function getAllCorporations(): CorpSummary[] {
	const json = bridge?.get_all_corporations() ?? '[]';
	return JSON.parse(json);
}

export function getGridCells(): GridCell[] {
	const json = bridge?.get_grid_cells() ?? '[]';
	return JSON.parse(json);
}

export function isInitialized(): boolean {
	return bridge !== null;
}
