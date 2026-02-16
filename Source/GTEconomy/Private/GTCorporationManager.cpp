#include "GTCorporationManager.h"
#include "GTCorporation.h"

void UGTCorporationManager::Initialize(FSubsystemCollectionBase& Collection)
{
	Super::Initialize(Collection);
	NextCorporationId = 0;
}

void UGTCorporationManager::Deinitialize()
{
	Corporations.Empty();
	Super::Deinitialize();
}

int32 UGTCorporationManager::CreateCorporation(const FString& Name, double StartingCapital, bool bIsAI, int32 ArchetypeIndex)
{
	const int32 Id = NextCorporationId++;

	UGTCorporation* Corp = NewObject<UGTCorporation>(this);
	Corp->CorporationId = Id;
	Corp->CorporationName = Name;
	Corp->bIsAI = bIsAI;
	Corp->ArchetypeIndex = ArchetypeIndex;

	// Initialize balance sheet with starting capital.
	Corp->BalanceSheet.CashOnHand = StartingCapital;
	Corp->BalanceSheet.TotalAssets = StartingCapital;
	Corp->BalanceSheet.TotalLiabilities = 0.0;
	Corp->BalanceSheet.InfrastructureValue = 0.0;

	Corp->TotalDebt = 0.0;
	Corp->CreditRating = EGTCreditRating::A;

	Corporations.Add(Id, Corp);

	UE_LOG(LogTemp, Log, TEXT("GTCorporationManager: Created %s corporation '%s' (ID=%d, Capital=$%.0f)"),
		bIsAI ? TEXT("AI") : TEXT("Player"), *Name, Id, StartingCapital);

	return Id;
}

bool UGTCorporationManager::DestroyCorporation(int32 CorporationId)
{
	if (!Corporations.Contains(CorporationId))
	{
		return false;
	}

	UGTCorporation* Corp = Corporations[CorporationId];
	UE_LOG(LogTemp, Log, TEXT("GTCorporationManager: Destroyed corporation '%s' (ID=%d)"),
		*Corp->CorporationName, CorporationId);

	Corporations.Remove(CorporationId);
	return true;
}

UGTCorporation* UGTCorporationManager::GetCorporation(int32 CorporationId) const
{
	const TObjectPtr<UGTCorporation>* Found = Corporations.Find(CorporationId);
	return Found ? Found->Get() : nullptr;
}

TArray<UGTCorporation*> UGTCorporationManager::GetAllCorporations() const
{
	TArray<UGTCorporation*> Result;
	Result.Reserve(Corporations.Num());
	for (const auto& Pair : Corporations)
	{
		Result.Add(Pair.Value.Get());
	}
	return Result;
}

TArray<UGTCorporation*> UGTCorporationManager::GetAICorporations() const
{
	TArray<UGTCorporation*> Result;
	for (const auto& Pair : Corporations)
	{
		if (Pair.Value && Pair.Value->bIsAI)
		{
			Result.Add(Pair.Value.Get());
		}
	}
	return Result;
}

TArray<UGTCorporation*> UGTCorporationManager::GetPlayerCorporations() const
{
	TArray<UGTCorporation*> Result;
	for (const auto& Pair : Corporations)
	{
		if (Pair.Value && !Pair.Value->bIsAI)
		{
			Result.Add(Pair.Value.Get());
		}
	}
	return Result;
}

void UGTCorporationManager::ProcessAllCorporationTicks(float TickDeltaSeconds)
{
	for (auto& Pair : Corporations)
	{
		if (Pair.Value)
		{
			Pair.Value->ProcessEconomicTick(TickDeltaSeconds);
		}
	}
}
