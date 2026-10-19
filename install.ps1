# Lumen installer for Windows.
#
#   irm https://raw.githubusercontent.com/AbelHristodor/lumen/main/install.ps1 | iex
#
# Downloads the latest Lumen msi and installs it for the current user into
# %LOCALAPPDATA%\Programs\Lumen, with Start menu and Desktop shortcuts. No
# admin rights needed.

$ErrorActionPreference = "Stop"

$repo = "AbelHristodor/lumen"

# The asset names carry the version, so ask GitHub which tag is latest.
$release = Invoke-RestMethod -UseBasicParsing "https://api.github.com/repos/$repo/releases/latest"
$tag = $release.tag_name
$version = $tag.TrimStart("v")
$asset = "lumen-$version-windows-x64.msi"
$url = "https://github.com/$repo/releases/download/$tag/$asset"

$msi = Join-Path $env:TEMP $asset
Write-Host "Downloading $asset ..."
try {
    Invoke-WebRequest -Uri $url -OutFile $msi -UseBasicParsing
} catch {
    Write-Error "Download failed from $url."
    exit 1
}

$p = Start-Process msiexec.exe -ArgumentList "/i", "`"$msi`"", "/passive" -Wait -PassThru
Remove-Item $msi -ErrorAction SilentlyContinue
if ($p.ExitCode -ne 0) {
    Write-Error "The installer failed with code $($p.ExitCode)."
    exit 1
}
Write-Host "Installed Lumen $version."

# v1.x installed to Programs\hymnal-gui and added it to PATH. Clean that up.
$oldDir = Join-Path $env:LOCALAPPDATA "Programs\hymnal-gui"
Remove-Item -Recurse -Force $oldDir -ErrorAction SilentlyContinue
Remove-Item (Join-Path ([Environment]::GetFolderPath("Desktop")) "SDA Manager.lnk") -ErrorAction SilentlyContinue
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath) {
    $kept = ($userPath -split ";") | Where-Object { $_ -and $_ -ne $oldDir }
    [Environment]::SetEnvironmentVariable("Path", ($kept -join ";"), "User")
}
