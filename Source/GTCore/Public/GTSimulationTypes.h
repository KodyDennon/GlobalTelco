#pragma once

#include "CoreMinimal.h"
#include "GTSimulationTypes.generated.h"

/** Categories of simulation events flowing through the central event queue. */
UENUM(BlueprintType)
enum class EGTSimulationEventType : uint8
{
	None = 0,
	InfrastructureBuilt,
	InfrastructureDestroyed,
	InfrastructureUpgraded,
	InfrastructureDegraded,
	RoutingRecalculation,
	DisasterStrike,
	DisasterResolved,
	EconomicTick,
	RevenueComputed,
	CorporationBankrupt,
	ContractSigned,
	ContractBreached,
	PlayerAction,
	AllianceFormed,
	AllianceDissolved,
	LandPurchased,
	LandLeased,
	RegulatoryChange,
	PoliticalEvent
};

/**
 * FGTSimulationEvent
 *
 * Atomic event that flows through the centralized event queue.
 * All simulation state changes are expressed as events — infrastructure changes,
 * economic ticks, disasters, player actions, and AI decisions.
 */
USTRUCT(BlueprintType)
struct GTCORE_API FGTSimulationEvent
{
	GENERATED_BODY()

	UPROPERTY(BlueprintReadOnly)
	EGTSimulationEventType EventType = EGTSimulationEventType::None;

	/** Simulation tick at which this event was enqueued. */
	UPROPERTY(BlueprintReadOnly)
	int64 Tick = 0;

	/** Timestamp (world seconds) when this event was created. */
	UPROPERTY(BlueprintReadOnly)
	double Timestamp = 0.0;

	/** ID of the entity (node, edge, corporation, player) that originated this event. */
	UPROPERTY(BlueprintReadOnly)
	int32 SourceEntityId = -1;

	/** ID of the entity affected by this event. */
	UPROPERTY(BlueprintReadOnly)
	int32 TargetEntityId = -1;

	/** Freeform payload for event-specific data. */
	UPROPERTY(BlueprintReadOnly)
	TMap<FName, FString> Payload;
};

/** Terrain classification for hex-based land parcels. */
UENUM(BlueprintType)
enum class EGTTerrainType : uint8
{
	Urban,
	Suburban,
	Rural,
	Mountainous,
	Desert,
	Coastal,
	OceanShallow,
	OceanDeep,
	Tundra,
	Frozen
};

/** Hierarchy levels in the global network graph. */
UENUM(BlueprintType)
enum class EGTNetworkLevel : uint8
{
	Local = 0,
	Regional,
	National,
	Continental,
	Global
};

/**
 * FGTSaveSlotInfo
 *
 * Metadata about a save slot for display in the load game UI.
 * Defined in GTCore so both GTFrontend and GlobalTelco can use it.
 */
USTRUCT(BlueprintType)
struct GTCORE_API FGTSaveSlotInfo
{
	GENERATED_BODY()

	UPROPERTY(BlueprintReadOnly)
	FString SlotName;

	UPROPERTY(BlueprintReadOnly)
	FString SaveDisplayName;

	UPROPERTY(BlueprintReadOnly)
	FDateTime SaveTimestamp;

	UPROPERTY(BlueprintReadOnly)
	int64 SimulationTick = 0;

	UPROPERTY(BlueprintReadOnly)
	FString PlayerCorporationName;

	UPROPERTY(BlueprintReadOnly)
	FString DifficultyName;
};
