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

export interface CorpSummary {
	id: number;
	name: string;
	is_player: boolean;
	credit_rating: string;
	cash: number;
	revenue: number;
	cost: number;
}
