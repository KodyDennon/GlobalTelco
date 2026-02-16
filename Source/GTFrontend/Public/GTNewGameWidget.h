#pragma once

#include "CoreMinimal.h"
#include "Blueprint/UserWidget.h"
#include "GTWorldSettings.h"
#include "GTNewGameWidget.generated.h"

class UEditableTextBox;
class UComboBoxString;
class USlider;
class UTextBlock;
class UButton;

/**
 * UGTNewGameWidget
 *
 * New game configuration widget. Exposes all world settings to the player:
 * corporation name, difficulty, AI count, disaster severity, world seed.
 * On start, creates a UGTWorldSettings, passes it to the GameInstance,
 * and opens the game level.
 *
 * Subclass in Blueprints for visual layout.
 */
UCLASS(Abstract, Blueprintable)
class GTFRONTEND_API UGTNewGameWidget : public UUserWidget
{
	GENERATED_BODY()

public:
	virtual void NativeConstruct() override;

	// --- Bound widgets ---

	UPROPERTY(BlueprintReadOnly, meta = (BindWidget), Category = "New Game")
	TObjectPtr<UEditableTextBox> CorporationNameInput;

	UPROPERTY(BlueprintReadOnly, meta = (BindWidget), Category = "New Game")
	TObjectPtr<UComboBoxString> DifficultyDropdown;

	UPROPERTY(BlueprintReadOnly, meta = (BindWidget), Category = "New Game")
	TObjectPtr<USlider> AICorpCountSlider;

	UPROPERTY(BlueprintReadOnly, meta = (BindWidgetOptional), Category = "New Game")
	TObjectPtr<UTextBlock> AICorpCountLabel;

	UPROPERTY(BlueprintReadOnly, meta = (BindWidget), Category = "New Game")
	TObjectPtr<UComboBoxString> DisasterSeverityDropdown;

	UPROPERTY(BlueprintReadOnly, meta = (BindWidgetOptional), Category = "New Game")
	TObjectPtr<UEditableTextBox> WorldSeedInput;

	UPROPERTY(BlueprintReadOnly, meta = (BindWidget), Category = "New Game")
	TObjectPtr<UButton> StartGameButton;

	UPROPERTY(BlueprintReadOnly, meta = (BindWidgetOptional), Category = "New Game")
	TObjectPtr<UButton> BackButton;

	// --- Events ---

	/** Fired when user clicks Back. Blueprint should show main menu. */
	DECLARE_DYNAMIC_MULTICAST_DELEGATE(FOnBackRequested);
	UPROPERTY(BlueprintAssignable, Category = "New Game")
	FOnBackRequested OnBackRequested;

	/** Fired when game start is requested. Carries the built world settings and corp name.
	 *  The listener (e.g., Blueprint or GameInstance code in GlobalTelco module) should
	 *  set up the GameInstance and open the game level. */
	DECLARE_DYNAMIC_MULTICAST_DELEGATE_TwoParams(FOnStartGameRequested, UGTWorldSettings*, WorldSettings, const FString&, CorporationName);
	UPROPERTY(BlueprintAssignable, Category = "New Game")
	FOnStartGameRequested OnStartGameRequested;

	// --- Public API ---

	/** Get the configured world settings from the current UI state. */
	UFUNCTION(BlueprintCallable, Category = "New Game")
	UGTWorldSettings* BuildWorldSettings() const;

	/** Get the entered corporation name. */
	UFUNCTION(BlueprintPure, Category = "New Game")
	FString GetCorporationName() const;

protected:
	UFUNCTION()
	void HandleStartGameClicked();

	UFUNCTION()
	void HandleBackClicked();

	UFUNCTION()
	void HandleAISliderChanged(float Value);

	UFUNCTION()
	void HandleDifficultyChanged(FString SelectedItem, ESelectInfo::Type SelectionType);

	/** Map level to open when starting the game. */
	UPROPERTY(EditDefaultsOnly, BlueprintReadOnly, Category = "New Game")
	FString GameMapName = TEXT("/Game/Maps/GameWorld");
};
