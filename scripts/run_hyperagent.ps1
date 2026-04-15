# Hyperagent 配置和运行脚本
# 用法: .\scripts\run_hyperagent.ps1 [-Mode "evolution"|"research"|"self_evolve"] [-Iterations 10]

param(
    [Parameter(Mandatory=$false)]
    [ValidateSet("evolution", "research", "self_evolve")]
    [string]$Mode = "evolution",
    
    [Parameter(Mandatory=$false)]
    [int]$Iterations = 5,
    
    [Parameter(Mandatory=$false)]
    [switch]$DryRun,
    
    [Parameter(Mandatory=$false)]
    [switch]$Strict
)

Write-Host "╔══════════════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║           Hyperagent 配置和运行工具                      ║" -ForegroundColor Cyan
Write-Host "╚══════════════════════════════════════════════════════════╝" -ForegroundColor Cyan
Write-Host ""

# 检查 .env 文件
$envFile = Join-Path $PSScriptRoot "..\..\.env"
if (-not (Test-Path $envFile)) {
    Write-Host "⚠️  未找到 .env 文件" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "请创建 .env 文件并配置以下环境变量:" -ForegroundColor Yellow
    Write-Host "  LLM_PROVIDER=openai" -ForegroundColor White
    Write-Host "  LLM_MODEL=gpt-4o" -ForegroundColor White
    Write-Host "  LLM_API_KEY=sk-..." -ForegroundColor White
    Write-Host ""
    
    $createEnv = Read-Host "是否现在创建示例 .env 文件? (y/n)"
    if ($createEnv -eq "y" -or $createEnv -eq "Y") {
        $envContent = @"
# LLM 配置 (支持任何 OpenAI 兼容 API)
LLM_PROVIDER=openai
LLM_MODEL=gpt-4o
LLM_API_KEY=sk-your-api-key-here

# 可选: 自定义端点
# LLM_BASE_URL=https://api.openai.com/v1

# 其他提供商示例 (取消注释以使用)
# LLM_PROVIDER=ollama
# LLM_MODEL=llama2
# LLM_BASE_URL=http://localhost:11434
"@
        $envContent | Out-File -FilePath $envFile -Encoding UTF8
        Write-Host "✅ 已创建示例 .env 文件" -ForegroundColor Green
        Write-Host "⚠️  请编辑 $envFile 并填入真实的 API 密钥" -ForegroundColor Yellow
        return
    } else {
        Write-Host "❌ 请先配置 .env 文件后再运行" -ForegroundColor Red
        return
    }
}

# 读取 .env 文件并检查 API 密钥
$envContent = Get-Content $envFile -Raw
if ($envContent -match "LLM_API_KEY=sk-your-api-key-here") {
    Write-Host "⚠️  .env 文件包含示例 API 密钥" -ForegroundColor Yellow
    Write-Host "请编辑 .env 文件并填入真实的 API 密钥" -ForegroundColor Yellow
    $continue = Read-Host "是否继续? (y/n)"
    if ($continue -ne "y" -and $continue -ne "Y") {
        return
    }
}

Write-Host "✅ 找到 .env 文件" -ForegroundColor Green
Write-Host ""

# 切换到 hyperagent 目录
$hyperagentDir = Join-Path $PSScriptRoot "..\hyperagent"
if (-not (Test-Path $hyperagentDir)) {
    Write-Host "❌ 未找到 hyperagent 目录: $hyperagentDir" -ForegroundColor Red
    return
}

Set-Location $hyperagentDir

Write-Host "📂 工作目录: $(Get-Location)" -ForegroundColor Cyan
Write-Host ""

# 设置环境变量
$env:ITERATIONS = $Iterations.ToString()

if ($DryRun) {
    $env:RESEARCH_DRY_RUN = "true"
}

if ($Strict) {
    $env:RESEARCH_STRICT = "true"
}

# 根据模式运行
Write-Host "🚀 启动 Hyperagent ($Mode 模式)" -ForegroundColor Green
Write-Host "   迭代次数: $Iterations" -ForegroundColor White
if ($DryRun) { Write-Host "   安全模式: 启用" -ForegroundColor Yellow }
if ($Strict) { Write-Host "   严格模式: 启用" -ForegroundColor Yellow }
Write-Host ""

switch ($Mode) {
    "evolution" {
        Write-Host "═══════════════════════════════════════════════════════" -ForegroundColor Cyan
        Write-Host "  进化引擎模式" -ForegroundColor Cyan
        Write-Host "  多分支进化 + 热力学优化 + 多样性选择" -ForegroundColor Cyan
        Write-Host "═══════════════════════════════════════════════════════" -ForegroundColor Cyan
        Write-Host ""
        cargo run
    }
    "research" {
        Write-Host "═══════════════════════════════════════════════════════" -ForegroundColor Cyan
        Write-Host "  自动研究模式 (Karpathy 风格)" -ForegroundColor Cyan
        Write-Host "  假设 → 实验 → 反思 → git commit → 循环" -ForegroundColor Cyan
        Write-Host "═══════════════════════════════════════════════════════" -ForegroundColor Cyan
        Write-Host ""
        cargo run --bin research
    }
    "self_evolve" {
        Write-Host "═══════════════════════════════════════════════════════" -ForegroundColor Cyan
        Write-Host "  结构化自改进模式" -ForegroundColor Cyan
        Write-Host "  递归自我优化 + 测试验证" -ForegroundColor Cyan
        Write-Host "═══════════════════════════════════════════════════════" -ForegroundColor Cyan
        Write-Host ""
        cargo run --bin self_evolve
    }
}

Write-Host ""
Write-Host "═══════════════════════════════════════════════════════" -ForegroundColor Green
Write-Host "  运行完成!" -ForegroundColor Green
Write-Host "═══════════════════════════════════════════════════════" -ForegroundColor Green
Write-Host ""

# 显示结果文件
$dataDir = Join-Path $hyperagentDir ".hyperagent\data"
$experimentsDir = Join-Path $hyperagentDir ".hyperagent\experiments"

if (Test-Path $dataDir) {
    Write-Host "📊 结果文件:" -ForegroundColor Cyan
    
    $archiveFile = Join-Path $dataDir "archive.json"
    if (Test-Path $archiveFile) {
        Write-Host "  📁 进化存档: $archiveFile" -ForegroundColor White
    }
    
    $lineageFile = Join-Path $dataDir "lineage.json"
    if (Test-Path $lineageFile) {
        Write-Host "  🌳 血统树: $lineageFile" -ForegroundColor White
    }
}

if (Test-Path $experimentsDir) {
    $researchLog = Join-Path $experimentsDir "research_log.md"
    if (Test-Path $researchLog) {
        Write-Host "  📝 研究日志: $researchLog" -ForegroundColor White
    }
}

Write-Host ""
Write-Host "💡 提示:" -ForegroundColor Yellow
Write-Host "  - 查看 Git 提交历史: git log --oneline" -ForegroundColor White
Write-Host "  - 查看详细日志: cargo run -- --verbose" -ForegroundColor White
Write-Host "  - 使用不同模式: .\scripts\run_hyperagent.ps1 -Mode research" -ForegroundColor White
