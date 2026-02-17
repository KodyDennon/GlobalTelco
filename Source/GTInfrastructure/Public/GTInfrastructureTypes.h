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

	/** True while the node is being built. Under-construction nodes don't generate revenue or route traffic. */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Infrastructure")
	bool bUnderConstruction = false;

	/** Remaining simulation ticks until construction completes. Decremented each tick. */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Infrastructure")
	int32 RemainingConstructionTicks = 0;

	/** Current utilization ratio 0.0-1.0. Traffic routed / Capacity. Updated by revenue calculator. */
	UPROPERTY(BlueprintReadOnly, Category = "Infrastructure")
	float CurrentUtilization = 0.0f;

	/** Construction cost for building this node type (base cost before multipliers). */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Infrastructure")
	float ConstructionCost = 200000.0f;

	/** Base construction time in ticks before terrain/world-settings multipliers. */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Infrastructure")
	int32 BaseConstructionTicks = 5;
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

	/** True while the edge is being built. Under-construction edges don't route traffic. */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Infrastructure")
	bool bUnderConstruction = false;

	/** Remaining simulation ticks until construction completes. */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Infrastructure")
	int32 RemainingConstructionTicks = 0;

	/** Current utilization ratio 0.0-1.0. Traffic routed / Capacity. */
	UPROPERTY(BlueprintReadOnly, Category = "Infrastructure")
	float CurrentUtilization = 0.0f;

	/** Construction cost for building this edge type (base cost before multipliers). */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Infrastructure")
	float ConstructionCost = 50000.0f;

	/** Base construction time in ticks before terrain/world-settings multipliers. */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Infrastructure")
	int32 BaseConstructionTicks = 3;
};
