using UnrealBuildTool;

public class GTInfrastructure : ModuleRules
{
	public GTInfrastructure(ReadOnlyTargetRules Target) : base(Target)
	{
		PCHUsage = PCHUsageMode.UseExplicitOrSharedPCHs;

		PublicDependencyModuleNames.AddRange(new string[]
		{
			"Core",
			"CoreUObject",
			"Engine",
			"GTCore"
		});

		PrivateDependencyModuleNames.AddRange(new string[] { });
	}
}
