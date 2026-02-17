#include "GTGlobePawn.h"
#include "GTGlobeActor.h"
#include "GTGeoCoordinates.h"
#include "Camera/CameraComponent.h"
#include "CesiumGlobeAnchorComponent.h"
#include "CesiumFlyToComponent.h"
#include "CesiumGeoreference.h"
#include "EnhancedInputComponent.h"
#include "EnhancedInputSubsystems.h"
#include "InputAction.h"
#include "InputActionValue.h"
#include "InputMappingContext.h"
#include "InputModifiers.h"
#include "InputTriggers.h"
#include "Engine/World.h"
#include "GameFramework/PlayerController.h"
#include "Kismet/GameplayStatics.h"

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

	// Detect offline mode by finding the globe actor.
	CachedGlobeActor = Cast<AGTGlobeActor>(
		UGameplayStatics::GetActorOfClass(GetWorld(), AGTGlobeActor::StaticClass()));
	bOfflineMode = CachedGlobeActor && CachedGlobeActor->IsOfflineMode();

	if (bOfflineMode)
	{
		UE_LOG(LogTemp, Log, TEXT("GTGlobePawn: Offline mode — using math-based orbit."));
		// Disable Cesium components in offline mode.
		if (GlobeAnchor)
		{
			GlobeAnchor->DestroyComponent();
			GlobeAnchor = nullptr;
		}
		if (FlyToComponent)
		{
			FlyToComponent->DestroyComponent();
			FlyToComponent = nullptr;
		}
	}

	// Create input actions and mapping context programmatically if not set.
	CreateDefaultInputActions();

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

	// Set initial position.
	TargetLongitude = FocusLongitude;
	TargetLatitude = FocusLatitude;
	TargetAltitude = CurrentAltitude;

	if (bOfflineMode)
	{
		// Set initial position immediately.
		UpdateCameraPositionOffline(0.0f);
	}
	else if (GlobeAnchor)
	{
		GlobeAnchor->MoveToLongitudeLatitudeHeight(
			FVector(TargetLongitude, TargetLatitude, TargetAltitude));
	}
}

void AGTGlobePawn::Tick(float DeltaTime)
{
	Super::Tick(DeltaTime);

	if (bOfflineMode)
	{
		UpdateCameraPositionOffline(DeltaTime);
	}
	else
	{
		UpdateCameraPosition(DeltaTime);
	}
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

	const double AltitudeScale = FMath::Clamp(CurrentAltitude / MaxAltitude, 0.01, 1.0);
	TargetLongitude -= Delta.X * OrbitSensitivity * AltitudeScale;
	TargetLatitude += Delta.Y * OrbitSensitivity * AltitudeScale;

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

void AGTGlobePawn::HandleOrbitToggle(const FInputActionValue& Value)
{
	bOrbitActive = Value.Get<bool>();
}

void AGTGlobePawn::HandleZoom(const FInputActionValue& Value)
{
	const float ScrollDelta = Value.Get<float>();

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

	FocusLongitude = FMath::Lerp(FocusLongitude, TargetLongitude, static_cast<double>(Alpha));
	FocusLatitude = FMath::Lerp(FocusLatitude, TargetLatitude, static_cast<double>(Alpha));
	CurrentAltitude = FMath::Lerp(CurrentAltitude, TargetAltitude, static_cast<double>(Alpha));

	if (GlobeAnchor)
	{
		GlobeAnchor->MoveToLongitudeLatitudeHeight(
			FVector(FocusLongitude, FocusLatitude, CurrentAltitude));
	}
}

void AGTGlobePawn::UpdateCameraPositionOffline(float DeltaTime)
{
	const float Alpha = (DeltaTime > 0.0f) ? FMath::Clamp(DeltaTime * CameraSmoothing, 0.0f, 1.0f) : 1.0f;

	FocusLongitude = FMath::Lerp(FocusLongitude, TargetLongitude, static_cast<double>(Alpha));
	FocusLatitude = FMath::Lerp(FocusLatitude, TargetLatitude, static_cast<double>(Alpha));
	CurrentAltitude = FMath::Lerp(CurrentAltitude, TargetAltitude, static_cast<double>(Alpha));

	// Compute camera world position orbiting the globe at current altitude.
	const FVector CameraWorldPos = UGTGeoCoordinates::LonLatHeightToWorld(
		FocusLongitude, FocusLatitude, CurrentAltitude);

	SetActorLocation(CameraWorldPos);

	// Point camera toward globe center (origin).
	const FVector ToCenter = -CameraWorldPos.GetSafeNormal();
	const FRotator LookAtRotation = ToCenter.Rotation();
	SetActorRotation(LookAtRotation);
}

void AGTGlobePawn::FlyTo(double Longitude, double Latitude, double Altitude)
{
	TargetLongitude = Longitude;
	TargetLatitude = FMath::Clamp(Latitude, -89.0, 89.0);
	TargetAltitude = FMath::Clamp(Altitude, MinAltitude, MaxAltitude);

	// Use Cesium's FlyTo for cinematic movement in online mode.
	if (!bOfflineMode && FlyToComponent)
	{
		FlyToComponent->FlyToLocationLongitudeLatitudeHeight(
			FVector(Longitude, Latitude, Altitude),
			0.0f, 0.0f, false);
	}
}

void AGTGlobePawn::CreateDefaultInputActions()
{
	// Only create if no editor assets are assigned.
	if (GlobeMappingContext)
	{
		return;
	}

	// Orbit: Mouse XY (2D axis) — right-click drag.
	if (!OrbitAction)
	{
		OrbitAction = NewObject<UInputAction>(this, TEXT("IA_Orbit"));
		OrbitAction->ValueType = EInputActionValueType::Axis2D;
	}

	// OrbitToggle: Right mouse button (bool).
	if (!OrbitToggleAction)
	{
		OrbitToggleAction = NewObject<UInputAction>(this, TEXT("IA_OrbitToggle"));
		OrbitToggleAction->ValueType = EInputActionValueType::Boolean;
	}

	// Zoom: Mouse wheel (1D axis).
	if (!ZoomAction)
	{
		ZoomAction = NewObject<UInputAction>(this, TEXT("IA_Zoom"));
		ZoomAction->ValueType = EInputActionValueType::Axis1D;
	}

	// Pan: Mouse XY (2D axis) — middle-click drag.
	if (!PanAction)
	{
		PanAction = NewObject<UInputAction>(this, TEXT("IA_Pan"));
		PanAction->ValueType = EInputActionValueType::Axis2D;
	}

	// PanToggle: Middle mouse button (bool).
	if (!PanToggleAction)
	{
		PanToggleAction = NewObject<UInputAction>(this, TEXT("IA_PanToggle"));
		PanToggleAction->ValueType = EInputActionValueType::Boolean;
	}

	// Select: Left mouse button (bool).
	if (!SelectAction)
	{
		SelectAction = NewObject<UInputAction>(this, TEXT("IA_Select"));
		SelectAction->ValueType = EInputActionValueType::Boolean;
	}

	// Create mapping context and bind keys.
	GlobeMappingContext = NewObject<UInputMappingContext>(this, TEXT("IMC_Globe"));

	// Orbit: Mouse2D
	{
		FEnhancedActionKeyMapping& Mapping = GlobeMappingContext->MapKey(OrbitAction, EKeys::Mouse2D);
		(void)Mapping;
	}

	// OrbitToggle: Right Mouse Button
	GlobeMappingContext->MapKey(OrbitToggleAction, EKeys::RightMouseButton);

	// Zoom: Mouse Wheel Axis
	GlobeMappingContext->MapKey(ZoomAction, EKeys::MouseWheelAxis);

	// Pan: Mouse2D (same input, different toggle)
	GlobeMappingContext->MapKey(PanAction, EKeys::Mouse2D);

	// PanToggle: Middle Mouse Button
	GlobeMappingContext->MapKey(PanToggleAction, EKeys::MiddleMouseButton);

	// Select: Left Mouse Button
	GlobeMappingContext->MapKey(SelectAction, EKeys::LeftMouseButton);

	UE_LOG(LogTemp, Log, TEXT("GTGlobePawn: Created default input actions and mapping context."));
}
