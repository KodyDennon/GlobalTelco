using UnrealBuildTool;
using System.Collections.Generic;

public class GlobalTelcoTarget : TargetRules
{
	public GlobalTelcoTarget(TargetInfo Target) : base(Target)
	{
		Type = TargetType.Game;
		DefaultBuildSettings = BuildSettingsVersion.Latest;
		IncludeOrderVersion = EngineIncludeOrderVersion.Latest;
		ExtraModuleNames.AddRange(new string[]
		{
			"GlobalTelco",
			"GTCore",
			"GTInfrastructure",
			"GTEconomy",
			"GTMultiplayer",
			"GTFrontend"
		});
	}
}
