#pragma once

#include "CoreMinimal.h"
#include "GTSimulationTypes.h"
#include "GTInfrastructureTypes.generated.h"

/** Types of infrastructure nodes that can be placed in the world. */
UENUM(BlueprintType)
enum class EGTNodeType : uint8
{
	None = 0,
	AccessTower,
	FiberDistributionHub,
	DataCenter,
	InternetExchangePoint,
	SubseaLandingStation,
	SatelliteGroundStation
};

/** Types of network edges connecting nodes. */
UENUM(BlueprintType)
enum class EGTEdgeType : uint8
{
	None = 0,
	LocalFiber,
	RegionalFiber,
	NationalBackbone,
	SubseaCable,
	Microwave,
	SatelliteLink
};

/** Current operational status of an infrastructure element. */
UENUM(BlueprintType)
enum class EGTInfrastructureStatus : uint8
{
	UnderConstruction,
	Operational,
	Degraded,
	Destroyed,
	Decommissioned
};

/**
 * FGTNodeAttributes
 *
 * Runtime attributes for an infrastructure node.
 * These values change over time due to maintenance, disasters, and upgrades.
 */
USTRUCT(BlueprintType)
struct GTINFRASTRUCTURE_API FGTNodeAttributes
{
	GENERATED_BODY()

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Infrastructure")
	float Capacity = 100.0f;

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Infrastructure")
	float LatencyMs = 1.0f;

	/** Reliability rating 0.0 - 1.0. */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Infrastructure", meta = (ClampMin = "0.0", ClampMax = "1.0"))
	float Reliability = 0.99f;

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Infrastructure")
	float MaintenanceCostPerTick = 10.0f;

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Infrastructure")
	float DisasterRiskMultiplier = 1.0f;
};

/**
 * FGTEdgeAttributes
 *
 * Runtime attributes for a network edge (link between two nodes).
 */
USTRUCT(BlueprintType)
struct GTINFRASTRUCTURE_API FGTEdgeAttributes
{
	GENERATED_BODY()

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Infrastructure")
	float Capacity = 1000.0f;

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Infrastructure")
	float LatencyWeightMs = 5.0f;

	/** Reliability rating 0.0 - 1.0. */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Infrastructure", meta = (ClampMin = "0.0", ClampMax = "1.0"))
	float Reliability = 0.98f;

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Infrastructure")
	float MaintenanceCostPerTick = 25.0f;

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Infrastructure")
	float ConstructionTimeSeconds = 60.0f;

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Infrastructure")
	float TerrainRiskMultiplier = 1.0f;
};
