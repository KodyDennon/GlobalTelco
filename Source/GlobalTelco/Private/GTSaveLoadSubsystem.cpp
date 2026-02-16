#include "GTSaveLoadSubsystem.h"
#include "GTSaveGame.h"
#include "GTSimulationSubsystem.h"
#include "GTCorporationManager.h"
#include "GTLandParcelSystem.h"
#include "GTRegionalEconomy.h"
#include "Kismet/GameplayStatics.h"
#include "Engine/World.h"
#include "Engine/GameInstance.h"

void UGTSaveLoadSubsystem::Initialize(FSubsystemCollectionBase& Collection)
{
	Super::Initialize(Collection);

	// Scan for existing save slots on disk.
	TArray<FString> FoundFiles;
	const FString SaveDir = FPaths::ProjectSavedDir() / TEXT("SaveGames");

	IFileManager::Get().FindFiles(FoundFiles, *SaveDir, TEXT(".sav"));

	for (const FString& FileName : FoundFiles)
	{
		// Our saves use the convention "GT_<SlotName>.sav".
		if (FileName.StartsWith(TEXT("GT_")))
		{
			FString SlotName = FileName;
			SlotName.RemoveFromEnd(TEXT(".sav"));
			SlotName.RemoveFromStart(TEXT("GT_"));
			KnownSlotNames.AddUnique(SlotName);
		}
	}

	UE_LOG(LogTemp, Log, TEXT("GTSaveLoad: Initialized — found %d existing save slots."), KnownSlotNames.Num());
}

bool UGTSaveLoadSubsystem::SaveGame(const FString& SlotName, const FString& DisplayName,
	int32 PlayerCorporationId, const FString& PlayerCorpName)
{
	UGameInstance* GI = GetGameInstance();
	if (!GI)
	{
		UE_LOG(LogTemp, Error, TEXT("GTSaveLoad: No GameInstance available."));
		return false;
	}

	UWorld* World = GI->GetWorld();
	if (!World)
	{
		UE_LOG(LogTemp, Error, TEXT("GTSaveLoad: No World available for save."));
		return false;
	}

	// Create the save game object.
	UGTSaveGame* SaveGameObj = NewObject<UGTSaveGame>(this);
	SaveGameObj->SaveDisplayName = DisplayName;

	// Capture the full world state.
	SaveGameObj->CaptureWorldState(World, PlayerCorporationId, PlayerCorpName);

	// Serialize to disk.
	const FString InternalSlot = GetInternalSlotName(SlotName);
	const bool bSuccess = UGameplayStatics::SaveGameToSlot(SaveGameObj, InternalSlot, UserIndex);

	if (bSuccess)
	{
		KnownSlotNames.AddUnique(SlotName);
		LastLoadedSave = SaveGameObj;
		UE_LOG(LogTemp, Log, TEXT("GTSaveLoad: Saved to slot '%s' (internal: '%s')."), *SlotName, *InternalSlot);
	}
	else
	{
		UE_LOG(LogTemp, Error, TEXT("GTSaveLoad: Failed to save to slot '%s'."), *SlotName);
	}

	return bSuccess;
}

bool UGTSaveLoadSubsystem::LoadGame(const FString& SlotName)
{
	const FString InternalSlot = GetInternalSlotName(SlotName);

	if (!UGameplayStatics::DoesSaveGameExist(InternalSlot, UserIndex))
	{
		UE_LOG(LogTemp, Warning, TEXT("GTSaveLoad: Slot '%s' does not exist."), *SlotName);
		return false;
	}

	USaveGame* LoadedBase = UGameplayStatics::LoadGameFromSlot(InternalSlot, UserIndex);
	UGTSaveGame* LoadedSave = Cast<UGTSaveGame>(LoadedBase);

	if (!LoadedSave)
	{
		UE_LOG(LogTemp, Error, TEXT("GTSaveLoad: Failed to deserialize slot '%s'."), *SlotName);
		return false;
	}

	// Version check — forward compatibility guard.
	if (LoadedSave->SaveVersion > GT_SAVE_VERSION)
	{
		UE_LOG(LogTemp, Error,
			TEXT("GTSaveLoad: Save version %d is newer than current version %d. Cannot load."),
			LoadedSave->SaveVersion, GT_SAVE_VERSION);
		return false;
	}

	UGameInstance* GI = GetGameInstance();
	UWorld* World = GI ? GI->GetWorld() : nullptr;

	if (!World)
	{
		UE_LOG(LogTemp, Error, TEXT("GTSaveLoad: No World available for restore."));
		return false;
	}

	// Restore world state from save.
	LoadedSave->RestoreWorldState(World);

	// Restore simulation tick state.
	UGTSimulationSubsystem* SimSub = World->GetSubsystem<UGTSimulationSubsystem>();
	if (SimSub)
	{
		SimSub->EconomicTickInterval = LoadedSave->WorldSettings.TickIntervalSeconds;
	}

	LastLoadedSave = LoadedSave;
	KnownSlotNames.AddUnique(SlotName);

	UE_LOG(LogTemp, Log, TEXT("GTSaveLoad: Loaded slot '%s' — tick %lld, %d parcels, %d corps."),
		*SlotName, LoadedSave->SimulationTick, LoadedSave->AllParcels.Num(), LoadedSave->Corporations.Num());

	return true;
}

bool UGTSaveLoadSubsystem::DeleteSave(const FString& SlotName)
{
	const FString InternalSlot = GetInternalSlotName(SlotName);

	if (!UGameplayStatics::DoesSaveGameExist(InternalSlot, UserIndex))
	{
		UE_LOG(LogTemp, Warning, TEXT("GTSaveLoad: Cannot delete — slot '%s' does not exist."), *SlotName);
		return false;
	}

	const bool bDeleted = UGameplayStatics::DeleteGameInSlot(InternalSlot, UserIndex);

	if (bDeleted)
	{
		KnownSlotNames.Remove(SlotName);
		UE_LOG(LogTemp, Log, TEXT("GTSaveLoad: Deleted slot '%s'."), *SlotName);
	}
	else
	{
		UE_LOG(LogTemp, Error, TEXT("GTSaveLoad: Failed to delete slot '%s'."), *SlotName);
	}

	return bDeleted;
}

bool UGTSaveLoadSubsystem::DoesSlotExist(const FString& SlotName) const
{
	const FString InternalSlot = GetInternalSlotName(SlotName);
	return UGameplayStatics::DoesSaveGameExist(InternalSlot, UserIndex);
}

TArray<FGTSaveSlotInfo> UGTSaveLoadSubsystem::GetAllSaveSlots() const
{
	TArray<FGTSaveSlotInfo> Results;
	Results.Reserve(KnownSlotNames.Num());

	for (const FString& SlotName : KnownSlotNames)
	{
		const FString InternalSlot = GetInternalSlotName(SlotName);

		if (!UGameplayStatics::DoesSaveGameExist(InternalSlot, UserIndex))
		{
			continue;
		}

		USaveGame* LoadedBase = UGameplayStatics::LoadGameFromSlot(InternalSlot, UserIndex);
		UGTSaveGame* SaveObj = Cast<UGTSaveGame>(LoadedBase);

		if (SaveObj)
		{
			Results.Add(SaveObj->GetSlotInfo(SlotName));
		}
	}

	// Sort by timestamp descending (most recent first).
	Results.Sort([](const FGTSaveSlotInfo& A, const FGTSaveSlotInfo& B)
	{
		return A.SaveTimestamp > B.SaveTimestamp;
	});

	return Results;
}

bool UGTSaveLoadSubsystem::AutoSave(int32 PlayerCorporationId, const FString& PlayerCorpName)
{
	return SaveGame(TEXT("AutoSave"), TEXT("Auto Save"), PlayerCorporationId, PlayerCorpName);
}

FString UGTSaveLoadSubsystem::GetInternalSlotName(const FString& SlotName)
{
	return FString::Printf(TEXT("GT_%s"), *SlotName);
}
