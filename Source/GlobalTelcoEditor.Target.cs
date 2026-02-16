using UnrealBuildTool;
using System.Collections.Generic;

public class GlobalTelcoEditorTarget : TargetRules
{
	public GlobalTelcoEditorTarget(TargetInfo Target) : base(Target)
	{
		Type = TargetType.Editor;
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
