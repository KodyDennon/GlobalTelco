#pragma once

#include "CoreMinimal.h"
#include "GameFramework/GameModeBase.h"
#include "GTMainMenuGameMode.generated.h"

class UGTMainMenuWidget;
class UGTNewGameWidget;

/**
 * AGTMainMenuGameMode
 *
 * Game mode for the main menu level. Creates the menu UI and wires all
 * delegate connections between the UI widgets (GTFrontend module) and
 * the game systems (GlobalTelco module: GameInstance, SaveLoadSubsystem).
 *
 * Set this as the GameMode override for the MainMenu map in World Settings.
 */
UCLASS()
class GLOBALTELCO_API AGTMainMenuGameMode : public AGameModeBase
{
	GENERATED_BODY()

public:
	AGTMainMenuGameMode();

	virtual void StartPlay() override;

	/** Widget class to instantiate for the main menu. Must be a Blueprint subclass. */
	UPROPERTY(EditDefaultsOnly, BlueprintReadOnly, Category = "Main Menu")
	TSubclassOf<UGTMainMenuWidget> MainMenuWidgetClass;

	/** Widget class for the new game settings panel. Must be a Blueprint subclass. */
	UPROPERTY(EditDefaultsOnly, BlueprintReadOnly, Category = "Main Menu")
	TSubclassOf<UGTNewGameWidget> NewGameWidgetClass;

	/** Map name for the game world level. */
	UPROPERTY(EditDefaultsOnly, BlueprintReadOnly, Category = "Main Menu")
	FString GameWorldMapName = TEXT("/Game/Maps/GameWorld");

protected:
	// --- Delegate handlers for main menu widget ---

	UFUNCTION()
	void HandleNewGameRequested();

	UFUNCTION()
	void HandleLoadSlotSelected(const FString& SlotName);

	UFUNCTION()
	void HandleRefreshSaveSlotsRequested();

	UFUNCTION()
	void HandleDeleteSaveSlotRequested(const FString& SlotName);

	// --- Delegate handlers for new game widget ---

	UFUNCTION()
	void HandleStartGameRequested(UGTWorldSettings* WorldSettings, const FString& CorporationName);

	UFUNCTION()
	void HandleBackToMainMenu();

	/** Show the main menu and hide new game panel. */
	void ShowMainMenu();

	/** Show the new game panel and hide the main menu. */
	void ShowNewGamePanel();

private:
	UPROPERTY()
	TObjectPtr<UGTMainMenuWidget> MainMenuWidget;

	UPROPERTY()
	TObjectPtr<UGTNewGameWidget> NewGameWidget;
};
