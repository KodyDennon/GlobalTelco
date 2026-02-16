#include "GTWorldGenerator.h"
#include "GTWorldSettings.h"
#include "GTGeodesicGrid.h"
#include "GTLandParcelSystem.h"
#include "GTRegionalEconomy.h"

void UGTWorldGenerator::Initialize(FSubsystemCollectionBase& Collection)
{
	Super::Initialize(Collection);
	bWorldGenerated = false;
}

void UGTWorldGenerator::Deinitialize()
{
	Super::Deinitialize();
}

void UGTWorldGenerator::GenerateWorld(UGTWorldSettings* Settings)
{
	if (!Settings)
	{
		UE_LOG(LogTemp, Error, TEXT("GTWorldGenerator: Cannot generate world — no settings provided."));
		return;
	}

	UWorld* World = GetWorld();
	if (!World)
	{
		return;
	}

	UGTGeodesicGrid* Grid = World->GetSubsystem<UGTGeodesicGrid>();
	UGTLandParcelSystem* ParcelSystem = World->GetSubsystem<UGTLandParcelSystem>();
	UGTRegionalEconomy* Economy = World->GetSubsystem<UGTRegionalEconomy>();

	if (!Grid || !ParcelSystem || !Economy)
	{
		UE_LOG(LogTemp, Error, TEXT("GTWorldGenerator: Required subsystems not available."));
		return;
	}

	const int32 Seed = (Settings->WorldSeed != 0) ? Settings->WorldSeed : FMath::Rand();
	FMath::SRandInit(Seed);

	// Step 1: Generate the geodesic grid.
	Grid->GenerateGrid(Settings->HexGridResolution);

	// Step 2: Create a parcel for each grid cell.
	GenerateParcels(Settings, Grid, ParcelSystem);

	// Step 3: Assign terrain types.
	AssignTerrain(Settings, ParcelSystem, Grid);

	// Step 4: Create regions and assign parcels to them.
	GenerateRegions(Settings, Grid, ParcelSystem, Economy);

	// Step 5: Seed economic data.
	SeedEconomicData(Settings, Economy, ParcelSystem);

	// Step 6: Assign zoning.
	AssignZoning(Settings, ParcelSystem);

	bWorldGenerated = true;
	UE_LOG(LogTemp, Log, TEXT("GTWorldGenerator: World generated — %d parcels, %d regions."),
		ParcelSystem->GetParcelCount(), Settings->RegionCount);
}

void UGTWorldGenerator::GenerateParcels(UGTWorldSettings* Settings, UGTGeodesicGrid* Grid, UGTLandParcelSystem* ParcelSystem)
{
	const TArray<FGTGeodesicCell>& Cells = Grid->GetAllCells();

	for (const FGTGeodesicCell& Cell : Cells)
	{
		FGTLandParcel Parcel;
		Parcel.HexCoordinates = FVector2D(Cell.HexCoord.Q, Cell.HexCoord.R);
		Parcel.GeodesicCellIndex = Cell.CellIndex;
		Parcel.Longitude = Cell.Longitude;
		Parcel.Latitude = Cell.Latitude;
		Parcel.OwnershipType = EGTParcelOwnership::Government;
		Parcel.OwnerCorporationId = -1;

		ParcelSystem->RegisterParcel(Parcel);
	}
}

void UGTWorldGenerator::AssignTerrain(UGTWorldSettings* Settings, UGTLandParcelSystem* ParcelSystem, UGTGeodesicGrid* Grid)
{
	const int32 Seed = (Settings->WorldSeed != 0) ? Settings->WorldSeed : 42;
	const TArray<FGTGeodesicCell>& Cells = Grid->GetAllCells();

	for (int32 i = 0; i < Cells.Num(); ++i)
	{
		const FGTGeodesicCell& Cell = Cells[i];
		const int32 ParcelId = ParcelSystem->FindParcelByCellIndex(Cell.CellIndex);
		if (ParcelId < 0)
		{
			continue;
		}

		FGTLandParcel Parcel = ParcelSystem->GetParcel(ParcelId);
		const double AbsLat = FMath::Abs(Cell.Latitude);

		// Generate terrain noise for variation.
		const double NoiseVal = Noise2D(Cell.Longitude * 0.1, Cell.Latitude * 0.1, Seed);
		const double ElevationNoise = Noise2D(Cell.Longitude * 0.05, Cell.Latitude * 0.05, Seed + 1);

		// Determine terrain type based on latitude + noise heuristics.
		EGTTerrainType Terrain;

		if (AbsLat > 75.0)
		{
			// Polar regions.
			Terrain = EGTTerrainType::Desert; // Ice desert.
		}
		else if (AbsLat < 5.0 && NoiseVal < -0.2)
		{
			// Near equator, low noise = ocean.
			Terrain = EGTTerrainType::OceanDeep;
		}
		else if (NoiseVal < -0.4)
		{
			Terrain = EGTTerrainType::OceanDeep;
		}
		else if (NoiseVal < -0.2)
		{
			Terrain = EGTTerrainType::OceanShallow;
		}
		else if (NoiseVal < -0.05)
		{
			Terrain = EGTTerrainType::Coastal;
		}
		else if (ElevationNoise > 0.5)
		{
			Terrain = EGTTerrainType::Mountainous;
		}
		else if (AbsLat > 20.0 && AbsLat < 35.0 && NoiseVal > 0.3)
		{
			Terrain = EGTTerrainType::Desert;
		}
		else if (NoiseVal > 0.4)
		{
			Terrain = EGTTerrainType::Urban;
		}
		else if (NoiseVal > 0.2)
		{
			Terrain = EGTTerrainType::Suburban;
		}
		else
		{
			Terrain = EGTTerrainType::Rural;
		}

		Parcel.Terrain = Terrain;

		// Set disaster risk based on terrain.
		switch (Terrain)
		{
		case EGTTerrainType::Coastal:
			Parcel.DisasterRisk = 0.4f;
			break;
		case EGTTerrainType::Mountainous:
			Parcel.DisasterRisk = 0.3f;
			break;
		case EGTTerrainType::OceanShallow:
		case EGTTerrainType::OceanDeep:
			Parcel.DisasterRisk = 0.5f;
			break;
		case EGTTerrainType::Desert:
			Parcel.DisasterRisk = 0.2f;
			break;
		default:
			Parcel.DisasterRisk = 0.1f;
			break;
		}

		// Set labor cost multiplier by terrain.
		switch (Terrain)
		{
		case EGTTerrainType::Urban:
			Parcel.LaborCostMultiplier = 1.5f;
			break;
		case EGTTerrainType::Mountainous:
			Parcel.LaborCostMultiplier = 2.0f;
			break;
		case EGTTerrainType::OceanShallow:
		case EGTTerrainType::OceanDeep:
			Parcel.LaborCostMultiplier = 3.0f;
			break;
		default:
			Parcel.LaborCostMultiplier = 1.0f;
			break;
		}

		ParcelSystem->UpdateParcel(ParcelId, Parcel);
	}
}

void UGTWorldGenerator::GenerateRegions(UGTWorldSettings* Settings, UGTGeodesicGrid* Grid, UGTLandParcelSystem* ParcelSystem, UGTRegionalEconomy* Economy)
{
	const int32 NumRegions = FMath::Max(Settings->RegionCount, 1);
	const int32 CellCount = Grid->GetCellCount();

	if (CellCount == 0)
	{
		return;
	}

	// K-means-like clustering on sphere positions.
	// Initialize region centers uniformly distributed across cells.
	TArray<FVector> RegionCenters;
	RegionCenters.Reserve(NumRegions);

	const int32 Step = FMath::Max(CellCount / NumRegions, 1);
	for (int32 i = 0; i < NumRegions; ++i)
	{
		const int32 CellIdx = (i * Step) % CellCount;
		RegionCenters.Add(Grid->GetCell(CellIdx).UnitSpherePosition);
	}

	// Assign each cell to nearest region center (single pass for performance).
	TArray<int32> CellRegionAssignments;
	CellRegionAssignments.SetNumZeroed(CellCount);

	for (int32 CellIdx = 0; CellIdx < CellCount; ++CellIdx)
	{
		const FVector& CellPos = Grid->GetCell(CellIdx).UnitSpherePosition;
		int32 BestRegion = 0;
		double BestDot = -2.0;

		for (int32 RegIdx = 0; RegIdx < NumRegions; ++RegIdx)
		{
			const double Dot = FVector::DotProduct(CellPos, RegionCenters[RegIdx]);
			if (Dot > BestDot)
			{
				BestDot = Dot;
				BestRegion = RegIdx;
			}
		}

		CellRegionAssignments[CellIdx] = BestRegion;
	}

	// Refine: run 3 iterations of K-means to improve clustering.
	for (int32 Iter = 0; Iter < 3; ++Iter)
	{
		// Recompute centers.
		TArray<FVector> NewCenters;
		NewCenters.SetNumZeroed(NumRegions);
		TArray<int32> Counts;
		Counts.SetNumZeroed(NumRegions);

		for (int32 CellIdx = 0; CellIdx < CellCount; ++CellIdx)
		{
			const int32 RegIdx = CellRegionAssignments[CellIdx];
			NewCenters[RegIdx] += Grid->GetCell(CellIdx).UnitSpherePosition;
			Counts[RegIdx]++;
		}

		for (int32 RegIdx = 0; RegIdx < NumRegions; ++RegIdx)
		{
			if (Counts[RegIdx] > 0)
			{
				NewCenters[RegIdx] /= static_cast<double>(Counts[RegIdx]);
				NewCenters[RegIdx].Normalize();
			}
			else
			{
				// Empty region — keep old center.
				NewCenters[RegIdx] = RegionCenters[RegIdx];
			}
		}

		RegionCenters = MoveTemp(NewCenters);

		// Reassign cells.
		for (int32 CellIdx = 0; CellIdx < CellCount; ++CellIdx)
		{
			const FVector& CellPos = Grid->GetCell(CellIdx).UnitSpherePosition;
			int32 BestRegion = 0;
			double BestDot = -2.0;

			for (int32 RegIdx = 0; RegIdx < NumRegions; ++RegIdx)
			{
				const double Dot = FVector::DotProduct(CellPos, RegionCenters[RegIdx]);
				if (Dot > BestDot)
				{
					BestDot = Dot;
					BestRegion = RegIdx;
				}
			}

			CellRegionAssignments[CellIdx] = BestRegion;
		}
	}

	// Register regions in the economy subsystem and assign RegionId to parcels.
	for (int32 RegIdx = 0; RegIdx < NumRegions; ++RegIdx)
	{
		FGTRegionalEconomyData RegionData;
		RegionData.RegionName = FString::Printf(TEXT("Region_%d"), RegIdx);

		// Compute center lon/lat for region naming.
		double CenterLon, CenterLat;
		const double CenterLatRad = FMath::Asin(FMath::Clamp(static_cast<double>(RegionCenters[RegIdx].Z), -1.0, 1.0));
		CenterLat = FMath::RadiansToDegrees(CenterLatRad);
		CenterLon = FMath::RadiansToDegrees(FMath::Atan2(
			static_cast<double>(RegionCenters[RegIdx].Y),
			static_cast<double>(RegionCenters[RegIdx].X)));

		Economy->RegisterRegion(RegionData);
	}

	// Write region assignments back to parcels.
	for (int32 CellIdx = 0; CellIdx < CellCount; ++CellIdx)
	{
		FGTLandParcel Parcel = ParcelSystem->GetParcel(CellIdx);
		Parcel.RegionId = CellRegionAssignments[CellIdx];
		ParcelSystem->UpdateParcel(CellIdx, Parcel);
	}
}

void UGTWorldGenerator::SeedEconomicData(UGTWorldSettings* Settings, UGTRegionalEconomy* Economy, UGTLandParcelSystem* ParcelSystem)
{
	const int32 Seed = (Settings->WorldSeed != 0) ? Settings->WorldSeed : 42;
	const int32 NumRegions = Settings->RegionCount;

	for (int32 RegIdx = 0; RegIdx < NumRegions; ++RegIdx)
	{
		FGTRegionalEconomyData Data = Economy->GetRegionData(RegIdx);

		// Use noise to vary economic parameters.
		const double RegNoise = Noise2D(RegIdx * 13.7, RegIdx * 7.3, Seed + 100);

		// Population: 100k to 50M based on noise.
		Data.Population = FMath::Lerp(100000.0, 50000000.0, (RegNoise + 1.0) * 0.5);

		// GDP proxy scales with population and noise.
		const double GdpNoise = Noise2D(RegIdx * 5.1, RegIdx * 11.9, Seed + 200);
		Data.GDPProxy = Data.Population * FMath::Lerp(1000.0, 60000.0, (GdpNoise + 1.0) * 0.5);

		// Tech adoption correlates with GDP per capita.
		const double GdpPerCapita = Data.GDPProxy / FMath::Max(Data.Population, 1.0);
		Data.TechAdoptionIndex = FMath::Clamp(static_cast<float>(GdpPerCapita / 60000.0), 0.05f, 0.95f);

		// Political stability.
		const double StabNoise = Noise2D(RegIdx * 3.3, RegIdx * 9.7, Seed + 300);
		Data.PoliticalStability = FMath::Clamp(static_cast<float>((StabNoise + 1.0) * 0.5), 0.2f, 0.95f);

		// Data demand growth rate.
		Data.DataDemandGrowthRate = 0.03f + Data.TechAdoptionIndex * 0.07f;

		// Business density correlates with urbanization.
		Data.BusinessDensity = FMath::Clamp(Data.TechAdoptionIndex * 1.2f, 0.1f, 0.9f);

		// Urbanization index.
		Data.UrbanizationIndex = FMath::Clamp(Data.TechAdoptionIndex * 0.8f + 0.1f, 0.1f, 0.95f);

		// Initial demand.
		Data.CurrentDemand = Data.Population * Data.TechAdoptionIndex * 0.001;

		// Apply difficulty multiplier.
		Data.CurrentDemand *= Settings->DemandGrowthMultiplier;

		// Update region in economy.
		// The economy subsystem assigned sequential IDs, so RegIdx == RegionId.
		Economy->SetRegionConnectivity(RegIdx, 0.0f); // Start with no connectivity.
	}
}

void UGTWorldGenerator::AssignZoning(UGTWorldSettings* Settings, UGTLandParcelSystem* ParcelSystem)
{
	const int32 ParcelCount = ParcelSystem->GetParcelCount();

	for (int32 i = 0; i < ParcelCount; ++i)
	{
		FGTLandParcel Parcel = ParcelSystem->GetParcel(i);

		switch (Parcel.Terrain)
		{
		case EGTTerrainType::Urban:
			Parcel.Zoning = EGTZoningCategory::Commercial;
			break;
		case EGTTerrainType::Suburban:
			Parcel.Zoning = EGTZoningCategory::Residential;
			break;
		case EGTTerrainType::Rural:
			Parcel.Zoning = EGTZoningCategory::Unrestricted;
			break;
		case EGTTerrainType::Mountainous:
			Parcel.Zoning = EGTZoningCategory::Protected;
			break;
		case EGTTerrainType::Desert:
			Parcel.Zoning = EGTZoningCategory::Industrial;
			break;
		case EGTTerrainType::Coastal:
			Parcel.Zoning = EGTZoningCategory::Commercial;
			break;
		case EGTTerrainType::OceanShallow:
		case EGTTerrainType::OceanDeep:
			Parcel.Zoning = EGTZoningCategory::Unrestricted;
			break;
		}

		ParcelSystem->UpdateParcel(i, Parcel);
	}
}

double UGTWorldGenerator::Noise2D(double X, double Y, int32 Seed)
{
	// Simple value noise based on integer lattice hashing.
	auto Hash = [](int32 IX, int32 IY, int32 S) -> double
	{
		int32 N = IX * 374761393 + IY * 668265263 + S;
		N = (N ^ (N >> 13)) * 1274126177;
		N = N ^ (N >> 16);
		return static_cast<double>(N & 0x7FFFFFFF) / static_cast<double>(0x7FFFFFFF) * 2.0 - 1.0;
	};

	const int32 IX = FMath::FloorToInt32(X);
	const int32 IY = FMath::FloorToInt32(Y);
	const double FX = X - IX;
	const double FY = Y - IY;

	// Smoothstep interpolation.
	const double SX = FX * FX * (3.0 - 2.0 * FX);
	const double SY = FY * FY * (3.0 - 2.0 * FY);

	const double N00 = Hash(IX, IY, Seed);
	const double N10 = Hash(IX + 1, IY, Seed);
	const double N01 = Hash(IX, IY + 1, Seed);
	const double N11 = Hash(IX + 1, IY + 1, Seed);

	const double NX0 = FMath::Lerp(N00, N10, SX);
	const double NX1 = FMath::Lerp(N01, N11, SX);

	return FMath::Lerp(NX0, NX1, SY);
}
