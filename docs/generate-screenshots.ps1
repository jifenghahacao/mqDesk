# 生成 MQDesk 手册示意图
# 使用 Trae text_to_image API

$baseUrl = "https://trae-api-cn.mchost.guru/api/ide/v1/text_to_image"
$size = "landscape_16_9"

$images = @{
    "main-interface" = "A clean modern desktop application window screenshot, light theme, soft blue-green gradient background, left sidebar with icons for connections overview queues messages settings, main content area showing RabbitMQ connection cards, title bar with MQDesk logo and window controls, minimalist glassmorphism UI, Chinese labels, 16:9 aspect ratio, UI mockup"
    "new-connection" = "A desktop application modal dialog screenshot, light theme, glassmorphism style, form with input fields for connection name, host address, AMQP port, management port, virtual host, username, password, test connection button and save button labeled in Chinese, soft gradient background, clean UI design, 16:9 aspect ratio"
    "overview-dashboard" = "A desktop dashboard screenshot, light theme, glassmorphism cards, green health status banner at top, four statistic cards showing queue count, exchange count, ready messages count, online consumers count, soft gradient background, Chinese labels, clean modern UI, 16:9 aspect ratio"
    "queue-list" = "A desktop application table screenshot, light theme, glassmorphism UI, list of RabbitMQ queues with colored status dots green yellow red, search bar at top, column headers in Chinese, soft gradient background, 16:9 aspect ratio"
    "queue-detail" = "A desktop application detail page screenshot, light theme, glassmorphism cards, queue health status banner, four metric cards, simple line chart, message preview area with grab button, Chinese labels, soft gradient background, 16:9 aspect ratio"
    "publish-message" = "A desktop application message publish page screenshot, light theme, glassmorphism UI, mode toggle direct and exchange, target input field, content type dropdown, large JSON text area, send button, message history list with status pills below, Chinese labels, 16:9 aspect ratio"
    "settings-page" = "A desktop application settings page screenshot, light theme, glassmorphism UI, appearance section with light dark system toggle buttons, terminology glossary table below, Chinese labels, soft gradient background, 16:9 aspect ratio"
    "system-tray" = "Windows 11 system tray area close-up, small MQDesk application icon with context menu showing Chinese options show window and exit, taskbar corner, realistic screenshot style, 16:9 aspect ratio"
}

foreach ($item in $images.GetEnumerator()) {
    $name = $item.Key
    $prompt = $item.Value
    $encoded = [System.Web.HttpUtility]::UrlEncode($prompt)
    $url = "$baseUrl`?prompt=$encoded&image_size=$size"
    $outPath = "d:\project\RabbitConsumerHub-main\docs\screenshots\$name.png"

    Write-Host "正在生成 $name.png ..."
    try {
        Invoke-WebRequest -Uri $url -OutFile $outPath -UseBasicParsing -TimeoutSec 120
        Write-Host "  已保存 $outPath" -ForegroundColor Green
    } catch {
        Write-Host "  生成失败：$($_.Exception.Message)" -ForegroundColor Red
    }
}

Write-Host "全部完成"
