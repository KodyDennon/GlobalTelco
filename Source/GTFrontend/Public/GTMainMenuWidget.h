#pragma once

#include "CoreMinimal.h"
#include "Blueprint/UserWidget.h"
#include "GTSaveGame.h"
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

	// --- Events for Blueprint to handle transitions ---

	/** Fired when user clicks New Game. Blueprint should open the new game settings widget. */
	DECLARE_DYNAMIC_MULTICAST_DELEGATE(FOnNewGameRequested);
	UPROPERTY(BlueprintAssignable, Category = "Main Menu")
	FOnNewGameRequested OnNewGameRequested;

	/** Fired when user selects a save slot to load. */
	DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FOnLoadSlotSelected, const FString&, SlotName);
	UPROPERTY(BlueprintAssignable, Category = "Main Menu")
	FOnLoadSlotSelected OnLoadSlotSelected;

	// --- Public API ---

	/** Refresh the save slot list from the save/load subsystem. */
	UFUNCTION(BlueprintCallable, Category = "Main Menu")
	void RefreshSaveSlotList();

	/** Delete a save slot and refresh the list. */
	UFUNCTION(BlueprintCallable, Category = "Main Menu")
	void DeleteSaveSlot(const FString& SlotName);

	/** Load a specific save slot (triggers level transition). */
	UFUNCTION(BlueprintCallable, Category = "Main Menu")
	void LoadSaveSlot(const FString& SlotName);

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
