# 在 Windows 上运行，自动调用 WSL2 编译 MQDesk 的 Linux .deb 安装包
# 要求：已经安装 WSL2 + Ubuntu，且项目位于 d:\project\RabbitConsumerHub-main
# 注意：WSL2 需要主板 BIOS/UEFI 中开启虚拟化（Intel VT-x / AMD-V）

$ErrorActionPreference = "Stop"

$ProjectDir = "d:\project\RabbitConsumerHub-main"
$WslProjectDir = "/mnt/d/project/RabbitConsumerHub-main"

function Test-WslReady {
    $status = wsl --status 2>&1
    $global:LASTEXITCODE = 0
    if ($status -match "未安装|not installed|无法启动|cannot start|虚拟化|virtualization") {
        return $false
    }
    return $true
}

function Test-UbuntuInstalled {
    $list = wsl --list --quiet 2>&1
    $global:LASTEXITCODE = 0
    return $list -match "Ubuntu"
}

Write-Host "检查 WSL 可用性..." -ForegroundColor Cyan
if (-not (Test-WslReady)) {
    Write-Host ""
    Write-Host "❌ 当前环境无法启动 WSL2。" -ForegroundColor Red
    Write-Host "   常见原因：" -ForegroundColor Yellow
    Write-Host "   1. 主板 BIOS/UEFI 中未启用 Intel VT-x / AMD-V 虚拟化；" -ForegroundColor Yellow
    Write-Host "   2. WSL 或虚拟机平台可选组件未启用；" -ForegroundColor Yellow
    Write-Host "   3. 需要重启以完成 WSL 组件安装。" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "解决方案：" -ForegroundColor Cyan
    Write-Host "   A. 以管理员身份运行 scripts/setup-wsl.ps1 自动修复（如硬件支持）；" -ForegroundColor White
    Write-Host "   B. 在支持虚拟化的机器/服务器上运行 bash scripts/build-linux-deb.sh；" -ForegroundColor White
    Write-Host "   C. 使用 GitHub Actions 工作流 .github/workflows/build.yml 远程构建 .deb。" -ForegroundColor White
    exit 1
}

if (-not (Test-UbuntuInstalled)) {
    Write-Host ""
    Write-Host "❌ 未检测到 Ubuntu WSL 发行版。" -ForegroundColor Red
    Write-Host "请先以管理员身份运行 scripts/setup-wsl.ps1 安装 Ubuntu 24.04。" -ForegroundColor Yellow
    exit 1
}

Write-Host "开始通过 WSL2 编译 Linux 安装包..." -ForegroundColor Cyan
wsl -d Ubuntu-24.04 -e bash -c "cd '$WslProjectDir'; bash scripts/build-linux-deb.sh"

if ($LASTEXITCODE -eq 0) {
    Write-Host ""
    Write-Host "✅ 编译完成。生成的 .deb 文件在：" -ForegroundColor Green
    Write-Host "   src-tauri\target\x86_64-unknown-linux-gnu\release\bundle\deb\" -ForegroundColor Yellow
} else {
    Write-Host "❌ Linux 打包失败，请查看上方日志。" -ForegroundColor Red
    exit 1
}
