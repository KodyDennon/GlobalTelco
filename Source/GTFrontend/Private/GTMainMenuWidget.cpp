#include "GTMainMenuWidget.h"
#include "Components/Button.h"
#include "Components/VerticalBox.h"
#include "Components/TextBlock.h"
#include "Kismet/KismetSystemLibrary.h"

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

	// Request initial save slot data from GameInstance.
	RefreshSaveSlotList();
}

void UGTMainMenuWidget::HandleNewGameClicked()
{
	OnNewGameRequested.Broadcast();
}

void UGTMainMenuWidget::HandleLoadGameClicked()
{
	RefreshSaveSlotList();
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
	// Fire delegate — external code (GameInstance) should call SetSaveSlots() in response.
	OnRefreshSaveSlotsRequested.Broadcast();
}

void UGTMainMenuWidget::DeleteSaveSlot(const FString& SlotName)
{
	// Fire delegate — external code (GameInstance) handles deletion and refreshes.
	OnDeleteSaveSlotRequested.Broadcast(SlotName);
}

void UGTMainMenuWidget::LoadSaveSlot(const FString& SlotName)
{
	OnLoadSlotSelected.Broadcast(SlotName);
}

void UGTMainMenuWidget::SetSaveSlots(const TArray<FGTSaveSlotInfo>& Slots)
{
	CachedSaveSlots = Slots;

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
}
