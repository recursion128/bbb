# Desktop launcher wrapper for the Windows x86_64 bbb-client binary.
# Resolves a sibling bbb-client.exe in extracted release archives first, then PATH.

$ErrorActionPreference = "Stop"

$SiblingBin = Join-Path $PSScriptRoot "bbb-client.exe"
if (Test-Path -LiteralPath $SiblingBin -PathType Leaf) {
    $ClientBin = $SiblingBin
} else {
    $Cmd = Get-Command "bbb-client.exe" -ErrorAction SilentlyContinue
    if ($null -ne $Cmd) {
        $ClientBin = $Cmd.Source
    } else {
        [Console]::Error.WriteLine("bbb-client.exe not found next to $PSCommandPath or on PATH")
        exit 1
    }
}

function Test-ClientFlag {
    param(
        [string]$Needle,
        [string[]]$ArgList
    )
    foreach ($item in $ArgList) {
        if ($item -eq $Needle -or $item.StartsWith("$Needle=")) {
            return $true
        }
    }
    return $false
}

$ClientArgs = @($args | ForEach-Object { [string]$_ })
$DefaultGameDir = if ($env:APPDATA) {
    Join-Path $env:APPDATA "bbb-client\game"
} elseif ($env:USERPROFILE) {
    Join-Path $env:USERPROFILE "AppData\Roaming\bbb-client\game"
} else {
    Join-Path $PSScriptRoot "game"
}
$GameDir = if ($env:BBB_GAME_DIR) { $env:BBB_GAME_DIR } else { $DefaultGameDir }

$PrefixArgs = [System.Collections.Generic.List[string]]::new()
$IsHelpOrVersion = (Test-ClientFlag "--help" $ClientArgs) -or
    (Test-ClientFlag "-h" $ClientArgs) -or
    (Test-ClientFlag "--version" $ClientArgs) -or
    (Test-ClientFlag "-V" $ClientArgs)

if (-not $IsHelpOrVersion) {
    if (-not (Test-ClientFlag "--game-dir" $ClientArgs)) {
        New-Item -ItemType Directory -Force -Path $GameDir | Out-Null
        $PrefixArgs.Add("--game-dir")
        $PrefixArgs.Add($GameDir)
    }
    if ($env:BBB_VANILLA_ROOT -and -not (Test-ClientFlag "--vanilla-root" $ClientArgs)) {
        $PrefixArgs.Add("--vanilla-root")
        $PrefixArgs.Add($env:BBB_VANILLA_ROOT)
    }
    if ($env:BBB_SERVER -and
        -not (Test-ClientFlag "--server" $ClientArgs) -and
        -not (Test-ClientFlag "--quick-play-multiplayer" $ClientArgs) -and
        -not (Test-ClientFlag "--status-server" $ClientArgs)) {
        $PrefixArgs.Add("--server")
        $PrefixArgs.Add($env:BBB_SERVER)
    }
    if ($env:BBB_FULLSCREEN -match "^(?i:1|true|yes)$" -and
        -not (Test-ClientFlag "--fullscreen" $ClientArgs)) {
        $PrefixArgs.Add("--fullscreen")
    }
}

$AllArgs = @($PrefixArgs.ToArray()) + $ClientArgs
& $ClientBin @AllArgs
exit $LASTEXITCODE
