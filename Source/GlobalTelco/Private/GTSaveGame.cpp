#include "GTSaveGame.h"
#include "GTSimulationSubsystem.h"
#include "GTCorporationManager.h"
#include "GTCorporation.h"
#include "GTLandParcelSystem.h"
#include "GTRegionalEconomy.h"
#include "GTAllianceManager.h"
#include "GTNetworkGraph.h"

void UGTSaveGame::CaptureWorldState(UWorld* World, int32 InPlayerCorporationId, const FString& InPlayerCorpName)
{
	if (!World)
	{
		return;
	}

	SaveTimestamp = FDateTime::Now();
	SaveVersion = GT_SAVE_VERSION;
	PlayerCorporationId = InPlayerCorporationId;
	PlayerCorporationName = InPlayerCorpName;

	// Simulation state.
	UGTSimulationSubsystem* SimSub = World->GetSubsystem<UGTSimulationSubsystem>();
	if (SimSub)
	{
		SimulationTick = SimSub->GetCurrentTick();
		SimulationTimeSeconds = World->GetTimeSeconds();

		// Capture world settings.
		WorldSettings.TickIntervalSeconds = SimSub->EconomicTickInterval;
	}

	// Parcels.
	UGTLandParcelSystem* ParcelSystem = World->GetSubsystem<UGTLandParcelSystem>();
	if (ParcelSystem)
	{
		AllParcels.Empty();
		const int32 ParcelCount = ParcelSystem->GetParcelCount();
		AllParcels.Reserve(ParcelCount);
		for (int32 i = 0; i < ParcelCount; ++i)
		{
			AllParcels.Add(ParcelSystem->GetParcel(i));
		}
	}

	// Corporations.
	UGTCorporationManager* CorpManager = World->GetSubsystem<UGTCorporationManager>();
	if (CorpManager)
	{
		Corporations.Empty();
		const TArray<UGTCorporation*> AllCorps = CorpManager->GetAllCorporations();
		for (const UGTCorporation* Corp : AllCorps)
		{
			if (!Corp)
			{
				continue;
			}

			FGTSavedCorporation Saved;
			Saved.CorporationId = Corp->CorporationId;
			Saved.CorporationName = Corp->CorporationName;
			Saved.bIsAI = Corp->bIsAI;
			Saved.ArchetypeIndex = Corp->ArchetypeIndex;
			Saved.BalanceSheet = Corp->BalanceSheet;
			Saved.LastTickIncome = Corp->LastTickIncome;
			Saved.CreditRating = Corp->CreditRating;
			Saved.TotalDebt = Corp->TotalDebt;
			Saved.DebtInstruments = Corp->DebtInstruments;
			Saved.ShareholderEquity = Corp->ShareholderEquity;
			Saved.OwnedNodeIds = Corp->OwnedNodeIds;
			Saved.OwnedEdgeIds = Corp->OwnedEdgeIds;
			Corporations.Add(Saved);
		}
	}

	// Regional economy.
	UGTRegionalEconomy* Economy = World->GetSubsystem<UGTRegionalEconomy>();
	if (Economy)
	{
		Regions.Empty();
		// Get region count from world settings.
		const int32 RegionCount = WorldSettings.RegionCount;
		for (int32 i = 0; i < RegionCount; ++i)
		{
			Regions.Add(Economy->GetRegionData(i));
		}
	}

	// Contracts and alliances.
	UGTAllianceManager* AllianceMgr = World->GetSubsystem<UGTAllianceManager>();
	if (AllianceMgr)
	{
		// The alliance manager doesn't expose iteration yet —
		// we save what we can track. For now, contracts are tracked
		// on the manager. We'll capture them when the API supports it.
		ActiveContracts.Empty();
		ActiveAlliances.Empty();
	}

	UE_LOG(LogTemp, Log, TEXT("GTSaveGame: Captured world state — tick %lld, %d parcels, %d corps"),
		SimulationTick, AllParcels.Num(), Corporations.Num());
}

void UGTSaveGame::RestoreWorldState(UWorld* World) const
{
	if (!World)
	{
		return;
	}

	// Restore parcels.
	UGTLandParcelSystem* ParcelSystem = World->GetSubsystem<UGTLandParcelSystem>();
	if (ParcelSystem)
	{
		// Re-register all parcels. Clear existing first.
		// Note: The subsystem Initialize clears parcels, so if called after
		// subsystem init, we just register all saved parcels.
		for (const FGTLandParcel& Parcel : AllParcels)
		{
			ParcelSystem->RegisterParcel(Parcel);
		}
	}

	// Restore corporations.
	UGTCorporationManager* CorpManager = World->GetSubsystem<UGTCorporationManager>();
	if (CorpManager)
	{
		for (const FGTSavedCorporation& Saved : Corporations)
		{
			int32 CorpId = CorpManager->CreateCorporation(
				Saved.CorporationName,
				Saved.BalanceSheet.CashOnHand,
				Saved.bIsAI,
				Saved.ArchetypeIndex
			);

			UGTCorporation* Corp = CorpManager->GetCorporation(CorpId);
			if (Corp)
			{
				Corp->BalanceSheet = Saved.BalanceSheet;
				Corp->LastTickIncome = Saved.LastTickIncome;
				Corp->CreditRating = Saved.CreditRating;
				Corp->TotalDebt = Saved.TotalDebt;
				Corp->DebtInstruments = Saved.DebtInstruments;
				Corp->ShareholderEquity = Saved.ShareholderEquity;
				Corp->OwnedNodeIds = Saved.OwnedNodeIds;
				Corp->OwnedEdgeIds = Saved.OwnedEdgeIds;
			}
		}
	}

	// Restore regional economy.
	UGTRegionalEconomy* Economy = World->GetSubsystem<UGTRegionalEconomy>();
	if (Economy)
	{
		for (const FGTRegionalEconomyData& Region : Regions)
		{
			Economy->RegisterRegion(Region);
		}
	}

	UE_LOG(LogTemp, Log, TEXT("GTSaveGame: Restored world state — tick %lld, %d parcels, %d corps"),
		SimulationTick, AllParcels.Num(), Corporations.Num());
}

FGTSaveSlotInfo UGTSaveGame::GetSlotInfo(const FString& SlotName) const
{
	FGTSaveSlotInfo Info;
	Info.SlotName = SlotName;
	Info.SaveDisplayName = SaveDisplayName;
	Info.SaveTimestamp = SaveTimestamp;
	// Convert difficulty enum to string name for the slot info.
	switch (WorldSettings.Difficulty)
	{
	case EGTDifficulty::Easy:    Info.DifficultyName = TEXT("Easy"); break;
	case EGTDifficulty::Normal:  Info.DifficultyName = TEXT("Normal"); break;
	case EGTDifficulty::Hard:    Info.DifficultyName = TEXT("Hard"); break;
	case EGTDifficulty::Custom:  Info.DifficultyName = TEXT("Custom"); break;
	default:                     Info.DifficultyName = TEXT("Normal"); break;
	}
	Info.SimulationTick = SimulationTick;
	Info.PlayerCorporationName = PlayerCorporationName;
	return Info;
}
