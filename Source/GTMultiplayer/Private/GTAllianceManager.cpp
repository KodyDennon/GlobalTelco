#include "GTAllianceManager.h"

void UGTAllianceManager::Initialize(FSubsystemCollectionBase& Collection)
{
	Super::Initialize(Collection);
	NextAllianceId = 0;
	NextContractId = 0;
}

void UGTAllianceManager::Deinitialize()
{
	Alliances.Empty();
	Contracts.Empty();
	Super::Deinitialize();
}

// --- Alliances ---

int32 UGTAllianceManager::ProposeAlliance(const FString& Name, const TArray<int32>& MemberCorporationIds)
{
	const int32 Id = NextAllianceId++;

	FGTAlliance Alliance;
	Alliance.AllianceId = Id;
	Alliance.AllianceName = Name;
	Alliance.Status = EGTAllianceStatus::Proposed;
	Alliance.MemberCorporationIds = MemberCorporationIds;

	Alliances.Add(Id, Alliance);
	return Id;
}

bool UGTAllianceManager::ActivateAlliance(int32 AllianceId)
{
	FGTAlliance* Alliance = Alliances.Find(AllianceId);
	if (!Alliance || Alliance->Status != EGTAllianceStatus::Proposed)
	{
		return false;
	}

	Alliance->Status = EGTAllianceStatus::Active;
	return true;
}

void UGTAllianceManager::DissolveAlliance(int32 AllianceId)
{
	if (FGTAlliance* Alliance = Alliances.Find(AllianceId))
	{
		Alliance->Status = EGTAllianceStatus::Dissolved;
	}
}

FGTAlliance UGTAllianceManager::GetAlliance(int32 AllianceId) const
{
	const FGTAlliance* Found = Alliances.Find(AllianceId);
	return Found ? *Found : FGTAlliance();
}

TArray<int32> UGTAllianceManager::GetCorporationAlliances(int32 CorporationId) const
{
	TArray<int32> Result;
	for (const auto& Pair : Alliances)
	{
		if (Pair.Value.Status == EGTAllianceStatus::Active &&
			Pair.Value.MemberCorporationIds.Contains(CorporationId))
		{
			Result.Add(Pair.Key);
		}
	}
	return Result;
}

// --- Contracts ---

int32 UGTAllianceManager::ProposeContract(const FGTContract& InContract)
{
	const int32 Id = NextContractId++;

	FGTContract Contract = InContract;
	Contract.ContractId = Id;
	Contract.Status = EGTContractStatus::Proposed;

	Contracts.Add(Id, Contract);
	return Id;
}

bool UGTAllianceManager::AcceptContract(int32 ContractId)
{
	FGTContract* Contract = Contracts.Find(ContractId);
	if (!Contract || Contract->Status != EGTContractStatus::Proposed)
	{
		return false;
	}

	Contract->Status = EGTContractStatus::Active;
	return true;
}

void UGTAllianceManager::TerminateContract(int32 ContractId)
{
	if (FGTContract* Contract = Contracts.Find(ContractId))
	{
		if (Contract->Status == EGTContractStatus::Active)
		{
			Contract->Status = EGTContractStatus::Terminated;
			// Breach penalty handling will be done by the economy module.
		}
	}
}

FGTContract UGTAllianceManager::GetContract(int32 ContractId) const
{
	const FGTContract* Found = Contracts.Find(ContractId);
	return Found ? *Found : FGTContract();
}

void UGTAllianceManager::ProcessContractTick(int64 CurrentTick)
{
	for (auto& Pair : Contracts)
	{
		FGTContract& Contract = Pair.Value;

		if (Contract.Status != EGTContractStatus::Active)
		{
			continue;
		}

		// Check expiration.
		if (Contract.DurationTicks > 0 &&
			(CurrentTick - Contract.StartTick) >= Contract.DurationTicks)
		{
			Contract.Status = EGTContractStatus::Expired;
			continue;
		}

		// Per-tick payment processing will be handled by the economy module
		// through the centralized event queue.
	}
}
