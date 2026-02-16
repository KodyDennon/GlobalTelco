#include "GTGameInstance.h"
#include "GTWorldSettings.h"

UGTGameInstance::UGTGameInstance()
{
}

void UGTGameInstance::Init()
{
	Super::Init();

	UE_LOG(LogTemp, Log, TEXT("GTGameInstance: Initialized."));
}

void UGTGameInstance::ClearPendingSession()
{
	PendingWorldSettings = nullptr;
	PendingPlayerCorpName = TEXT("Player Corp");
	bPendingLoadFromSave = false;
	PendingLoadSlotName.Empty();
}

void UGTGameInstance::PrepareNewGame(UGTWorldSettings* WorldSettings, const FString& PlayerCorpName)
{
	PendingWorldSettings = WorldSettings;
	PendingPlayerCorpName = PlayerCorpName;
	bPendingLoadFromSave = false;
	PendingLoadSlotName.Empty();

	UE_LOG(LogTemp, Log, TEXT("GTGameInstance: Prepared new game — Corp: '%s', Difficulty: %d"),
		*PlayerCorpName, static_cast<int32>(WorldSettings ? WorldSettings->Difficulty : EGTDifficulty::Normal));
}

void UGTGameInstance::PrepareLoadGame(const FString& SlotName)
{
	PendingWorldSettings = nullptr;
	PendingPlayerCorpName.Empty();
	bPendingLoadFromSave = true;
	PendingLoadSlotName = SlotName;

	UE_LOG(LogTemp, Log, TEXT("GTGameInstance: Prepared load game — Slot: '%s'"), *SlotName);
}
