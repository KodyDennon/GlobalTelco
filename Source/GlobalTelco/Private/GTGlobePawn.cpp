#include "GTGlobePawn.h"
#include "Camera/CameraComponent.h"
#include "CesiumGlobeAnchorComponent.h"
#include "CesiumFlyToComponent.h"
#include "CesiumGeoreference.h"
#include "EnhancedInputComponent.h"
#include "EnhancedInputSubsystems.h"
#include "InputActionValue.h"
#include "Engine/World.h"
#include "GameFramework/PlayerController.h"

AGTGlobePawn::AGTGlobePawn()
{
	PrimaryActorTick.bCanEverTick = true;

	GlobeAnchor = CreateDefaultSubobject<UCesiumGlobeAnchorComponent>(TEXT("GlobeAnchor"));

	FlyToComponent = CreateDefaultSubobject<UCesiumFlyToComponent>(TEXT("FlyTo"));

	GlobeCamera = CreateDefaultSubobject<UCameraComponent>(TEXT("GlobeCamera"));
	GlobeCamera->SetupAttachment(RootComponent);
	GlobeCamera->SetRelativeLocation(FVector::ZeroVector);
	GlobeCamera->bUsePawnControlRotation = false;
}

void AGTGlobePawn::BeginPlay()
{
	Super::BeginPlay();

	// Add input mapping context.
	if (APlayerController* PC = Cast<APlayerController>(GetController()))
	{
		if (UEnhancedInputLocalPlayerSubsystem* InputSubsystem =
			ULocalPlayer::GetSubsystem<UEnhancedInputLocalPlayerSubsystem>(PC->GetLocalPlayer()))
		{
			if (GlobeMappingContext)
			{
				InputSubsystem->AddMappingContext(GlobeMappingContext, 0);
			}
		}

		PC->bShowMouseCursor = true;
		PC->bEnableClickEvents = true;
	}

	// Set initial position on globe.
	TargetLongitude = FocusLongitude;
	TargetLatitude = FocusLatitude;
	TargetAltitude = CurrentAltitude;

	if (GlobeAnchor)
	{
		GlobeAnchor->MoveToLongitudeLatitudeHeight(
			FVector(TargetLongitude, TargetLatitude, TargetAltitude));
	}
}

void AGTGlobePawn::Tick(float DeltaTime)
{
	Super::Tick(DeltaTime);
	UpdateCameraPosition(DeltaTime);
}

void AGTGlobePawn::SetupPlayerInputComponent(UInputComponent* PlayerInputComponent)
{
	Super::SetupPlayerInputComponent(PlayerInputComponent);

	UEnhancedInputComponent* EnhancedInput = Cast<UEnhancedInputComponent>(PlayerInputComponent);
	if (!EnhancedInput)
	{
		return;
	}

	if (OrbitAction)
	{
		EnhancedInput->BindAction(OrbitAction, ETriggerEvent::Triggered, this, &AGTGlobePawn::HandleOrbit);
	}
	if (OrbitToggleAction)
	{
		EnhancedInput->BindAction(OrbitToggleAction, ETriggerEvent::Started, this, &AGTGlobePawn::HandleOrbitToggle);
		EnhancedInput->BindAction(OrbitToggleAction, ETriggerEvent::Completed, this, &AGTGlobePawn::HandleOrbitToggle);
	}
	if (ZoomAction)
	{
		EnhancedInput->BindAction(ZoomAction, ETriggerEvent::Triggered, this, &AGTGlobePawn::HandleZoom);
	}
	if (PanAction)
	{
		EnhancedInput->BindAction(PanAction, ETriggerEvent::Triggered, this, &AGTGlobePawn::HandlePan);
	}
	if (PanToggleAction)
	{
		EnhancedInput->BindAction(PanToggleAction, ETriggerEvent::Started, this, &AGTGlobePawn::HandlePanToggle);
		EnhancedInput->BindAction(PanToggleAction, ETriggerEvent::Completed, this, &AGTGlobePawn::HandlePanToggle);
	}
	if (SelectAction)
	{
		EnhancedInput->BindAction(SelectAction, ETriggerEvent::Started, this, &AGTGlobePawn::HandleSelect);
	}
}

void AGTGlobePawn::HandleOrbit(const FInputActionValue& Value)
{
	if (!bOrbitActive)
	{
		return;
	}

	const FVector2D Delta = Value.Get<FVector2D>();

	// Horizontal drag rotates longitude, vertical drag rotates latitude.
	const double AltitudeScale = FMath::Clamp(CurrentAltitude / MaxAltitude, 0.01, 1.0);
	TargetLongitude -= Delta.X * OrbitSensitivity * AltitudeScale;
	TargetLatitude += Delta.Y * OrbitSensitivity * AltitudeScale;

	// Clamp latitude to avoid pole singularity.
	TargetLatitude = FMath::Clamp(TargetLatitude, -89.0, 89.0);

	// Wrap longitude.
	if (TargetLongitude > 180.0)
	{
		TargetLongitude -= 360.0;
	}
	else if (TargetLongitude < -180.0)
	{
		TargetLongitude += 360.0;
	}
}

void AGTGlobePawn::HandleOrbitToggle(const FInputActionValue& Value)
{
	bOrbitActive = Value.Get<bool>();
}

void AGTGlobePawn::HandleZoom(const FInputActionValue& Value)
{
	const float ScrollDelta = Value.Get<float>();

	// Logarithmic zoom for smooth transition across vast distances.
	const double LogAlt = FMath::Loge(TargetAltitude);
	const double NewLogAlt = LogAlt - ScrollDelta * ZoomSensitivity;
	TargetAltitude = FMath::Clamp(FMath::Exp(NewLogAlt), MinAltitude, MaxAltitude);
}

void AGTGlobePawn::HandlePan(const FInputActionValue& Value)
{
	if (!bPanActive)
	{
		return;
	}

	const FVector2D Delta = Value.Get<FVector2D>();

	// At low altitude, pan shifts the focus point on the globe surface.
	const double AltitudeScale = FMath::Clamp(CurrentAltitude / MaxAltitude, 0.001, 1.0);
	TargetLongitude -= Delta.X * OrbitSensitivity * AltitudeScale * 0.5;
	TargetLatitude += Delta.Y * OrbitSensitivity * AltitudeScale * 0.5;

	TargetLatitude = FMath::Clamp(TargetLatitude, -89.0, 89.0);

	if (TargetLongitude > 180.0)
	{
		TargetLongitude -= 360.0;
	}
	else if (TargetLongitude < -180.0)
	{
		TargetLongitude += 360.0;
	}
}

void AGTGlobePawn::HandlePanToggle(const FInputActionValue& Value)
{
	bPanActive = Value.Get<bool>();
}

void AGTGlobePawn::HandleSelect(const FInputActionValue& Value)
{
	APlayerController* PC = Cast<APlayerController>(GetController());
	if (!PC)
	{
		return;
	}

	FVector WorldLocation, WorldDirection;
	if (!PC->DeprojectMousePositionToWorld(WorldLocation, WorldDirection))
	{
		return;
	}

	// Line trace from mouse cursor into the world.
	FHitResult Hit;
	const FVector TraceEnd = WorldLocation + WorldDirection * 1000000000.0;

	if (GetWorld()->LineTraceSingleByChannel(Hit, WorldLocation, TraceEnd, ECC_Visibility))
	{
		OnGlobeClicked.Broadcast(Hit.ImpactPoint);
	}
}

void AGTGlobePawn::UpdateCameraPosition(float DeltaTime)
{
	const float Alpha = FMath::Clamp(DeltaTime * CameraSmoothing, 0.0f, 1.0f);

	// Smooth interpolation toward target values.
	FocusLongitude = FMath::Lerp(FocusLongitude, TargetLongitude, static_cast<double>(Alpha));
	FocusLatitude = FMath::Lerp(FocusLatitude, TargetLatitude, static_cast<double>(Alpha));
	CurrentAltitude = FMath::Lerp(CurrentAltitude, TargetAltitude, static_cast<double>(Alpha));

	// Update globe anchor position.
	if (GlobeAnchor)
	{
		GlobeAnchor->MoveToLongitudeLatitudeHeight(
			FVector(FocusLongitude, FocusLatitude, CurrentAltitude));
	}
}

void AGTGlobePawn::FlyTo(double Longitude, double Latitude, double Altitude)
{
	TargetLongitude = Longitude;
	TargetLatitude = FMath::Clamp(Latitude, -89.0, 89.0);
	TargetAltitude = FMath::Clamp(Altitude, MinAltitude, MaxAltitude);

	// Also use Cesium's FlyTo for cinematic movement if the component exists.
	if (FlyToComponent)
	{
		FlyToComponent->FlyToLocationLongitudeLatitudeHeight(
			FVector(Longitude, Latitude, Altitude),
			0.0f, 0.0f, false);
	}
}
