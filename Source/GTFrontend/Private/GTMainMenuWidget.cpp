#include "GTMainMenuWidget.h"
#include "GTSaveLoadSubsystem.h"
#include "Components/Button.h"
#include "Components/VerticalBox.h"
#include "Components/TextBlock.h"
#include "Kismet/GameplayStatics.h"
#include "Kismet/KismetSystemLibrary.h"
#include "Engine/GameInstance.h"

void UGTMainMenuWidget::NativeConstruct()
{
	Super::NativeConstruct();

	if (NewGameButton)
	{
		NewGameButton->OnClicked.AddDynamic(this, &UGTMainMenuWidget::HandleNewGameClicked);
	}

	if (LoadGameButton)
	{
		LoadGameButton->OnClicked.AddDynamic(this, &UGTMainMenuWidget::HandleLoadGameClicked);
	}

	if (QuitButton)
	{
		QuitButton->OnClicked.AddDynamic(this, &UGTMainMenuWidget::HandleQuitClicked);
	}

	// Initially populate save slots.
	RefreshSaveSlotList();
}

void UGTMainMenuWidget::HandleNewGameClicked()
{
	OnNewGameRequested.Broadcast();
}

void UGTMainMenuWidget::HandleLoadGameClicked()
{
	RefreshSaveSlotList();

	// If save slot list container is visible, the Blueprint layout
	// should toggle it. We just refresh the data here.
}

void UGTMainMenuWidget::HandleQuitClicked()
{
	UKismetSystemLibrary::QuitGame(
		GetWorld(),
		GetOwningPlayer(),
		EQuitPreference::Quit,
		false
	);
}

void UGTMainMenuWidget::RefreshSaveSlotList()
{
	UGameInstance* GI = GetGameInstance();
	if (!GI)
	{
		return;
	}

	UGTSaveLoadSubsystem* SaveLoad = GI->GetSubsystem<UGTSaveLoadSubsystem>();
	if (!SaveLoad)
	{
		if (StatusText)
		{
			StatusText->SetText(FText::FromString(TEXT("Save system unavailable.")));
		}
		return;
	}

	CachedSaveSlots = SaveLoad->GetAllSaveSlots();

	if (StatusText)
	{
		if (CachedSaveSlots.Num() == 0)
		{
			StatusText->SetText(FText::FromString(TEXT("No saved games found.")));
		}
		else
		{
			StatusText->SetText(FText::FromString(
				FString::Printf(TEXT("%d saved game(s) found."), CachedSaveSlots.Num())
			));
		}
	}

	// Clear and repopulate the save slot list container.
	// Blueprint subclass should override and create entry widgets.
	// Here we provide the data for Blueprints to consume.
}

void UGTMainMenuWidget::DeleteSaveSlot(const FString& SlotName)
{
	UGameInstance* GI = GetGameInstance();
	if (!GI)
	{
		return;
	}

	UGTSaveLoadSubsystem* SaveLoad = GI->GetSubsystem<UGTSaveLoadSubsystem>();
	if (SaveLoad)
	{
		SaveLoad->DeleteSave(SlotName);
		RefreshSaveSlotList();
	}
}

void UGTMainMenuWidget::LoadSaveSlot(const FString& SlotName)
{
	OnLoadSlotSelected.Broadcast(SlotName);
}
