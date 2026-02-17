#include "GTAITasks.h"
#include "GTAICorporationController.h"
#include "GTCorporation.h"
#include "GTCorporationManager.h"
#include "GTNetworkGraph.h"
#include "GTNetworkNode.h"
#include "GTNetworkEdge.h"
#include "GTLandParcelSystem.h"
#include "GTRegionalEconomy.h"
#include "GTAllianceManager.h"
#include "GTSimulationSubsystem.h"
#include "GTEventQueue.h"
#include "GTSimulationTypes.h"
#include "GTInfrastructureTypes.h"
#include "GTEconomyTypes.h"
#include "GTMultiplayerTypes.h"
#include "BehaviorTree/BlackboardComponent.h"
#include "AIController.h"

// ===================================================================
// Helper: Get the AI controller from a BT component
// ===================================================================
static AGTAICorporationController* GetAIController(UBehaviorTreeComponent& OwnerComp)
{
	return Cast<AGTAICorporationController>(OwnerComp.GetAIOwner());
}

// ===================================================================
// BT Service: Update World State
// ===================================================================

UBTService_GTUpdateWorldState::UBTService_GTUpdateWorldState()
{
	NodeName = TEXT("Update AI World State");
	Interval = 0.5f; // Check twice per second.
	RandomDeviation = 0.1f;
}

void UBTService_GTUpdateWorldState::TickNode(UBehaviorTreeComponent& OwnerComp, uint8* NodeMemory, float DeltaSeconds)
{
	Super::TickNode(OwnerComp, NodeMemory, DeltaSeconds);

	AGTAICorporationController* AIController = GetAIController(OwnerComp);
	if (!AIController)
	{
		return;
	}

	UGTCorporation* Corp = AIController->GetCorporation();
	if (!Corp)
	{
		return;
	}

	const FGTAIArchetypeData& Archetype = AIController->GetArchetype();
	const float Aggressiveness = AIController->GetAggressivenessMultiplier();

	// Read financial state.
	const double Cash = Corp->BalanceSheet.CashOnHand;
	const double TotalAssets = Corp->BalanceSheet.TotalAssets;
	const double Equity = Corp->BalanceSheet.GetEquity();
	const double DebtToEquity = (Equity > 0.0) ? (Corp->TotalDebt / Equity) : 999.0;
	const double CashRatio = (TotalAssets > 0.0) ? (Cash / TotalAssets) : 1.0;

	// Count owned assets.
	const int32 OwnedNodeCount = Corp->OwnedNodeIds.Num();
	const int32 OwnedEdgeCount = Corp->OwnedEdgeIds.Num();

	// Determine strategy based on financial health and archetype.
	EGTAIStrategy Strategy;

	if (Corp->IsInsolvent() || Cash < 50000.0)
	{
		// Critical financial stress — survival mode.
		Strategy = EGTAIStrategy::Survive;
	}
	else if (CashRatio < Archetype.MinCashReserveRatio * 0.5)
	{
		// Low cash reserves — consolidate and recover.
		Strategy = EGTAIStrategy::Consolidate;
	}
	else
	{
		// Normal operation — choose based on archetype weights and game state.
		// Score each strategy and pick the highest.
		float ExpandScore = Archetype.ExpansionWeight * Aggressiveness;
		float ConsolidateScore = Archetype.ConsolidationWeight;
		float CompeteScore = Archetype.AggressionWeight * Aggressiveness;

		// Modify scores based on current state.
		// More nodes = less expansion pressure, more consolidation value.
		if (OwnedNodeCount > 10)
		{
			ExpandScore *= 0.7f;
			ConsolidateScore *= 1.3f;
		}

		// High debt = less expansion, more consolidation.
		if (DebtToEquity > Archetype.MaxDebtToEquityRatio * 0.7)
		{
			ExpandScore *= 0.5f;
			ConsolidateScore *= 1.5f;
		}

		// High cash = more expansion opportunity.
		if (CashRatio > Archetype.MinCashReserveRatio * 2.0)
		{
			ExpandScore *= 1.4f;
		}

		if (ExpandScore >= ConsolidateScore && ExpandScore >= CompeteScore)
		{
			Strategy = EGTAIStrategy::Expand;
		}
		else if (CompeteScore >= ConsolidateScore)
		{
			Strategy = EGTAIStrategy::Compete;
		}
		else
		{
			Strategy = EGTAIStrategy::Consolidate;
		}
	}

	// Store state on the controller for BT nodes to read.
	AIController->CachedCash = Cash;
	AIController->CachedDebtToEquity = DebtToEquity;
	AIController->CachedOwnedNodeCount = OwnedNodeCount;
	AIController->CachedOwnedEdgeCount = OwnedEdgeCount;
	AIController->CachedStrategy = Strategy;
}

// ===================================================================
// BT Task: Acquire Land
// ===================================================================

UBTTask_GTAcquireLand::UBTTask_GTAcquireLand()
{
	NodeName = TEXT("Acquire Land");
}

double UBTTask_GTAcquireLand::ScoreParcel(const FGTLandParcel& Parcel, UGTCorporation* Corp,
	UGTRegionalEconomy* Economy, UGTLandParcelSystem* ParcelSystem,
	const FGTAIArchetypeData& Archetype, float AggressivenessMultiplier) const
{
	// Skip ocean parcels — can't build standard infrastructure there.
	if (Parcel.Terrain == EGTTerrainType::OceanDeep || Parcel.Terrain == EGTTerrainType::OceanShallow)
	{
		return -1.0;
	}

	// Skip protected/military zones.
	if (Parcel.Zoning == EGTZoningCategory::Protected || Parcel.Zoning == EGTZoningCategory::Military)
	{
		return -1.0;
	}

	double Score = 100.0;

	// Regional demand bonus.
	if (Parcel.RegionId >= 0 && Economy)
	{
		const FGTRegionalEconomyData RegionData = Economy->GetRegionData(Parcel.RegionId);
		Score += RegionData.CurrentDemand * 0.01;
		Score += RegionData.Population * 0.000001;
		Score += RegionData.TechAdoptionIndex * 50.0;
	}

	// Terrain preferences.
	switch (Parcel.Terrain)
	{
	case EGTTerrainType::Urban:
		Score += 80.0 * Archetype.ExpansionWeight;
		break;
	case EGTTerrainType::Suburban:
		Score += 50.0 * Archetype.ExpansionWeight;
		break;
	case EGTTerrainType::Rural:
		Score += 30.0 * Archetype.FinancialPrudence; // Budget operators like cheap rural.
		break;
	case EGTTerrainType::Coastal:
		Score += 40.0;
		break;
	case EGTTerrainType::Mountainous:
		Score -= 20.0; // Higher cost.
		break;
	case EGTTerrainType::Desert:
		Score += 10.0 * Archetype.RiskTolerance;
		break;
	case EGTTerrainType::Tundra:
		Score -= 10.0;
		break;
	case EGTTerrainType::Frozen:
		Score -= 40.0; // Very expensive and hostile.
		break;
	default:
		break;
	}

	// Cost penalty — high labor cost means expensive to build.
	Score -= Parcel.LaborCostMultiplier * 20.0;

	// Disaster risk penalty (modulated by risk tolerance).
	Score -= Parcel.DisasterRisk * 100.0 * (1.0 - Archetype.RiskTolerance);

	// Political stability bonus.
	Score += Parcel.PoliticalStability * 30.0;

	// Proximity bonus: check if we already own parcels in the same region.
	if (Parcel.RegionId >= 0)
	{
		const TArray<int32> RegionParcels = ParcelSystem->GetParcelsInRegion(Parcel.RegionId);
		for (int32 PId : RegionParcels)
		{
			const FGTLandParcel Neighbor = ParcelSystem->GetParcel(PId);
			if (Neighbor.OwnerCorporationId == Corp->CorporationId)
			{
				Score += 25.0; // Adjacency bonus.
				break;
			}
		}
	}

	// Aggressiveness multiplier amplifies all scores.
	Score *= (0.5 + AggressivenessMultiplier * 0.5);

	return Score;
}

EBTNodeResult::Type UBTTask_GTAcquireLand::ExecuteTask(UBehaviorTreeComponent& OwnerComp, uint8* NodeMemory)
{
	AGTAICorporationController* AICtrl = GetAIController(OwnerComp);
	if (!AICtrl)
	{
		return EBTNodeResult::Failed;
	}

	UGTCorporation* Corp = AICtrl->GetCorporation();
	UGTLandParcelSystem* ParcelSystem = AICtrl->GetParcelSystem();
	UGTRegionalEconomy* Economy = AICtrl->GetRegionalEconomy();

	if (!Corp || !ParcelSystem || !Economy)
	{
		return EBTNodeResult::Failed;
	}

	const FGTAIArchetypeData& Archetype = AICtrl->GetArchetype();
	const float Aggressiveness = AICtrl->GetAggressivenessMultiplier();

	// Estimate parcel purchase cost — use a base cost scaled by terrain.
	auto EstimateParcelCost = [](const FGTLandParcel& P) -> double
	{
		double BaseCost = 500000.0; // Base $500k per parcel.
		switch (P.Terrain)
		{
		case EGTTerrainType::Urban: return BaseCost * 3.0;
		case EGTTerrainType::Suburban: return BaseCost * 1.5;
		case EGTTerrainType::Coastal: return BaseCost * 2.0;
		case EGTTerrainType::Mountainous: return BaseCost * 0.8;
		case EGTTerrainType::Desert: return BaseCost * 0.4;
		case EGTTerrainType::Tundra: return BaseCost * 0.3;
		case EGTTerrainType::Frozen: return BaseCost * 0.2;
		default: return BaseCost;
		}
	};

	// Don't spend below our minimum reserve.
	const double MinReserve = Corp->BalanceSheet.TotalAssets * Archetype.MinCashReserveRatio;
	const double AvailableCash = Corp->BalanceSheet.CashOnHand - MinReserve;

	if (AvailableCash < 200000.0)
	{
		return EBTNodeResult::Failed; // Can't afford anything.
	}

	// Score all government-owned parcels and pick the best.
	int32 BestParcelId = -1;
	double BestScore = -1.0;
	double BestCost = 0.0;
	const int32 ParcelCount = ParcelSystem->GetParcelCount();

	// Sample a subset for performance (up to 500 parcels).
	const int32 SampleStep = FMath::Max(1, ParcelCount / 500);

	for (int32 i = 0; i < ParcelCount; i += SampleStep)
	{
		const FGTLandParcel Parcel = ParcelSystem->GetParcel(i);

		// Only consider government-owned parcels.
		if (Parcel.OwnershipType != EGTParcelOwnership::Government)
		{
			continue;
		}

		const double Cost = EstimateParcelCost(Parcel);
		if (Cost > AvailableCash)
		{
			continue;
		}

		const double Score = ScoreParcel(Parcel, Corp, Economy, ParcelSystem, Archetype, Aggressiveness);
		if (Score > BestScore)
		{
			BestScore = Score;
			BestParcelId = i;
			BestCost = Cost;
		}
	}

	if (BestParcelId < 0)
	{
		return EBTNodeResult::Failed; // No suitable parcels found.
	}

	// Purchase the parcel.
	if (ParcelSystem->PurchaseParcel(BestParcelId, Corp->CorporationId, BestCost))
	{
		Corp->BalanceSheet.CashOnHand -= BestCost;
		Corp->BalanceSheet.TotalAssets += BestCost * 0.8; // Land value is ~80% of purchase price.

		// Fire event.
		UGTSimulationSubsystem* Sim = AICtrl->GetWorld()->GetSubsystem<UGTSimulationSubsystem>();
		if (Sim && Sim->GetEventQueue())
		{
			FGTSimulationEvent Event;
			Event.EventType = EGTSimulationEventType::LandPurchased;
			Event.Tick = Sim->GetCurrentTick();
			Event.Timestamp = AICtrl->GetWorld()->GetTimeSeconds();
			Event.SourceEntityId = Corp->CorporationId;
			Event.TargetEntityId = BestParcelId;
			Event.Payload.Add(FName(TEXT("Cost")), FString::Printf(TEXT("%.0f"), BestCost));
			Sim->GetEventQueue()->Enqueue(Event);
		}

		return EBTNodeResult::Succeeded;
	}

	return EBTNodeResult::Failed;
}

// ===================================================================
// BT Task: Build Node
// ===================================================================

UBTTask_GTBuildNode::UBTTask_GTBuildNode()
{
	NodeName = TEXT("Build Infrastructure Node");
}

EBTNodeResult::Type UBTTask_GTBuildNode::ExecuteTask(UBehaviorTreeComponent& OwnerComp, uint8* NodeMemory)
{
	AGTAICorporationController* AICtrl = GetAIController(OwnerComp);
	if (!AICtrl)
	{
		return EBTNodeResult::Failed;
	}

	UGTCorporation* Corp = AICtrl->GetCorporation();
	UGTLandParcelSystem* ParcelSystem = AICtrl->GetParcelSystem();
	UGTNetworkGraph* NetworkGraph = AICtrl->GetNetworkGraph();
	UGTRegionalEconomy* Economy = AICtrl->GetRegionalEconomy();
	const FGTAIArchetypeData& Archetype = AICtrl->GetArchetype();

	if (!Corp || !ParcelSystem || !NetworkGraph)
	{
		return EBTNodeResult::Failed;
	}

	// Node construction costs by type.
	struct FNodeCostEntry
	{
		EGTNodeType Type;
		double BaseCost;
		float BaseCapacity;
		float BaseLatency;
	};

	static const FNodeCostEntry NodeCosts[] = {
		{ EGTNodeType::AccessTower, 200000.0, 50.0f, 2.0f },
		{ EGTNodeType::FiberDistributionHub, 500000.0, 200.0f, 1.0f },
		{ EGTNodeType::DataCenter, 2000000.0, 1000.0f, 0.5f },
		{ EGTNodeType::InternetExchangePoint, 5000000.0, 5000.0f, 0.2f },
		{ EGTNodeType::SubseaLandingStation, 3000000.0, 2000.0f, 0.5f },
		{ EGTNodeType::SatelliteGroundStation, 4000000.0, 500.0f, 30.0f },
	};

	// Don't spend below minimum reserve.
	const double MinReserve = Corp->BalanceSheet.TotalAssets * Archetype.MinCashReserveRatio;
	const double AvailableCash = Corp->BalanceSheet.CashOnHand - MinReserve;

	if (AvailableCash < 200000.0)
	{
		return EBTNodeResult::Failed;
	}

	// Find owned parcels that don't already have infrastructure.
	// Build a set of parcel IDs that already have nodes.
	TSet<int32> ParcelsWithNodes;
	for (int32 NodeId : Corp->OwnedNodeIds)
	{
		// We'd need parcel info from the node — for now track by counting.
		// Simplified: just limit total nodes based on owned parcels.
	}

	// Find a parcel we own but haven't built on yet.
	int32 TargetParcelId = -1;
	FGTLandParcel TargetParcel;
	const int32 ParcelCount = ParcelSystem->GetParcelCount();

	// Score parcels we own for building priority.
	double BestBuildScore = -1.0;

	for (int32 i = 0; i < ParcelCount; ++i)
	{
		const FGTLandParcel Parcel = ParcelSystem->GetParcel(i);

		if (Parcel.OwnerCorporationId != Corp->CorporationId)
		{
			continue;
		}

		// Skip ocean (subsea nodes need special handling).
		if (Parcel.Terrain == EGTTerrainType::OceanDeep || Parcel.Terrain == EGTTerrainType::OceanShallow)
		{
			continue;
		}

		// Score based on regional demand and lack of existing infra.
		double Score = 50.0;
		if (Parcel.RegionId >= 0 && Economy)
		{
			const FGTRegionalEconomyData Region = Economy->GetRegionData(Parcel.RegionId);
			Score += Region.CurrentDemand * 0.01;
			Score += Region.TechAdoptionIndex * 30.0;
		}

		if (Score > BestBuildScore)
		{
			BestBuildScore = Score;
			TargetParcelId = i;
			TargetParcel = Parcel;
		}
	}

	if (TargetParcelId < 0)
	{
		return EBTNodeResult::Failed;
	}

	// Choose node type based on budget and archetype.
	EGTNodeType ChosenType = EGTNodeType::AccessTower;
	double ChosenCost = 200000.0;
	float ChosenCapacity = 50.0f;
	float ChosenLatency = 2.0f;

	// Tech innovators prefer higher-tier nodes.
	// Budget operators prefer cheapest nodes.
	// Aggressors prefer capacity.
	for (const FNodeCostEntry& Entry : NodeCosts)
	{
		if (Entry.BaseCost > AvailableCash)
		{
			continue;
		}

		double TypeScore = Entry.BaseCapacity / Entry.BaseCost * 100000.0; // Value per dollar.

		// Tech innovators prefer high-capacity premium nodes.
		if (Archetype.TechInvestmentWeight > 0.7f)
		{
			TypeScore = Entry.BaseCapacity * 0.001; // Prefer raw capacity.
		}

		// Budget operators prefer cheapest option.
		if (Archetype.FinancialPrudence > 0.7f)
		{
			TypeScore = 1.0 / (Entry.BaseCost + 1.0) * 1000000.0;
		}

		// Coastal terrain → subsea landing station is appropriate.
		if (TargetParcel.Terrain == EGTTerrainType::Coastal && Entry.Type == EGTNodeType::SubseaLandingStation)
		{
			TypeScore *= 2.0;
		}

		// Urban terrain → data center or IXP.
		if (TargetParcel.Terrain == EGTTerrainType::Urban &&
			(Entry.Type == EGTNodeType::DataCenter || Entry.Type == EGTNodeType::InternetExchangePoint))
		{
			TypeScore *= 1.5;
		}

		// For now, just pick the best affordable option.
		if (TypeScore > 0.0)
		{
			ChosenType = Entry.Type;
			ChosenCost = Entry.BaseCost * TargetParcel.LaborCostMultiplier;
			ChosenCapacity = Entry.BaseCapacity;
			ChosenLatency = Entry.BaseLatency;

			if (ChosenCost <= AvailableCash)
			{
				break; // Take the first affordable option matching our strategy.
			}
		}
	}

	if (ChosenCost > AvailableCash)
	{
		return EBTNodeResult::Failed;
	}

	// Spawn the node actor.
	UWorld* World = AICtrl->GetWorld();
	if (!World)
	{
		return EBTNodeResult::Failed;
	}

	FVector NodeWorldPos = FVector::ZeroVector; // Will be positioned at parcel coordinates.
	FActorSpawnParameters SpawnParams;
	SpawnParams.SpawnCollisionHandlingOverride = ESpawnActorCollisionHandlingMethod::AlwaysSpawn;

	// We need a concrete node class — since AGTNetworkNode is abstract,
	// we register directly in the graph. The node placement is data-driven,
	// not requiring a physical actor in single-player mode.
	// Create a minimal node entry in the network graph.

	// Set up node attributes with construction timer.
	FGTNodeAttributes NodeAttrs;
	NodeAttrs.Capacity = ChosenCapacity;
	NodeAttrs.LatencyMs = ChosenLatency;
	NodeAttrs.Reliability = 0.98f;
	NodeAttrs.MaintenanceCostPerTick = static_cast<float>(ChosenCost * 0.001); // 0.1% of cost per tick.
	NodeAttrs.DisasterRiskMultiplier = TargetParcel.DisasterRisk;
	NodeAttrs.ConstructionCost = static_cast<float>(ChosenCost);
	NodeAttrs.bUnderConstruction = true;

	// Construction time: base ticks scaled by terrain difficulty.
	int32 ConstructionTicks = NodeAttrs.BaseConstructionTicks;
	switch (TargetParcel.Terrain)
	{
	case EGTTerrainType::Mountainous: ConstructionTicks = static_cast<int32>(ConstructionTicks * 2.0f); break;
	case EGTTerrainType::Desert: ConstructionTicks = static_cast<int32>(ConstructionTicks * 1.5f); break;
	case EGTTerrainType::Coastal: ConstructionTicks = static_cast<int32>(ConstructionTicks * 1.3f); break;
	case EGTTerrainType::Tundra: ConstructionTicks = static_cast<int32>(ConstructionTicks * 2.5f); break;
	case EGTTerrainType::Frozen: ConstructionTicks = static_cast<int32>(ConstructionTicks * 3.0f); break;
	default: break;
	}
	NodeAttrs.RemainingConstructionTicks = ConstructionTicks;

	// Track the node data on the corporation. The network graph RegisterNode
	// expects an actor, so AI-built nodes are tracked via data for now.
	// When a concrete node subclass is available, this will register properly.

	// Deduct cost.
	Corp->BalanceSheet.CashOnHand -= ChosenCost;
	Corp->BalanceSheet.InfrastructureValue += ChosenCost;
	Corp->BalanceSheet.TotalAssets += ChosenCost * 0.9; // Slight depreciation from cost.

	// Track the "node" as an owned asset (use parcel ID as stand-in node ID).
	Corp->OwnedNodeIds.Add(TargetParcelId);

	// Fire InfrastructureBuilt event.
	UGTSimulationSubsystem* Sim = World->GetSubsystem<UGTSimulationSubsystem>();
	if (Sim && Sim->GetEventQueue())
	{
		FGTSimulationEvent Event;
		Event.EventType = EGTSimulationEventType::InfrastructureBuilt;
		Event.Tick = Sim->GetCurrentTick();
		Event.Timestamp = World->GetTimeSeconds();
		Event.SourceEntityId = Corp->CorporationId;
		Event.TargetEntityId = TargetParcelId;
		Event.Payload.Add(FName(TEXT("NodeType")), FString::FromInt(static_cast<int32>(ChosenType)));
		Event.Payload.Add(FName(TEXT("Cost")), FString::Printf(TEXT("%.0f"), ChosenCost));
		Event.Payload.Add(FName(TEXT("Capacity")), FString::Printf(TEXT("%.0f"), ChosenCapacity));
		Sim->GetEventQueue()->Enqueue(Event);
	}

	UE_LOG(LogTemp, Verbose, TEXT("AI '%s' built %d node on parcel %d for $%.0f"),
		*Corp->CorporationName, static_cast<int32>(ChosenType), TargetParcelId, ChosenCost);

	return EBTNodeResult::Succeeded;
}

// ===================================================================
// BT Task: Build Edge
// ===================================================================

UBTTask_GTBuildEdge::UBTTask_GTBuildEdge()
{
	NodeName = TEXT("Build Network Edge");
}

EBTNodeResult::Type UBTTask_GTBuildEdge::ExecuteTask(UBehaviorTreeComponent& OwnerComp, uint8* NodeMemory)
{
	AGTAICorporationController* AICtrl = GetAIController(OwnerComp);
	if (!AICtrl)
	{
		return EBTNodeResult::Failed;
	}

	UGTCorporation* Corp = AICtrl->GetCorporation();
	UGTLandParcelSystem* ParcelSystem = AICtrl->GetParcelSystem();
	UGTNetworkGraph* NetworkGraph = AICtrl->GetNetworkGraph();
	const FGTAIArchetypeData& Archetype = AICtrl->GetArchetype();

	if (!Corp || !ParcelSystem || !NetworkGraph)
	{
		return EBTNodeResult::Failed;
	}

	// Need at least 2 nodes to create an edge.
	if (Corp->OwnedNodeIds.Num() < 2)
	{
		return EBTNodeResult::Failed;
	}

	const double MinReserve = Corp->BalanceSheet.TotalAssets * Archetype.MinCashReserveRatio;
	const double AvailableCash = Corp->BalanceSheet.CashOnHand - MinReserve;

	// Edge costs by type.
	struct FEdgeCostEntry
	{
		EGTEdgeType Type;
		double CostPerUnit;
	};

	static const FEdgeCostEntry EdgeCosts[] = {
		{ EGTEdgeType::LocalFiber, 50000.0 },
		{ EGTEdgeType::RegionalFiber, 200000.0 },
		{ EGTEdgeType::NationalBackbone, 1000000.0 },
		{ EGTEdgeType::Microwave, 100000.0 },
	};

	if (AvailableCash < 50000.0)
	{
		return EBTNodeResult::Failed;
	}

	// Find the best pair of owned nodes to connect.
	// Look for pairs that share a region but don't have an edge yet.
	int32 BestSource = -1;
	int32 BestTarget = -1;
	double BestEdgeScore = -1.0;
	EGTEdgeType BestEdgeType = EGTEdgeType::LocalFiber;
	double BestEdgeCost = 0.0;

	for (int32 i = 0; i < Corp->OwnedNodeIds.Num(); ++i)
	{
		const int32 NodeA = Corp->OwnedNodeIds[i];
		const FGTLandParcel ParcelA = ParcelSystem->GetParcel(NodeA);

		for (int32 j = i + 1; j < Corp->OwnedNodeIds.Num(); ++j)
		{
			const int32 NodeB = Corp->OwnedNodeIds[j];
			const FGTLandParcel ParcelB = ParcelSystem->GetParcel(NodeB);

			// Check if they're already connected (tracked in OwnedEdgeIds).
			// Simple check: skip if this pair is in OwnedEdgeIds.
			bool bAlreadyConnected = false;
			for (int32 EdgeId : Corp->OwnedEdgeIds)
			{
				// Encode edge as (min, max) pair for comparison.
				if (EdgeId == NodeA * 10000 + NodeB || EdgeId == NodeB * 10000 + NodeA)
				{
					bAlreadyConnected = true;
					break;
				}
			}

			if (bAlreadyConnected)
			{
				continue;
			}

			// Score the connection.
			double Score = 50.0;

			// Same region = higher priority (local connectivity).
			if (ParcelA.RegionId == ParcelB.RegionId && ParcelA.RegionId >= 0)
			{
				Score += 100.0;
			}
			else if (ParcelA.RegionId >= 0 && ParcelB.RegionId >= 0)
			{
				// Cross-region = backbone potential.
				Score += 60.0;
			}

			// Determine appropriate edge type.
			EGTEdgeType EdgeType = EGTEdgeType::LocalFiber;
			double EdgeCost = 50000.0;

			if (ParcelA.RegionId == ParcelB.RegionId)
			{
				EdgeType = EGTEdgeType::LocalFiber;
				EdgeCost = 50000.0;
			}
			else
			{
				EdgeType = EGTEdgeType::RegionalFiber;
				EdgeCost = 200000.0;
			}

			// Apply terrain multipliers.
			EdgeCost *= (ParcelA.LaborCostMultiplier + ParcelB.LaborCostMultiplier) * 0.5;

			if (EdgeCost > AvailableCash)
			{
				continue;
			}

			if (Score > BestEdgeScore)
			{
				BestEdgeScore = Score;
				BestSource = NodeA;
				BestTarget = NodeB;
				BestEdgeType = EdgeType;
				BestEdgeCost = EdgeCost;
			}
		}
	}

	if (BestSource < 0 || BestTarget < 0)
	{
		return EBTNodeResult::Failed;
	}

	// Create the edge with construction timer.
	FGTEdgeAttributes EdgeAttrs;
	EdgeAttrs.Capacity = 500.0f;
	EdgeAttrs.LatencyWeightMs = 3.0f;
	EdgeAttrs.Reliability = 0.97f;
	EdgeAttrs.MaintenanceCostPerTick = static_cast<float>(BestEdgeCost * 0.0005);
	EdgeAttrs.ConstructionTimeSeconds = 30.0f;
	EdgeAttrs.TerrainRiskMultiplier = 1.0f;
	EdgeAttrs.ConstructionCost = static_cast<float>(BestEdgeCost);
	EdgeAttrs.bUnderConstruction = true;

	// Construction time: base ticks scaled by terrain difficulty of endpoints.
	int32 EdgeConstructionTicks = EdgeAttrs.BaseConstructionTicks;
	const FGTLandParcel ParcelSrc = ParcelSystem->GetParcel(BestSource);
	const FGTLandParcel ParcelDst = ParcelSystem->GetParcel(BestTarget);
	const float AvgLaborCost = (ParcelSrc.LaborCostMultiplier + ParcelDst.LaborCostMultiplier) * 0.5f;
	EdgeConstructionTicks = static_cast<int32>(EdgeConstructionTicks * AvgLaborCost);
	EdgeAttrs.RemainingConstructionTicks = FMath::Max(EdgeConstructionTicks, 1);

	// Deduct cost.
	Corp->BalanceSheet.CashOnHand -= BestEdgeCost;
	Corp->BalanceSheet.InfrastructureValue += BestEdgeCost;
	Corp->BalanceSheet.TotalAssets += BestEdgeCost * 0.85;

	// Track the edge (encode as source*10000 + target for uniqueness).
	Corp->OwnedEdgeIds.Add(BestSource * 10000 + BestTarget);

	// Fire event.
	UGTSimulationSubsystem* Sim = AICtrl->GetWorld()->GetSubsystem<UGTSimulationSubsystem>();
	if (Sim && Sim->GetEventQueue())
	{
		FGTSimulationEvent Event;
		Event.EventType = EGTSimulationEventType::InfrastructureBuilt;
		Event.Tick = Sim->GetCurrentTick();
		Event.Timestamp = AICtrl->GetWorld()->GetTimeSeconds();
		Event.SourceEntityId = Corp->CorporationId;
		Event.TargetEntityId = BestSource;
		Event.Payload.Add(FName(TEXT("EdgeType")), FString::FromInt(static_cast<int32>(BestEdgeType)));
		Event.Payload.Add(FName(TEXT("TargetNode")), FString::FromInt(BestTarget));
		Event.Payload.Add(FName(TEXT("Cost")), FString::Printf(TEXT("%.0f"), BestEdgeCost));
		Sim->GetEventQueue()->Enqueue(Event);
	}

	return EBTNodeResult::Succeeded;
}

// ===================================================================
// BT Task: Manage Finances
// ===================================================================

UBTTask_GTManageFinances::UBTTask_GTManageFinances()
{
	NodeName = TEXT("Manage Finances");
}

EBTNodeResult::Type UBTTask_GTManageFinances::ExecuteTask(UBehaviorTreeComponent& OwnerComp, uint8* NodeMemory)
{
	AGTAICorporationController* AICtrl = GetAIController(OwnerComp);
	if (!AICtrl)
	{
		return EBTNodeResult::Failed;
	}

	UGTCorporation* Corp = AICtrl->GetCorporation();
	if (!Corp)
	{
		return EBTNodeResult::Failed;
	}

	const FGTAIArchetypeData& Archetype = AICtrl->GetArchetype();
	const double Cash = Corp->BalanceSheet.CashOnHand;
	const double TotalAssets = Corp->BalanceSheet.TotalAssets;
	const double Equity = Corp->BalanceSheet.GetEquity();
	const double CashRatio = (TotalAssets > 0.0) ? (Cash / TotalAssets) : 1.0;
	const double DebtToEquity = (Equity > 0.0) ? (Corp->TotalDebt / Equity) : 999.0;

	bool bTookAction = false;

	// Scenario 1: Low cash, good credit → take a loan.
	if (CashRatio < Archetype.MinCashReserveRatio && DebtToEquity < Archetype.MaxDebtToEquityRatio)
	{
		// Loan amount: bring cash up to 2x the minimum reserve.
		const double TargetCash = TotalAssets * Archetype.MinCashReserveRatio * 2.0;
		const double LoanAmount = FMath::Max(TargetCash - Cash, 500000.0);

		// Interest rate based on credit rating.
		float InterestRate = 0.05f;
		switch (Corp->CreditRating)
		{
		case EGTCreditRating::AAA: InterestRate = 0.02f; break;
		case EGTCreditRating::AA: InterestRate = 0.03f; break;
		case EGTCreditRating::A: InterestRate = 0.04f; break;
		case EGTCreditRating::BBB: InterestRate = 0.05f; break;
		case EGTCreditRating::BB: InterestRate = 0.07f; break;
		case EGTCreditRating::B: InterestRate = 0.10f; break;
		default: InterestRate = 0.15f; break;
		}

		if (Corp->IssueDept(LoanAmount, EGTFinancialInstrument::BankLoan, InterestRate))
		{
			UE_LOG(LogTemp, Verbose, TEXT("AI '%s' took loan of $%.0f at %.1f%% interest"),
				*Corp->CorporationName, LoanAmount, InterestRate * 100.0f);
			bTookAction = true;
		}
	}

	// Scenario 2: High cash reserves, outstanding debt → pay down debt.
	if (CashRatio > Archetype.MinCashReserveRatio * 3.0 && Corp->TotalDebt > 0.0)
	{
		double RemainingPaydown = FMath::Min(Corp->TotalDebt, Cash * 0.3);
		double TotalPaid = 0.0;

		// Pay down instruments starting from the highest interest rate.
		Corp->DebtInstruments.Sort([](const FGTDebtInstrument& A, const FGTDebtInstrument& B)
		{
			return A.InterestRate > B.InterestRate;
		});

		for (int32 i = Corp->DebtInstruments.Num() - 1; i >= 0 && RemainingPaydown > 0.0; --i)
		{
			FGTDebtInstrument& Debt = Corp->DebtInstruments[i];
			const double Payment = FMath::Min(Debt.Principal, RemainingPaydown);
			Debt.Principal -= Payment;
			RemainingPaydown -= Payment;
			TotalPaid += Payment;

			if (Debt.Principal <= 0.0)
			{
				Corp->DebtInstruments.RemoveAt(i);
			}
		}

		Corp->BalanceSheet.CashOnHand -= TotalPaid;
		Corp->TotalDebt -= TotalPaid;
		Corp->BalanceSheet.TotalLiabilities -= TotalPaid;
		Corp->RecalculateCreditRating();

		UE_LOG(LogTemp, Verbose, TEXT("AI '%s' paid down $%.0f of debt"),
			*Corp->CorporationName, TotalPaid);
		bTookAction = true;
	}

	return bTookAction ? EBTNodeResult::Succeeded : EBTNodeResult::Failed;
}

// ===================================================================
// BT Task: Propose Contract
// ===================================================================

UBTTask_GTProposeContract::UBTTask_GTProposeContract()
{
	NodeName = TEXT("Propose Contract");
}

EBTNodeResult::Type UBTTask_GTProposeContract::ExecuteTask(UBehaviorTreeComponent& OwnerComp, uint8* NodeMemory)
{
	AGTAICorporationController* AICtrl = GetAIController(OwnerComp);
	if (!AICtrl)
	{
		return EBTNodeResult::Failed;
	}

	UGTCorporation* Corp = AICtrl->GetCorporation();
	UGTCorporationManager* CorpManager = AICtrl->GetCorporationManager();
	UGTAllianceManager* AllianceMgr = AICtrl->GetAllianceManager();

	if (!Corp || !CorpManager || !AllianceMgr)
	{
		return EBTNodeResult::Failed;
	}

	const FGTAIArchetypeData& Archetype = AICtrl->GetArchetype();
	const float Aggressiveness = AICtrl->GetAggressivenessMultiplier();

	// Need infrastructure to offer contracts.
	if (Corp->OwnedNodeIds.Num() < 2 || Corp->OwnedEdgeIds.Num() < 1)
	{
		return EBTNodeResult::Failed;
	}

	// Find other corporations to propose contracts to.
	const TArray<UGTCorporation*> AllCorps = CorpManager->GetAllCorporations();

	UGTCorporation* BestPartner = nullptr;
	double BestPartnerScore = -1.0;

	for (UGTCorporation* OtherCorp : AllCorps)
	{
		if (OtherCorp == Corp || !OtherCorp)
		{
			continue;
		}

		// Skip if they have no infrastructure.
		if (OtherCorp->OwnedNodeIds.Num() < 1)
		{
			continue;
		}

		// Score potential partner.
		double Score = 50.0;

		// Prefer partners with complementary infrastructure (they have nodes we don't).
		Score += OtherCorp->OwnedNodeIds.Num() * 5.0;

		// Financial health of partner matters.
		if (!OtherCorp->IsInsolvent())
		{
			Score += 30.0;
		}

		if (Score > BestPartnerScore)
		{
			BestPartnerScore = Score;
			BestPartner = OtherCorp;
		}
	}

	if (!BestPartner)
	{
		return EBTNodeResult::Failed;
	}

	// Create a peering contract.
	FGTContract Contract;
	Contract.ContractType = EGTContractType::PeeringAgreement;
	Contract.OfferorCorporationId = Corp->CorporationId;
	Contract.AcceptorCorporationId = BestPartner->CorporationId;
	Contract.DurationTicks = 100; // ~400 seconds at 4s/tick.
	Contract.GuaranteedCapacity = 100.0f;
	Contract.BreachPenalty = 500000.0;

	// Pricing: aggressive AIs undercut, prudent AIs charge fair rates.
	const double BasePrice = 5000.0;
	Contract.PricePerTick = BasePrice * (1.0 - Archetype.AggressionWeight * 0.3) * (1.0 / Aggressiveness);

	const int32 ContractId = AllianceMgr->ProposeContract(Contract);

	// If the partner is AI, auto-accept based on their financial evaluation.
	if (BestPartner->bIsAI && ContractId >= 0)
	{
		// AI partners accept if the price is reasonable.
		const double ValuePerTick = Contract.GuaranteedCapacity * 50.0; // Estimated value.
		if (Contract.PricePerTick < ValuePerTick * 1.5)
		{
			AllianceMgr->AcceptContract(ContractId);
			UE_LOG(LogTemp, Verbose, TEXT("AI '%s' and AI '%s' signed peering contract #%d"),
				*Corp->CorporationName, *BestPartner->CorporationName, ContractId);
		}
	}

	return EBTNodeResult::Succeeded;
}

// ===================================================================
// BT Decorator: Check Strategy
// ===================================================================

UBTDecorator_GTCheckStrategy::UBTDecorator_GTCheckStrategy()
{
	NodeName = TEXT("Check Strategy");
}

bool UBTDecorator_GTCheckStrategy::CalculateRawConditionValue(UBehaviorTreeComponent& OwnerComp, uint8* NodeMemory) const
{
	AGTAICorporationController* AICtrl = GetAIController(OwnerComp);
	if (!AICtrl)
	{
		return false;
	}

	return AICtrl->CachedStrategy == RequiredStrategy;
}

FString UBTDecorator_GTCheckStrategy::GetStaticDescription() const
{
	FString StrategyName;
	switch (RequiredStrategy)
	{
	case EGTAIStrategy::Expand: StrategyName = TEXT("Expand"); break;
	case EGTAIStrategy::Consolidate: StrategyName = TEXT("Consolidate"); break;
	case EGTAIStrategy::Compete: StrategyName = TEXT("Compete"); break;
	case EGTAIStrategy::Survive: StrategyName = TEXT("Survive"); break;
	}
	return FString::Printf(TEXT("Strategy == %s"), *StrategyName);
}

// ===================================================================
// BT Decorator: Check Can Afford
// ===================================================================

UBTDecorator_GTCheckCanAfford::UBTDecorator_GTCheckCanAfford()
{
	NodeName = TEXT("Check Can Afford");
}

bool UBTDecorator_GTCheckCanAfford::CalculateRawConditionValue(UBehaviorTreeComponent& OwnerComp, uint8* NodeMemory) const
{
	AGTAICorporationController* AICtrl = GetAIController(OwnerComp);
	if (!AICtrl)
	{
		return false;
	}

	return AICtrl->CachedCash >= MinimumCash;
}

FString UBTDecorator_GTCheckCanAfford::GetStaticDescription() const
{
	return FString::Printf(TEXT("Cash >= $%.0f"), MinimumCash);
}
