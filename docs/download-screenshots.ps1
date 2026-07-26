# 下载已生成的 MQDesk 手册示意图
$base = "d:/project/RabbitConsumerHub-main/docs/screenshots"

function Download-Image($name, $url) {
    $out = Join-Path $base "$name.png"
    Write-Host "正在下载 $name.png ..."
    try {
        Invoke-WebRequest -Uri $url -OutFile $out -UseBasicParsing -TimeoutSec 60
        Write-Host "  已保存 $out" -ForegroundColor Green
    } catch {
        Write-Host "  下载失败：$($_.Exception.Message)" -ForegroundColor Red
    }
}

Download-Image "main-interface" "https://aka.doubaocdn.com/s/JWju1wqD4o"
Download-Image "new-connection" "https://aka.doubaocdn.com/s/JiuI1wqD54"
Download-Image "overview-dashboard" "https://aka.doubaocdn.com/s/1LWB1wqD57"
Download-Image "queue-list" "https://aka.doubaocdn.com/s/iQWZ1wqD5A"
Download-Image "queue-detail" "https://aka.doubaocdn.com/s/O3Is1wqD5C"
Download-Image "publish-message" "https://aka.doubaocdn.com/s/3dEt1wqD5N"
Download-Image "settings-page" "https://aka.doubaocdn.com/s/7WuD1wqD5P"
Download-Image "system-tray" "https://aka.doubaocdn.com/s/CscW1wqD5R"

Write-Host "全部完成"
