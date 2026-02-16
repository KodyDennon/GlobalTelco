#pragma once

#include "CoreMinimal.h"
#include "GameFramework/DefaultPawn.h"
#include "GTGlobePawn.generated.h"

class UCameraComponent;
class USpringArmComponent;
class UCesiumGlobeAnchorComponent;
class UCesiumFlyToComponent;
class UInputAction;
class UInputMappingContext;
class AGTGlobeActor;

/**
 * AGTGlobePawn
 *
 * Spectator-style pawn for navigating the globe. Provides:
 * - Orbit: Right-click + drag rotates around the globe center
 * - Zoom: Scroll wheel zooms in/out (from space view to street level)
 * - Pan: Middle-click + drag pans at close zoom levels
 * - Select: Left-click performs line trace to select hex parcel
 *
 * All camera movements use smooth interpolation. Uses Enhanced Input
 * with Input Actions and a Mapping Context assigned in the editor.
 *
 * Supports two positioning modes:
 * - Online: UCesiumGlobeAnchorComponent for WGS84 positioning
 * - Offline: Pure math orbit using UGTGeoCoordinates (singleplayer)
 */
UCLASS()
class GLOBALTELCO_API AGTGlobePawn : public ADefaultPawn
{
	GENERATED_BODY()

public:
	AGTGlobePawn();

	virtual void BeginPlay() override;
	virtual void Tick(float DeltaTime) override;
	virtual void SetupPlayerInputComponent(UInputComponent* PlayerInputComponent) override;

	// --- Input Actions (assign in editor or via code) ---

	UPROPERTY(EditAnywhere, BlueprintReadOnly, Category = "Input")
	TObjectPtr<UInputAction> OrbitAction;

	UPROPERTY(EditAnywhere, BlueprintReadOnly, Category = "Input")
	TObjectPtr<UInputAction> OrbitToggleAction;

	UPROPERTY(EditAnywhere, BlueprintReadOnly, Category = "Input")
	TObjectPtr<UInputAction> ZoomAction;

	UPROPERTY(EditAnywhere, BlueprintReadOnly, Category = "Input")
	TObjectPtr<UInputAction> PanAction;

	UPROPERTY(EditAnywhere, BlueprintReadOnly, Category = "Input")
	TObjectPtr<UInputAction> PanToggleAction;

	UPROPERTY(EditAnywhere, BlueprintReadOnly, Category = "Input")
	TObjectPtr<UInputAction> SelectAction;

	UPROPERTY(EditAnywhere, BlueprintReadOnly, Category = "Input")
	TObjectPtr<UInputMappingContext> GlobeMappingContext;

	// --- Camera Settings ---

	/** Minimum distance from globe center (meters). Closest zoom. */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Camera")
	double MinAltitude = 500.0;

	/** Maximum distance from globe center (meters). Space view. */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Camera")
	double MaxAltitude = 50000000.0;

	/** Current altitude above the globe surface (meters). */
	UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "Camera")
	double CurrentAltitude = 20000000.0;

	/** Mouse orbit sensitivity. */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Camera")
	float OrbitSensitivity = 0.3f;

	/** Scroll zoom speed factor. */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Camera")
	float ZoomSensitivity = 0.1f;

	/** Smoothing speed for camera interpolation (higher = snappier). */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Camera")
	float CameraSmoothing = 8.0f;

	/** Current longitude of the camera focus point (degrees). */
	UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "Camera")
	double FocusLongitude = 0.0;

	/** Current latitude of the camera focus point (degrees). */
	UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "Camera")
	double FocusLatitude = 20.0;

	/** Fly the camera to a specific longitude/latitude/altitude with smooth interpolation. */
	UFUNCTION(BlueprintCallable, Category = "Camera")
	void FlyTo(double Longitude, double Latitude, double Altitude);

	/** Delegate fired when user clicks the globe. Passes the hit world position. */
	DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FOnGlobeClicked, FVector, HitLocation);

	UPROPERTY(BlueprintAssignable, Category = "Globe")
	FOnGlobeClicked OnGlobeClicked;

protected:
	UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "Components")
	TObjectPtr<UCesiumGlobeAnchorComponent> GlobeAnchor;

	UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "Components")
	TObjectPtr<UCesiumFlyToComponent> FlyToComponent;

	UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "Components")
	TObjectPtr<UCameraComponent> GlobeCamera;

	void HandleOrbit(const struct FInputActionValue& Value);
	void HandleOrbitToggle(const struct FInputActionValue& Value);
	void HandleZoom(const struct FInputActionValue& Value);
	void HandlePan(const struct FInputActionValue& Value);
	void HandlePanToggle(const struct FInputActionValue& Value);
	void HandleSelect(const struct FInputActionValue& Value);

	void UpdateCameraPosition(float DeltaTime);

	/** Update camera using pure math (offline mode). */
	void UpdateCameraPositionOffline(float DeltaTime);

	/** Whether we're in offline mode (detected at BeginPlay). */
	bool bOfflineMode = false;

	/** Cached reference to the globe actor for coordinate conversion. */
	UPROPERTY()
	TObjectPtr<AGTGlobeActor> CachedGlobeActor;

private:
	/** Current target values for smooth interpolation. */
	double TargetAltitude = 20000000.0;
	double TargetLongitude = 0.0;
	double TargetLatitude = 20.0;

	bool bOrbitActive = false;
	bool bPanActive = false;
};
