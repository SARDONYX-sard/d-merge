# d_merge_cli

Generates Nemesis/FNIS behavior patches for Skyrim SE/LE.

---

## Overview

There are two usage patterns depending on whether you use a mod manager with a
virtual filesystem (e.g. Mod Organizer 2) or manage mods directly.

### VFS mode (Mod Organizer 2)

`info mods --vfs` returns only the mod code (e.g. `colisc`), not a full path.
You must look up the Skyrim data directory from the registry and assemble the
full path yourself before writing the ini.

```txt
info skyrim-dir  →  info mods --vfs  →  (assemble paths + write ini)  →  patch
```

### Manual mode

`info mods` returns absolute or relative paths directly as the mod ID.
No path assembly is needed.

```txt
info mods  →  (write ini)  →  patch
```

---

## Quick start (cli + PowerShell)

```powershell
<#
.SYNOPSIS
    Generate Nemesis/FNIS behavior patches for Skyrim SE/LE.
.DESCRIPTION
    Supports both VFS mode (Mod Organizer 2) and manual mode.
    Set $VFS = $true for MO2 VFS, $false = manual mode (MO2).
    VFS mode:    glob points to Skyrim data dir (resolved from registry)
    Manual mode: glob points to MO2 mods directory
#>
$ErrorActionPreference = "Stop"
# ============================================================
# Encoding (set once, never revert)
# ============================================================
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$OutputEncoding           = [System.Text.Encoding]::UTF8
chcp 65001 | Out-Null

# ============================================================
# Configuration
# ============================================================
$VFS          = $false   # $true = VFS mode (Steam), $false = manual mode (MO2)

$MANUAL_GLOB  = "D:/GAME/ModOrganizer Skyrim SE/mods/*" # Manual mode : ids returned as full paths

$OUTPUT_DIR   = "./output"
$RESOURCE_DIR = "./assets/templates"
$BIN          = "./d_merge_cli"

# ============================================================
# Functions
# ============================================================

function Invoke-Cli {
    param(
        [string[]]$Arguments,
        [bool]$Capture = $true
    )
    if ($Capture) {
        $stdout = & $BIN @Arguments | Out-String
    } else {
        & $BIN @Arguments
        $stdout = $null
    }

    if ($LASTEXITCODE -ne 0) {
        throw "d_merge_cli failed: $($Arguments -join ' ')"
    }

    if ($Capture) { $stdout.Trim() }
}

function Write-File {
    param(
        [string]$Path,
        [string]$Content
    )
    $enc = New-Object System.Text.UTF8Encoding $false  # UTF-8 without BOM
    [System.IO.File]::WriteAllText($Path, $Content.Replace("`r`n", "`n"), $enc)
}

function Get-Mods {
    param(
        [string]$Glob,
        [bool]$Vfs
    )
    $cliArgs = @("info", "mods", "--glob", $Glob)
    if ($Vfs) { $cliArgs += "--vfs" }

    $json_str = Invoke-Cli $cliArgs
    Write-File "./mods.json" $json_str
    Write-Host "  wrote mods.json"

    $json_str | ConvertFrom-Json
}

function Write-IniFile {
    param(
        [string]$Path,
        [string[]]$Lines
    )
    $content = "; Auto-generated`n" + ($Lines -join "`n") + "`n"
    Write-File $Path $content
    Write-Host "  wrote $Path ($($Lines.Count) entries)"
}

function Invoke-Patch {
    param([string]$SkyrimDataGlob)

    & $BIN patch `
        --nemesis-ini           nemesis_ids.ini `
        --fnis-ini              fnis_ids.ini `
        --output-dir            $OUTPUT_DIR `
        --resource-dir          $RESOURCE_DIR `
        --skyrim-data-dir-glob  $SkyrimDataGlob

    if ($LASTEXITCODE -ne 0) { throw "patch failed" }
}

function Wait-ForConfirmation {
    param([string]$Message = "Edit the ini files if needed, then press any key to continue...")
    Write-Host ""
    Write-Host $Message -ForegroundColor Yellow
    Write-Host "  nemesis_ids.ini" -ForegroundColor Gray
    Write-Host "  fnis_ids.ini" -ForegroundColor Gray
    Write-Host ""
    $null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")
    Write-Host ""
}

# ============================================================
# Main
# ============================================================

function Main {
    if ($VFS) {
        $skyrim_data = Invoke-Cli @("info", "skyrim-dir", "--runtime", "SkyrimSE")
        Write-Host "Skyrim data dir: $skyrim_data"

        $mods = Get-Mods -Glob $skyrim_data -Vfs $true
        Write-Host "Found $($mods.Count) mods (VFS mode)"

        $nemesis_base  = Join-Path $skyrim_data "Nemesis_Engine\mod"
        $nemesis_paths = $mods | Where-Object { $_.mod_type -eq "nemesis" } |
            ForEach-Object { Join-Path $nemesis_base $_.id }
        $fnis_ids      = $mods | Where-Object { $_.mod_type -eq "fnis" } |
            ForEach-Object { $_.id }

        Write-IniFile "nemesis_ids.ini" $nemesis_paths
        Write-IniFile "fnis_ids.ini"    $fnis_ids
        Wait-ForConfirmation
        Invoke-Patch -SkyrimDataGlob $skyrim_data
    }
    else {
        $mods = Get-Mods -Glob $MANUAL_GLOB -Vfs $false
        Write-Host "Found $($mods.Count) mods (manual mode)"

        $nemesis_ids = $mods | Where-Object { $_.mod_type -eq "nemesis" } |
            ForEach-Object { $_.id }
        $fnis_ids    = $mods | Where-Object { $_.mod_type -eq "fnis" } |
            ForEach-Object { $_.id }

        Write-IniFile "nemesis_ids.ini" $nemesis_ids
        Write-IniFile "fnis_ids.ini"    $fnis_ids
        Wait-ForConfirmation
        Invoke-Patch -SkyrimDataGlob $MANUAL_GLOB
    }

    Write-Host "Done — behaviors written to $OUTPUT_DIR"
}

Main
```

---

## ini file format

### nemesis_ids.ini

Whether or not the directory is under VFS does not matter for the `patch` command.
When running within the MO2 virtualization environment, the path will always be
`<data dir>\Nemesis_Engine\mod\<mod_code>`.

```ini
; Nemesis mod IDs — order determines patch priority (top = highest)
D:\STEAM\steamapps\common\Skyrim Special Edition\Data\Nemesis_Engine\mod\slide
D:\STEAM\steamapps\common\Skyrim Special Edition\Data\Nemesis_Engine\mod\dmco
D:\STEAM\steamapps\common\Skyrim Special Edition\Data\Nemesis_Engine\mod\dwulkr
D:\STEAM\steamapps\common\Skyrim Special Edition\Data\Nemesis_Engine\mod\flinch
D:\STEAM\steamapps\common\Skyrim Special Edition\Data\Nemesis_Engine\mod\poise
D:\STEAM\steamapps\common\Skyrim Special Edition\Data\Nemesis_Engine\mod\para
D:\STEAM\steamapps\common\Skyrim Special Edition\Data\Nemesis_Engine\mod\scar
D:\STEAM\steamapps\common\Skyrim Special Edition\Data\Nemesis_Engine\mod\draw
```

### fnis_ids.ini

FNIS specifies the namespace located directly under the `animations` directory
within the `meshes` directory. Path resolution is handled internally.

Use the `--skyrim-data-dir-glob` option to specify the search range during patching

```ini
; FNIS mod IDs (namespaces)
FNISBase
FNISCreatureVersion
FNISZoo
P1FlyingRing
XPMSE
backgrab
backgrabnosneak
backstabnosneak
frontgrab
newfightcb1
```
