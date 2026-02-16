#include "GTHUDWidget.h"

void UGTHUDWidget::RefreshSimulationData_Implementation(int64 CurrentTick, double SimulationTimeSeconds)
{
	// Base implementation — Blueprint subclasses override to update UI elements.
}

void UGTHUDWidget::PushNotification_Implementation(const FString& Message, bool bIsUrgent)
{
	// Base implementation — Blueprint subclasses override to display notifications.
}
