#include "GTAICorporationController.h"
#include "GTCorporation.h"
#include "GTCorporationManager.h"
#include "GTNetworkGraph.h"
#include "GTLandParcelSystem.h"
#include "GTRegionalEconomy.h"
#include "GTAllianceManager.h"
#include "GTSimulationSubsystem.h"
#include "GTEventQueue.h"
#include "GTSimulationTypes.h"
#include "BehaviorTree/BehaviorTree.h"
#include "BehaviorTree/BehaviorTreeComponent.h"
#include "BehaviorTree/BlackboardComponent.h"
#include "BehaviorTree/Composites/BTComposite_Selector.h"
#include "BehaviorTree/Composites/BTComposite_Sequence.h"

AGTAICorporationController::AGTAICorporationController()
{
	// Create the behavior tree component — this is the BT runner.
	BTComponent = CreateDefaultSubobject<UBehaviorTreeComponent>(TEXT("BehaviorTreeComponent"));
	BrainComponent = BTComponent;
}

void AGTAICorporationController::BeginPlay()
{
	Super::BeginPlay();
	CacheSubsystems();
}

void AGTAICorporationController::EndPlay(const EEndPlayReason::Type EndPlayReason)
{
	// Unsubscribe from events.
	if (CachedSimSubsystem && CachedSimSubsystem->GetEventQueue() && EventDelegateHandle.IsValid())
	{
		CachedSimSubsystem->GetEventQueue()->OnEventDispatched.Remove(EventDelegateHandle);
		EventDelegateHandle.Reset();
	}

	Super::EndPlay(EndPlayReason);
}

void AGTAICorporationController::InitializeForCorporation(int32 InCorporationId, const FGTAIArchetypeData& InArchetype, float InAggressiveness)
{
	CorporationId = InCorporationId;
	Archetype = InArchetype;
	AggressivenessMultiplier = InAggressiveness;

	CacheSubsystems();

	// Subscribe to economic tick events.
	if (CachedSimSubsystem && CachedSimSubsystem->GetEventQueue())
	{
		EventDelegateHandle = CachedSimSubsystem->GetEventQueue()->OnEventDispatched.AddUObject(
			this, &AGTAICorporationController::OnEconomicTick);
	}

	// Construct and start the behavior tree.
	ConstructAndRunBehaviorTree();

	UE_LOG(LogTemp, Log, TEXT("GTAICorporationController: Initialized for corp %d ('%s'), archetype='%s', aggression=%.1f"),
		CorporationId, *Archetype.ArchetypeName, *Archetype.ArchetypeName, AggressivenessMultiplier);
}

UGTCorporation* AGTAICorporationController::GetCorporation() const
{
	if (CachedCorpManager)
	{
		return CachedCorpManager->GetCorporation(CorporationId);
	}
	return nullptr;
}

void AGTAICorporationController::CacheSubsystems()
{
	UWorld* World = GetWorld();
	if (!World)
	{
		return;
	}

	CachedCorpManager = World->GetSubsystem<UGTCorporationManager>();
	CachedNetworkGraph = World->GetSubsystem<UGTNetworkGraph>();
	CachedParcelSystem = World->GetSubsystem<UGTLandParcelSystem>();
	CachedRegionalEconomy = World->GetSubsystem<UGTRegionalEconomy>();
	CachedAllianceManager = World->GetSubsystem<UGTAllianceManager>();
	CachedSimSubsystem = World->GetSubsystem<UGTSimulationSubsystem>();
}

void AGTAICorporationController::OnEconomicTick(const FGTSimulationEvent& Event)
{
	if (Event.EventType != EGTSimulationEventType::EconomicTick)
	{
		return;
	}

	// The BT service handles state updates. The BT itself is always running
	// via the BehaviorTreeComponent tick. Economic tick events can be used
	// for triggering revenue calculation on AI corporations.
	UGTCorporation* Corp = GetCorporation();
	if (!Corp)
	{
		return;
	}

	// Generate base revenue from owned infrastructure.
	// Revenue = sum of node capacities * base rate per capacity unit.
	const double BaseRatePerCapacityUnit = 10.0; // $10 per capacity unit per tick.
	double TickRevenue = 0.0;

	for (int32 NodeId : Corp->OwnedNodeIds)
	{
		// Each "node" generates revenue based on the regional demand it serves.
		if (CachedParcelSystem)
		{
			const FGTLandParcel Parcel = CachedParcelSystem->GetParcel(NodeId);
			if (Parcel.RegionId >= 0 && CachedRegionalEconomy)
			{
				const FGTRegionalEconomyData Region = CachedRegionalEconomy->GetRegionData(Parcel.RegionId);
				// Revenue proportional to regional demand and connectivity.
				TickRevenue += Region.CurrentDemand * BaseRatePerCapacityUnit * 0.01 /
					FMath::Max(1.0, static_cast<double>(Corp->OwnedNodeIds.Num()));
			}
		}
	}

	// Apply connectivity bonus from edges.
	TickRevenue *= (1.0 + Corp->OwnedEdgeIds.Num() * 0.1);

	if (TickRevenue > 0.0)
	{
		Corp->AddRevenue(TickRevenue, EGTRevenueSource::BandwidthDelivery);
	}

	// Calculate maintenance costs.
	double MaintenanceCost = 0.0;
	MaintenanceCost += Corp->OwnedNodeIds.Num() * 5000.0;  // $5k per node per tick.
	MaintenanceCost += Corp->OwnedEdgeIds.Num() * 2000.0;  // $2k per edge per tick.

	Corp->LastTickIncome.MaintenanceCosts += MaintenanceCost;

	// Interest on debt (simplified: 0.1% per tick).
	if (Corp->TotalDebt > 0.0)
	{
		const double InterestExpense = Corp->TotalDebt * 0.001;
		Corp->LastTickIncome.InterestExpense += InterestExpense;
	}
}

void AGTAICorporationController::ConstructAndRunBehaviorTree()
{
	if (!BTComponent)
	{
		return;
	}

	// Create the behavior tree asset programmatically.
	UBehaviorTree* BT = NewObject<UBehaviorTree>(this, TEXT("AICorpBehaviorTree"));
	if (!BT)
	{
		return;
	}

	// Root node: Selector (tries children left-to-right until one succeeds).
	UBTComposite_Selector* RootSelector = NewObject<UBTComposite_Selector>(BT, TEXT("RootSelector"));

	// Service on root: Update world state every tick.
	UBTService_GTUpdateWorldState* UpdateService = NewObject<UBTService_GTUpdateWorldState>(RootSelector);
	RootSelector->Services.Add(UpdateService);

	// --- Branch 1: Survive (when in financial crisis) ---
	{
		UBTComposite_Sequence* SurviveSeq = NewObject<UBTComposite_Sequence>(RootSelector, TEXT("SurviveSequence"));

		UBTDecorator_GTCheckStrategy* SurviveCheck = NewObject<UBTDecorator_GTCheckStrategy>(SurviveSeq);
		SurviveCheck->RequiredStrategy = EGTAIStrategy::Survive;
		SurviveSeq->Decorators.Add(SurviveCheck);

		UBTTask_GTManageFinances* ManageFinances = NewObject<UBTTask_GTManageFinances>(SurviveSeq);

		FBTCompositeChild SurviveChild;
		SurviveChild.ChildTask = ManageFinances;
		SurviveSeq->Children.Add(SurviveChild);

		FBTCompositeChild Branch1;
		Branch1.ChildComposite = SurviveSeq;
		RootSelector->Children.Add(Branch1);
	}

	// --- Branch 2: Expand (acquire land, build nodes, build edges) ---
	{
		UBTComposite_Sequence* ExpandSeq = NewObject<UBTComposite_Sequence>(RootSelector, TEXT("ExpandSequence"));

		UBTDecorator_GTCheckStrategy* ExpandCheck = NewObject<UBTDecorator_GTCheckStrategy>(ExpandSeq);
		ExpandCheck->RequiredStrategy = EGTAIStrategy::Expand;
		ExpandSeq->Decorators.Add(ExpandCheck);

		UBTDecorator_GTCheckCanAfford* AffordCheck = NewObject<UBTDecorator_GTCheckCanAfford>(ExpandSeq);
		AffordCheck->MinimumCash = 200000.0;
		ExpandSeq->Decorators.Add(AffordCheck);

		UBTTask_GTAcquireLand* AcquireLand = NewObject<UBTTask_GTAcquireLand>(ExpandSeq);
		UBTTask_GTBuildNode* BuildNode = NewObject<UBTTask_GTBuildNode>(ExpandSeq);
		UBTTask_GTBuildEdge* BuildEdge = NewObject<UBTTask_GTBuildEdge>(ExpandSeq);

		FBTCompositeChild Child1;
		Child1.ChildTask = AcquireLand;
		ExpandSeq->Children.Add(Child1);

		FBTCompositeChild Child2;
		Child2.ChildTask = BuildNode;
		ExpandSeq->Children.Add(Child2);

		FBTCompositeChild Child3;
		Child3.ChildTask = BuildEdge;
		ExpandSeq->Children.Add(Child3);

		FBTCompositeChild Branch2;
		Branch2.ChildComposite = ExpandSeq;
		RootSelector->Children.Add(Branch2);
	}

	// --- Branch 3: Consolidate (build edges, propose contracts) ---
	{
		UBTComposite_Sequence* ConsolidateSeq = NewObject<UBTComposite_Sequence>(RootSelector, TEXT("ConsolidateSequence"));

		UBTDecorator_GTCheckStrategy* ConsolidateCheck = NewObject<UBTDecorator_GTCheckStrategy>(ConsolidateSeq);
		ConsolidateCheck->RequiredStrategy = EGTAIStrategy::Consolidate;
		ConsolidateSeq->Decorators.Add(ConsolidateCheck);

		UBTTask_GTBuildEdge* BuildEdge = NewObject<UBTTask_GTBuildEdge>(ConsolidateSeq);
		UBTTask_GTProposeContract* ProposeContract = NewObject<UBTTask_GTProposeContract>(ConsolidateSeq);
		UBTTask_GTManageFinances* ManageFinances = NewObject<UBTTask_GTManageFinances>(ConsolidateSeq);

		FBTCompositeChild Child1;
		Child1.ChildTask = BuildEdge;
		ConsolidateSeq->Children.Add(Child1);

		FBTCompositeChild Child2;
		Child2.ChildTask = ProposeContract;
		ConsolidateSeq->Children.Add(Child2);

		FBTCompositeChild Child3;
		Child3.ChildTask = ManageFinances;
		ConsolidateSeq->Children.Add(Child3);

		FBTCompositeChild Branch3;
		Branch3.ChildComposite = ConsolidateSeq;
		RootSelector->Children.Add(Branch3);
	}

	// --- Branch 4: Compete (aggressive expansion + contracts) ---
	{
		UBTComposite_Sequence* CompeteSeq = NewObject<UBTComposite_Sequence>(RootSelector, TEXT("CompeteSequence"));

		UBTDecorator_GTCheckStrategy* CompeteCheck = NewObject<UBTDecorator_GTCheckStrategy>(CompeteSeq);
		CompeteCheck->RequiredStrategy = EGTAIStrategy::Compete;
		CompeteSeq->Decorators.Add(CompeteCheck);

		UBTTask_GTAcquireLand* AcquireLand = NewObject<UBTTask_GTAcquireLand>(CompeteSeq);
		UBTTask_GTBuildNode* BuildNode = NewObject<UBTTask_GTBuildNode>(CompeteSeq);
		UBTTask_GTProposeContract* ProposeContract = NewObject<UBTTask_GTProposeContract>(CompeteSeq);

		FBTCompositeChild Child1;
		Child1.ChildTask = AcquireLand;
		CompeteSeq->Children.Add(Child1);

		FBTCompositeChild Child2;
		Child2.ChildTask = BuildNode;
		CompeteSeq->Children.Add(Child2);

		FBTCompositeChild Child3;
		Child3.ChildTask = ProposeContract;
		CompeteSeq->Children.Add(Child3);

		FBTCompositeChild Branch4;
		Branch4.ChildComposite = CompeteSeq;
		RootSelector->Children.Add(Branch4);
	}

	// --- Fallback: Manage Finances (always available) ---
	{
		UBTTask_GTManageFinances* FallbackFinances = NewObject<UBTTask_GTManageFinances>(RootSelector);

		FBTCompositeChild FallbackBranch;
		FallbackBranch.ChildTask = FallbackFinances;
		RootSelector->Children.Add(FallbackBranch);
	}

	// Set the root node on the behavior tree.
	BT->RootNode = RootSelector;

	// Run the behavior tree.
	BTComponent->StartTree(*BT);

	UE_LOG(LogTemp, Log, TEXT("GTAICorporationController: Behavior tree started for corp %d"), CorporationId);
}
