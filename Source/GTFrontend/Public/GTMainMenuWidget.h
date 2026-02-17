#pragma once

#include "CoreMinimal.h"
#include "Blueprint/UserWidget.h"
#include "GTSimulationTypes.h"
#include "GTMainMenuWidget.generated.h"

class UButton;
class UVerticalBox;
class UTextBlock;

/**
 * UGTMainMenuWidget
 *
 * Main menu widget for single-player mode.
 * Provides New Game, Load Game, and Quit functionality.
 * Subclass in Blueprints for visual layout; C++ handles all logic.
 *
 * Save/Load operations are delegated via events so GTFrontend doesn't
 * depend on the GlobalTelco module. The GameInstance binds to
 * OnRefreshSaveSlotsRequested and OnDeleteSaveSlotRequested to perform
 * actual save/load operations and call SetSaveSlots() with results.
 */
UCLASS(Abstract, Blueprintable)
class GTFRONTEND_API UGTMainMenuWidget : public UUserWidget
{
	GENERATED_BODY()

public:
	virtual void NativeConstruct() override;

	// --- Blueprint-bindable buttons ---

	UPROPERTY(BlueprintReadOnly, meta = (BindWidget), Category = "Main Menu")
	TObjectPtr<UButton> NewGameButton;

	UPROPERTY(BlueprintReadOnly, meta = (BindWidget), Category = "Main Menu")
	TObjectPtr<UButton> LoadGameButton;

	UPROPERTY(BlueprintReadOnly, meta = (BindWidget), Category = "Main Menu")
	TObjectPtr<UButton> QuitButton;

	/** Container for save slot entries in the load game panel. */
	UPROPERTY(BlueprintReadOnly, meta = (BindWidgetOptional), Category = "Main Menu")
	TObjectPtr<UVerticalBox> SaveSlotList;

	/** Status text for feedback (e.g., "No saves found"). */
	UPROPERTY(BlueprintReadOnly, meta = (BindWidgetOptional), Category = "Main Menu")
	TObjectPtr<UTextBlock> StatusText;

	// --- Events for external systems ---

	/** Fired when user clicks New Game. */
	DECLARE_DYNAMIC_MULTICAST_DELEGATE(FOnNewGameRequested);
	UPROPERTY(BlueprintAssignable, Category = "Main Menu")
	FOnNewGameRequested OnNewGameRequested;

	/** Fired when user selects a save slot to load. */
	DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FOnLoadSlotSelected, const FString&, SlotName);
	UPROPERTY(BlueprintAssignable, Category = "Main Menu")
	FOnLoadSlotSelected OnLoadSlotSelected;

	/** Fired when the widget needs the save slot list refreshed. GameInstance should respond. */
	DECLARE_DYNAMIC_MULTICAST_DELEGATE(FOnRefreshSaveSlotsRequested);
	UPROPERTY(BlueprintAssignable, Category = "Main Menu")
	FOnRefreshSaveSlotsRequested OnRefreshSaveSlotsRequested;

	/** Fired when the widget wants to delete a save slot. GameInstance should respond. */
	DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FOnDeleteSaveSlotRequested, const FString&, SlotName);
	UPROPERTY(BlueprintAssignable, Category = "Main Menu")
	FOnDeleteSaveSlotRequested OnDeleteSaveSlotRequested;

	// --- Public API ---

	/** Request a save slot list refresh (fires delegate for external handling). */
	UFUNCTION(BlueprintCallable, Category = "Main Menu")
	void RefreshSaveSlotList();

	/** Request deletion of a save slot (fires delegate for external handling). */
	UFUNCTION(BlueprintCallable, Category = "Main Menu")
	void DeleteSaveSlot(const FString& SlotName);

	/** Load a specific save slot (fires OnLoadSlotSelected delegate). */
	UFUNCTION(BlueprintCallable, Category = "Main Menu")
	void LoadSaveSlot(const FString& SlotName);

	/** Called by external code (GameInstance) to provide save slot data. */
	UFUNCTION(BlueprintCallable, Category = "Main Menu")
	void SetSaveSlots(const TArray<FGTSaveSlotInfo>& Slots);

	/** Get the currently cached save slots. */
	UFUNCTION(BlueprintPure, Category = "Main Menu")
	const TArray<FGTSaveSlotInfo>& GetCachedSaveSlots() const { return CachedSaveSlots; }

protected:
	UFUNCTION()
	void HandleNewGameClicked();

	UFUNCTION()
	void HandleLoadGameClicked();

	UFUNCTION()
	void HandleQuitClicked();

	/** Cached save slot info from last refresh. */
	UPROPERTY()
	TArray<FGTSaveSlotInfo> CachedSaveSlots;
};
