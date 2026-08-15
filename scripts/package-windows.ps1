$ErrorActionPreference = 'Stop'

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$manifestPath = Join-Path $repositoryRoot 'Cargo.toml'
$manifest = Get-Content -LiteralPath $manifestPath -Raw
$versionMatch = [regex]::Match($manifest, '(?m)^version\s*=\s*"([^"]+)"')
if (-not $versionMatch.Success) {
    throw 'Could not read the package version from Cargo.toml.'
}

$version = $versionMatch.Groups[1].Value
$packageName = "Lilo-$version-windows-x64"
$distRoot = Join-Path $repositoryRoot 'dist'
$packageDirectory = Join-Path $distRoot $packageName
$archivePath = Join-Path $distRoot "$packageName.zip"
$resolvedDistRoot = [System.IO.Path]::GetFullPath($distRoot)
$resolvedPackageDirectory = [System.IO.Path]::GetFullPath($packageDirectory)
if ([System.IO.Path]::GetDirectoryName($resolvedPackageDirectory) -ne $resolvedDistRoot) {
    throw 'Refusing to package outside the repository dist directory.'
}

Push-Location $repositoryRoot
try {
    cargo build --release --locked
    if ($LASTEXITCODE -ne 0) {
        throw "cargo build failed with exit code $LASTEXITCODE."
    }

    $executable = Join-Path $repositoryRoot 'target\release\Lilo.exe'
    if (-not (Test-Path -LiteralPath $executable -PathType Leaf)) {
        throw 'The release executable was not produced.'
    }

    New-Item -ItemType Directory -Path $resolvedDistRoot -Force | Out-Null
    if (Test-Path -LiteralPath $packageDirectory) {
        Remove-Item -LiteralPath $packageDirectory -Recurse -Force
    }
    if (Test-Path -LiteralPath $archivePath) {
        Remove-Item -LiteralPath $archivePath -Force
    }
    New-Item -ItemType Directory -Path $packageDirectory | Out-Null

    Copy-Item -LiteralPath $executable -Destination $packageDirectory
    foreach ($document in @('README.md', 'ROADMAP.md', 'CHANGELOG.md', 'RELEASE.md', 'LICENSE')) {
        Copy-Item -LiteralPath (Join-Path $repositoryRoot $document) -Destination $packageDirectory
    }

    Compress-Archive -LiteralPath $packageDirectory -DestinationPath $archivePath -CompressionLevel Optimal
    $checksum = Get-FileHash -LiteralPath $archivePath -Algorithm SHA256
    "$($checksum.Hash)  $([System.IO.Path]::GetFileName($archivePath))" |
        Set-Content -LiteralPath "$archivePath.sha256" -Encoding ascii

    Write-Host "Created $archivePath"
    Write-Host "SHA256: $($checksum.Hash)"
}
finally {
    Pop-Location
}
