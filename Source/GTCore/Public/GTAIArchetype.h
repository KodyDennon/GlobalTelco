#pragma once

#include "CoreMinimal.h"
#include "GTAIArchetype.generated.h"

/**
 * FGTAIArchetypeData
 *
 * Defines the personality profile of an AI corporation.
 * Weights (0.0-1.0) control how the AI prioritizes different strategic axes.
 * These weights feed directly into the behavior tree's utility scoring
 * to produce distinct corporate personalities.
 */
USTRUCT(BlueprintType)
struct GTCORE_API FGTAIArchetypeData
{
	GENERATED_BODY()

	/** Display name for this archetype (e.g., "Aggressive Expander"). */
	UPROPERTY(EditAnywhere, BlueprintReadOnly, Category = "AI Archetype")
	FString ArchetypeName;

	/** Short description of this archetype's strategy. */
	UPROPERTY(EditAnywhere, BlueprintReadOnly, Category = "AI Archetype")
	FString Description;

	/** Pool of possible corporation names for this archetype. */
	UPROPERTY(EditAnywhere, BlueprintReadOnly, Category = "AI Archetype")
	TArray<FString> CompanyNamePool;

	/** Priority for acquiring new land and building new infrastructure. */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "AI Archetype", meta = (ClampMin = "0.0", ClampMax = "1.0"))
	float ExpansionWeight = 0.5f;

	/** Priority for upgrading and optimizing existing network. */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "AI Archetype", meta = (ClampMin = "0.0", ClampMax = "1.0"))
	float ConsolidationWeight = 0.5f;

	/** Priority for R&D investment and technology advancement. */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "AI Archetype", meta = (ClampMin = "0.0", ClampMax = "1.0"))
	float TechInvestmentWeight = 0.5f;

	/** Tendency toward competitive/hostile actions (undercutting, sabotage). */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "AI Archetype", meta = (ClampMin = "0.0", ClampMax = "1.0"))
	float AggressionWeight = 0.5f;

	/** Willingness to take on debt and operate in risky regions. */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "AI Archetype", meta = (ClampMin = "0.0", ClampMax = "1.0"))
	float RiskTolerance = 0.5f;

	/** Preference for maintaining cash reserves and avoiding debt. */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "AI Archetype", meta = (ClampMin = "0.0", ClampMax = "1.0"))
	float FinancialPrudence = 0.5f;

	/** Minimum cash reserve ratio (cash / total assets) before AI stops spending. */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "AI Archetype", meta = (ClampMin = "0.0", ClampMax = "1.0"))
	float MinCashReserveRatio = 0.15f;

	/** Maximum debt-to-equity ratio the AI will tolerate. */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "AI Archetype", meta = (ClampMin = "0.1", ClampMax = "5.0"))
	float MaxDebtToEquityRatio = 1.5f;
};

/**
 * UGTAIArchetypeRegistry
 *
 * Static registry of built-in AI corporation archetypes.
 * Provides 4 distinct personality profiles that create varied and
 * interesting AI competitors in single-player mode.
 *
 * Archetypes:
 * 0 - Aggressive Expander: Rapid territorial expansion, high debt tolerance
 * 1 - Defensive Consolidator: Strong network, low risk, steady growth
 * 2 - Tech Innovator: R&D focused, premium services, moderate expansion
 * 3 - Budget Operator: Cost-efficient, conservative, high cash reserves
 */
UCLASS()
class GTCORE_API UGTAIArchetypeRegistry : public UObject
{
	GENERATED_BODY()

public:
	/** Get the array of built-in archetype definitions. */
	static const TArray<FGTAIArchetypeData>& GetArchetypes();

	/** Get a specific archetype by index. Returns default if out of range. */
	static const FGTAIArchetypeData& GetArchetype(int32 Index);

	/** Number of built-in archetypes. */
	static int32 GetArchetypeCount();

	/** Pick a random company name from the archetype's pool using the given seed. */
	static FString GetRandomCompanyName(int32 ArchetypeIndex, int32 Seed);

private:
	static TArray<FGTAIArchetypeData> BuiltInArchetypes;
	static bool bInitialized;
	static void InitializeArchetypes();
};
