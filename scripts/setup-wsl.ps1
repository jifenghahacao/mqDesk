# 在 Windows 上以管理员身份运行，安装 WSL2 + Ubuntu 24.04
# 说明：WSL2 需要 Windows 10 版本 2004+ 或 Windows 11，且主板 BIOS/UEFI 已开启虚拟化

$ErrorActionPreference = "Stop"

function Test-Admin {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object Security.Principal.WindowsPrincipal($identity)
    return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Test-VirtualizationEnabled {
    try {
        $info = Get-ComputerInfo -Property HyperVisorPresent, HyperVRequirementVirtualizationFirmwareEnabled
        if ($info.HyperVisorPresent -eq $true) {
            return $true
        }
        if ($info.HyperVRequirementVirtualizationFirmwareEnabled -eq $false) {
            return $false
        }
    } catch {
        # 旧版系统或权限不足时跳过精确检测
    }

    # 通过 wsl --status 辅助判断
    $status = wsl --status 2>&1
    $global:LASTEXITCODE = 0
    if ($status -match "未启用虚拟化|virtualization") {
        return $false
    }
    return $true
}

if (-not (Test-Admin)) {
    Write-Host "请以管理员身份运行本脚本。" -ForegroundColor Red
    exit 1
}

if (-not (Test-VirtualizationEnabled)) {
    Write-Host ""
    Write-Host "❌ 检测到虚拟化未启用，WSL2 将无法启动。" -ForegroundColor Red
    Write-Host "   请进入主板 BIOS/UEFI 设置，开启 Intel VT-x 或 AMD-V 后重试。" -ForegroundColor Yellow
    Write-Host "   若无法开启虚拟化，可使用 GitHub Actions 工作流 .github/workflows/build.yml 远程构建 Linux .deb。" -ForegroundColor Cyan
    exit 1
}

Write-Host "[1/4] 启用 WSL 和虚拟机平台..." -ForegroundColor Cyan
dism.exe /online /enable-feature /featurename:Microsoft-Windows-Subsystem-Linux /all /norestart | Out-Null
dism.exe /online /enable-feature /featurename:VirtualMachinePlatform /all /norestart | Out-Null

Write-Host "[2/4] 设置 WSL 默认版本为 2..." -ForegroundColor Cyan
wsl --set-default-version 2

Write-Host "[3/4] 安装 Ubuntu 24.04 LTS..." -ForegroundColor Cyan
# 如果已经安装过，会跳过；如果网络慢，请手动从 Microsoft Store 安装
wsl --install -d Ubuntu-24.04

Write-Host "[4/4] 安装完成。" -ForegroundColor Green
Write-Host ""
Write-Host "接下来请：" -ForegroundColor Yellow
Write-Host "1. 重启电脑（如脚本要求）；"
Write-Host "2. 首次启动 Ubuntu 时设置用户名和密码；"
Write-Host "3. 运行 scripts/build-linux.ps1 开始打包 Linux 版本。"
