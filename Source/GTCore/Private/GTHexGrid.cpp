#include "GTHexGrid.h"

const FGTHexCoord UGTHexGrid::Directions[6] = {
	FGTHexCoord(+1,  0), // East
	FGTHexCoord(+1, -1), // NE
	FGTHexCoord( 0, -1), // NW
	FGTHexCoord(-1,  0), // West
	FGTHexCoord(-1, +1), // SW
	FGTHexCoord( 0, +1), // SE
};

FGTHexCoord UGTHexGrid::GetNeighbor(const FGTHexCoord& Coord, int32 Direction)
{
	const int32 Dir = ((Direction % 6) + 6) % 6;
	return Coord + Directions[Dir];
}

TArray<FGTHexCoord> UGTHexGrid::GetAllNeighbors(const FGTHexCoord& Coord)
{
	TArray<FGTHexCoord> Result;
	Result.Reserve(6);
	for (int32 i = 0; i < 6; ++i)
	{
		Result.Add(Coord + Directions[i]);
	}
	return Result;
}

int32 UGTHexGrid::Distance(const FGTHexCoord& A, const FGTHexCoord& B)
{
	const FGTHexCoord Delta = A - B;
	return (FMath::Abs(Delta.Q) + FMath::Abs(Delta.R) + FMath::Abs(Delta.S())) / 2;
}

TArray<FGTHexCoord> UGTHexGrid::GetRing(const FGTHexCoord& Center, int32 Radius)
{
	TArray<FGTHexCoord> Result;

	if (Radius <= 0)
	{
		Result.Add(Center);
		return Result;
	}

	Result.Reserve(6 * Radius);

	// Start at the hex Radius steps in direction 4 (SW) from center.
	FGTHexCoord Current = Center + Directions[4] * Radius;

	for (int32 Side = 0; Side < 6; ++Side)
	{
		for (int32 Step = 0; Step < Radius; ++Step)
		{
			Result.Add(Current);
			Current = Current + Directions[Side];
		}
	}

	return Result;
}

TArray<FGTHexCoord> UGTHexGrid::GetSpiral(const FGTHexCoord& Center, int32 Radius)
{
	TArray<FGTHexCoord> Result;
	Result.Add(Center);

	for (int32 Ring = 1; Ring <= Radius; ++Ring)
	{
		Result.Append(GetRing(Center, Ring));
	}

	return Result;
}

TArray<FGTHexCoord> UGTHexGrid::GetLine(const FGTHexCoord& A, const FGTHexCoord& B)
{
	const int32 N = Distance(A, B);

	TArray<FGTHexCoord> Result;
	Result.Reserve(N + 1);

	if (N == 0)
	{
		Result.Add(A);
		return Result;
	}

	for (int32 i = 0; i <= N; ++i)
	{
		const double T = static_cast<double>(i) / N;
		const double FracQ = FMath::Lerp(static_cast<double>(A.Q), static_cast<double>(B.Q), T);
		const double FracR = FMath::Lerp(static_cast<double>(A.R), static_cast<double>(B.R), T);
		Result.Add(RoundHex(FracQ, FracR));
	}

	return Result;
}

FGTHexCoord UGTHexGrid::RoundHex(double FracQ, double FracR)
{
	const double FracS = -FracQ - FracR;

	int32 RoundQ = FMath::RoundToInt32(FracQ);
	int32 RoundR = FMath::RoundToInt32(FracR);
	const int32 RoundS = FMath::RoundToInt32(FracS);

	const double DiffQ = FMath::Abs(static_cast<double>(RoundQ) - FracQ);
	const double DiffR = FMath::Abs(static_cast<double>(RoundR) - FracR);
	const double DiffS = FMath::Abs(static_cast<double>(RoundS) - FracS);

	// Reset the component with the largest rounding error.
	if (DiffQ > DiffR && DiffQ > DiffS)
	{
		RoundQ = -RoundR - RoundS;
	}
	else if (DiffR > DiffS)
	{
		RoundR = -RoundQ - RoundS;
	}
	// else S would be recalculated, but we only store Q,R.

	return FGTHexCoord(RoundQ, RoundR);
}
