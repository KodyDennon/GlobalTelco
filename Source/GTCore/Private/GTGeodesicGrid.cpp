#include "GTGeodesicGrid.h"

const FGTGeodesicCell UGTGeodesicGrid::InvalidCell = FGTGeodesicCell();

void UGTGeodesicGrid::Initialize(FSubsystemCollectionBase& Collection)
{
	Super::Initialize(Collection);
}

void UGTGeodesicGrid::Deinitialize()
{
	Cells.Empty();
	Adjacency.Empty();
	SpatialHash.Empty();
	Super::Deinitialize();
}

void UGTGeodesicGrid::GenerateGrid(int32 Frequency)
{
	Cells.Empty();
	Adjacency.Empty();
	SpatialHash.Empty();

	Frequency = FMath::Max(Frequency, 1);

	// Adjust spatial hash bucket size based on frequency.
	// At frequency F, average cell spacing is roughly 2/F on the unit sphere.
	// We want buckets ~3x the cell spacing for good neighbor coverage.
	SpatialHashBucketSize = FMath::Max(6.0f / static_cast<float>(Frequency), 0.01f);

	TArray<FVector> Vertices;
	SubdivideIcosahedron(Frequency, Vertices);
	DeduplicateVertices(Vertices, 1e-6);

	Cells.Reserve(Vertices.Num());

	for (int32 i = 0; i < Vertices.Num(); ++i)
	{
		FGTGeodesicCell Cell;
		Cell.CellIndex = i;
		Cell.UnitSpherePosition = Vertices[i];

		CartesianToLonLat(Vertices[i], Cell.Longitude, Cell.Latitude);

		// Assign hex coordinates from cell index.
		// For geodesic grids the hex coord is a linearized index —
		// neighbor relationships come from sphere adjacency, not coord arithmetic.
		Cell.HexCoord = FGTHexCoord(i, 0);

		Cells.Add(Cell);
	}

	BuildSpatialHash();
	BuildNeighborAdjacency();

	UE_LOG(LogTemp, Log, TEXT("GTGeodesicGrid: Generated %d cells (Frequency=%d)"), Cells.Num(), Frequency);
}

const FGTGeodesicCell& UGTGeodesicGrid::GetCell(int32 CellIndex) const
{
	if (Cells.IsValidIndex(CellIndex))
	{
		return Cells[CellIndex];
	}
	return InvalidCell;
}

void UGTGeodesicGrid::SetCellTerrain(int32 CellIndex, float InElevation, bool bInIsLand)
{
	if (Cells.IsValidIndex(CellIndex))
	{
		Cells[CellIndex].Elevation = InElevation;
		Cells[CellIndex].bIsLand = bInIsLand;
	}
}

int32 UGTGeodesicGrid::FindNearestCell(double Longitude, double Latitude) const
{
	const FVector TargetPos = LonLatToCartesian(Longitude, Latitude);
	return FindNearestCellFromUnitPosition(TargetPos);
}

int32 UGTGeodesicGrid::FindNearestCellFromUnitPosition(const FVector& UnitPos) const
{
	if (Cells.Num() == 0)
	{
		return -1;
	}

	// Use spatial hash for fast lookup: check the target bucket and its neighbors.
	int32 BestIndex = -1;
	double BestDot = -2.0;

	const int32 BX = FMath::FloorToInt32(UnitPos.X / SpatialHashBucketSize);
	const int32 BY = FMath::FloorToInt32(UnitPos.Y / SpatialHashBucketSize);
	const int32 BZ = FMath::FloorToInt32(UnitPos.Z / SpatialHashBucketSize);

	// Search the 3x3x3 neighborhood of buckets.
	for (int32 DX = -1; DX <= 1; ++DX)
	{
		for (int32 DY = -1; DY <= 1; ++DY)
		{
			for (int32 DZ = -1; DZ <= 1; ++DZ)
			{
				const int64 Key = static_cast<int64>(BX + DX) * 73856093LL
					^ static_cast<int64>(BY + DY) * 19349663LL
					^ static_cast<int64>(BZ + DZ) * 83492791LL;

				const TArray<int32>* Bucket = SpatialHash.Find(Key);
				if (!Bucket)
				{
					continue;
				}

				for (int32 CellIdx : *Bucket)
				{
					const double Dot = FVector::DotProduct(UnitPos, Cells[CellIdx].UnitSpherePosition);
					if (Dot > BestDot)
					{
						BestDot = Dot;
						BestIndex = CellIdx;
					}
				}
			}
		}
	}

	// Fallback to brute force if spatial hash missed (shouldn't happen with correct bucket size).
	if (BestIndex < 0)
	{
		for (int32 i = 0; i < Cells.Num(); ++i)
		{
			const double Dot = FVector::DotProduct(UnitPos, Cells[i].UnitSpherePosition);
			if (Dot > BestDot)
			{
				BestDot = Dot;
				BestIndex = i;
			}
		}
	}

	return BestIndex;
}

TArray<int32> UGTGeodesicGrid::GetCellNeighbors(int32 CellIndex) const
{
	const TArray<int32>* Found = Adjacency.Find(CellIndex);
	return Found ? *Found : TArray<int32>();
}

// --- Spatial Hash ---

int64 UGTGeodesicGrid::SpatialHashKey(const FVector& Pos) const
{
	const int32 BX = FMath::FloorToInt32(Pos.X / SpatialHashBucketSize);
	const int32 BY = FMath::FloorToInt32(Pos.Y / SpatialHashBucketSize);
	const int32 BZ = FMath::FloorToInt32(Pos.Z / SpatialHashBucketSize);

	return static_cast<int64>(BX) * 73856093LL
		^ static_cast<int64>(BY) * 19349663LL
		^ static_cast<int64>(BZ) * 83492791LL;
}

void UGTGeodesicGrid::BuildSpatialHash()
{
	SpatialHash.Empty();
	SpatialHash.Reserve(Cells.Num());

	for (int32 i = 0; i < Cells.Num(); ++i)
	{
		const int64 Key = SpatialHashKey(Cells[i].UnitSpherePosition);
		SpatialHash.FindOrAdd(Key).Add(i);
	}
}

// --- Icosahedral Subdivision ---

void UGTGeodesicGrid::SubdivideIcosahedron(int32 Frequency, TArray<FVector>& OutVertices)
{
	// Golden ratio for icosahedron vertex construction.
	const double Phi = (1.0 + FMath::Sqrt(5.0)) / 2.0;

	// 12 vertices of a regular icosahedron (normalized to unit sphere).
	TArray<FVector> IcoVerts;
	IcoVerts.Reserve(12);

	auto AddNormalized = [&](double X, double Y, double Z)
	{
		FVector V(X, Y, Z);
		V.Normalize();
		IcoVerts.Add(V);
	};

	AddNormalized(-1.0,  Phi, 0.0);
	AddNormalized( 1.0,  Phi, 0.0);
	AddNormalized(-1.0, -Phi, 0.0);
	AddNormalized( 1.0, -Phi, 0.0);
	AddNormalized(0.0, -1.0,  Phi);
	AddNormalized(0.0,  1.0,  Phi);
	AddNormalized(0.0, -1.0, -Phi);
	AddNormalized(0.0,  1.0, -Phi);
	AddNormalized( Phi, 0.0, -1.0);
	AddNormalized( Phi, 0.0,  1.0);
	AddNormalized(-Phi, 0.0, -1.0);
	AddNormalized(-Phi, 0.0,  1.0);

	// 20 triangular faces of the icosahedron.
	struct Face { int32 A, B, C; };
	TArray<Face> Faces = {
		{0, 11, 5},  {0, 5, 1},   {0, 1, 7},   {0, 7, 10},  {0, 10, 11},
		{1, 5, 9},   {5, 11, 4},  {11, 10, 2},  {10, 7, 6},  {7, 1, 8},
		{3, 9, 4},   {3, 4, 2},   {3, 2, 6},   {3, 6, 8},   {3, 8, 9},
		{4, 9, 5},   {2, 4, 11},  {6, 2, 10},  {8, 6, 7},   {9, 8, 1}
	};

	// For each face, subdivide into Frequency^2 sub-triangles
	// and project vertices onto the unit sphere.
	OutVertices.Reserve(10 * Frequency * Frequency + 2);

	for (const Face& F : Faces)
	{
		const FVector& VA = IcoVerts[F.A];
		const FVector& VB = IcoVerts[F.B];
		const FVector& VC = IcoVerts[F.C];

		for (int32 i = 0; i <= Frequency; ++i)
		{
			for (int32 j = 0; j <= Frequency - i; ++j)
			{
				const double U = static_cast<double>(i) / Frequency;
				const double V = static_cast<double>(j) / Frequency;
				const double W = 1.0 - U - V;

				FVector Point = VA * W + VB * U + VC * V;
				Point.Normalize();
				OutVertices.Add(Point);
			}
		}
	}
}

void UGTGeodesicGrid::DeduplicateVertices(TArray<FVector>& Vertices, double Tolerance)
{
	// Use spatial hashing for O(n) deduplication instead of O(n²) brute force.
	TArray<FVector> Unique;
	Unique.Reserve(Vertices.Num());

	const float BucketSize = FMath::Max(static_cast<float>(Tolerance) * 10.0f, 0.0001f);
	const double ToleranceSq = Tolerance * Tolerance;

	TMap<int64, TArray<int32>> HashGrid;
	HashGrid.Reserve(Vertices.Num());

	auto HashKey = [BucketSize](const FVector& V) -> int64
	{
		const int32 BX = FMath::FloorToInt32(V.X / BucketSize);
		const int32 BY = FMath::FloorToInt32(V.Y / BucketSize);
		const int32 BZ = FMath::FloorToInt32(V.Z / BucketSize);
		return static_cast<int64>(BX) * 73856093LL
			^ static_cast<int64>(BY) * 19349663LL
			^ static_cast<int64>(BZ) * 83492791LL;
	};

	for (const FVector& V : Vertices)
	{
		// Check neighboring buckets for duplicates.
		const int32 BX = FMath::FloorToInt32(V.X / BucketSize);
		const int32 BY = FMath::FloorToInt32(V.Y / BucketSize);
		const int32 BZ = FMath::FloorToInt32(V.Z / BucketSize);

		bool bFound = false;
		for (int32 DX = -1; DX <= 1 && !bFound; ++DX)
		{
			for (int32 DY = -1; DY <= 1 && !bFound; ++DY)
			{
				for (int32 DZ = -1; DZ <= 1 && !bFound; ++DZ)
				{
					const int64 Key = static_cast<int64>(BX + DX) * 73856093LL
						^ static_cast<int64>(BY + DY) * 19349663LL
						^ static_cast<int64>(BZ + DZ) * 83492791LL;

					const TArray<int32>* Bucket = HashGrid.Find(Key);
					if (!Bucket)
					{
						continue;
					}
					for (int32 Idx : *Bucket)
					{
						if (FVector::DistSquared(V, Unique[Idx]) < ToleranceSq)
						{
							bFound = true;
							break;
						}
					}
				}
			}
		}

		if (!bFound)
		{
			const int32 NewIdx = Unique.Num();
			Unique.Add(V);
			HashGrid.FindOrAdd(HashKey(V)).Add(NewIdx);
		}
	}

	Vertices = MoveTemp(Unique);
}

void UGTGeodesicGrid::CartesianToLonLat(const FVector& Pos, double& OutLon, double& OutLat)
{
	OutLat = FMath::RadiansToDegrees(FMath::Asin(FMath::Clamp(Pos.Z, -1.0, 1.0)));
	OutLon = FMath::RadiansToDegrees(FMath::Atan2(Pos.Y, Pos.X));
}

FVector UGTGeodesicGrid::LonLatToCartesian(double Longitude, double Latitude)
{
	const double LonRad = FMath::DegreesToRadians(Longitude);
	const double LatRad = FMath::DegreesToRadians(Latitude);
	const double CosLat = FMath::Cos(LatRad);

	return FVector(
		CosLat * FMath::Cos(LonRad),
		CosLat * FMath::Sin(LonRad),
		FMath::Sin(LatRad)
	);
}

void UGTGeodesicGrid::BuildNeighborAdjacency()
{
	Adjacency.Empty();
	Adjacency.Reserve(Cells.Num());

	// Use the spatial hash for O(n) neighbor finding.
	// For each cell, search nearby buckets for the closest 6 cells.
	const int32 MaxNeighbors = 7;

	for (int32 i = 0; i < Cells.Num(); ++i)
	{
		struct FNeighborCandidate
		{
			int32 Index;
			double Dot;
		};

		TArray<FNeighborCandidate> Best;
		Best.Reserve(MaxNeighbors + 1);

		const FVector& Pos = Cells[i].UnitSpherePosition;

		// Search nearby buckets (2-ring to catch all neighbors).
		const int32 BX = FMath::FloorToInt32(Pos.X / SpatialHashBucketSize);
		const int32 BY = FMath::FloorToInt32(Pos.Y / SpatialHashBucketSize);
		const int32 BZ = FMath::FloorToInt32(Pos.Z / SpatialHashBucketSize);

		for (int32 DX = -2; DX <= 2; ++DX)
		{
			for (int32 DY = -2; DY <= 2; ++DY)
			{
				for (int32 DZ = -2; DZ <= 2; ++DZ)
				{
					const int64 Key = static_cast<int64>(BX + DX) * 73856093LL
						^ static_cast<int64>(BY + DY) * 19349663LL
						^ static_cast<int64>(BZ + DZ) * 83492791LL;

					const TArray<int32>* Bucket = SpatialHash.Find(Key);
					if (!Bucket)
					{
						continue;
					}

					for (int32 j : *Bucket)
					{
						if (i == j)
						{
							continue;
						}

						const double Dot = FVector::DotProduct(Pos, Cells[j].UnitSpherePosition);

						if (Best.Num() < MaxNeighbors)
						{
							Best.Add({j, Dot});
							if (Best.Num() == MaxNeighbors)
							{
								Best.Sort([](const FNeighborCandidate& A, const FNeighborCandidate& B)
								{
									return A.Dot > B.Dot;
								});
							}
						}
						else if (Dot > Best.Last().Dot)
						{
							Best.Last() = {j, Dot};
							Best.Sort([](const FNeighborCandidate& A, const FNeighborCandidate& B)
							{
								return A.Dot > B.Dot;
							});
						}
					}
				}
			}
		}

		TArray<int32> Neighbors;
		Neighbors.Reserve(6);
		for (int32 k = 0; k < FMath::Min(6, Best.Num()); ++k)
		{
			Neighbors.Add(Best[k].Index);
		}

		Adjacency.Add(i, MoveTemp(Neighbors));
	}
}
