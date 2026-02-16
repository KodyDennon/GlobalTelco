#pragma once

#include "CoreMinimal.h"
#include "Engine/GameInstance.h"
#include "GTGameInstance.generated.h"

class UGTWorldSettings;

/**
 * UGTGameInstance
 *
 * Game instance that persists across level transitions.
 * Bridges the main menu (settings selection) to the gameplay level.
 * Holds pending world settings and player configuration for
 * the next game session to consume in its GameMode::InitGame().
 *
 * For single-player: stores world settings, player corp name,
 * and whether we're loading from a save vs starting a new game.
 */
UCLASS()
class GLOBALTELCO_API UGTGameInstance : public UGameInstance
{
	GENERATED_BODY()

public:
	UGTGameInstance();

	virtual void Init() override;

	// --- Session Configuration (set by menu, consumed by GameMode) ---

	/** World settings for the next single-player session. */
	UPROPERTY(BlueprintReadWrite, Category = "Session")
	TObjectPtr<UGTWorldSettings> PendingWorldSettings;

	/** Player corporation name for the next session. */
	UPROPERTY(BlueprintReadWrite, Category = "Session")
	FString PendingPlayerCorpName = TEXT("Player Corp");

	/** Whether the pending session should load from a save. */
	UPROPERTY(BlueprintReadWrite, Category = "Session")
	bool bPendingLoadFromSave = false;

	/** Save slot name to load from (if bPendingLoadFromSave is true). */
	UPROPERTY(BlueprintReadWrite, Category = "Session")
	FString PendingLoadSlotName;

	// --- Active Session State ---

	/** The current player's corporation ID (set during gameplay). */
	UPROPERTY(BlueprintReadOnly, Category = "Session")
	int32 ActivePlayerCorporationId = -1;

	/** The current player's corporation name (set during gameplay). */
	UPROPERTY(BlueprintReadOnly, Category = "Session")
	FString ActivePlayerCorpName;

	// --- Utility ---

	/** Clear pending session data after the GameMode consumes it. */
	UFUNCTION(BlueprintCallable, Category = "Session")
	void ClearPendingSession();

	/** Set up a new game session from menu selections. */
	UFUNCTION(BlueprintCallable, Category = "Session")
	void PrepareNewGame(UGTWorldSettings* WorldSettings, const FString& PlayerCorpName);

	/** Set up a load-from-save session from menu selection. */
	UFUNCTION(BlueprintCallable, Category = "Session")
	void PrepareLoadGame(const FString& SlotName);
};
