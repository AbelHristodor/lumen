# Lumen installer for Windows.
#
#   irm https://raw.githubusercontent.com/AbelHristodor/lumen/main/install.ps1 | iex
#
# Downloads the latest released lumen.exe and installs it to
# %LOCALAPPDATA%\Programs\lumen, adding that directory to the user PATH
# and creating a Desktop shortcut.

$ErrorActionPreference = "Stop"

$repo   = "AbelHristodor/lumen"
$target = "x86_64-pc-windows-msvc"
$asset  = "lumen-$target.zip"
$url    = "https://github.com/$repo/releases/latest/download/$asset"

$installDir = Join-Path $env:LOCALAPPDATA "Programs\lumen"
New-Item -ItemType Directory -Force -Path $installDir | Out-Null

$tmp = New-Item -ItemType Directory -Force -Path (Join-Path $env:TEMP "lumen-install")
$zip = Join-Path $tmp $asset

Write-Host "Downloading $asset ..."
try {
    Invoke-WebRequest -Uri $url -OutFile $zip -UseBasicParsing
} catch {
    Write-Error "Download failed from $url. Make sure a release has been published for $repo."
    exit 1
}

Expand-Archive -Path $zip -DestinationPath $tmp -Force
$exePath = Join-Path $installDir "lumen.exe"
Copy-Item (Join-Path $tmp "lumen-$target\lumen.exe") $exePath -Force
Remove-Item -Recurse -Force $tmp

Write-Host "Installed lumen.exe to $installDir"

# Before the rename the app lived in Programs\sda-manager.
$oldDir = Join-Path $env:LOCALAPPDATA "Programs\sda-manager"
Remove-Item -Recurse -Force $oldDir -ErrorAction SilentlyContinue

# Create a Desktop shortcut pointing at the installed exe. GetFolderPath resolves
# a redirected Desktop (e.g. OneDrive) correctly; WScript.Shell ships with Windows.
try {
    $desktop  = [Environment]::GetFolderPath("Desktop")
    Remove-Item (Join-Path $desktop "SDA Manager.lnk") -ErrorAction SilentlyContinue
    $lnkPath  = Join-Path $desktop "Lumen.lnk"
    $shell    = New-Object -ComObject WScript.Shell
    $shortcut = $shell.CreateShortcut($lnkPath)
    $shortcut.TargetPath       = $exePath
    $shortcut.WorkingDirectory = $installDir
    $shortcut.IconLocation     = $exePath
    $shortcut.Description       = "Lumen"
    $shortcut.Save()
    Write-Host "Created Desktop shortcut: $lnkPath"
} catch {
    Write-Warning "Could not create Desktop shortcut: $($_.Exception.Message)"
}

# Add to the user PATH if it isn't already there.
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
$userPath = (($userPath -split ";") | Where-Object { $_ -and $_ -ne $oldDir }) -join ";"
[Environment]::SetEnvironmentVariable("Path", $userPath, "User")
if ($userPath -notlike "*$installDir*") {
    $newPath = if ($userPath) { "$userPath;$installDir" } else { $installDir }
    [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
    Write-Host "Added $installDir to your user PATH. Restart your terminal to pick it up."
}
