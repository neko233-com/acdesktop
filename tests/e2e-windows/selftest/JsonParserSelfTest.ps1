$ErrorActionPreference = 'Stop'
Import-Module (Join-Path $PSScriptRoot '../Harness.psm1') -Force

$json = '{"ok":true,"data":{"children":[{"value":2},null,false],"empty":[]}}'
$parsed = ConvertFrom-AgentJson -Json $json
if ($parsed['ok'] -ne $true -or $parsed['data']['children'].Count -ne 3 -or
    $parsed['data']['children'][0]['value'] -ne 2 -or
    $null -ne $parsed['data']['children'][1] -or
    $parsed['data']['children'][2] -ne $false -or $parsed['data']['empty'].Count -ne 0) {
    throw 'The JSON parser did not preserve the envelope and nested values.'
}

$depth = 300
$json = ('{"node":' * $depth) + 'true' + ('}' * $depth)
$parsed = ConvertFrom-AgentJson -Json $json
for ($i = 0; $i -lt $depth; $i++) { $parsed = $parsed['node'] }
if ($parsed -ne $true) { throw 'The JSON parser did not preserve deeply nested values.' }
Write-Output "JSON parser passed on PowerShell $($PSVersionTable.PSVersion)"
