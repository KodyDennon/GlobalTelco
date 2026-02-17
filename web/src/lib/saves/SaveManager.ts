import { openDB, type IDBPDatabase } from 'idb';

const DB_NAME = 'globaltelco';
const DB_VERSION = 1;
const STORE_NAME = 'saves';
const SAVE_VERSION = 1;

export interface SaveMetadata {
	slot: string;
	name: string;
	timestamp: number;
	tick: number;
	corpName: string;
	difficulty: string;
	era: string;
	version: number;
}

interface SaveRecord {
	slot: string;
	metadata: SaveMetadata;
	data: string; // JSON game state from WASM
}

let db: IDBPDatabase | null = null;

async function getDB(): Promise<IDBPDatabase> {
	if (db) return db;
	db = await openDB(DB_NAME, DB_VERSION, {
		upgrade(database) {
			if (!database.objectStoreNames.contains(STORE_NAME)) {
				database.createObjectStore(STORE_NAME, { keyPath: 'slot' });
			}
		}
	});
	return db;
}

export async function saveToSlot(
	slot: string,
	name: string,
	gameData: string,
	tick: number,
	corpName: string,
	difficulty: string,
	era: string
): Promise<void> {
	const database = await getDB();
	const record: SaveRecord = {
		slot,
		metadata: {
			slot,
			name,
			timestamp: Date.now(),
			tick,
			corpName,
			difficulty,
			era,
			version: SAVE_VERSION
		},
		data: gameData
	};
	await database.put(STORE_NAME, record);
}

export async function loadFromSlot(slot: string): Promise<{ metadata: SaveMetadata; data: string } | null> {
	const database = await getDB();
	const record = (await database.get(STORE_NAME, slot)) as SaveRecord | undefined;
	if (!record) return null;
	if (record.metadata.version !== SAVE_VERSION) {
		throw new Error(`Incompatible save version: ${record.metadata.version} (expected ${SAVE_VERSION})`);
	}
	return { metadata: record.metadata, data: record.data };
}

export async function listSaves(): Promise<SaveMetadata[]> {
	const database = await getDB();
	const records = (await database.getAll(STORE_NAME)) as SaveRecord[];
	return records
		.map((r) => r.metadata)
		.sort((a, b) => b.timestamp - a.timestamp);
}

export async function deleteSave(slot: string): Promise<void> {
	const database = await getDB();
	await database.delete(STORE_NAME, slot);
}

export async function renameSave(slot: string, newName: string): Promise<void> {
	const database = await getDB();
	const record = (await database.get(STORE_NAME, slot)) as SaveRecord | undefined;
	if (!record) return;
	record.metadata.name = newName;
	await database.put(STORE_NAME, record);
}

// Auto-save slot rotation: AutoSave1, AutoSave2, AutoSave3
let autoSaveIndex = 0;

export function getNextAutoSaveSlot(): string {
	const slot = `AutoSave${(autoSaveIndex % 3) + 1}`;
	autoSaveIndex++;
	return slot;
}

export const QUICK_SAVE_SLOT = 'QuickSave';
