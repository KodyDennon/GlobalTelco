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
#include "GTEconomyTypes.h"
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
		CachedSimSubsystem->GetEventQueue()->OnEventDispatchedNative.Remove(EventDelegateHandle);
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
		EventDelegateHandle = CachedSimSubsystem->GetEventQueue()->OnEventDispatchedNative.AddUObject(
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

	// Revenue, maintenance, and interest are now handled centrally by
	// UGTRevenueCalculator and UGTCorporation::ProcessEconomicTick.
	// This handler is kept for AI-specific per-tick logic (e.g., event responses).
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

		UBTTask_GTManageFinances* ManageFinances = NewObject<UBTTask_GTManageFinances>(SurviveSeq);

		FBTCompositeChild SurviveChild;
		SurviveChild.ChildTask = ManageFinances;
		SurviveSeq->Children.Add(SurviveChild);

		FBTCompositeChild Branch1;
		Branch1.ChildComposite = SurviveSeq;
		Branch1.Decorators.Add(SurviveCheck);
		RootSelector->Children.Add(Branch1);
	}

	// --- Branch 2: Expand (acquire land, build nodes, build edges) ---
	{
		UBTComposite_Sequence* ExpandSeq = NewObject<UBTComposite_Sequence>(RootSelector, TEXT("ExpandSequence"));

		UBTDecorator_GTCheckStrategy* ExpandCheck = NewObject<UBTDecorator_GTCheckStrategy>(ExpandSeq);
		ExpandCheck->RequiredStrategy = EGTAIStrategy::Expand;

		UBTDecorator_GTCheckCanAfford* AffordCheck = NewObject<UBTDecorator_GTCheckCanAfford>(ExpandSeq);
		AffordCheck->MinimumCash = 200000.0;

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
		Branch2.Decorators.Add(ExpandCheck);
		Branch2.Decorators.Add(AffordCheck);
		RootSelector->Children.Add(Branch2);
	}

	// --- Branch 3: Consolidate (build edges, propose contracts) ---
	{
		UBTComposite_Sequence* ConsolidateSeq = NewObject<UBTComposite_Sequence>(RootSelector, TEXT("ConsolidateSequence"));

		UBTDecorator_GTCheckStrategy* ConsolidateCheck = NewObject<UBTDecorator_GTCheckStrategy>(ConsolidateSeq);
		ConsolidateCheck->RequiredStrategy = EGTAIStrategy::Consolidate;

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
		Branch3.Decorators.Add(ConsolidateCheck);
		RootSelector->Children.Add(Branch3);
	}

	// --- Branch 4: Compete (aggressive expansion + contracts) ---
	{
		UBTComposite_Sequence* CompeteSeq = NewObject<UBTComposite_Sequence>(RootSelector, TEXT("CompeteSequence"));

		UBTDecorator_GTCheckStrategy* CompeteCheck = NewObject<UBTDecorator_GTCheckStrategy>(CompeteSeq);
		CompeteCheck->RequiredStrategy = EGTAIStrategy::Compete;

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
		Branch4.Decorators.Add(CompeteCheck);
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
