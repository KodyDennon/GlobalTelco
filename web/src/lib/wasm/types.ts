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
}

export interface City {
	id: number;
	name: string;
	region_id: number;
	cell_index: number;
	population: number;
	growth_rate: number;
	development: number;
	telecom_demand: number;
	infrastructure_satisfaction: number;
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

export interface Parcel {
	id: number;
	cell_index: number;
	terrain: string;
	elevation: number;
	zoning: string;
	cost_modifier: number;
	x: number;
	y: number;
}

export interface Notification {
	tick: number;
	event: string;
}

export interface GridCell {
	index: number;
	lat: number;
	lon: number;
	terrain: string;
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
