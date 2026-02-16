using UnrealBuildTool;

public class GTFrontend : ModuleRules
{
	public GTFrontend(ReadOnlyTargetRules Target) : base(Target)
	{
		PCHUsage = PCHUsageMode.UseExplicitOrSharedPCHs;

		PublicDependencyModuleNames.AddRange(new string[]
		{
			"Core",
			"CoreUObject",
			"Engine",
			"CesiumRuntime",
			"GTCore",
			"GTEconomy",
			"GTMultiplayer",
			"GTInfrastructure",
			"Slate",
			"SlateCore",
			"UMG"
		});

		PrivateDependencyModuleNames.AddRange(new string[] { });
	}
}
