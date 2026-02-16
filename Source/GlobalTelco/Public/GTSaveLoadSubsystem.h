#pragma once

#include "CoreMinimal.h"
#include "Subsystems/GameInstanceSubsystem.h"
#include "GTSaveGame.h"
#include "GTSaveLoadSubsystem.generated.h"

/**
 * UGTSaveLoadSubsystem
 *
 * Game instance subsystem that manages save/load operations for single-player.
 * Provides save, load, delete, and enumeration of save slots.
 * Uses UE5's built-in USaveGame serialization (FMemoryArchive binary format).
 *
 * Save slots are stored in the platform's save directory (typically
 * Saved/SaveGames/) with the naming convention "GT_<SlotName>".
 */
UCLASS()
class GLOBALTELCO_API UGTSaveLoadSubsystem : public UGameInstanceSubsystem
{
	GENERATED_BODY()

public:
	virtual void Initialize(FSubsystemCollectionBase& Collection) override;

	/**
	 * Save the current game state to a named slot.
	 * @param SlotName Name of the save slot (e.g., "Save1", "AutoSave").
	 * @param DisplayName User-facing display name for this save.
	 * @param PlayerCorporationId The player's corporation ID.
	 * @param PlayerCorpName The player's corporation name.
	 * @return True if save succeeded.
	 */
	UFUNCTION(BlueprintCallable, Category = "Save/Load")
	bool SaveGame(const FString& SlotName, const FString& DisplayName,
		int32 PlayerCorporationId, const FString& PlayerCorpName);

	/**
	 * Load a saved game from a named slot.
	 * @param SlotName Name of the save slot to load.
	 * @return True if load succeeded. World state is restored.
	 */
	UFUNCTION(BlueprintCallable, Category = "Save/Load")
	bool LoadGame(const FString& SlotName);

	/** Delete a save slot. */
	UFUNCTION(BlueprintCallable, Category = "Save/Load")
	bool DeleteSave(const FString& SlotName);

	/** Check if a save slot exists. */
	UFUNCTION(BlueprintPure, Category = "Save/Load")
	bool DoesSlotExist(const FString& SlotName) const;

	/** Get metadata for all existing save slots. */
	UFUNCTION(BlueprintCallable, Category = "Save/Load")
	TArray<FGTSaveSlotInfo> GetAllSaveSlots() const;

	/** Auto-save to the "AutoSave" slot. Call periodically from simulation. */
	UFUNCTION(BlueprintCallable, Category = "Save/Load")
	bool AutoSave(int32 PlayerCorporationId, const FString& PlayerCorpName);

	/** Get the last loaded save game (for inspection/debugging). */
	UFUNCTION(BlueprintPure, Category = "Save/Load")
	UGTSaveGame* GetLastLoadedSave() const { return LastLoadedSave; }

private:
	/** Convert a user-facing slot name to the internal save slot name. */
	static FString GetInternalSlotName(const FString& SlotName);

	/** Known save slot names (cached for enumeration). */
	UPROPERTY()
	TArray<FString> KnownSlotNames;

	/** Last loaded save for reference. */
	UPROPERTY()
	TObjectPtr<UGTSaveGame> LastLoadedSave;

	/** User index for save operations (always 0 for single-player). */
	static constexpr int32 UserIndex = 0;
};
