#pragma once

#include "CoreMinimal.h"
#include "UObject/NoExportTypes.h"
#include "GTEconomyTypes.h"
#include "GTCorporation.generated.h"

/**
 * UGTCorporation
 *
 * Represents a player-owned (or AI-owned) telecom corporation.
 * Tracks balance sheet, income, debt, credit rating, and owned assets.
 * Multi-player corporations are supported: multiple players can share
 * ownership with voting rights proportional to equity stake.
 */
UCLASS(BlueprintType)
class GTECONOMY_API UGTCorporation : public UObject
{
	GENERATED_BODY()

public:
	UPROPERTY(BlueprintReadOnly, Category = "Corporation")
	int32 CorporationId = -1;

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Corporation")
	FString CorporationName;

	/** True if this corporation is controlled by AI (single-player opponents). */
	UPROPERTY(BlueprintReadOnly, Category = "Corporation")
	bool bIsAI = false;

	/** Index into UGTAIArchetypeRegistry for AI personality. -1 for player corps. */
	UPROPERTY(BlueprintReadOnly, Category = "Corporation")
	int32 ArchetypeIndex = -1;

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Finance")
	FGTBalanceSheet BalanceSheet;

	UPROPERTY(BlueprintReadOnly, Category = "Finance")
	FGTIncomeStatement LastTickIncome;

	UPROPERTY(BlueprintReadOnly, Category = "Finance")
	EGTCreditRating CreditRating = EGTCreditRating::BBB;

	UPROPERTY(BlueprintReadOnly, Category = "Finance")
	double TotalDebt = 0.0;

	/** Player IDs that own shares in this corporation. Key = PlayerId, Value = equity fraction 0-1. */
	UPROPERTY(BlueprintReadOnly, Category = "Ownership")
	TMap<int32, float> ShareholderEquity;

	/** IDs of all infrastructure nodes owned by this corporation. */
	UPROPERTY(BlueprintReadOnly, Category = "Assets")
	TArray<int32> OwnedNodeIds;

	/** IDs of all network edges owned by this corporation. */
	UPROPERTY(BlueprintReadOnly, Category = "Assets")
	TArray<int32> OwnedEdgeIds;

	/** Process a single economic tick: compute revenue, costs, and update financials. */
	UFUNCTION(BlueprintCallable, Category = "Finance")
	void ProcessEconomicTick(float TickDeltaSeconds);

	/** Add revenue from a specific source. */
	UFUNCTION(BlueprintCallable, Category = "Finance")
	void AddRevenue(double Amount, EGTRevenueSource Source);

	/** Take on new debt. Returns false if credit rating is too low. */
	UFUNCTION(BlueprintCallable, Category = "Finance")
	bool IssueDept(double Amount, EGTFinancialInstrument Instrument, float InterestRate);

	/** Check if the corporation is insolvent (liabilities exceed assets). */
	UFUNCTION(BlueprintPure, Category = "Finance")
	bool IsInsolvent() const;

	/** Recalculate credit rating based on current financial health. */
	UFUNCTION(BlueprintCallable, Category = "Finance")
	void RecalculateCreditRating();
};
