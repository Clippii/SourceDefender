param([switch]$Test)
$ErrorActionPreference='Stop'
& dotnet build (Join-Path $PSScriptRoot 'Service.csproj') -c Release --nologo
if($LASTEXITCODE -ne 0){throw 'Compile failed'}
if($Test){& dotnet run --project (Join-Path $PSScriptRoot 'Service.csproj') -c Release --no-build -- --test;if($LASTEXITCODE -ne 0){throw 'Tests failed'}}
