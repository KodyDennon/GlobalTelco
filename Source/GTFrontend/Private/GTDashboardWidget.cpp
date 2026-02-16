#include "GTDashboardWidget.h"

void UGTDashboardWidget::RefreshFinancials_Implementation(const FGTBalanceSheet& BalanceSheet, const FGTIncomeStatement& Income, EGTCreditRating CreditRating)
{
	// Base implementation — Blueprint subclasses override to populate UI panels.
}

void UGTDashboardWidget::RefreshInfrastructureStatus_Implementation(int32 TotalNodes, int32 OperationalNodes, int32 DegradedNodes, int32 DestroyedNodes)
{
	// Base implementation — Blueprint subclasses override to populate UI panels.
}

void UGTDashboardWidget::RefreshRegionalData_Implementation(const TArray<FGTRegionalEconomyData>& Regions)
{
	// Base implementation — Blueprint subclasses override to populate UI panels.
}
