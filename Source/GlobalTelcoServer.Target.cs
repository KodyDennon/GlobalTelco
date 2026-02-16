using UnrealBuildTool;
using System.Collections.Generic;

public class GlobalTelcoServerTarget : TargetRules
{
	public GlobalTelcoServerTarget(TargetInfo Target) : base(Target)
	{
		Type = TargetType.Server;
		DefaultBuildSettings = BuildSettingsVersion.Latest;
		IncludeOrderVersion = EngineIncludeOrderVersion.Latest;
		ExtraModuleNames.AddRange(new string[]
		{
			"GlobalTelco",
			"GTCore",
			"GTInfrastructure",
			"GTEconomy",
			"GTMultiplayer"
		});
	}
}
