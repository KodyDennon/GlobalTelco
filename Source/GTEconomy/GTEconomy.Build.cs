using UnrealBuildTool;

public class GTEconomy : ModuleRules
{
	public GTEconomy(ReadOnlyTargetRules Target) : base(Target)
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
