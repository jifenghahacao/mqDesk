# 截取 MQDesk 操作手册所需的应用界面截图
# 前置条件：npm run dev 已启动，且 src/lib/api.js 临时指向 api-mock.js

param(
    [string]$BaseUrl = "http://127.0.0.1:1420",
    [string]$OutDir = "public\manual\screenshots",
    [int]$Width = 1366,
    [int]$Height = 768,
    [int]$WaitSeconds = 4
)

$ErrorActionPreference = "Stop"
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$captureScript = Join-Path $scriptDir "capture-screenshot.ps1"

$shots = @(
    @{ Name = "main-interface";          Url = "$BaseUrl/?view=overview" },
    @{ Name = "test-main";               Url = "$BaseUrl/?view=connections&noConnection" },
    @{ Name = "new-connection";          Url = "$BaseUrl/?view=connections&create=1" },
    @{ Name = "overview-dashboard";      Url = "$BaseUrl/?view=overview" },
    @{ Name = "queue-list";              Url = "$BaseUrl/?view=queues" },
    @{ Name = "queue-detail";            Url = "$BaseUrl/?view=queue-detail&queue=order.created" },
    @{ Name = "queue-detail-bindings";   Url = "$BaseUrl/?view=queue-detail&queue=order.created&tab=bindings" },
    @{ Name = "publish-message";         Url = "$BaseUrl/?view=messages&tab=send" },
    @{ Name = "message-inspect";         Url = "$BaseUrl/?view=messages&tab=inspect" },
    @{ Name = "connections-channels";    Url = "$BaseUrl/?view=rabbit-connections" },
    @{ Name = "consumer-studio";         Url = "$BaseUrl/?view=consumer-studio" },
    @{ Name = "consumers-list";          Url = "$BaseUrl/?view=consumers" },
    @{ Name = "nodes-view";              Url = "$BaseUrl/?view=nodes" },
    @{ Name = "stale-banner";            Url = "$BaseUrl/?view=overview&stale=1" },
    @{ Name = "settings-page";           Url = "$BaseUrl/?view=settings" }
)

foreach ($shot in $shots) {
    $out = Join-Path $OutDir "$($shot.Name).png"
    Write-Host "Capturing $($shot.Name) ..."
    & $captureScript -Url $shot.Url -OutputPath $out -Width $Width -Height $Height -WaitSeconds $WaitSeconds
}

Write-Host "All screenshots saved to $OutDir"
