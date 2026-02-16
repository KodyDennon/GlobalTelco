#pragma once

#include "CoreMinimal.h"
#include "Blueprint/UserWidget.h"
#include "GTSpeedControlWidget.generated.h"

class UButton;
class UTextBlock;

/**
 * UGTSpeedControlWidget
 *
 * In-game widget for controlling simulation speed.
 * Buttons: Pause, Play (1x), Fast (2x), Faster (4x).
 * Also provides quick-save and quick-load buttons.
 *
 * Subclass in Blueprints for visual layout.
 */
UCLASS(Abstract, Blueprintable)
class GTFRONTEND_API UGTSpeedControlWidget : public UUserWidget
{
	GENERATED_BODY()

public:
	virtual void NativeConstruct() override;
	virtual void NativeTick(const FGeometry& MyGeometry, float InDeltaTime) override;

	// --- Bound widgets ---

	UPROPERTY(BlueprintReadOnly, meta = (BindWidget), Category = "Speed Control")
	TObjectPtr<UButton> PauseButton;

	UPROPERTY(BlueprintReadOnly, meta = (BindWidget), Category = "Speed Control")
	TObjectPtr<UButton> PlayButton;

	UPROPERTY(BlueprintReadOnly, meta = (BindWidget), Category = "Speed Control")
	TObjectPtr<UButton> FastButton;

	UPROPERTY(BlueprintReadOnly, meta = (BindWidget), Category = "Speed Control")
	TObjectPtr<UButton> FasterButton;

	UPROPERTY(BlueprintReadOnly, meta = (BindWidgetOptional), Category = "Speed Control")
	TObjectPtr<UTextBlock> SpeedLabel;

	UPROPERTY(BlueprintReadOnly, meta = (BindWidgetOptional), Category = "Speed Control")
	TObjectPtr<UTextBlock> TickCountLabel;

	UPROPERTY(BlueprintReadOnly, meta = (BindWidgetOptional), Category = "Speed Control")
	TObjectPtr<UButton> QuickSaveButton;

	UPROPERTY(BlueprintReadOnly, meta = (BindWidgetOptional), Category = "Speed Control")
	TObjectPtr<UButton> QuickLoadButton;

	// --- Events ---

	DECLARE_DYNAMIC_MULTICAST_DELEGATE(FOnQuickSaveRequested);
	UPROPERTY(BlueprintAssignable, Category = "Speed Control")
	FOnQuickSaveRequested OnQuickSaveRequested;

	DECLARE_DYNAMIC_MULTICAST_DELEGATE(FOnQuickLoadRequested);
	UPROPERTY(BlueprintAssignable, Category = "Speed Control")
	FOnQuickLoadRequested OnQuickLoadRequested;

	// --- Public API ---

	/** Set the simulation speed. Syncs button visual state. */
	UFUNCTION(BlueprintCallable, Category = "Speed Control")
	void SetSpeed(float Multiplier);

	/** Toggle pause on/off. */
	UFUNCTION(BlueprintCallable, Category = "Speed Control")
	void TogglePause();

	/** Update the display from current simulation state. Called automatically in NativeTick. */
	UFUNCTION(BlueprintCallable, Category = "Speed Control")
	void RefreshDisplay();

protected:
	UFUNCTION()
	void HandlePauseClicked();

	UFUNCTION()
	void HandlePlayClicked();

	UFUNCTION()
	void HandleFastClicked();

	UFUNCTION()
	void HandleFasterClicked();

	UFUNCTION()
	void HandleQuickSaveClicked();

	UFUNCTION()
	void HandleQuickLoadClicked();

	/** Tick display refresh interval. */
	UPROPERTY(EditDefaultsOnly, Category = "Speed Control")
	float DisplayRefreshInterval = 0.5f;

	float TimeSinceLastRefresh = 0.0f;
};
