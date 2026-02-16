#pragma once

#include "CoreMinimal.h"
#include "GTEconomyTypes.generated.h"

/** Credit rating tiers for corporation debt instruments. */
UENUM(BlueprintType)
enum class EGTCreditRating : uint8
{
	AAA,
	AA,
	A,
	BBB,
	BB,
	B,
	CCC,
	Default
};

/** Types of financial instruments available to corporations. */
UENUM(BlueprintType)
enum class EGTFinancialInstrument : uint8
{
	BankLoan,
	CorporateBond,
	GovernmentGrant,
	DevelopmentFunding
};

/** Revenue source categories. */
UENUM(BlueprintType)
enum class EGTRevenueSource : uint8
{
	BandwidthDelivery,
	TransitAgreement,
	PeeringContract,
	LandLease,
	GovernmentGrant,
	DevelopmentContract
};

/**
 * FGTBalanceSheet
 *
 * Snapshot of a corporation's financial position.
 */
USTRUCT(BlueprintType)
struct GTECONOMY_API FGTBalanceSheet
{
	GENERATED_BODY()

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Finance")
	double TotalAssets = 0.0;

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Finance")
	double TotalLiabilities = 0.0;

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Finance")
	double CashOnHand = 0.0;

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Finance")
	double InfrastructureValue = 0.0;

	double GetEquity() const { return TotalAssets - TotalLiabilities; }
};

/**
 * FGTIncomeStatement
 *
 * Per-tick income statement for a corporation.
 */
USTRUCT(BlueprintType)
struct GTECONOMY_API FGTIncomeStatement
{
	GENERATED_BODY()

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Finance")
	double Revenue = 0.0;

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Finance")
	double OperatingCosts = 0.0;

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Finance")
	double MaintenanceCosts = 0.0;

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Finance")
	double InterestExpense = 0.0;

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Finance")
	double TaxExpense = 0.0;

	double NetIncome() const { return Revenue - OperatingCosts - MaintenanceCosts - InterestExpense - TaxExpense; }
};

/**
 * FGTRegionalEconomyData
 *
 * Economic data for a geographic region. Regions track population,
 * GDP, technology adoption, and demand — all influenced by
 * connectivity and infrastructure quality.
 */
USTRUCT(BlueprintType)
struct GTECONOMY_API FGTRegionalEconomyData
{
	GENERATED_BODY()

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Regional Economy")
	int32 RegionId = -1;

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Regional Economy")
	FString RegionName;

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Regional Economy")
	double Population = 0.0;

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Regional Economy")
	double GDPProxy = 0.0;

	/** 0.0 to 1.0 */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Regional Economy", meta = (ClampMin = "0.0", ClampMax = "1.0"))
	float TechAdoptionIndex = 0.5f;

	/** 0.0 to 1.0 */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Regional Economy", meta = (ClampMin = "0.0", ClampMax = "1.0"))
	float PoliticalStability = 0.7f;

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Regional Economy")
	float DataDemandGrowthRate = 0.05f;

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Regional Economy")
	float BusinessDensity = 0.5f;

	/** 0.0 to 1.0 */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Regional Economy", meta = (ClampMin = "0.0", ClampMax = "1.0"))
	float UrbanizationIndex = 0.5f;

	/** Current aggregate data demand in this region (units per tick). */
	UPROPERTY(BlueprintReadOnly, Category = "Regional Economy")
	double CurrentDemand = 0.0;
};
