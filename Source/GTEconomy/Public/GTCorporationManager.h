#pragma once

#include "CoreMinimal.h"
#include "Subsystems/WorldSubsystem.h"
#include "GTCorporationManager.generated.h"

class UGTCorporation;

/**
 * UGTCorporationManager
 *
 * World subsystem that manages the lifecycle of all corporations in the simulation.
 * Creates, tracks, and processes corporations for both player-owned and AI-owned entities.
 * Provides lookup by ID and filtering by ownership type (player vs AI).
 *
 * In single-player, this subsystem creates the player's corporation and all AI
 * competitors. In multiplayer, it creates one corporation per connected player.
 */
UCLASS()
class GTECONOMY_API UGTCorporationManager : public UWorldSubsystem
{
	GENERATED_BODY()

public:
	virtual void Initialize(FSubsystemCollectionBase& Collection) override;
	virtual void Deinitialize() override;

	/**
	 * Create a new corporation and register it in the manager.
	 * @param Name Display name for the corporation.
	 * @param StartingCapital Initial cash on hand (USD).
	 * @param bIsAI True if this corporation is AI-controlled.
	 * @param ArchetypeIndex Index into AI archetype registry (-1 for player corps).
	 * @return The assigned CorporationId (sequential, starting from 0).
	 */
	UFUNCTION(BlueprintCallable, Category = "Corporations")
	int32 CreateCorporation(const FString& Name, double StartingCapital, bool bIsAI, int32 ArchetypeIndex = -1);

	/** Destroy a corporation (bankruptcy, dissolution). Removes from tracking. */
	UFUNCTION(BlueprintCallable, Category = "Corporations")
	bool DestroyCorporation(int32 CorporationId);

	/** Look up a corporation by ID. Returns nullptr if not found. */
	UFUNCTION(BlueprintPure, Category = "Corporations")
	UGTCorporation* GetCorporation(int32 CorporationId) const;

	/** Get all living corporations (player + AI). */
	UFUNCTION(BlueprintPure, Category = "Corporations")
	TArray<UGTCorporation*> GetAllCorporations() const;

	/** Get only AI-controlled corporations. */
	UFUNCTION(BlueprintPure, Category = "Corporations")
	TArray<UGTCorporation*> GetAICorporations() const;

	/** Get only player-controlled corporations. */
	UFUNCTION(BlueprintPure, Category = "Corporations")
	TArray<UGTCorporation*> GetPlayerCorporations() const;

	/** Process one economic tick for all corporations. */
	UFUNCTION(BlueprintCallable, Category = "Corporations")
	void ProcessAllCorporationTicks(float TickDeltaSeconds);

	/** Total number of active corporations. */
	UFUNCTION(BlueprintPure, Category = "Corporations")
	int32 GetCorporationCount() const { return Corporations.Num(); }

	/** Check if a corporation ID is valid and active. */
	UFUNCTION(BlueprintPure, Category = "Corporations")
	bool IsValidCorporation(int32 CorporationId) const { return Corporations.Contains(CorporationId); }

private:
	UPROPERTY()
	TMap<int32, TObjectPtr<UGTCorporation>> Corporations;

	int32 NextCorporationId = 0;
};
