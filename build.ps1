param([switch]$Test, [switch]$EngineOnly, [string]$Runtime='win-x64')
$ErrorActionPreference='Stop'
$portableSDK=Join-Path $env:LOCALAPPDATA 'SourceDefenderBuildTools\dotnet\dotnet.exe'
$dotnet=if(Test-Path $portableSDK){$portableSDK}else{(Get-Command dotnet -ErrorAction Stop).Source}
$env:DOTNET_ROOT=Split-Path $dotnet
$env:PATH=(Split-Path $dotnet)+';'+$env:PATH
$env:DOTNET_NOLOGO='1'
$env:DOTNET_CLI_TELEMETRY_OPTOUT='1'
$publish=Join-Path $PSScriptRoot ('.build\engine\'+$Runtime)
$distribution=Join-Path $PSScriptRoot ('dist\'+$Runtime)
New-Item -ItemType Directory -Path $distribution -Force | Out-Null
Push-Location $PSScriptRoot
try {
    & $dotnet publish engine/SourceDefender.Engine.csproj -c Release -r $Runtime --self-contained true -p:PublishSingleFile=true -p:IncludeNativeLibrariesForSelfExtract=true -p:EnableCompressionInSingleFile=true -o $publish --nologo
    if($LASTEXITCODE -ne 0){throw 'Game engine build failed'}
    Copy-Item (Join-Path $publish 'SourceDefender.Engine.exe') (Join-Path $PSScriptRoot 'SourceDefender.Engine.exe') -Force
    Copy-Item (Join-Path $publish 'SourceDefender.Engine.exe') $distribution -Force
    if(!$EngineOnly) {
        $cargo=(Get-Command cargo -ErrorAction SilentlyContinue).Source
        if(!$cargo){$cargo=Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'}
        $rustc=Join-Path (Split-Path $cargo) 'rustc.exe'
        if((& $rustc -vV) -match 'host: x86_64-pc-windows-gnu') {
            $gcc=(Get-Command gcc -ErrorAction SilentlyContinue).Source
            if(!$gcc){$gcc=Join-Path $env:LOCALAPPDATA 'SourceDefenderBuildTools\w64devkit\bin\gcc.exe'}
            if(!(Test-Path $gcc)){throw 'Install MinGW-w64 (gcc, as, dlltool) or use Rust MSVC with Visual Studio Build Tools.'}
            $env:PATH=(Split-Path $gcc)+';'+(Split-Path $cargo)+';'+$env:PATH
            $sysroot=& $rustc --print sysroot
            $env:CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=Join-Path $sysroot 'lib\rustlib\x86_64-pc-windows-gnu\bin\rust-lld.exe'
            $env:RUSTFLAGS='-C linker-flavor=ld.lld -C link-self-contained=yes'
        }
        & $cargo build --release --locked
        if($LASTEXITCODE -ne 0){throw 'Rust interface build failed'}
        Copy-Item target/release/SourceDefender.exe $PSScriptRoot -Force
        Copy-Item target/release/SourceDefender.exe $distribution -Force
        if($Test) {
            & $cargo test --release --locked
            if($LASTEXITCODE -ne 0){throw 'Rust tests failed'}
        }
    }
    Copy-Item README.md $distribution -Force
    Copy-Item docs/MCP.md $distribution -Force
    Copy-Item docs/IDE.md $distribution -Force
    New-Item -ItemType Directory -Path (Join-Path $distribution 'docs') -Force | Out-Null
    Copy-Item docs/MCP.md (Join-Path $distribution 'docs') -Force
    Copy-Item docs/IDE.md (Join-Path $distribution 'docs') -Force
    if($Test) {
        $info=New-Object System.Diagnostics.ProcessStartInfo
        $info.FileName=Join-Path $PSScriptRoot 'SourceDefender.Engine.exe'
        $info.Arguments='--self-test'
        $info.UseShellExecute=$false
        $info.CreateNoWindow=$true
        $process=[System.Diagnostics.Process]::Start($info)
        $process.WaitForExit()
        Get-Content test-results.txt
        if($process.ExitCode -ne 0){throw 'Engine self-tests failed'}
    }
    Write-Host ('Build available in '+$distribution)
} finally {Pop-Location}
