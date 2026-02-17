#include "GTSinglePlayerGameMode.h"
#include "GTGameInstance.h"
#include "GTGameState.h"
#include "GTPlayerController.h"
#include "GTGlobePawn.h"
#include "GTGlobeInteraction.h"
#include "GTWorldSettings.h"
#include "GTWorldGenerator.h"
#include "GTCorporationManager.h"
#include "GTAICorporationController.h"
#include "GTAIArchetype.h"
#include "GTSimulationSubsystem.h"
#include "GTEventQueue.h"
#include "GTSaveLoadSubsystem.h"
#include "GTSaveGame.h"
#include "GTRegionalEconomy.h"
#include "GTNetworkGraph.h"
#include "GTRevenueCalculator.h"
#include "GTSpeedControlWidget.h"
#include "GTParcelInfoWidget.h"
#include "GTLandParcelSystem.h"
#include "Blueprint/UserWidget.h"
#include "Kismet/GameplayStatics.h"
#include "Engine/World.h"

AGTSinglePlayerGameMode::AGTSinglePlayerGameMode()
{
	GameStateClass = AGTGameState::StaticClass();
	PlayerControllerClass = AGTPlayerController::StaticClass();
	DefaultPawnClass = AGTGlobePawn::StaticClass();
}

void AGTSinglePlayerGameMode::InitGame(const FString& MapName, const FString& Options, FString& ErrorMessage)
{
	Super::InitGame(MapName, Options, ErrorMessage);

	UWorld* World = GetWorld();
	if (!World)
	{
		ErrorMessage = TEXT("No world available.");
		return;
	}

	// Pull session configuration from GameInstance if available.
	UGTGameInstance* GTGI = Cast<UGTGameInstance>(GetGameInstance());
	if (GTGI)
	{
		if (GTGI->bPendingLoadFromSave)
		{
			bLoadingFromSave = true;
			LoadSlotName = GTGI->PendingLoadSlotName;
		}
		else
		{
			if (GTGI->PendingWorldSettings)
			{
				WorldSettings = GTGI->PendingWorldSettings;
			}
			if (!GTGI->PendingPlayerCorpName.IsEmpty())
			{
				PlayerCorporationName = GTGI->PendingPlayerCorpName;
			}
		}
		GTGI->ClearPendingSession();
	}

	// Create default world settings if none provided.
	if (!WorldSettings)
	{
		WorldSettings = NewObject<UGTWorldSettings>(this);
		WorldSettings->ApplyDifficultyDefaults();
	}

	if (bLoadingFromSave)
	{
		// Load from save: restore world state via SaveLoadSubsystem.
		UGTSaveLoadSubsystem* SaveLoad = GetGameInstance()->GetSubsystem<UGTSaveLoadSubsystem>();
		if (SaveLoad && SaveLoad->LoadGame(LoadSlotName))
		{
			UGTSaveGame* Loaded = SaveLoad->GetLastLoadedSave();
			if (Loaded)
			{
				PlayerCorporationId = Loaded->PlayerCorporationId;
				PlayerCorporationName = Loaded->PlayerCorporationName;
			}
			UE_LOG(LogTemp, Log, TEXT("GTSinglePlayer: Loaded save '%s'."), *LoadSlotName);
		}
		else
		{
			UE_LOG(LogTemp, Error, TEXT("GTSinglePlayer: Failed to load save '%s'. Starting new game."), *LoadSlotName);
			bLoadingFromSave = false;
		}
	}

	if (!bLoadingFromSave)
	{
		// New game: generate the world.
		UGTWorldGenerator* WorldGen = World->GetSubsystem<UGTWorldGenerator>();
		if (WorldGen)
		{
			WorldGen->GenerateWorld(WorldSettings);
			UE_LOG(LogTemp, Log, TEXT("GTSinglePlayer: World generated."));
		}
		else
		{
			UE_LOG(LogTemp, Error, TEXT("GTSinglePlayer: WorldGenerator subsystem not found."));
		}

		// Create player corporation.
		UGTCorporationManager* CorpManager = World->GetSubsystem<UGTCorporationManager>();
		if (CorpManager)
		{
			PlayerCorporationId = CorpManager->CreateCorporation(
				PlayerCorporationName,
				WorldSettings->StartingCapital,
				false, // bIsAI
				-1     // No archetype
			);
			UE_LOG(LogTemp, Log, TEXT("GTSinglePlayer: Player corporation '%s' created (ID=%d)."),
				*PlayerCorporationName, PlayerCorporationId);
		}

		// Spawn AI corporations.
		SpawnAICorporations();
	}

	// Configure simulation speed.
	UGTSimulationSubsystem* SimSubsystem = World->GetSubsystem<UGTSimulationSubsystem>();
	if (SimSubsystem)
	{
		SimSubsystem->EconomicTickInterval = WorldSettings->TickIntervalSeconds;
		UE_LOG(LogTemp, Log, TEXT("GTSinglePlayer: Tick interval set to %.1fs."),
			WorldSettings->TickIntervalSeconds);

		// Subscribe to economic ticks for auto-save and corp manager processing.
		UGTEventQueue* EQ = SimSubsystem->GetEventQueue();
		if (EQ)
		{
			EQ->OnEventDispatchedNative.AddUObject(
				this, &AGTSinglePlayerGameMode::OnEconomicTick);
		}
	}
}

void AGTSinglePlayerGameMode::StartPlay()
{
	Super::StartPlay();

	APlayerController* PC = GetWorld()->GetFirstPlayerController();

	// Assign corporation ID to the player controller.
	if (PC)
	{
		if (AGTPlayerController* GTPC = Cast<AGTPlayerController>(PC))
		{
			GTPC->CorporationId = PlayerCorporationId;
		}
	}

	// Store active session state on GameInstance for save/load access.
	UGTGameInstance* GTGI = Cast<UGTGameInstance>(GetGameInstance());
	if (GTGI)
	{
		GTGI->ActivePlayerCorporationId = PlayerCorporationId;
		GTGI->ActivePlayerCorpName = PlayerCorporationName;
	}

	// Create in-game HUD widgets and wire delegates.
	if (PC)
	{
		CreateHUDWidgets(PC);
	}

	UE_LOG(LogTemp, Log, TEXT("GTSinglePlayer: Game started. %d AI corporations active."),
		AIControllers.Num());
}

void AGTSinglePlayerGameMode::EndPlay(const EEndPlayReason::Type EndPlayReason)
{
	// Clean up AI controllers.
	for (AGTAICorporationController* AICtrl : AIControllers)
	{
		if (AICtrl)
		{
			AICtrl->Destroy();
		}
	}
	AIControllers.Empty();

	Super::EndPlay(EndPlayReason);
}

void AGTSinglePlayerGameMode::SpawnAICorporations()
{
	if (!WorldSettings)
	{
		return;
	}

	UWorld* World = GetWorld();
	UGTCorporationManager* CorpManager = World ? World->GetSubsystem<UGTCorporationManager>() : nullptr;
	if (!CorpManager)
	{
		UE_LOG(LogTemp, Error, TEXT("GTSinglePlayer: CorporationManager not available."));
		return;
	}

	const int32 NumAI = WorldSettings->AICorpCount;
	const float Aggressiveness = WorldSettings->AIAggressiveness;
	const int32 NumArchetypes = UGTAIArchetypeRegistry::GetArchetypeCount();
	const int32 Seed = (WorldSettings->WorldSeed != 0) ? WorldSettings->WorldSeed : FMath::Rand();

	for (int32 i = 0; i < NumAI; ++i)
	{
		// Round-robin archetypes, varying by seed for variety.
		const int32 ArchetypeIndex = (i + Seed) % NumArchetypes;
		const FGTAIArchetypeData& Archetype = UGTAIArchetypeRegistry::GetArchetype(ArchetypeIndex);

		// Pick a unique company name.
		const FString CorpName = UGTAIArchetypeRegistry::GetRandomCompanyName(ArchetypeIndex, Seed + i);

		// Create the corporation.
		const int32 CorpId = CorpManager->CreateCorporation(
			CorpName,
			WorldSettings->StartingCapital, // Same starting capital as player.
			true,  // bIsAI
			ArchetypeIndex
		);

		// Spawn the AI controller actor.
		FActorSpawnParameters SpawnParams;
		SpawnParams.SpawnCollisionHandlingOverride = ESpawnActorCollisionHandlingMethod::AlwaysSpawn;

		AGTAICorporationController* AIController = World->SpawnActor<AGTAICorporationController>(
			AGTAICorporationController::StaticClass(),
			FVector::ZeroVector,
			FRotator::ZeroRotator,
			SpawnParams
		);

		if (AIController)
		{
			AIController->InitializeForCorporation(CorpId, Archetype, Aggressiveness);
			AIControllers.Add(AIController);
		}
	}

	UE_LOG(LogTemp, Log, TEXT("GTSinglePlayer: Spawned %d AI corporations."), AIControllers.Num());
}

void AGTSinglePlayerGameMode::OnEconomicTick(const FGTSimulationEvent& Event)
{
	if (Event.EventType != EGTSimulationEventType::EconomicTick)
	{
		return;
	}

	UWorld* World = GetWorld();
	if (!World)
	{
		return;
	}

	const float TickDelta = WorldSettings ? WorldSettings->TickIntervalSeconds : 4.0f;
	const float MaintenanceMult = WorldSettings ? WorldSettings->MaintenanceCostMultiplier : 1.0f;
	const float DemandMult = WorldSettings ? WorldSettings->DemandGrowthMultiplier : 1.0f;

	// Step 1: Advance construction timers on nodes and edges.
	UGTNetworkGraph* Graph = World->GetSubsystem<UGTNetworkGraph>();
	if (Graph)
	{
		TArray<int32> CompletedNodes;
		TArray<int32> CompletedEdges;
		Graph->ProcessConstructionTick(CompletedNodes, CompletedEdges);
		Graph->RecalculateDirtyRoutes();

		// Fire InfrastructureBuilt events for completed constructions.
		UGTSimulationSubsystem* SimSub = World->GetSubsystem<UGTSimulationSubsystem>();
		UGTEventQueue* EQ = SimSub ? SimSub->GetEventQueue() : nullptr;
		if (EQ)
		{
			for (int32 NodeId : CompletedNodes)
			{
				FGTSimulationEvent BuiltEvent;
				BuiltEvent.EventType = EGTSimulationEventType::InfrastructureBuilt;
				BuiltEvent.TargetEntityId = NodeId;
				BuiltEvent.Payload.Add(FName("Type"), TEXT("Node"));
				EQ->Enqueue(BuiltEvent);
			}
			for (int32 EdgeId : CompletedEdges)
			{
				FGTSimulationEvent BuiltEvent;
				BuiltEvent.EventType = EGTSimulationEventType::InfrastructureBuilt;
				BuiltEvent.TargetEntityId = EdgeId;
				BuiltEvent.Payload.Add(FName("Type"), TEXT("Edge"));
				EQ->Enqueue(BuiltEvent);
			}
		}

		// Step 1b: Calculate utilization/congestion after routes settle.
		Graph->CalculateUtilization();
	}

	// Step 2: Update regional economy (demand growth, connectivity effects).
	UGTRegionalEconomy* Economy = World->GetSubsystem<UGTRegionalEconomy>();
	if (Economy)
	{
		Economy->ProcessEconomicTick(TickDelta);
	}

	// Step 3: Calculate revenue and costs for all corporations.
	UGTRevenueCalculator* RevenueCalc = World->GetSubsystem<UGTRevenueCalculator>();
	if (RevenueCalc)
	{
		RevenueCalc->ProcessRevenueTick(MaintenanceMult, DemandMult);
	}

	// Step 4: Process corporation economic ticks (debt interest, net income, credit rating).
	UGTCorporationManager* CorpManager = World->GetSubsystem<UGTCorporationManager>();
	if (CorpManager)
	{
		CorpManager->ProcessAllCorporationTicks(TickDelta);
	}

	// Auto-save check.
	if (AutoSaveTickInterval > 0)
	{
		TicksSinceAutoSave++;
		if (TicksSinceAutoSave >= AutoSaveTickInterval)
		{
			TicksSinceAutoSave = 0;

			UGTSaveLoadSubsystem* SaveLoad = GetGameInstance()->GetSubsystem<UGTSaveLoadSubsystem>();
			if (SaveLoad)
			{
				SaveLoad->AutoSave(PlayerCorporationId, PlayerCorporationName);
			}
		}
	}
}

void AGTSinglePlayerGameMode::CreateHUDWidgets(APlayerController* PC)
{
	if (!PC)
	{
		return;
	}

	// Set input mode to game and UI so player can interact with both.
	PC->SetInputMode(FInputModeGameAndUI());
	PC->bShowMouseCursor = true;

	// Create speed control widget.
	if (SpeedControlWidgetClass)
	{
		SpeedControlWidget = CreateWidget<UGTSpeedControlWidget>(PC, SpeedControlWidgetClass);
		if (SpeedControlWidget)
		{
			SpeedControlWidget->OnQuickSaveRequested.AddDynamic(this, &AGTSinglePlayerGameMode::HandleQuickSave);
			SpeedControlWidget->OnQuickLoadRequested.AddDynamic(this, &AGTSinglePlayerGameMode::HandleQuickLoad);
			SpeedControlWidget->AddToViewport(10); // Above game world, below popups.
		}
	}

	// Create parcel info widget (hidden initially, shown on hex selection).
	if (ParcelInfoWidgetClass)
	{
		ParcelInfoWidget = CreateWidget<UGTParcelInfoWidget>(PC, ParcelInfoWidgetClass);
		if (ParcelInfoWidget)
		{
			ParcelInfoWidget->AddToViewport(5);
			ParcelInfoWidget->SetVisibility(ESlateVisibility::Collapsed);
		}
	}

	// Wire hex selection → parcel info display via globe interaction on the player controller.
	AGTPlayerController* GTPC = Cast<AGTPlayerController>(PC);
	if (GTPC && GTPC->GlobeInteraction)
	{
		GTPC->GlobeInteraction->OnHexSelected.AddDynamic(this, &AGTSinglePlayerGameMode::HandleHexSelected);
		GTPC->GlobeInteraction->OnSelectionCleared.AddDynamic(this, &AGTSinglePlayerGameMode::HandleSelectionCleared);
	}
}

void AGTSinglePlayerGameMode::HandleQuickSave()
{
	UGTSaveLoadSubsystem* SaveLoad = GetGameInstance()->GetSubsystem<UGTSaveLoadSubsystem>();
	if (SaveLoad)
	{
		SaveLoad->SaveGame(
			TEXT("QuickSave"),
			TEXT("Quick Save"),
			PlayerCorporationId,
			PlayerCorporationName
		);
		UE_LOG(LogTemp, Log, TEXT("GTSinglePlayer: Quick-saved."));
	}
}

void AGTSinglePlayerGameMode::HandleQuickLoad()
{
	UGTGameInstance* GTGI = Cast<UGTGameInstance>(GetGameInstance());
	UGTSaveLoadSubsystem* SaveLoad = GetGameInstance()->GetSubsystem<UGTSaveLoadSubsystem>();

	if (GTGI && SaveLoad && SaveLoad->DoesSlotExist(TEXT("QuickSave")))
	{
		GTGI->PrepareLoadGame(TEXT("QuickSave"));
		UGameplayStatics::OpenLevel(GetWorld(), TEXT("/Game/Maps/GameWorld"),
			false, TEXT("?game=/Script/GlobalTelco.GTSinglePlayerGameMode"));
	}
	else
	{
		UE_LOG(LogTemp, Warning, TEXT("GTSinglePlayer: No quick-save found."));
	}
}

void AGTSinglePlayerGameMode::HandleHexSelected(int32 ParcelId)
{
	if (!ParcelInfoWidget)
	{
		return;
	}

	UWorld* World = GetWorld();
	if (!World)
	{
		return;
	}

	UGTLandParcelSystem* ParcelSystem = World->GetSubsystem<UGTLandParcelSystem>();
	if (ParcelSystem && ParcelId >= 0)
	{
		const FGTLandParcel Parcel = ParcelSystem->GetParcel(ParcelId);
		ParcelInfoWidget->ShowParcelInfo(Parcel);
		ParcelInfoWidget->SetVisibility(ESlateVisibility::SelfHitTestInvisible);
	}
}

void AGTSinglePlayerGameMode::HandleSelectionCleared()
{
	if (ParcelInfoWidget)
	{
		ParcelInfoWidget->HideParcelInfo();
		ParcelInfoWidget->SetVisibility(ESlateVisibility::Collapsed);
	}
}
