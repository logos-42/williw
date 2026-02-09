# P2P 模型分发测试脚本 (PowerShell)
# 测试发送端和接收端的完整功能

param(
    [string]$TestOutputDir = "./test_output/p2p_test_$(Get-Date -Format 'yyyyMMdd_HHmmss')",
    [string]$ShardDir = "./test_models/test_models/simple_split",
    [int]$SenderPort = 9235,
    [int]$ReceiverPort = 9236,
    [switch]$SkipBuild
)

Write-Host "🚀 开始 P2P 模型分发测试" -ForegroundColor Green

# 检查必要的目录
if (-not (Test-Path $ShardDir)) {
    Write-Host "❌ 错误: 找不到模型分片目录 $ShardDir" -ForegroundColor Red
    Write-Host "请先运行模型切分脚本" -ForegroundColor Yellow
    exit 1
}

# 创建测试输出目录
if (-not (Test-Path $TestOutputDir)) {
    New-Item -ItemType Directory -Path $TestOutputDir -Force | Out-Null
}
$ReceivedDir = Join-Path $TestOutputDir "received"
if (-not (Test-Path $ReceivedDir)) {
    New-Item -ItemType Directory -Path $ReceivedDir -Force | Out-Null
}

Write-Host "📁 测试输出目录: $TestOutputDir" -ForegroundColor Cyan

# 编译项目
if (-not $SkipBuild) {
    Write-Host "🔨 编译项目..." -ForegroundColor Yellow
    cargo build --release --example p2p_model_distribution_demo
    if ($LASTEXITCODE -ne 0) {
        Write-Host "❌ 编译失败" -ForegroundColor Red
        exit 1
    }
}

# 步骤1: 测试文件完整性
Write-Host ""
Write-Host "🔍 步骤1: 测试文件完整性..." -ForegroundColor Yellow

$TestFile = Join-Path $ShardDir "node_001.json"
if (Test-Path $TestFile) {
    cargo run --release --example p2p_model_distribution_demo -- test-integrity `
        --file-path "$TestFile" `
        --algorithm sha256
} else {
    Write-Host "⚠️  跳过完整性测试（未找到测试文件）" -ForegroundColor Yellow
}

# 步骤2: 启动接收端（后台）
Write-Host ""
Write-Host "📡 步骤2: 启动接收端..." -ForegroundColor Yellow

$ReceiverLog = Join-Path $TestOutputDir "receiver.log"
$ReceiverJob = Start-Job -ScriptBlock {
    param($OutputDir, $Port, $LogFile)
    cargo run --release --example p2p_model_distribution_demo -- receive `
        --node-id "test_receiver" `
        --output-dir "$OutputDir" `
        --port $Port `
        --auto-accept 2>&1 | Out-File -FilePath $LogFile
} -ArgumentList $ReceivedDir, $ReceiverPort, $ReceiverLog

Write-Host "   接收端 Job ID: $($ReceiverJob.Id)" -ForegroundColor Cyan
Write-Host "   日志文件: $ReceiverLog" -ForegroundColor Cyan

# 等待接收端启动
Write-Host "⏳ 等待接收端启动..." -ForegroundColor Yellow
Start-Sleep -Seconds 3

# 检查接收端是否正常启动
$ReceiverState = Get-Job -Id $ReceiverJob.Id | Select-Object -ExpandProperty State
if ($ReceiverState -eq "Failed" -or $ReceiverState -eq "Stopped") {
    Write-Host "❌ 接收端启动失败" -ForegroundColor Red
    Receive-Job -Id $ReceiverJob.Id | Out-String | Write-Host -ForegroundColor Red
    exit 1
}

Write-Host "✅ 接收端已启动" -ForegroundColor Green

# 步骤3: 启动发送端
Write-Host ""
Write-Host "📤 步骤3: 启动发送端..." -ForegroundColor Yellow

$SenderLog = Join-Path $TestOutputDir "sender.log"
$SenderJob = Start-Job -ScriptBlock {
    param($ShardDir, $Port, $LogFile)
    cargo run --release --example p2p_model_distribution_demo -- send `
        --node-id "test_sender" `
        --target-peer "test_receiver" `
        --shard-dir "$ShardDir" `
        --chunk-size 1048576 `
        --port $Port 2>&1 | Out-File -FilePath $LogFile
} -ArgumentList $ShardDir, $SenderPort, $SenderLog

Write-Host "   发送端 Job ID: $($SenderJob.Id)" -ForegroundColor Cyan
Write-Host "   日志文件: $SenderLog" -ForegroundColor Cyan

# 等待发送完成
Write-Host "⏳ 等待发送完成..." -ForegroundColor Yellow
Wait-Job -Id $SenderJob.Id -Timeout 300 | Out-Null
$SenderExitCode = if ($?) { 0 } else { 1 }

Write-Host "发送端退出代码: $SenderExitCode" -ForegroundColor Cyan

# 等待一段时间确保接收完成
Write-Host "⏳ 等待接收完成..." -ForegroundColor Yellow
Start-Sleep -Seconds 5

# 停止接收端
Write-Host "🛑 停止接收端..." -ForegroundColor Yellow
Stop-Job -Id $ReceiverJob.Id -Force | Out-Null
Remove-Job -Id $ReceiverJob.Id -Force | Out-Null

# 步骤4: 验证结果
Write-Host ""
Write-Host "🔍 步骤4: 验证传输结果..." -ForegroundColor Yellow

# 统计源文件
$SourceFiles = Get-ChildItem -Path $ShardDir -File | Where-Object { $_.Extension -in @('.json', '.pth', '.safetensors') } | Measure-Object | Select-Object -ExpandProperty Count
$SourceSize = (Get-ChildItem -Path $ShardDir -Recurse -File | Measure-Object -Property Length -Sum).Sum
$SourceSizeMB = [math]::Round($SourceSize / 1MB, 2)

Write-Host "📊 源文件统计:" -ForegroundColor Cyan
Write-Host "   文件数量: $SourceFiles" -ForegroundColor White
Write-Host "   总大小: $SourceSizeMB MB" -ForegroundColor White

# 统计接收文件
$ReceivedFiles = Get-ChildItem -Path $ReceivedDir -File | Measure-Object | Select-Object -ExpandProperty Count
$ReceivedSize = if ($ReceivedFiles -gt 0) { 
    (Get-ChildItem -Path $ReceivedDir -Recurse -File | Measure-Object -Property Length -Sum).Sum 
} else { 
    0 
}
$ReceivedSizeMB = [math]::Round($ReceivedSize / 1MB, 2)

Write-Host "📊 接收文件统计:" -ForegroundColor Cyan
Write-Host "   文件数量: $ReceivedFiles" -ForegroundColor White
Write-Host "   总大小: $ReceivedSizeMB MB" -ForegroundColor White

# 验证文件完整性
Write-Host ""
Write-Host "🔍 验证接收文件完整性..." -ForegroundColor Yellow
$ValidationFailed = 0

Get-ChildItem -Path $ShardDir -Filter "*.json" | ForEach-Object {
    $SourceFile = $_
    $ReceivedFile = Join-Path $ReceivedDir $SourceFile.Name
    
    if (Test-Path $ReceivedFile) {
        # 比较文件大小
        if ($SourceFile.Length -eq (Get-Item $ReceivedFile).Length) {
            Write-Host "✅ $($SourceFile.Name) (大小匹配)" -ForegroundColor Green
        } else {
            Write-Host "❌ $($SourceFile.Name) (大小不匹配: $($SourceFile.Length) vs $((Get-Item $ReceivedFile).Length))" -ForegroundColor Red
            $ValidationFailed++
        }
    } else {
        Write-Host "❌ $($SourceFile.Name) (未接收到)" -ForegroundColor Red
        $ValidationFailed++
    }
}

# 步骤5: 生成测试报告
Write-Host ""
Write-Host "📋 生成测试报告..." -ForegroundColor Yellow

$ReportFile = Join-Path $TestOutputDir "test_report.json"
$Report = @{
    test_type = "p2p_model_distribution"
    timestamp = (Get-Date -Format "yyyy-MM-ddTHH:mm:sszzz")
    source = @{
        directory = $ShardDir
        file_count = $SourceFiles
        total_size = "$SourceSizeMB MB"
    }
    received = @{
        directory = $ReceivedDir
        file_count = $ReceivedFiles
        total_size = "$ReceivedSizeMB MB"
    }
    sender = @{
        exit_code = $SenderExitCode
        log_file = $SenderLog
    }
    receiver = @{
        log_file = $ReceiverLog
    }
    validation = @{
        passed = ($ValidationFailed -eq 0)
        failed_files = $ValidationFailed
    }
    success = ($SenderExitCode -eq 0 -and $ValidationFailed -eq 0)
}

$Report | ConvertTo-Json -Depth 3 | Out-File -FilePath $ReportFile -Encoding UTF8
Write-Host "📁 测试报告已保存: $ReportFile" -ForegroundColor Cyan

# 显示测试结果摘要
Write-Host ""
Write-Host "📊 测试结果摘要:" -ForegroundColor Cyan
Write-Host "   测试目录: $TestOutputDir" -ForegroundColor White
Write-Host "   源文件数: $SourceFiles" -ForegroundColor White
Write-Host "   接收文件数: $ReceivedFiles" -ForegroundColor White
Write-Host "   发送端状态: $(if ($SenderExitCode -eq 0) { '成功' } else { '失败' })" -ForegroundColor $(if ($SenderExitCode -eq 0) { 'Green' } else { 'Red' })
Write-Host "   验证状态: $(if ($ValidationFailed -eq 0) { '通过' } else { '失败' })" -ForegroundColor $(if ($ValidationFailed -eq 0) { 'Green' } else { 'Red' })

if ($SenderExitCode -eq 0 -and $ValidationFailed -eq 0) {
    Write-Host ""
    Write-Host "🎉 P2P 模型分发测试成功完成！" -ForegroundColor Green
    Write-Host ""
    Write-Host "📁 查看详细日志:" -ForegroundColor Cyan
    Write-Host "   发送端: Get-Content $SenderLog" -ForegroundColor White
    Write-Host "   接收端: Get-Content $ReceiverLog" -ForegroundColor White
    Write-Host ""
    Write-Host "📁 查看接收的文件:" -ForegroundColor Cyan
    Write-Host "   Get-ChildItem $ReceivedDir" -ForegroundColor White
} else {
    Write-Host ""
    Write-Host "❌ P2P 模型分发测试失败" -ForegroundColor Red
    Write-Host ""
    Write-Host "🔍 查看错误日志:" -ForegroundColor Yellow
    Write-Host "   发送端: Get-Content $SenderLog" -ForegroundColor White
    Write-Host "   接收端: Get-Content $ReceiverLog" -ForegroundColor White
    exit 1
}

# 清理后台任务
Get-Job | Where-Object { $_.State -eq "Completed" -or $_.State -eq "Failed" } | Remove-Job | Out-Null
