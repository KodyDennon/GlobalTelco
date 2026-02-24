export interface WorldInfo {
	tick: number;
	speed: string;
	entity_count: number;
	region_count: number;
	city_count: number;
	corporation_count: number;
	infra_node_count: number;
	infra_edge_count: number;
	player_corp_id: number;
	cell_spacing_km: number;
}

export interface CorporationData {
	id: number;
	name: string;
	is_player: boolean;
	credit_rating: string;
	cash: number;
	revenue_per_tick: number;
	cost_per_tick: number;
	debt: number;
	profit_per_tick: number;
	employee_count: number;
	morale: number;
	infrastructure_count: number;
}

export interface Region {
	id: number;
	name: string;
	center_lat: number;
	center_lon: number;
	population: number;
	gdp: number;
	development: number;
	tax_rate: number;
	regulatory_strictness: number;
	disaster_risk: number;
	cell_count: number;
	city_ids: number[];
	boundary_polygon: [number, number][];
}

export interface City {
	id: number;
	name: string;
	region_id: number;
	cell_index: number;
	cells: number[];
	cell_positions: { index: number; lat: number; lon: number }[];
	population: number;
	growth_rate: number;
	development: number;
	telecom_demand: number;
	infrastructure_satisfaction: number;
	employment_rate: number;
	jobs_available: number;
	birth_rate: number;
	death_rate: number;
	migration_pressure: number;
	x: number;
	y: number;
}

export interface InfraNode {
	id: number;
	node_type: string;
	network_level: string;
	max_throughput: number;
	current_load: number;
	latency_ms: number;
	reliability: number;
	construction_cost: number;
	maintenance_cost: number;
	cell_index: number;
	x: number;
	y: number;
	health: number;
	utilization: number;
	under_construction: boolean;
}

export interface InfraEdge {
	id: number;
	edge_type: string;
	source: number;
	target: number;
	bandwidth: number;
	current_load: number;
	latency_ms: number;
	length_km: number;
	utilization: number;
	src_x: number;
	src_y: number;
	dst_x: number;
	dst_y: number;
	src_cell: number;
	dst_cell: number;
}

export interface InfrastructureList {
	nodes: InfraNode[];
	edges: InfraEdge[];
}

export interface VisibleEntities {
	nodes: VisibleNode[];
	cities: VisibleCity[];
}

export interface VisibleNode {
	id: number;
	type: 'node';
	node_type: string;
	owner: number;
	x: number;
	y: number;
	health: number;
	utilization: number;
	under_construction: boolean;
}

export interface VisibleCity {
	id: number;
	type: 'city';
	name: string;
	population: number;
	x: number;
	y: number;
}

// GameEvent is a serde-serialized Rust enum: { "VariantName": { ...fields } }
// e.g. { "DisasterStruck": { region: 5, severity: 0.7, disaster_type: "Earthquake", affected_nodes: 3 } }
export type GameEvent = Record<string, Record<string, unknown>>;

export interface Notification {
	tick: number;
	event: GameEvent;
}

// Helper to get the variant name (first key) of a GameEvent
export function eventType(event: GameEvent): string {
	return Object.keys(event)[0] ?? 'Unknown';
}

// Helper to get the fields of a GameEvent
export function eventData(event: GameEvent): Record<string, unknown> {
	const key = Object.keys(event)[0];
	return key ? (event[key] as Record<string, unknown>) : {};
}

export interface GridCell {
	index: number;
	lat: number;
	lon: number;
	terrain: string;
	neighbors: number[];
}

export interface CellCoverage {
	cell_index: number;
	lat: number;
	lon: number;
	signal_strength: number;
	bandwidth: number;
	node_count: number;
	best_signal: number;
	dominant_owner: number | null;
}

export interface AllInfraNode {
	id: number;
	node_type: string;
	network_level: string;
	max_throughput: number;
	current_load: number;
	latency_ms: number;
	reliability: number;
	cell_index: number;
	owner: number;
	owner_name: string;
	x: number;
	y: number;
	health: number;
	utilization: number;
	under_construction: boolean;
}

export interface AllInfraEdge {
	id: number;
	edge_type: string;
	source: number;
	target: number;
	bandwidth: number;
	current_load: number;
	latency_ms: number;
	length_km: number;
	utilization: number;
	owner: number;
	owner_name: string;
	src_x: number;
	src_y: number;
	dst_x: number;
	dst_y: number;
	src_cell: number;
	dst_cell: number;
}

export interface AllInfrastructure {
	nodes: AllInfraNode[];
	edges: AllInfraEdge[];
}

export interface ContractInfo {
	id: number;
	contract_type: string;
	from: number;
	to: number;
	from_name: string;
	to_name: string;
	capacity: number;
	price_per_tick: number;
	start_tick: number;
	end_tick: number;
	status: string;
	penalty: number;
}

export interface DebtInfo {
	id: number;
	principal: number;
	interest_rate: number;
	remaining_ticks: number;
	payment_per_tick: number;
	is_paid_off: boolean;
}

export interface ResearchInfo {
	id: number;
	category: string;
	category_name: string;
	name: string;
	description: string;
	progress: number;
	total_cost: number;
	progress_pct: number;
	researcher: number | null;
	researcher_name: string | null;
	completed: boolean;
	patent_status: string;
	patent_owner: number | null;
	patent_owner_name: string | null;
	license_price: number;
	prerequisites: string[];
	throughput_bonus: number;
	cost_reduction: number;
	reliability_bonus: number;
}

export interface BuildOption {
	label: string;
	node_type: string;
	network_level: string;
	cost: number;
	build_ticks: number;
	affordable: boolean;
}

export interface EdgeTarget {
	target_id: number;
	target_type: string;
	x: number;
	y: number;
	distance_km: number;
	cost: number;
	affordable: boolean;
}

export interface DamagedNode {
	id: number;
	node_type: string;
	health: number;
	repair_cost: number;
	emergency_cost: number;
	x: number;
	y: number;
}

export interface CorpSummary {
	id: number;
	name: string;
	is_player: boolean;
	credit_rating: string;
	cash: number;
	revenue: number;
	cost: number;
}

// Phase 10 types
export interface AuctionInfo {
	id: number;
	seller: number;
	seller_name: string;
	asset_count: number;
	bid_count: number;
	highest_bid: number;
	highest_bidder: number;
	start_tick: number;
	end_tick: number;
	status: string;
}

export interface AcquisitionInfo {
	id: number;
	acquirer: number;
	acquirer_name: string;
	target: number;
	target_name: string;
	offer: number;
	status: string;
	tick: number;
}

export interface CovertOpsInfo {
	security_level: number;
	active_missions: number;
	detection_count: number;
}

export interface LobbyingInfo {
	id: number;
	region: number;
	region_name: string;
	policy: string;
	budget_spent: number;
	budget_total: number;
	influence: number;
	threshold: number;
	active: boolean;
}

export interface AchievementsInfo {
	unlocked: string[];
	progress: Record<string, number>;
}

export interface VictoryInfo {
	domination_score: number;
	tech_score: number;
	wealth_score: number;
	infrastructure_score: number;
	total_score: number;
	victory_type: string | null;
}

// World configuration types for new game setup

export interface WorldConfig {
	seed: number;
	starting_era: string;
	difficulty: string;
	map_size: string;
	ai_corporations: number;
	use_real_earth: boolean;
	corp_name: string;
	continent_count: number;
	ocean_percentage: number;
	terrain_roughness: number;
	climate_variation: number;
	city_density: number;
}

export type WorldPreset = 'real_earth' | 'pangaea' | 'archipelago' | 'continents' | 'random';

export interface WorldPreviewData {
	cells: Array<{ lat: number; lon: number; terrain: string }>;
	continentCount: number;
	oceanPercent: number;
	cityCount: number;
	regionCount: number;
}

// Traffic flow types
export interface EdgeFlow {
	id: number;
	traffic: number;
	bandwidth: number;
	utilization: number;
	health: number;
	edge_type: string;
	owner: number;
	src_x: number;
	src_y: number;
	dst_x: number;
	dst_y: number;
}

export interface NodeFlow {
	id: number;
	traffic: number;
	max_throughput: number;
	utilization: number;
	node_type: string;
	owner: number;
	x: number;
	y: number;
}

export interface CongestedEdge {
	id: number;
	utilization: number;
	edge_type: string;
	owner: number;
}

export interface TrafficFlows {
	edge_flows: EdgeFlow[];
	node_flows: NodeFlow[];
	total_served: number;
	total_dropped: number;
	total_demand: number;
	player_served: number;
	player_dropped: number;
	top_congested: CongestedEdge[];
}
