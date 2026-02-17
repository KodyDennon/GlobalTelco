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

	// Step 2: Generate elevation map using fractal noise on sphere.
	GenerateElevation(Grid, Seed);

	// Step 3: Create a parcel for each grid cell.
	GenerateParcels(Settings, Grid, ParcelSystem);

	// Step 4: Assign terrain types based on elevation and latitude.
	AssignTerrain(Settings, ParcelSystem, Grid);

	// Step 5: Create regions and assign parcels to them.
	GenerateRegions(Settings, Grid, ParcelSystem, Economy);

	// Step 6: Seed economic data.
	SeedEconomicData(Settings, Economy, ParcelSystem);

	// Step 7: Assign zoning.
	AssignZoning(Settings, ParcelSystem);

	bWorldGenerated = true;

	// Count terrain distribution for debug logging.
	int32 LandCount = 0;
	int32 OceanCount = 0;
	for (int32 i = 0; i < Grid->GetCellCount(); ++i)
	{
		if (Grid->GetCell(i).bIsLand)
		{
			LandCount++;
		}
		else
		{
			OceanCount++;
		}
	}

	UE_LOG(LogTemp, Log, TEXT("GTWorldGenerator: World generated — %d parcels, %d regions, %d land, %d ocean (%.0f%% land)."),
		ParcelSystem->GetParcelCount(), Settings->RegionCount, LandCount, OceanCount,
		ParcelSystem->GetParcelCount() > 0 ? 100.0f * LandCount / ParcelSystem->GetParcelCount() : 0.0f);
}

// --- Noise Functions ---

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

double UGTWorldGenerator::Noise3D(double X, double Y, double Z, int32 Seed)
{
	// 3D value noise using integer lattice hashing. Trilinear interpolation.
	auto Hash = [](int32 IX, int32 IY, int32 IZ, int32 S) -> double
	{
		int32 N = IX * 374761393 + IY * 668265263 + IZ * 1274126177 + S;
		N = (N ^ (N >> 13)) * 1103515245;
		N = (N ^ (N >> 16)) * 2654435761;
		N = N ^ (N >> 16);
		return static_cast<double>(N & 0x7FFFFFFF) / static_cast<double>(0x7FFFFFFF) * 2.0 - 1.0;
	};

	const int32 IX = FMath::FloorToInt32(X);
	const int32 IY = FMath::FloorToInt32(Y);
	const int32 IZ = FMath::FloorToInt32(Z);
	const double FX = X - IX;
	const double FY = Y - IY;
	const double FZ = Z - IZ;

	// Smoothstep.
	const double SX = FX * FX * (3.0 - 2.0 * FX);
	const double SY = FY * FY * (3.0 - 2.0 * FY);
	const double SZ = FZ * FZ * (3.0 - 2.0 * FZ);

	const double N000 = Hash(IX, IY, IZ, Seed);
	const double N100 = Hash(IX + 1, IY, IZ, Seed);
	const double N010 = Hash(IX, IY + 1, IZ, Seed);
	const double N110 = Hash(IX + 1, IY + 1, IZ, Seed);
	const double N001 = Hash(IX, IY, IZ + 1, Seed);
	const double N101 = Hash(IX + 1, IY, IZ + 1, Seed);
	const double N011 = Hash(IX, IY + 1, IZ + 1, Seed);
	const double N111 = Hash(IX + 1, IY + 1, IZ + 1, Seed);

	// Trilinear interpolation.
	const double NX00 = FMath::Lerp(N000, N100, SX);
	const double NX10 = FMath::Lerp(N010, N110, SX);
	const double NX01 = FMath::Lerp(N001, N101, SX);
	const double NX11 = FMath::Lerp(N011, N111, SX);

	const double NXY0 = FMath::Lerp(NX00, NX10, SY);
	const double NXY1 = FMath::Lerp(NX01, NX11, SY);

	return FMath::Lerp(NXY0, NXY1, SZ);
}

double UGTWorldGenerator::FractalNoise3D(const FVector& Pos, int32 Octaves, double Frequency, double Persistence, int32 Seed)
{
	// Fractional Brownian Motion (FBM) using 3D noise on unit sphere positions.
	// This avoids the seam artifacts that happen with lon/lat noise sampling.
	double Total = 0.0;
	double Amplitude = 1.0;
	double MaxAmplitude = 0.0;
	double Freq = Frequency;

	for (int32 i = 0; i < Octaves; ++i)
	{
		Total += Noise3D(Pos.X * Freq, Pos.Y * Freq, Pos.Z * Freq, Seed + i * 31) * Amplitude;
		MaxAmplitude += Amplitude;
		Amplitude *= Persistence;
		Freq *= 2.0;
	}

	return Total / MaxAmplitude; // Normalize to [-1, 1].
}

// --- Elevation Generation ---

void UGTWorldGenerator::GenerateElevation(UGTGeodesicGrid* Grid, int32 Seed)
{
	if (!Grid)
	{
		return;
	}

	const int32 CellCount = Grid->GetCellCount();

	for (int32 i = 0; i < CellCount; ++i)
	{
		const FGTGeodesicCell& Cell = Grid->GetCell(i);
		const FVector& Pos = Cell.UnitSpherePosition;
		const double AbsLat = FMath::Abs(Cell.Latitude);

		// Layer 1: Low-frequency continent mask.
		// Uses 2-3 octaves at low frequency to create large continent shapes.
		const double ContinentNoise = FractalNoise3D(Pos, 3, 2.0, 0.5, Seed);

		// Layer 2: Medium-frequency terrain detail.
		// Adds hills, valleys, mountain ranges within continents.
		const double TerrainDetail = FractalNoise3D(Pos, 5, 8.0, 0.5, Seed + 100);

		// Layer 3: High-frequency surface roughness.
		const double Roughness = FractalNoise3D(Pos, 3, 32.0, 0.4, Seed + 200);

		// Combine layers with weights.
		// Continent noise dominates land/ocean, detail adds mountains/valleys.
		double Elevation = ContinentNoise * 0.6 + TerrainDetail * 0.3 + Roughness * 0.1;

		// Latitude-based modifications:
		// Push poles slightly above sea level for ice caps.
		if (AbsLat > 70.0)
		{
			const double PolarBias = (AbsLat - 70.0) / 20.0; // 0-1 from 70° to 90°.
			Elevation += PolarBias * 0.2;
		}

		// Continental shelf: slightly lower elevation near land/ocean boundaries.
		// (This emerges naturally from the noise, but we add a subtle equatorial bias
		// to prevent the world from being all ocean or all land.)

		// Ocean threshold: cells below this elevation are water.
		// Tuned to give roughly 70% ocean, 30% land (Earth-like ratio).
		const double SeaLevel = -0.05;
		const bool bIsLand = Elevation > SeaLevel;

		// Remap elevation for storage:
		// Land: 0.0 to 1.0 (sea level to mountain peak)
		// Ocean: -1.0 to 0.0 (deep ocean to sea level)
		float StoredElevation;
		if (bIsLand)
		{
			StoredElevation = static_cast<float>(FMath::Clamp((Elevation - SeaLevel) / (1.0 - SeaLevel), 0.0, 1.0));
		}
		else
		{
			StoredElevation = static_cast<float>(FMath::Clamp((Elevation - (-1.0)) / (SeaLevel - (-1.0)) - 1.0, -1.0, 0.0));
		}

		Grid->SetCellTerrain(i, StoredElevation, bIsLand);
	}
}

// --- Parcel Generation ---

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

// --- Terrain Assignment ---

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
		const float Elevation = Cell.Elevation;
		const bool bIsLand = Cell.bIsLand;

		EGTTerrainType Terrain;

		if (!bIsLand)
		{
			// Ocean cells.
			if (Elevation < -0.5f)
			{
				Terrain = EGTTerrainType::OceanDeep;
			}
			else if (Elevation < -0.1f)
			{
				Terrain = EGTTerrainType::OceanShallow;
			}
			else
			{
				// Very shallow — coastal shelf.
				Terrain = EGTTerrainType::Coastal;
			}
		}
		else if (AbsLat > 75.0)
		{
			// Polar regions.
			Terrain = EGTTerrainType::Frozen;
		}
		else if (AbsLat > 60.0)
		{
			// Sub-polar: tundra.
			if (Elevation > 0.5f)
			{
				Terrain = EGTTerrainType::Mountainous;
			}
			else
			{
				Terrain = EGTTerrainType::Tundra;
			}
		}
		else if (Elevation > 0.6f)
		{
			// High elevation: mountains.
			Terrain = EGTTerrainType::Mountainous;
		}
		else if (Elevation < 0.05f)
		{
			// Very low land near sea level: coastal.
			Terrain = EGTTerrainType::Coastal;
		}
		else
		{
			// Mid-elevation land: use latitude + detail noise for biome.
			const FVector& Pos = Cell.UnitSpherePosition;
			const double BiomeNoise = FractalNoise3D(Pos, 3, 12.0, 0.5, Seed + 500);

			if (AbsLat > 15.0 && AbsLat < 35.0 && BiomeNoise > 0.1)
			{
				// Subtropical desert belt.
				Terrain = EGTTerrainType::Desert;
			}
			else if (BiomeNoise > 0.4 && Elevation > 0.15f)
			{
				// High population density areas at good elevation.
				Terrain = EGTTerrainType::Urban;
			}
			else if (BiomeNoise > 0.15)
			{
				Terrain = EGTTerrainType::Suburban;
			}
			else
			{
				Terrain = EGTTerrainType::Rural;
			}
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
		case EGTTerrainType::Tundra:
			Parcel.DisasterRisk = 0.25f;
			break;
		case EGTTerrainType::Frozen:
			Parcel.DisasterRisk = 0.15f;
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
		case EGTTerrainType::Tundra:
			Parcel.LaborCostMultiplier = 2.5f;
			break;
		case EGTTerrainType::Frozen:
			Parcel.LaborCostMultiplier = 4.0f;
			break;
		case EGTTerrainType::Desert:
			Parcel.LaborCostMultiplier = 1.8f;
			break;
		default:
			Parcel.LaborCostMultiplier = 1.0f;
			break;
		}

		// Power grid reliability varies by terrain.
		switch (Terrain)
		{
		case EGTTerrainType::Urban:
			Parcel.PowerGridReliability = 0.99f;
			break;
		case EGTTerrainType::Suburban:
			Parcel.PowerGridReliability = 0.97f;
			break;
		case EGTTerrainType::Rural:
			Parcel.PowerGridReliability = 0.93f;
			break;
		case EGTTerrainType::Desert:
		case EGTTerrainType::Mountainous:
			Parcel.PowerGridReliability = 0.88f;
			break;
		case EGTTerrainType::Tundra:
		case EGTTerrainType::Frozen:
			Parcel.PowerGridReliability = 0.80f;
			break;
		case EGTTerrainType::OceanShallow:
		case EGTTerrainType::OceanDeep:
			Parcel.PowerGridReliability = 0.70f;
			break;
		default:
			Parcel.PowerGridReliability = 0.95f;
			break;
		}

		ParcelSystem->UpdateParcel(ParcelId, Parcel);
	}
}

// --- Region Generation ---

void UGTWorldGenerator::GenerateRegions(UGTWorldSettings* Settings, UGTGeodesicGrid* Grid, UGTLandParcelSystem* ParcelSystem, UGTRegionalEconomy* Economy)
{
	const int32 NumRegions = FMath::Max(Settings->RegionCount, 1);
	const int32 CellCount = Grid->GetCellCount();

	if (CellCount == 0)
	{
		return;
	}

	// Collect land cell indices for land-aware K-means seeding.
	TArray<int32> LandCells;
	LandCells.Reserve(CellCount / 3);
	for (int32 i = 0; i < CellCount; ++i)
	{
		if (Grid->GetCell(i).bIsLand)
		{
			LandCells.Add(i);
		}
	}

	// K-means-like clustering on sphere positions.
	// Seed region centers preferring land cells so we get meaningful regions.
	TArray<FVector> RegionCenters;
	RegionCenters.Reserve(NumRegions);

	if (LandCells.Num() >= NumRegions)
	{
		// Place initial centers on land cells, evenly spaced through the land array.
		const int32 Step = FMath::Max(LandCells.Num() / NumRegions, 1);
		for (int32 i = 0; i < NumRegions; ++i)
		{
			const int32 CellIdx = LandCells[(i * Step) % LandCells.Num()];
			RegionCenters.Add(Grid->GetCell(CellIdx).UnitSpherePosition);
		}
	}
	else
	{
		// Fallback: not enough land cells, use all cells.
		const int32 Step = FMath::Max(CellCount / NumRegions, 1);
		for (int32 i = 0; i < NumRegions; ++i)
		{
			const int32 CellIdx = (i * Step) % CellCount;
			RegionCenters.Add(Grid->GetCell(CellIdx).UnitSpherePosition);
		}
	}

	// Assign each cell to nearest region center.
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

	// Refine: run 5 iterations of K-means (more than the old 3 for better convergence).
	for (int32 Iter = 0; Iter < 5; ++Iter)
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
	// Count land cells per region for naming hints.
	TArray<int32> RegionLandCount;
	RegionLandCount.SetNumZeroed(NumRegions);
	TArray<int32> RegionTotalCount;
	RegionTotalCount.SetNumZeroed(NumRegions);

	for (int32 CellIdx = 0; CellIdx < CellCount; ++CellIdx)
	{
		const int32 RegIdx = CellRegionAssignments[CellIdx];
		RegionTotalCount[RegIdx]++;
		if (Grid->GetCell(CellIdx).bIsLand)
		{
			RegionLandCount[RegIdx]++;
		}
	}

	for (int32 RegIdx = 0; RegIdx < NumRegions; ++RegIdx)
	{
		FGTRegionalEconomyData RegionData;

		// Generate region name from geographic position.
		double CenterLon, CenterLat;
		CenterLat = FMath::RadiansToDegrees(
			FMath::Asin(FMath::Clamp(static_cast<double>(RegionCenters[RegIdx].Z), -1.0, 1.0)));
		CenterLon = FMath::RadiansToDegrees(FMath::Atan2(
			static_cast<double>(RegionCenters[RegIdx].Y),
			static_cast<double>(RegionCenters[RegIdx].X)));

		// Name based on hemisphere position.
		const FString LatDir = (CenterLat >= 0) ? TEXT("N") : TEXT("S");
		const FString LonDir = (CenterLon >= 0) ? TEXT("E") : TEXT("W");
		const bool bMostlyOcean = RegionLandCount[RegIdx] < RegionTotalCount[RegIdx] / 2;

		if (bMostlyOcean)
		{
			RegionData.RegionName = FString::Printf(TEXT("Ocean_%d_%s%s"),
				RegIdx, *LatDir, *LonDir);
		}
		else
		{
			RegionData.RegionName = FString::Printf(TEXT("Region_%d_%s%.0f_%s%.0f"),
				RegIdx, *LatDir, FMath::Abs(CenterLat), *LonDir, FMath::Abs(CenterLon));
		}

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

// --- Economic Seeding ---

void UGTWorldGenerator::SeedEconomicData(UGTWorldSettings* Settings, UGTRegionalEconomy* Economy, UGTLandParcelSystem* ParcelSystem)
{
	const int32 Seed = (Settings->WorldSeed != 0) ? Settings->WorldSeed : 42;
	const int32 NumRegions = Settings->RegionCount;

	// Count land parcels per region and compute average terrain quality.
	TMap<int32, int32> RegionLandParcels;
	TMap<int32, int32> RegionUrbanParcels;

	const int32 ParcelCount = ParcelSystem->GetParcelCount();
	for (int32 i = 0; i < ParcelCount; ++i)
	{
		FGTLandParcel Parcel = ParcelSystem->GetParcel(i);
		if (Parcel.RegionId < 0)
		{
			continue;
		}

		if (Parcel.Terrain != EGTTerrainType::OceanShallow &&
			Parcel.Terrain != EGTTerrainType::OceanDeep)
		{
			RegionLandParcels.FindOrAdd(Parcel.RegionId)++;
		}
		if (Parcel.Terrain == EGTTerrainType::Urban ||
			Parcel.Terrain == EGTTerrainType::Suburban)
		{
			RegionUrbanParcels.FindOrAdd(Parcel.RegionId)++;
		}
	}

	for (int32 RegIdx = 0; RegIdx < NumRegions; ++RegIdx)
	{
		FGTRegionalEconomyData Data = Economy->GetRegionData(RegIdx);

		const int32 LandParcels = RegionLandParcels.FindRef(RegIdx);
		const int32 UrbanParcels = RegionUrbanParcels.FindRef(RegIdx);
		const float UrbanRatio = (LandParcels > 0) ? static_cast<float>(UrbanParcels) / LandParcels : 0.0f;

		// Use noise for variation, but base population on land area and urbanization.
		const double RegNoise = Noise2D(RegIdx * 13.7, RegIdx * 7.3, Seed + 100);

		// Population scales with land parcels and urbanization.
		// Ocean-dominant regions get minimal population.
		if (LandParcels <= 0)
		{
			Data.Population = 0.0;
			Data.GDPProxy = 0.0;
			Data.TechAdoptionIndex = 0.0f;
			Data.PoliticalStability = 0.5f;
			Data.DataDemandGrowthRate = 0.0f;
			Data.BusinessDensity = 0.0f;
			Data.UrbanizationIndex = 0.0f;
			Data.CurrentDemand = 0.0;
		}
		else
		{
			// Base population: 50k-5M per land parcel depending on urbanization.
			const double PopPerParcel = FMath::Lerp(50000.0, 5000000.0,
				static_cast<double>(UrbanRatio * 0.7 + (RegNoise + 1.0) * 0.15));
			Data.Population = PopPerParcel * LandParcels;

			// GDP proxy scales with population and noise.
			const double GdpNoise = Noise2D(RegIdx * 5.1, RegIdx * 11.9, Seed + 200);
			const double GdpPerCapita = FMath::Lerp(1000.0, 60000.0, (GdpNoise + 1.0) * 0.5);
			Data.GDPProxy = Data.Population * GdpPerCapita;

			// Tech adoption correlates with GDP per capita.
			Data.TechAdoptionIndex = FMath::Clamp(
				static_cast<float>(GdpPerCapita / 60000.0), 0.05f, 0.95f);

			// Political stability.
			const double StabNoise = Noise2D(RegIdx * 3.3, RegIdx * 9.7, Seed + 300);
			Data.PoliticalStability = FMath::Clamp(
				static_cast<float>((StabNoise + 1.0) * 0.5), 0.2f, 0.95f);

			// Data demand growth rate.
			Data.DataDemandGrowthRate = 0.03f + Data.TechAdoptionIndex * 0.07f;

			// Business density correlates with urbanization.
			Data.BusinessDensity = FMath::Clamp(UrbanRatio * 0.8f + Data.TechAdoptionIndex * 0.2f, 0.1f, 0.9f);

			// Urbanization index.
			Data.UrbanizationIndex = FMath::Clamp(UrbanRatio, 0.05f, 0.95f);

			// Initial demand.
			Data.CurrentDemand = Data.Population * Data.TechAdoptionIndex * 0.001;
			Data.CurrentDemand *= Settings->DemandGrowthMultiplier;
		}

		// Write the seeded data back to the economy subsystem.
		Economy->UpdateRegionData(RegIdx, Data);
		Economy->SetRegionConnectivity(RegIdx, 0.0f); // Start with no connectivity.
	}
}

// --- Zoning Assignment ---

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
		case EGTTerrainType::Tundra:
			Parcel.Zoning = EGTZoningCategory::Unrestricted;
			break;
		case EGTTerrainType::Frozen:
			Parcel.Zoning = EGTZoningCategory::Protected;
			break;
		}

		ParcelSystem->UpdateParcel(i, Parcel);
	}
}
