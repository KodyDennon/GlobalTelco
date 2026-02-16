#pragma once

#include "CoreMinimal.h"
#include "GTHexGrid.generated.h"

/**
 * FGTHexCoord
 *
 * Cube coordinate representation of a hex position.
 * Uses the axial (q, r) representation with s derived as s = -q - r.
 * Cube coordinates enable simple, efficient hex math: neighbors,
 * distance, rings, and lines are all O(1) or O(n) operations.
 *
 * These are abstract grid coordinates — mapping to world (WGS84)
 * positions is done by the geodesic grid system.
 */
USTRUCT(BlueprintType)
struct GTCORE_API FGTHexCoord
{
	GENERATED_BODY()

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Hex")
	int32 Q = 0;

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Hex")
	int32 R = 0;

	FGTHexCoord() = default;
	FGTHexCoord(int32 InQ, int32 InR) : Q(InQ), R(InR) {}

	/** Derived S coordinate: s = -q - r. */
	int32 S() const { return -Q - R; }

	bool operator==(const FGTHexCoord& Other) const { return Q == Other.Q && R == Other.R; }
	bool operator!=(const FGTHexCoord& Other) const { return !(*this == Other); }

	FGTHexCoord operator+(const FGTHexCoord& Other) const { return FGTHexCoord(Q + Other.Q, R + Other.R); }
	FGTHexCoord operator-(const FGTHexCoord& Other) const { return FGTHexCoord(Q - Other.Q, R - Other.R); }
	FGTHexCoord operator*(int32 Scale) const { return FGTHexCoord(Q * Scale, R * Scale); }

	friend uint32 GetTypeHash(const FGTHexCoord& Coord)
	{
		return HashCombine(GetTypeHash(Coord.Q), GetTypeHash(Coord.R));
	}
};

/**
 * UGTHexGrid
 *
 * Static utility library for hex coordinate math.
 * All functions are pure and operate on FGTHexCoord values.
 */
UCLASS()
class GTCORE_API UGTHexGrid : public UBlueprintFunctionLibrary
{
	GENERATED_BODY()

public:
	/** The 6 neighbor direction offsets in cube coordinates. */
	static const FGTHexCoord Directions[6];

	/** Get the hex coordinate of the neighbor in the given direction (0-5). */
	UFUNCTION(BlueprintPure, Category = "Hex Grid")
	static FGTHexCoord GetNeighbor(const FGTHexCoord& Coord, int32 Direction);

	/** Get all 6 neighbors of a hex. */
	UFUNCTION(BlueprintPure, Category = "Hex Grid")
	static TArray<FGTHexCoord> GetAllNeighbors(const FGTHexCoord& Coord);

	/** Hex distance between two coordinates (Manhattan distance in cube space). */
	UFUNCTION(BlueprintPure, Category = "Hex Grid")
	static int32 Distance(const FGTHexCoord& A, const FGTHexCoord& B);

	/** Get all hexes in a ring at the given radius from center. */
	UFUNCTION(BlueprintPure, Category = "Hex Grid")
	static TArray<FGTHexCoord> GetRing(const FGTHexCoord& Center, int32 Radius);

	/** Get all hexes in a filled circle (spiral) up to the given radius. */
	UFUNCTION(BlueprintPure, Category = "Hex Grid")
	static TArray<FGTHexCoord> GetSpiral(const FGTHexCoord& Center, int32 Radius);

	/** Get hexes along a line between two coordinates. */
	UFUNCTION(BlueprintPure, Category = "Hex Grid")
	static TArray<FGTHexCoord> GetLine(const FGTHexCoord& A, const FGTHexCoord& B);

	/**
	 * Round fractional cube coordinates to the nearest hex.
	 * Used when converting from world position to hex coordinate.
	 */
	UFUNCTION(BlueprintPure, Category = "Hex Grid")
	static FGTHexCoord RoundHex(double FracQ, double FracR);
};
