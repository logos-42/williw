# 完整的iroh P2P测试套件
# 测试多个不同的iroh实现

param(
    [string]$TestType = "all",  # all, simple, robust, working
    [string]$Message = "Hello from iroh test suite!"
)

Write-Host "🚀 iroh P2P测试套件" -ForegroundColor Green
Write-Host "===================" -ForegroundColor Green
Write-Host "测试类型: $TestType" -ForegroundColor Yellow
Write-Host "测试消息: $Message" -ForegroundColor Yellow
Write-Host ""

# 检查Rust环境
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "❌ 错误: 未找到cargo命令，请先安装Rust" -ForegroundColor Red
    exit 1
}

# 构建所有示例
Write-Host "🔨 构建所有iroh示例..." -ForegroundColor Blue
$examples = @("iroh_simple_local", "iroh_robust_local", "iroh_local_demo")

foreach ($example in $examples) {
    Write-Host "  构建 $example..." -ForegroundColor Cyan
    cargo build --example $example
    if ($LASTEXITCODE -ne 0) {
        Write-Host "❌ 构建 $example 失败" -ForegroundColor Red
        exit 1
    }
}

Write-Host "✅ 所有示例构建成功" -ForegroundColor Green
Write-Host ""

# 测试函数
function Test-IrohExample {
    param(
        [string]$ExampleName,
        [string]$DisplayName,
        [string]$Message,
        [hashtable]$ExtraArgs = @{}
    )
    
    Write-Host "🧪 测试 $DisplayName" -ForegroundColor Blue
    Write-Host "========================" -ForegroundColor Blue
    
    # 构建参数
    $receiveArgs = "receive"
    $sendArgs = "send --message `"$Message`""
    
    foreach ($key in $ExtraArgs.Keys) {
        $receiveArgs += " --$key $($ExtraArgs[$key])"
        $sendArgs += " --$key $($ExtraArgs[$key])"
    }
    
    # 启动接收端
    Write-Host "🎯 启动接收端..." -ForegroundColor Cyan
    $receiverJob = Start-Job -ScriptBlock {
        param($ExampleName, $ReceiveArgs)
        Set-Location $using:PWD
        $cmd = "cargo run --example $ExampleName -- $ReceiveArgs"
        Invoke-Expression $cmd
    } -ArgumentList $ExampleName, $receiveArgs
    
    # 等待接收端启动
    Write-Host "⏳ 等待接收端启动..." -ForegroundColor Yellow
    Start-Sleep -Seconds 5
    
    # 获取节点ID
    $receiverOutput = Receive-Job -Job $receiverJob -Keep
    $nodeIdLine = $receiverOutput | Where-Object { $_ -match "节点ID:" }
    
    if ($nodeIdLine) {
        $nodeId = ($nodeIdLine -split "节点ID: ")[1].Trim()
        Write-Host "🔑 检测到节点ID: $nodeId" -ForegroundColor Green
        
        # 等待接收端完全就绪
        Start-Sleep -Seconds 3
        
        # 发送消息
        Write-Host "📤 发送消息..." -ForegroundColor Cyan
        $sendCommand = "cargo run --example $ExampleName -- send --target $nodeId --message `"$Message`""
        
        # 添加额外参数
        foreach ($key in $ExtraArgs.Keys) {
            if ($key -ne "port" -or $ExampleName -eq "iroh_robust_local") {
                $sendCommand += " --$key $($ExtraArgs[$key])"
            }
        }
        
        Write-Host "执行命令: $sendCommand" -ForegroundColor Gray
        Invoke-Expression $sendCommand
        
        if ($LASTEXITCODE -eq 0) {
            Write-Host "✅ $DisplayName 测试成功！" -ForegroundColor Green
            $success = $true
        } else {
            Write-Host "❌ $DisplayName 测试失败" -ForegroundColor Red
            $success = $false
        }
    } else {
        Write-Host "❌ 无法获取节点ID" -ForegroundColor Red
        Write-Host "接收端输出:" -ForegroundColor Yellow
        $receiverOutput | ForEach-Object { Write-Host "  $_" -ForegroundColor White }
        $success = $false
    }
    
    # 清理
    Stop-Job -Job $receiverJob -ErrorAction SilentlyContinue
    Remove-Job -Job $receiverJob -ErrorAction SilentlyContinue
    
    Write-Host ""
    return $success
}

# 运行测试
$testResults = @{}

if ($TestType -eq "all" -or $TestType -eq "simple") {
    $testResults["simple"] = Test-IrohExample -ExampleName "iroh_simple_local" -DisplayName "简单本地测试" -Message $Message
}

if ($TestType -eq "all" -or $TestType -eq "robust") {
    $testResults["robust"] = Test-IrohExample -ExampleName "iroh_robust_local" -DisplayName "健壮本地测试" -Message $Message -ExtraArgs @{port = "11206"}
}

if ($TestType -eq "all" -or $TestType -eq "demo") {
    $testResults["demo"] = Test-IrohExample -ExampleName "iroh_local_demo" -DisplayName "演示版本测试" -Message $Message -ExtraArgs @{port = "11204"}
}

# 显示测试结果
Write-Host "📊 测试结果总结" -ForegroundColor Green
Write-Host "===============" -ForegroundColor Green

$successCount = 0
$totalCount = $testResults.Count

foreach ($test in $testResults.Keys) {
    $result = $testResults[$test]
    if ($result) {
        Write-Host "✅ $test : 成功" -ForegroundColor Green
        $successCount++
    } else {
        Write-Host "❌ $test : 失败" -ForegroundColor Red
    }
}

Write-Host ""
Write-Host "总计: $successCount/$totalCount 测试通过" -ForegroundColor $(if ($successCount -eq $totalCount) { "Green" } else { "Yellow" })

if ($successCount -eq $totalCount) {
    Write-Host "🎉 所有测试都通过了！iroh P2P通信工作正常。" -ForegroundColor Green
} else {
    Write-Host "⚠️ 部分测试失败，请检查网络配置或iroh版本。" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "💡 手动测试说明:" -ForegroundColor Cyan
Write-Host "1. 打开两个终端窗口" -ForegroundColor White
Write-Host "2. 在第一个终端运行: cargo run --example iroh_simple_local -- receive" -ForegroundColor White
Write-Host "3. 复制显示的节点ID" -ForegroundColor White
Write-Host "4. 在第二个终端运行: cargo run --example iroh_simple_local -- send --target <节点ID>" -ForegroundColor White