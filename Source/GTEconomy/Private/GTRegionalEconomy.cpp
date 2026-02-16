#include "GTRegionalEconomy.h"

void UGTRegionalEconomy::Initialize(FSubsystemCollectionBase& Collection)
{
	Super::Initialize(Collection);
	NextRegionId = 0;
}

void UGTRegionalEconomy::Deinitialize()
{
	Regions.Empty();
	ConnectivityScores.Empty();
	Super::Deinitialize();
}

int32 UGTRegionalEconomy::RegisterRegion(const FGTRegionalEconomyData& Data)
{
	const int32 Id = NextRegionId++;
	FGTRegionalEconomyData Region = Data;
	Region.RegionId = Id;
	Regions.Add(Id, Region);
	ConnectivityScores.Add(Id, 0.0f);
	return Id;
}

FGTRegionalEconomyData UGTRegionalEconomy::GetRegionData(int32 RegionId) const
{
	const FGTRegionalEconomyData* Found = Regions.Find(RegionId);
	return Found ? *Found : FGTRegionalEconomyData();
}

void UGTRegionalEconomy::ProcessEconomicTick(float TickDeltaSeconds)
{
	for (auto& Pair : Regions)
	{
		FGTRegionalEconomyData& Region = Pair.Value;
		float Connectivity = ConnectivityScores.FindRef(Region.RegionId);

		// Connectivity improves GDP growth.
		double GDPGrowthRate = 0.001 + (Connectivity * 0.01);
		Region.GDPProxy *= (1.0 + GDPGrowthRate);

		// Connectivity improves political stability (slowly).
		Region.PoliticalStability = FMath::Clamp(
			Region.PoliticalStability + (Connectivity * 0.001f),
			0.0f, 1.0f
		);

		// Data demand grows based on population, GDP, tech adoption, and connectivity.
		Region.CurrentDemand = Region.Population
			* Region.TechAdoptionIndex
			* (Region.GDPProxy / 1000.0)
			* (1.0f + Region.DataDemandGrowthRate)
			* (0.5f + Connectivity * 0.5f);

		// Technology adoption creeps up with connectivity and urbanization.
		Region.TechAdoptionIndex = FMath::Clamp(
			Region.TechAdoptionIndex + (Connectivity * Region.UrbanizationIndex * 0.0005f),
			0.0f, 1.0f
		);
	}
}

double UGTRegionalEconomy::GetGlobalDemand() const
{
	double Total = 0.0;
	for (const auto& Pair : Regions)
	{
		Total += Pair.Value.CurrentDemand;
	}
	return Total;
}

void UGTRegionalEconomy::SetRegionConnectivity(int32 RegionId, float ConnectivityScore)
{
	ConnectivityScores.Add(RegionId, FMath::Clamp(ConnectivityScore, 0.0f, 1.0f));
}
