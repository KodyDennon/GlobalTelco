#pragma once

#include "CoreMinimal.h"
#include "GameFramework/GameModeBase.h"
#include "GTSimulationTypes.h"
#include "GTSinglePlayerGameMode.generated.h"

class UGTWorldSettings;
class AGTAICorporationController;

/**
 * AGTSinglePlayerGameMode
 *
 * Game mode for offline single-player sessions. Runs the full simulation
 * locally without networking. On startup:
 * 1. Generates the world from UGTWorldSettings (proc-gen globe, parcels, regions)
 * 2. Creates the player's corporation via the Corporation Manager
 * 3. Spawns AI corporation controllers with distinct archetypes
 * 4. Configures simulation speed from world settings
 * 5. Starts the simulation
 *
 * Supports save/load via the GTSaveLoadSubsystem on the game instance.
 */
UCLASS()
class GLOBALTELCO_API AGTSinglePlayerGameMode : public AGameModeBase
{
	GENERATED_BODY()

public:
	AGTSinglePlayerGameMode();

	virtual void InitGame(const FString& MapName, const FString& Options, FString& ErrorMessage) override;
	virtual void StartPlay() override;
	virtual void EndPlay(const EEndPlayReason::Type EndPlayReason) override;

	/** World settings for this session. Set before InitGame (by GameInstance or directly). */
	UPROPERTY(BlueprintReadWrite, Category = "Single Player")
	TObjectPtr<UGTWorldSettings> WorldSettings;

	/** Player's corporation name. */
	UPROPERTY(BlueprintReadWrite, Category = "Single Player")
	FString PlayerCorporationName = TEXT("Player Corp");

	/** The player's assigned corporation ID. */
	UPROPERTY(BlueprintReadOnly, Category = "Single Player")
	int32 PlayerCorporationId = -1;

	/** Whether we're loading from a save (vs new game). */
	UPROPERTY(BlueprintReadWrite, Category = "Single Player")
	bool bLoadingFromSave = false;

	/** Save slot name if loading. */
	UPROPERTY(BlueprintReadWrite, Category = "Single Player")
	FString LoadSlotName;

	/** Auto-save interval in simulation ticks (0 = disabled). */
	UPROPERTY(BlueprintReadWrite, Category = "Single Player")
	int32 AutoSaveTickInterval = 50;

protected:
	/** Spawn AI corporation controllers based on world settings. */
	void SpawnAICorporations();

	/** Handle economic tick events for auto-save and global processing. */
	void OnEconomicTick(const FGTSimulationEvent& Event);

	/** Spawned AI controllers for cleanup. */
	UPROPERTY()
	TArray<TObjectPtr<AGTAICorporationController>> AIControllers;

	/** Ticks since last auto-save. */
	int32 TicksSinceAutoSave = 0;
};
