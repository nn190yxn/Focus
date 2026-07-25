[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateScript({ Test-Path -LiteralPath $_ -PathType Leaf })]
    [string]$BaselineInstallerPath,

    [Parameter(Mandatory = $true)]
    [ValidateScript({ Test-Path -LiteralPath $_ -PathType Leaf })]
    [string]$UpgradeInstallerPath,

    [string]$InstallDirectory = (Join-Path ([Environment]::GetFolderPath("LocalApplicationData")) "ArriveFocusSmoke"),
    [string]$AppDataDirectory = (Join-Path ([Environment]::GetFolderPath("ApplicationData")) "com.arrive.focus"),
    [string]$ExecutableName = "arrive-focus.exe",
    [string]$UninstallerName = "uninstall.exe",
    [ValidateRange(5, 120)]
    [int]$LaunchTimeoutSeconds = 30
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Write-SmokeStep {
    param([Parameter(Mandatory = $true)][string]$Message)

    Write-Host "[installer-smoke] $Message"
}

function Invoke-CheckedProcess {
    param(
        [Parameter(Mandatory = $true)][string]$FilePath,
        [Parameter(Mandatory = $true)][string[]]$ArgumentList,
        [Parameter(Mandatory = $true)][string]$Description
    )

    Write-SmokeStep $Description
    $process = Start-Process -FilePath $FilePath -ArgumentList $ArgumentList -Wait -PassThru
    if ($process.ExitCode -ne 0) {
        throw "$Description failed with exit code $($process.ExitCode)."
    }
}

function Wait-ForFile {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][int]$TimeoutSeconds
    )

    $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
    while ([DateTime]::UtcNow -lt $deadline) {
        if (Test-Path -LiteralPath $Path -PathType Leaf) {
            return
        }
        Start-Sleep -Milliseconds 250
    }

    throw "Timed out waiting for file: $Path"
}

function Wait-ForFileRemoval {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][int]$TimeoutSeconds
    )

    $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
    while ([DateTime]::UtcNow -lt $deadline) {
        if (-not (Test-Path -LiteralPath $Path)) {
            return
        }
        Start-Sleep -Milliseconds 250
    }

    throw "Timed out waiting for removal: $Path"
}

function Start-And-ProbeApplication {
    param(
        [Parameter(Mandatory = $true)][string]$ExecutablePath,
        [Parameter(Mandatory = $true)][string]$DatabasePath,
        [Parameter(Mandatory = $true)][int]$TimeoutSeconds
    )

    Write-SmokeStep "Starting the installed application"
    $process = Start-Process -FilePath $ExecutablePath -PassThru
    try {
        Wait-ForFile -Path $DatabasePath -TimeoutSeconds $TimeoutSeconds
        $process.Refresh()
        if ($process.HasExited) {
            throw "The installed application exited before the launch probe completed."
        }
    }
    finally {
        $process.Refresh()
        if (-not $process.HasExited) {
            Stop-Process -Id $process.Id
            $process.WaitForExit()
        }
    }
}

if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) {
    throw "This smoke test must run on Windows."
}

$baselineInstaller = (Resolve-Path -LiteralPath $BaselineInstallerPath).Path
$upgradeInstaller = (Resolve-Path -LiteralPath $UpgradeInstallerPath).Path
if ($baselineInstaller -eq $upgradeInstaller) {
    throw "BaselineInstallerPath and UpgradeInstallerPath must reference different installers."
}
if ([IO.Path]::GetExtension($baselineInstaller) -ne ".exe" -or [IO.Path]::GetExtension($upgradeInstaller) -ne ".exe") {
    throw "BaselineInstallerPath and UpgradeInstallerPath must reference NSIS .exe packages."
}

if (Test-Path -LiteralPath $InstallDirectory) {
    throw "InstallDirectory already exists. Run this test with a clean disposable Windows user."
}
if (Test-Path -LiteralPath $AppDataDirectory) {
    throw "AppDataDirectory already exists. Run this test with a clean disposable Windows user."
}

$processName = [IO.Path]::GetFileNameWithoutExtension($ExecutableName)
if (Get-Process -Name $processName -ErrorAction SilentlyContinue) {
    throw "$ExecutableName is already running. Close it before starting the smoke test."
}

$executablePath = Join-Path $InstallDirectory $ExecutableName
$uninstallerPath = Join-Path $InstallDirectory $UninstallerName
$databasePath = Join-Path $AppDataDirectory "arrive-focus.sqlite3"
$markerPath = Join-Path $AppDataDirectory "installer-smoke-data-marker.txt"
$markerValue = [Guid]::NewGuid().ToString("N")

Invoke-CheckedProcess -FilePath $baselineInstaller -ArgumentList @("/S", "/D=`"$InstallDirectory`"") -Description "Installing the baseline NSIS package silently"
Wait-ForFile -Path $executablePath -TimeoutSeconds $LaunchTimeoutSeconds
Wait-ForFile -Path $uninstallerPath -TimeoutSeconds $LaunchTimeoutSeconds
Start-And-ProbeApplication -ExecutablePath $executablePath -DatabasePath $databasePath -TimeoutSeconds $LaunchTimeoutSeconds

if ((Get-Item -LiteralPath $databasePath).Length -le 0) {
    throw "The application database is empty after the baseline launch."
}
Set-Content -LiteralPath $markerPath -Value $markerValue -NoNewline
$baselineHash = (Get-FileHash -LiteralPath $executablePath -Algorithm SHA256).Hash
$baselineVersion = (Get-Item -LiteralPath $executablePath).VersionInfo.ProductVersion

Invoke-CheckedProcess -FilePath $upgradeInstaller -ArgumentList @("/S", "/D=`"$InstallDirectory`"") -Description "Installing the upgrade NSIS package silently"
Wait-ForFile -Path $executablePath -TimeoutSeconds $LaunchTimeoutSeconds
$upgradeHash = (Get-FileHash -LiteralPath $executablePath -Algorithm SHA256).Hash
$upgradeVersion = (Get-Item -LiteralPath $executablePath).VersionInfo.ProductVersion
if ($upgradeHash -eq $baselineHash) {
    throw "The installed executable did not change during the upgrade."
}
if ((Get-Content -LiteralPath $markerPath -Raw) -ne $markerValue) {
    throw "Application data changed during the upgrade."
}
Start-And-ProbeApplication -ExecutablePath $executablePath -DatabasePath $databasePath -TimeoutSeconds $LaunchTimeoutSeconds

Invoke-CheckedProcess -FilePath $uninstallerPath -ArgumentList @("/S") -Description "Uninstalling the upgraded package silently"
Wait-ForFileRemoval -Path $executablePath -TimeoutSeconds $LaunchTimeoutSeconds
Wait-ForFileRemoval -Path $uninstallerPath -TimeoutSeconds $LaunchTimeoutSeconds
if (-not (Test-Path -LiteralPath $databasePath -PathType Leaf)) {
    throw "The application database was removed by uninstall."
}
if ((Get-Content -LiteralPath $markerPath -Raw) -ne $markerValue) {
    throw "Application data was removed or changed by uninstall."
}

Write-SmokeStep "Installation, launch, upgrade, uninstall, and data preservation checks passed"
[PSCustomObject]@{
    baselineVersion = $baselineVersion
    upgradeVersion = $upgradeVersion
    installDirectory = $InstallDirectory
    appDataDirectory = $AppDataDirectory
    dataPreserved = $true
} | ConvertTo-Json
