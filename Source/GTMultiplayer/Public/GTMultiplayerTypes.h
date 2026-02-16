#pragma once

#include "CoreMinimal.h"
#include "GTMultiplayerTypes.generated.h"

/** Current state of an alliance between corporations. */
UENUM(BlueprintType)
enum class EGTAllianceStatus : uint8
{
	Proposed,
	Active,
	Dissolved
};

/** Types of contracts between corporations. */
UENUM(BlueprintType)
enum class EGTContractType : uint8
{
	PeeringAgreement,
	TransitAgreement,
	LandLease,
	CapacityReservation,
	MergerAcquisition,
	DevelopmentContract
};

/** Status of a simulation contract. */
UENUM(BlueprintType)
enum class EGTContractStatus : uint8
{
	Proposed,
	Negotiating,
	Active,
	Breached,
	Expired,
	Terminated
};

/**
 * FGTContract
 *
 * A simulation entity representing a binding agreement between two corporations.
 * Contracts have duration, pricing terms, capacity guarantees, and breach penalties.
 */
USTRUCT(BlueprintType)
struct GTMULTIPLAYER_API FGTContract
{
	GENERATED_BODY()

	UPROPERTY(BlueprintReadOnly, Category = "Contract")
	int32 ContractId = -1;

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Contract")
	EGTContractType ContractType = EGTContractType::PeeringAgreement;

	UPROPERTY(BlueprintReadOnly, Category = "Contract")
	EGTContractStatus Status = EGTContractStatus::Proposed;

	/** Corporation ID of the offering party. */
	UPROPERTY(BlueprintReadOnly, Category = "Contract")
	int32 OfferorCorporationId = -1;

	/** Corporation ID of the accepting party. */
	UPROPERTY(BlueprintReadOnly, Category = "Contract")
	int32 AcceptorCorporationId = -1;

	/** Simulation tick when the contract becomes active. */
	UPROPERTY(BlueprintReadOnly, Category = "Contract")
	int64 StartTick = 0;

	/** Duration in simulation ticks. 0 = indefinite. */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Contract")
	int64 DurationTicks = 0;

	/** Payment per tick from acceptor to offeror. */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Contract")
	double PricePerTick = 0.0;

	/** Capacity guaranteed in the contract (bandwidth units). */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Contract")
	float GuaranteedCapacity = 0.0f;

	/** Penalty amount if the contract is breached. */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Contract")
	double BreachPenalty = 0.0;
};

/**
 * FGTAlliance
 *
 * An alliance between two or more corporations for cooperative play.
 */
USTRUCT(BlueprintType)
struct GTMULTIPLAYER_API FGTAlliance
{
	GENERATED_BODY()

	UPROPERTY(BlueprintReadOnly, Category = "Alliance")
	int32 AllianceId = -1;

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Alliance")
	FString AllianceName;

	UPROPERTY(BlueprintReadOnly, Category = "Alliance")
	EGTAllianceStatus Status = EGTAllianceStatus::Proposed;

	/** Corporation IDs of member corporations. */
	UPROPERTY(BlueprintReadOnly, Category = "Alliance")
	TArray<int32> MemberCorporationIds;

	/** Simulation tick when the alliance was formed. */
	UPROPERTY(BlueprintReadOnly, Category = "Alliance")
	int64 FormedAtTick = 0;
};
