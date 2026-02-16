#pragma once

#include "CoreMinimal.h"
#include "GameFramework/SaveGame.h"
#include "GTSimulationTypes.h"
#include "GTEconomyTypes.h"
#include "GTInfrastructureTypes.h"
#include "GTMultiplayerTypes.h"
#include "GTLandParcelSystem.h"
#include "GTWorldSettings.h"
#include "GTSaveGame.generated.h"

/** Current save format version. Increment when save format changes. */
static constexpr int32 GT_SAVE_VERSION = 1;

/**
 * FGTSavedCorporation
 *
 * Serializable snapshot of a corporation's full state.
 */
USTRUCT(BlueprintType)
struct FGTSavedCorporation
{
	GENERATED_BODY()

	UPROPERTY()
	int32 CorporationId = -1;

	UPROPERTY()
	FString CorporationName;

	UPROPERTY()
	bool bIsAI = false;

	UPROPERTY()
	int32 ArchetypeIndex = -1;

	UPROPERTY()
	FGTBalanceSheet BalanceSheet;

	UPROPERTY()
	FGTIncomeStatement LastTickIncome;

	UPROPERTY()
	EGTCreditRating CreditRating = EGTCreditRating::BBB;

	UPROPERTY()
	double TotalDebt = 0.0;

	UPROPERTY()
	TMap<int32, float> ShareholderEquity;

	UPROPERTY()
	TArray<int32> OwnedNodeIds;

	UPROPERTY()
	TArray<int32> OwnedEdgeIds;
};

/**
 * FGTSavedWorldSettings
 *
 * Snapshot of the world settings at save time.
 */
USTRUCT(BlueprintType)
struct FGTSavedWorldSettings
{
	GENERATED_BODY()

	UPROPERTY()
	EGTDifficulty Difficulty = EGTDifficulty::Normal;

	UPROPERTY()
	double StartingCapital = 10000000.0;

	UPROPERTY()
	float DemandGrowthMultiplier = 1.0f;

	UPROPERTY()
	float ConstructionCostMultiplier = 1.0f;

	UPROPERTY()
	float MaintenanceCostMultiplier = 1.0f;

	UPROPERTY()
	float TickIntervalSeconds = 4.0f;

	UPROPERTY()
	EGTResearchSpeed ResearchSpeed = EGTResearchSpeed::Normal;

	UPROPERTY()
	float ResearchSpeedMultiplier = 1.0f;

	UPROPERTY()
	EGTDisasterSeverity DisasterSeverity = EGTDisasterSeverity::Moderate;

	UPROPERTY()
	float DisasterFrequencyMultiplier = 1.0f;

	UPROPERTY()
	float DisasterDamageMultiplier = 1.0f;

	UPROPERTY()
	int32 AICorpCount = 5;

	UPROPERTY()
	float AIAggressiveness = 1.0f;

	UPROPERTY()
	int32 HexGridResolution = 100;

	UPROPERTY()
	int32 WorldSeed = 0;

	UPROPERTY()
	int32 RegionCount = 50;
};

/**
 * FGTSaveSlotInfo
 *
 * Metadata about a save slot for display in the load game UI.
 */
USTRUCT(BlueprintType)
struct FGTSaveSlotInfo
{
	GENERATED_BODY()

	UPROPERTY(BlueprintReadOnly)
	FString SlotName;

	UPROPERTY(BlueprintReadOnly)
	FString SaveDisplayName;

	UPROPERTY(BlueprintReadOnly)
	FDateTime SaveTimestamp;

	UPROPERTY(BlueprintReadOnly)
	EGTDifficulty Difficulty = EGTDifficulty::Normal;

	UPROPERTY(BlueprintReadOnly)
	int64 SimulationTick = 0;

	UPROPERTY(BlueprintReadOnly)
	FString PlayerCorporationName;
};

/**
 * UGTSaveGame
 *
 * Complete snapshot of a single-player game session.
 * Serialized via UE5's USaveGame + FMemoryArchive binary serialization.
 * Contains all data needed to fully restore a game session:
 * world state, corporations, infrastructure, economy, contracts.
 */
UCLASS()
class GLOBALTELCO_API UGTSaveGame : public USaveGame
{
	GENERATED_BODY()

public:
	// --- Metadata ---

	UPROPERTY()
	FString SaveDisplayName;

	UPROPERTY()
	FDateTime SaveTimestamp;

	UPROPERTY()
	int32 SaveVersion = GT_SAVE_VERSION;

	// --- Simulation State ---

	UPROPERTY()
	int64 SimulationTick = 0;

	UPROPERTY()
	double SimulationTimeSeconds = 0.0;

	UPROPERTY()
	int32 PlayerCorporationId = -1;

	UPROPERTY()
	FString PlayerCorporationName;

	// --- World Configuration ---

	UPROPERTY()
	FGTSavedWorldSettings WorldSettings;

	// --- Parcels ---

	UPROPERTY()
	TArray<FGTLandParcel> AllParcels;

	// --- Corporations ---

	UPROPERTY()
	TArray<FGTSavedCorporation> Corporations;

	// --- Regional Economy ---

	UPROPERTY()
	TArray<FGTRegionalEconomyData> Regions;

	// --- Contracts & Alliances ---

	UPROPERTY()
	TArray<FGTContract> ActiveContracts;

	UPROPERTY()
	TArray<FGTAlliance> ActiveAlliances;

	// --- Helper Methods ---

	/** Populate this save from the current world state. */
	void CaptureWorldState(UWorld* World, int32 InPlayerCorporationId, const FString& InPlayerCorpName);

	/** Restore saved state into the current world subsystems. */
	void RestoreWorldState(UWorld* World) const;

	/** Build a save slot info struct from this save's metadata. */
	FGTSaveSlotInfo GetSlotInfo(const FString& SlotName) const;
};
