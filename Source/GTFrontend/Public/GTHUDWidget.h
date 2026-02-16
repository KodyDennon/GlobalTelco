#pragma once

#include "CoreMinimal.h"
#include "Blueprint/UserWidget.h"
#include "GTHUDWidget.generated.h"

/**
 * UGTHUDWidget
 *
 * Base HUD widget for the GlobalTelco MMO.
 * Displays simulation tick, player corporation summary,
 * and notification feed. Subclass in Blueprints for visual layout.
 */
UCLASS(Abstract, Blueprintable)
class GTFRONTEND_API UGTHUDWidget : public UUserWidget
{
	GENERATED_BODY()

public:
	/** Called by the HUD each economic tick to refresh displayed data. */
	UFUNCTION(BlueprintCallable, BlueprintNativeEvent, Category = "HUD")
	void RefreshSimulationData(int64 CurrentTick, double SimulationTimeSeconds);

	/** Push a notification message to the HUD feed. */
	UFUNCTION(BlueprintCallable, BlueprintNativeEvent, Category = "HUD")
	void PushNotification(const FString& Message, bool bIsUrgent);

protected:
	virtual void RefreshSimulationData_Implementation(int64 CurrentTick, double SimulationTimeSeconds);
	virtual void PushNotification_Implementation(const FString& Message, bool bIsUrgent);
};
