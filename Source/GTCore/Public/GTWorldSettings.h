#pragma once

#include "CoreMinimal.h"
#include "Engine/DataAsset.h"
#include "GTWorldSettings.generated.h"

/** Difficulty presets controlling starting conditions and simulation parameters. */
UENUM(BlueprintType)
enum class EGTDifficulty : uint8
{
	Easy,
	Normal,
	Hard,
	Custom
};

/** Disaster frequency/severity presets. */
UENUM(BlueprintType)
enum class EGTDisasterSeverity : uint8
{
	Calm,
	Moderate,
	Brutal
};

/** Research speed presets. */
UENUM(BlueprintType)
enum class EGTResearchSpeed : uint8
{
	Fast,
	Normal,
	Slow
};

/**
 * UGTWorldSettings
 *
 * Data-driven world configuration asset. Create instances in the editor
 * for different difficulty presets (Easy/Normal/Hard/Custom).
 * The world generator reads these settings to configure starting conditions,
 * economic parameters, disaster intensity, and AI competition level.
 */
UCLASS(BlueprintType)
class GTCORE_API UGTWorldSettings : public UDataAsset
{
	GENERATED_BODY()

public:
	/** Display name for this settings preset. */
	UPROPERTY(EditAnywhere, BlueprintReadOnly, Category = "General")
	FString PresetName = TEXT("Normal");

	/** Base difficulty level. Setting this applies default values; Custom allows full override. */
	UPROPERTY(EditAnywhere, BlueprintReadOnly, Category = "General")
	EGTDifficulty Difficulty = EGTDifficulty::Normal;

	// --- Economy ---

	/** Starting cash for each player corporation (USD). */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Economy")
	double StartingCapital = 10000000.0;

	/** Global demand growth multiplier. 1.0 = baseline. */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Economy", meta = (ClampMin = "0.1", ClampMax = "5.0"))
	float DemandGrowthMultiplier = 1.0f;

	/** Infrastructure construction cost multiplier. */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Economy", meta = (ClampMin = "0.1", ClampMax = "5.0"))
	float ConstructionCostMultiplier = 1.0f;

	/** Maintenance cost multiplier. */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Economy", meta = (ClampMin = "0.1", ClampMax = "5.0"))
	float MaintenanceCostMultiplier = 1.0f;

	// --- Simulation ---

	/** Economic tick interval in seconds. */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Simulation", meta = (ClampMin = "1.0", ClampMax = "10.0"))
	float TickIntervalSeconds = 4.0f;

	/** Technology research speed preset. */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Simulation")
	EGTResearchSpeed ResearchSpeed = EGTResearchSpeed::Normal;

	/** Research speed multiplier (derived from preset or custom). */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Simulation", meta = (ClampMin = "0.1", ClampMax = "5.0"))
	float ResearchSpeedMultiplier = 1.0f;

	// --- Disasters ---

	/** Disaster frequency/severity preset. */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Disasters")
	EGTDisasterSeverity DisasterSeverity = EGTDisasterSeverity::Moderate;

	/** Disaster frequency multiplier. 0.0 = no disasters. */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Disasters", meta = (ClampMin = "0.0", ClampMax = "5.0"))
	float DisasterFrequencyMultiplier = 1.0f;

	/** Disaster damage multiplier. */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Disasters", meta = (ClampMin = "0.0", ClampMax = "5.0"))
	float DisasterDamageMultiplier = 1.0f;

	// --- AI Corporations ---

	/** Number of AI-controlled corporations (0 for pure multiplayer). */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "AI", meta = (ClampMin = "0", ClampMax = "10"))
	int32 AICorpCount = 5;

	/** AI aggressiveness multiplier. Higher = more competitive AI. */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "AI", meta = (ClampMin = "0.1", ClampMax = "3.0"))
	float AIAggressiveness = 1.0f;

	// --- World Generation ---

	/** Hex grid resolution for the geodesic grid (n parameter: total hexes ~ 10*n^2 + 2). */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "World Generation", meta = (ClampMin = "10", ClampMax = "200"))
	int32 HexGridResolution = 100;

	/** Random seed for world generation. 0 = random. */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "World Generation")
	int32 WorldSeed = 0;

	/** Number of geographic regions to create. */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "World Generation", meta = (ClampMin = "10", ClampMax = "200"))
	int32 RegionCount = 50;

	/** Apply default values based on the selected difficulty. */
	UFUNCTION(BlueprintCallable, Category = "World Settings")
	void ApplyDifficultyDefaults();
};
