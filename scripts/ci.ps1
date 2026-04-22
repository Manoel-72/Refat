# CI local (Windows): mesmo check basico que o workflow do GitHub.
# Uso: na raiz do repo, powershell -File scripts/ci.ps1
# Ou: .\scripts\ci.ps1

$ErrorActionPreference = 'Stop'
$repoRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
Set-Location $repoRoot

Write-Host '>> cargo build' -ForegroundColor Cyan
cargo build --verbose
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host '>> cargo test' -ForegroundColor Cyan
cargo test --verbose
exit $LASTEXITCODE
