using UnrealBuildTool;

public class GlobalTelco : ModuleRules
{
	public GlobalTelco(ReadOnlyTargetRules Target) : base(Target)
	{
		PCHUsage = PCHUsageMode.UseExplicitOrSharedPCHs;

		PublicDependencyModuleNames.AddRange(new string[]
		{
			"Core",
			"CoreUObject",
			"Engine",
			"InputCore",
			"EnhancedInput",
			"CesiumRuntime",
			"GTCore",
			"GTInfrastructure",
			"GTEconomy",
			"GTMultiplayer",
			"GTFrontend"
		});

		PrivateDependencyModuleNames.AddRange(new string[] { });
	}
}
