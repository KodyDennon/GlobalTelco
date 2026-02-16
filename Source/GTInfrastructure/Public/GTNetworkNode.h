#pragma once

#include "CoreMinimal.h"
#include "GameFramework/Actor.h"
#include "GTInfrastructureTypes.h"
#include "GTNetworkNode.generated.h"

/**
 * AGTNetworkNode
 *
 * Base actor for all infrastructure nodes placed in the world.
 * Each node exists at a specific hex parcel and belongs to one or more
 * owning corporations (cooperative ownership is supported).
 *
 * Subclass this for specific node types: towers, data centers, IXPs, etc.
 */
UCLASS(Abstract, Blueprintable)
class GTINFRASTRUCTURE_API AGTNetworkNode : public AActor
{
	GENERATED_BODY()

public:
	AGTNetworkNode();

	/** Unique graph ID for this node within the network graph. */
	UPROPERTY(Replicated, BlueprintReadOnly, Category = "Network")
	int32 NodeId = -1;

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Network")
	EGTNodeType NodeType = EGTNodeType::None;

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Network")
	EGTNetworkLevel NetworkLevel = EGTNetworkLevel::Local;

	UPROPERTY(Replicated, EditAnywhere, BlueprintReadWrite, Category = "Network")
	EGTInfrastructureStatus Status = EGTInfrastructureStatus::UnderConstruction;

	UPROPERTY(Replicated, EditAnywhere, BlueprintReadWrite, Category = "Network")
	FGTNodeAttributes Attributes;

	/** Terrain type of the parcel where this node is placed. */
	UPROPERTY(Replicated, BlueprintReadOnly, Category = "Network")
	EGTTerrainType Terrain = EGTTerrainType::Urban;

	/** Corporation IDs that co-own this node. */
	UPROPERTY(Replicated, BlueprintReadOnly, Category = "Ownership")
	TArray<int32> OwnerCorporationIds;

	/** Apply damage from a disaster or sabotage event. */
	UFUNCTION(BlueprintCallable, Category = "Network")
	virtual void ApplyDamage(float CapacityReduction, float LatencyIncrease);

	/** Mark this node as dirty so the network graph recalculates affected routes. */
	UFUNCTION(BlueprintCallable, Category = "Network")
	void MarkDirty();

	UFUNCTION(BlueprintPure, Category = "Network")
	bool IsOperational() const { return Status == EGTInfrastructureStatus::Operational; }

	virtual void GetLifetimeReplicatedProps(TArray<FLifetimeProperty>& OutLifetimeProps) const override;
};
