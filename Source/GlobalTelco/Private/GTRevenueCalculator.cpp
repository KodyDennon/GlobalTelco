#include "GTRevenueCalculator.h"
#include "GTCorporationManager.h"
#include "GTCorporation.h"
#include "GTNetworkGraph.h"
#include "GTNetworkNode.h"
#include "GTNetworkEdge.h"
#include "GTRegionalEconomy.h"
#include "GTLandParcelSystem.h"
#include "Engine/World.h"

void UGTRevenueCalculator::Initialize(FSubsystemCollectionBase& Collection)
{
	Super::Initialize(Collection);
}

void UGTRevenueCalculator::ProcessRevenueTick(float MaintenanceCostMultiplier, float DemandGrowthMultiplier)
{
	UWorld* World = GetWorld();
	if (!World)
	{
		return;
	}

	UGTCorporationManager* CorpManager = World->GetSubsystem<UGTCorporationManager>();
	UGTNetworkGraph* Graph = World->GetSubsystem<UGTNetworkGraph>();
	UGTRegionalEconomy* Economy = World->GetSubsystem<UGTRegionalEconomy>();
	UGTLandParcelSystem* ParcelSystem = World->GetSubsystem<UGTLandParcelSystem>();

	if (!CorpManager || !Graph || !Economy || !ParcelSystem)
	{
		return;
	}

	// Update connectivity scores before revenue calculation.
	UpdateRegionalConnectivity();

	LastBreakdowns.Empty();

	const TArray<UGTCorporation*> AllCorps = CorpManager->GetAllCorporations();
	for (UGTCorporation* Corp : AllCorps)
	{
		if (!Corp)
		{
			continue;
		}

		FGTCorpRevenueBreakdown Breakdown;
		Breakdown.CorporationId = Corp->CorporationId;

		// Calculate revenue.
		Breakdown.BandwidthRevenue = CalculateBandwidthRevenue(
			Corp, Graph, Economy, ParcelSystem, DemandGrowthMultiplier);

		// Calculate costs.
		Breakdown.NodeMaintenanceCost = CalculateNodeMaintenance(Corp, Graph, MaintenanceCostMultiplier);
		Breakdown.EdgeMaintenanceCost = CalculateEdgeMaintenance(Corp, Graph, MaintenanceCostMultiplier);

		// Note: Interest is computed by the corporation's own ProcessEconomicTick,
		// not here. The revenue calculator handles revenue and maintenance only.

		// Set the corporation's income statement with revenue and maintenance.
		// Interest and net income application happen in UGTCorporation::ProcessEconomicTick.
		Corp->LastTickIncome = FGTIncomeStatement();
		Corp->LastTickIncome.Revenue = Breakdown.TotalRevenue();
		Corp->LastTickIncome.MaintenanceCosts = Breakdown.NodeMaintenanceCost + Breakdown.EdgeMaintenanceCost;

		LastBreakdowns.Add(Corp->CorporationId, Breakdown);
	}
}

double UGTRevenueCalculator::CalculateBandwidthRevenue(UGTCorporation* Corp,
	UGTNetworkGraph* Graph, UGTRegionalEconomy* Economy,
	UGTLandParcelSystem* ParcelSystem, float DemandGrowthMultiplier) const
{
	if (!Corp || !Graph || !Economy || !ParcelSystem)
	{
		return 0.0;
	}

	double TotalRevenue = 0.0;

	// Track which regions this corp serves and with how much capacity.
	TMap<int32, float> RegionCapacity;

	// Track average utilization per region for congestion penalty.
	TMap<int32, float> RegionUtilizationSum;
	TMap<int32, int32> RegionNodeCount;

	for (int32 NodeId : Corp->OwnedNodeIds)
	{
		AGTNetworkNode* Node = Graph->GetNode(NodeId);
		if (!Node || !Node->IsOperational())
		{
			continue;
		}

		// Find which region this node's parcel belongs to.
		const int32 ParcelCount = ParcelSystem->GetParcelCount();
		for (int32 p = 0; p < ParcelCount; ++p)
		{
			FGTLandParcel Parcel = ParcelSystem->GetParcel(p);
			if (Parcel.OwnerCorporationId == Corp->CorporationId && Parcel.RegionId >= 0)
			{
				float& Cap = RegionCapacity.FindOrAdd(Parcel.RegionId);
				Cap += Node->Attributes.Capacity;

				// Track utilization for congestion penalty.
				RegionUtilizationSum.FindOrAdd(Parcel.RegionId) += Node->Attributes.CurrentUtilization;
				RegionNodeCount.FindOrAdd(Parcel.RegionId)++;
				break;
			}
		}
	}

	// Revenue from each served region proportional to capacity vs demand.
	for (const auto& Pair : RegionCapacity)
	{
		const int32 RegionId = Pair.Key;
		const float Capacity = Pair.Value;

		FGTRegionalEconomyData RegionData = Economy->GetRegionData(RegionId);
		if (RegionData.RegionId < 0)
		{
			continue;
		}

		const double Demand = RegionData.CurrentDemand * DemandGrowthMultiplier;
		if (Demand <= 0.0)
		{
			continue;
		}

		// Satisfaction ratio: how much of the demand can this corp serve.
		// Clamped to 1.0 — can't earn more than the demand.
		const double SatisfactionRatio = FMath::Min(static_cast<double>(Capacity) / Demand, 1.0);

		// Base revenue per unit of demand served (scales with GDP proxy).
		const double RevenuePerDemandUnit = RegionData.GDPProxy * 0.0001;

		// Revenue = demand served * price per unit.
		const double DemandServed = Demand * SatisfactionRatio;
		double RegionRevenue = DemandServed * RevenuePerDemandUnit;

		// Congestion penalty: high utilization degrades service quality, reducing revenue.
		// Below 70% utilization: no penalty. 70-100%: linear penalty up to 30%. Above 100%: capped at 50% penalty.
		const float* UtilSum = RegionUtilizationSum.Find(RegionId);
		const int32* NodeCnt = RegionNodeCount.Find(RegionId);
		if (UtilSum && NodeCnt && *NodeCnt > 0)
		{
			const float AvgUtil = *UtilSum / static_cast<float>(*NodeCnt);
			if (AvgUtil > 0.7f)
			{
				const float OverloadFraction = FMath::Min((AvgUtil - 0.7f) / 0.3f, 1.0f);
				const float CongestionPenalty = 0.5f * OverloadFraction; // Max 50% penalty.
				RegionRevenue *= (1.0 - CongestionPenalty);
			}
		}

		TotalRevenue += RegionRevenue;
	}

	return TotalRevenue;
}

double UGTRevenueCalculator::CalculateNodeMaintenance(UGTCorporation* Corp,
	UGTNetworkGraph* Graph, float CostMultiplier) const
{
	if (!Corp || !Graph)
	{
		return 0.0;
	}

	double TotalCost = 0.0;

	for (int32 NodeId : Corp->OwnedNodeIds)
	{
		AGTNetworkNode* Node = Graph->GetNode(NodeId);
		if (!Node)
		{
			continue;
		}

		// Only operational and degraded nodes cost maintenance.
		if (Node->Status == EGTInfrastructureStatus::Operational ||
			Node->Status == EGTInfrastructureStatus::Degraded)
		{
			float Cost = Node->Attributes.MaintenanceCostPerTick;

			// Terrain multiplier for maintenance.
			switch (Node->Terrain)
			{
			case EGTTerrainType::Mountainous:
				Cost *= 1.5f;
				break;
			case EGTTerrainType::OceanShallow:
			case EGTTerrainType::OceanDeep:
				Cost *= 2.0f;
				break;
			case EGTTerrainType::Desert:
				Cost *= 1.3f;
				break;
			case EGTTerrainType::Coastal:
				Cost *= 1.2f;
				break;
			case EGTTerrainType::Tundra:
				Cost *= 1.8f;
				break;
			case EGTTerrainType::Frozen:
				Cost *= 2.5f;
				break;
			default:
				break;
			}

			// Degraded infrastructure costs more to maintain.
			if (Node->Status == EGTInfrastructureStatus::Degraded)
			{
				Cost *= 1.5f;
			}

			TotalCost += Cost * CostMultiplier;
		}
	}

	return TotalCost;
}

double UGTRevenueCalculator::CalculateEdgeMaintenance(UGTCorporation* Corp,
	UGTNetworkGraph* Graph, float CostMultiplier) const
{
	if (!Corp || !Graph)
	{
		return 0.0;
	}

	double TotalCost = 0.0;

	for (int32 EdgeId : Corp->OwnedEdgeIds)
	{
		UGTNetworkEdge* Edge = Graph->GetEdge(EdgeId);
		if (!Edge)
		{
			continue;
		}

		if (Edge->Status == EGTInfrastructureStatus::Operational ||
			Edge->Status == EGTInfrastructureStatus::Degraded)
		{
			float Cost = Edge->Attributes.MaintenanceCostPerTick;

			// Terrain risk multiplier affects maintenance cost.
			Cost *= Edge->Attributes.TerrainRiskMultiplier;

			// Degraded edges cost more.
			if (Edge->Status == EGTInfrastructureStatus::Degraded)
			{
				Cost *= 1.5f;
			}

			TotalCost += Cost * CostMultiplier;
		}
	}

	return TotalCost;
}

void UGTRevenueCalculator::UpdateRegionalConnectivity()
{
	UWorld* World = GetWorld();
	if (!World)
	{
		return;
	}

	UGTNetworkGraph* Graph = World->GetSubsystem<UGTNetworkGraph>();
	UGTRegionalEconomy* Economy = World->GetSubsystem<UGTRegionalEconomy>();
	UGTLandParcelSystem* ParcelSystem = World->GetSubsystem<UGTLandParcelSystem>();

	if (!Graph || !Economy || !ParcelSystem)
	{
		return;
	}

	// Count operational nodes per region.
	TMap<int32, int32> RegionNodeCount;
	const int32 ParcelCount = ParcelSystem->GetParcelCount();

	for (int32 i = 0; i < ParcelCount; ++i)
	{
		FGTLandParcel Parcel = ParcelSystem->GetParcel(i);
		if (Parcel.RegionId >= 0 && Parcel.OwnerCorporationId >= 0)
		{
			// Check if there's infrastructure on this parcel.
			// Approximate by checking if the owning corp has any nodes.
			int32& Count = RegionNodeCount.FindOrAdd(Parcel.RegionId);
			Count++;
		}
	}

	// Connectivity score: logarithmic scaling — a few nodes help a lot,
	// each additional node helps less. Score = log(1 + nodes) / log(1 + max).
	const float MaxNodes = 50.0f;  // Saturation point.
	for (const auto& Pair : RegionNodeCount)
	{
		const float Score = FMath::Clamp(
			FMath::Loge(1.0f + static_cast<float>(Pair.Value)) / FMath::Loge(1.0f + MaxNodes),
			0.0f, 1.0f);
		Economy->SetRegionConnectivity(Pair.Key, Score);
	}
}

FGTCorpRevenueBreakdown UGTRevenueCalculator::GetLastBreakdown(int32 CorporationId) const
{
	const FGTCorpRevenueBreakdown* Found = LastBreakdowns.Find(CorporationId);
	return Found ? *Found : FGTCorpRevenueBreakdown();
}

TArray<FGTCorpRevenueBreakdown> UGTRevenueCalculator::GetAllBreakdowns() const
{
	TArray<FGTCorpRevenueBreakdown> Result;
	LastBreakdowns.GenerateValueArray(Result);
	return Result;
}
