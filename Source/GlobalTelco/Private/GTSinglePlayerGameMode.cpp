#include "GTSinglePlayerGameMode.h"
#include "GTGameInstance.h"
#include "GTGameState.h"
#include "GTPlayerController.h"
#include "GTGlobePawn.h"
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
			EQ->OnEventDispatched.AddUObject(
				this, &AGTSinglePlayerGameMode::OnEconomicTick);
		}
	}
}

void AGTSinglePlayerGameMode::StartPlay()
{
	Super::StartPlay();

	// Assign corporation ID to the player controller.
	if (APlayerController* PC = GetWorld()->GetFirstPlayerController())
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

	// Process all corporation ticks (revenue, expenses, etc.).
	UGTCorporationManager* CorpManager = World->GetSubsystem<UGTCorporationManager>();
	if (CorpManager)
	{
		CorpManager->ProcessAllCorporationTicks(WorldSettings ? WorldSettings->TickIntervalSeconds : 4.0f);
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
