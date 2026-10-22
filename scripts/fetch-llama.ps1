# Downloads the pinned llama.cpp Vulkan build and unpacks what llama-server needs into ./llama.
$ErrorActionPreference = 'Stop'
$build = 'b11815'
$sha256 = 'FF71DAD73D77C6673087F64FBB8F52EBDDCA3DC74E0CBFBA09A06A231583F8A5'
$root = Split-Path -Parent $PSScriptRoot
$out = Join-Path $root 'llama'

$licenseFile = Join-Path $out 'LICENSE-llama.cpp'
if ((Test-Path "$out/VERSION") -and ((Get-Content "$out/VERSION") -eq $build) -and (Test-Path $licenseFile) -and ((Get-Item $licenseFile).Length -gt 0)) {
    Write-Host "llama.cpp $build already present"
    exit 0
}

$tmp = Join-Path ([IO.Path]::GetTempPath()) "llama-$build"
New-Item -ItemType Directory -Force $tmp | Out-Null
$zip = Join-Path $tmp 'llama.zip'
Invoke-WebRequest "https://github.com/ggml-org/llama.cpp/releases/download/$build/llama-$build-bin-win-vulkan-x64.zip" -OutFile $zip
if ((Get-FileHash $zip -Algorithm SHA256).Hash -ne $sha256) { throw "llama.cpp archive checksum mismatch" }

$x = Join-Path $tmp 'x'
Expand-Archive $zip $x -Force
if (Test-Path $out) { Remove-Item -Recurse -Force $out }
New-Item -ItemType Directory $out | Out-Null
$keep = @('llama-server.exe', 'llama-server-impl.dll', 'llama-common.dll', 'llama.dll', 'mtmd.dll', 'libomp.dll', 'ggml*.dll', 'LICENSE*')
foreach ($pattern in $keep) { Copy-Item (Join-Path $x $pattern) $out }
# The release zip carries no llama.cpp licence (only LICENSE-LLVM-OpenMP); ship the MIT notice from the same tag.
Invoke-WebRequest "https://raw.githubusercontent.com/ggml-org/llama.cpp/$build/LICENSE" -OutFile $licenseFile
if (-not (Test-Path $licenseFile) -or (Get-Item $licenseFile).Length -eq 0) { throw "llama.cpp LICENSE missing or empty" }
Set-Content "$out/VERSION" $build
Write-Host "llama.cpp $build -> $out"
