#pragma once

#include "CoreMinimal.h"
#include "Blueprint/UserWidget.h"
#include "GTEconomyTypes.h"
#include "GTDashboardWidget.generated.h"

/**
 * UGTDashboardWidget
 *
 * Corporate dashboard widget displaying financial overview,
 * infrastructure status, and regional market data.
 * Subclass in Blueprints for visual layout.
 */
UCLASS(Abstract, Blueprintable)
class GTFRONTEND_API UGTDashboardWidget : public UUserWidget
{
	GENERATED_BODY()

public:
	/** Refresh the financial overview panel with current corporation data. */
	UFUNCTION(BlueprintCallable, BlueprintNativeEvent, Category = "Dashboard")
	void RefreshFinancials(const FGTBalanceSheet& BalanceSheet, const FGTIncomeStatement& Income, EGTCreditRating CreditRating);

	/** Update the infrastructure status panel. */
	UFUNCTION(BlueprintCallable, BlueprintNativeEvent, Category = "Dashboard")
	void RefreshInfrastructureStatus(int32 TotalNodes, int32 OperationalNodes, int32 DegradedNodes, int32 DestroyedNodes);

	/** Update the regional market data panel. */
	UFUNCTION(BlueprintCallable, BlueprintNativeEvent, Category = "Dashboard")
	void RefreshRegionalData(const TArray<FGTRegionalEconomyData>& Regions);

protected:
	virtual void RefreshFinancials_Implementation(const FGTBalanceSheet& BalanceSheet, const FGTIncomeStatement& Income, EGTCreditRating CreditRating);
	virtual void RefreshInfrastructureStatus_Implementation(int32 TotalNodes, int32 OperationalNodes, int32 DegradedNodes, int32 DestroyedNodes);
	virtual void RefreshRegionalData_Implementation(const TArray<FGTRegionalEconomyData>& Regions);
};
