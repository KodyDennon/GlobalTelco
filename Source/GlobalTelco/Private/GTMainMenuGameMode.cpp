#include "GTMainMenuGameMode.h"
#include "GTMainMenuWidget.h"
#include "GTNewGameWidget.h"
#include "GTGameInstance.h"
#include "GTSaveLoadSubsystem.h"
#include "GTWorldSettings.h"
#include "Blueprint/UserWidget.h"
#include "Kismet/GameplayStatics.h"
#include "Engine/World.h"

AGTMainMenuGameMode::AGTMainMenuGameMode()
{
	// Main menu doesn't need a pawn or player controller with gameplay features.
	DefaultPawnClass = nullptr;
}

void AGTMainMenuGameMode::StartPlay()
{
	Super::StartPlay();

	APlayerController* PC = GetWorld()->GetFirstPlayerController();
	if (!PC)
	{
		return;
	}

	// Show the mouse cursor on the main menu.
	PC->bShowMouseCursor = true;
	PC->SetInputMode(FInputModeUIOnly());

	// Create the main menu widget if the class is set.
	if (MainMenuWidgetClass)
	{
		MainMenuWidget = CreateWidget<UGTMainMenuWidget>(PC, MainMenuWidgetClass);
		if (MainMenuWidget)
		{
			MainMenuWidget->OnNewGameRequested.AddDynamic(this, &AGTMainMenuGameMode::HandleNewGameRequested);
			MainMenuWidget->OnLoadSlotSelected.AddDynamic(this, &AGTMainMenuGameMode::HandleLoadSlotSelected);
			MainMenuWidget->OnRefreshSaveSlotsRequested.AddDynamic(this, &AGTMainMenuGameMode::HandleRefreshSaveSlotsRequested);
			MainMenuWidget->OnDeleteSaveSlotRequested.AddDynamic(this, &AGTMainMenuGameMode::HandleDeleteSaveSlotRequested);

			MainMenuWidget->AddToViewport();
		}
	}

	// Create the new game widget if the class is set (hidden initially).
	if (NewGameWidgetClass)
	{
		NewGameWidget = CreateWidget<UGTNewGameWidget>(PC, NewGameWidgetClass);
		if (NewGameWidget)
		{
			NewGameWidget->OnStartGameRequested.AddDynamic(this, &AGTMainMenuGameMode::HandleStartGameRequested);
			NewGameWidget->OnBackRequested.AddDynamic(this, &AGTMainMenuGameMode::HandleBackToMainMenu);
			// Don't add to viewport yet — shown when user clicks New Game.
		}
	}
}

void AGTMainMenuGameMode::HandleNewGameRequested()
{
	ShowNewGamePanel();
}

void AGTMainMenuGameMode::HandleLoadSlotSelected(const FString& SlotName)
{
	UGTGameInstance* GTGI = Cast<UGTGameInstance>(GetGameInstance());
	if (GTGI)
	{
		GTGI->PrepareLoadGame(SlotName);
		UGameplayStatics::OpenLevel(GetWorld(), *GameWorldMapName,
			false, TEXT("?game=/Script/GlobalTelco.GTSinglePlayerGameMode"));
	}
}

void AGTMainMenuGameMode::HandleRefreshSaveSlotsRequested()
{
	UGTSaveLoadSubsystem* SaveLoad = GetGameInstance()->GetSubsystem<UGTSaveLoadSubsystem>();
	if (SaveLoad && MainMenuWidget)
	{
		TArray<FGTSaveSlotInfo> Slots = SaveLoad->GetAllSaveSlots();
		MainMenuWidget->SetSaveSlots(Slots);
	}
}

void AGTMainMenuGameMode::HandleDeleteSaveSlotRequested(const FString& SlotName)
{
	UGTSaveLoadSubsystem* SaveLoad = GetGameInstance()->GetSubsystem<UGTSaveLoadSubsystem>();
	if (SaveLoad)
	{
		SaveLoad->DeleteSave(SlotName);

		// Refresh the list after deletion.
		HandleRefreshSaveSlotsRequested();
	}
}

void AGTMainMenuGameMode::HandleStartGameRequested(UGTWorldSettings* WorldSettings, const FString& CorporationName)
{
	UGTGameInstance* GTGI = Cast<UGTGameInstance>(GetGameInstance());
	if (GTGI && WorldSettings)
	{
		GTGI->PrepareNewGame(WorldSettings, CorporationName);
		UGameplayStatics::OpenLevel(GetWorld(), *GameWorldMapName,
			false, TEXT("?game=/Script/GlobalTelco.GTSinglePlayerGameMode"));
	}
}

void AGTMainMenuGameMode::HandleBackToMainMenu()
{
	ShowMainMenu();
}

void AGTMainMenuGameMode::ShowMainMenu()
{
	if (NewGameWidget && NewGameWidget->IsInViewport())
	{
		NewGameWidget->RemoveFromParent();
	}

	if (MainMenuWidget && !MainMenuWidget->IsInViewport())
	{
		MainMenuWidget->AddToViewport();
	}
}

void AGTMainMenuGameMode::ShowNewGamePanel()
{
	if (MainMenuWidget && MainMenuWidget->IsInViewport())
	{
		MainMenuWidget->RemoveFromParent();
	}

	if (NewGameWidget && !NewGameWidget->IsInViewport())
	{
		NewGameWidget->AddToViewport();
	}
}
