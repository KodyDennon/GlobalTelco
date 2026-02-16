#pragma once

#include "CoreMinimal.h"
#include "Subsystems/WorldSubsystem.h"
#include "GTMultiplayerTypes.h"
#include "GTAllianceManager.generated.h"

/**
 * UGTAllianceManager
 *
 * World subsystem managing alliances and contracts between corporations.
 * Handles creation, dissolution, and per-tick contract processing
 * (payments, breach detection, expiration).
 */
UCLASS()
class GTMULTIPLAYER_API UGTAllianceManager : public UWorldSubsystem
{
	GENERATED_BODY()

public:
	virtual void Initialize(FSubsystemCollectionBase& Collection) override;
	virtual void Deinitialize() override;

	// --- Alliances ---

	/** Propose a new alliance. Returns AllianceId. */
	UFUNCTION(BlueprintCallable, Category = "Alliances")
	int32 ProposeAlliance(const FString& Name, const TArray<int32>& MemberCorporationIds);

	/** Accept a proposed alliance, making it active. */
	UFUNCTION(BlueprintCallable, Category = "Alliances")
	bool ActivateAlliance(int32 AllianceId);

	/** Dissolve an active alliance. */
	UFUNCTION(BlueprintCallable, Category = "Alliances")
	void DissolveAlliance(int32 AllianceId);

	UFUNCTION(BlueprintPure, Category = "Alliances")
	FGTAlliance GetAlliance(int32 AllianceId) const;

	/** Get all alliances a corporation belongs to. */
	UFUNCTION(BlueprintPure, Category = "Alliances")
	TArray<int32> GetCorporationAlliances(int32 CorporationId) const;

	// --- Contracts ---

	/** Create a new contract proposal. Returns ContractId. */
	UFUNCTION(BlueprintCallable, Category = "Contracts")
	int32 ProposeContract(const FGTContract& Contract);

	/** Accept a proposed contract, making it active. */
	UFUNCTION(BlueprintCallable, Category = "Contracts")
	bool AcceptContract(int32 ContractId);

	/** Terminate an active contract (may trigger breach penalties). */
	UFUNCTION(BlueprintCallable, Category = "Contracts")
	void TerminateContract(int32 ContractId);

	UFUNCTION(BlueprintPure, Category = "Contracts")
	FGTContract GetContract(int32 ContractId) const;

	/** Process all active contracts for one economic tick. */
	UFUNCTION(BlueprintCallable, Category = "Contracts")
	void ProcessContractTick(int64 CurrentTick);

private:
	TMap<int32, FGTAlliance> Alliances;
	TMap<int32, FGTContract> Contracts;

	int32 NextAllianceId = 0;
	int32 NextContractId = 0;
};
