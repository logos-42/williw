import React, { useState, useEffect, useCallback } from 'react';
import {
  Box,
  FormControl,
  Select,
  MenuItem,
  Typography,
  Card,
  CardContent,
  Button,
  Chip,
  LinearProgress,
  useTheme,
  alpha,
  CircularProgress,
  Tooltip,
  Tab,
  Tabs,
  IconButton,
} from '@mui/material';
import PlayArrowIcon from '@mui/icons-material/PlayArrow';
import HubIcon from '@mui/icons-material/Hub';
import MemoryIcon from '@mui/icons-material/Memory';
import DeviceHubIcon from '@mui/icons-material/DeviceHub';
import SmartToyIcon from '@mui/icons-material/SmartToy';
import AccountTreeIcon from '@mui/icons-material/AccountTree';
import RefreshIcon from '@mui/icons-material/Refresh';
import { emit } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { useUIStore } from '../store/uiStore';
import { useModelStore } from '../store/modelStore';

// ── 分布式大模型（需多节点算力）──
interface DistributedModelOption {
  id: string;
  name: string;
  params: string;
  vramNeeded: number; // GB total
  layers: number;
  hfRepo: string;
  minNodes: number;
}

const DISTRIBUTED_MODELS: DistributedModelOption[] = [
  {
    id: 'qwen2.5-72b',
    name: 'Qwen2.5-72B',
    params: '72B',
    vramNeeded: 144,
    layers: 80,
    hfRepo: 'Qwen/Qwen2.5-72B-Instruct',
    minNodes: 4,
  },
  {
    id: 'llama3-70b',
    name: 'Llama-3.1-70B',
    params: '70B',
    vramNeeded: 140,
    layers: 80,
    hfRepo: 'meta-llama/Llama-3.1-70B-Instruct',
    minNodes: 4,
  },
  {
    id: 'deepseek-r1-32b',
    name: 'DeepSeek-R1-32B',
    params: '32B',
    vramNeeded: 64,
    layers: 64,
    hfRepo: 'deepseek-ai/DeepSeek-R1-Distill-Qwen-32B',
    minNodes: 2,
  },
  {
    id: 'qwen2.5-7b',
    name: 'Qwen2.5-7B',
    params: '7B',
    vramNeeded: 14,
    layers: 28,
    hfRepo: 'Qwen/Qwen2.5-7B-Instruct',
    minNodes: 1,
  },
];

interface PeerDisplay {
  shortId: string;
  fullId: string;
  vram: number;
  layers: [number, number];
  status: 'negotiating' | 'ready' | 'loading';
  isLocal: boolean;
  gpuType?: string;
}

type ModeTab = 'local' | 'distributed';

export const ModelSelector: React.FC = () => {
  const theme = useTheme();
  const { showRightPanel } = useUIStore();
  const { setActiveSession } = useModelStore();

  // ── Tab state ──
  const [modeTab, setModeTab] = useState<ModeTab>('local');

  // ── Local mode state ──
  const [ollamaModels, setOllamaModels] = useState<string[]>([]);
  const [selectedLocalModel, setSelectedLocalModel] = useState<string>('');
  const [ollamaStatus, setOllamaStatus] = useState<'checking' | 'running' | 'stopped' | 'no-models'>('checking');

  // ── Distributed mode state ──
  const [selectedDistModelId, setSelectedDistModelId] = useState<string>(DISTRIBUTED_MODELS[0].id);
  const [registeredNodeCount, setRegisteredNodeCount] = useState(0);

  // ── Shared state ──
  const [isNegotiating, setIsNegotiating] = useState(false);
  const [negotiationPhase, setNegotiationPhase] = useState<string>('');
  const [negotiationProgress, setNegotiationProgress] = useState(0);
  const [peers, setPeers] = useState<PeerDisplay[]>([]);

  const selectedDistModel = DISTRIBUTED_MODELS.find(m => m.id === selectedDistModelId)!;

  // ── 检测本地 Ollama 状态 ──
  const checkOllama = useCallback(async () => {
    setOllamaStatus('checking');
    try {
      const result = await invoke<{
        found: boolean;
        has_models?: boolean;
        all_models?: string[];
        model_name?: string;
        message?: string;
      }>('quick_start_local_inference');

      if (!result.found) {
        setOllamaStatus('stopped');
        setOllamaModels([]);
        setSelectedLocalModel('');
      } else if (!result.has_models) {
        setOllamaStatus('no-models');
        setOllamaModels([]);
        setSelectedLocalModel('');
      } else {
        setOllamaStatus('running');
        const models = result.all_models ?? [];
        setOllamaModels(models);
        // 如果当前选择的模型不在列表中（或者未选择），自动选推荐模型
        if (!selectedLocalModel || !models.includes(selectedLocalModel)) {
          setSelectedLocalModel(result.model_name ?? models[0] ?? '');
        }
      }
    } catch {
      setOllamaStatus('stopped');
    }
  }, [selectedLocalModel]);

  useEffect(() => {
    checkOllama();
  }, []);

  // ── 轮询 Workers 注册节点数 ──
  useEffect(() => {
    const fetchNodeCount = async () => {
      try {
        const result = await invoke<any>('get_available_nodes_from_workers');
        setRegisteredNodeCount(result?.total ?? 0);
      } catch {
        setRegisteredNodeCount(0);
      }
    };
    fetchNodeCount();
    const interval = setInterval(fetchNodeCount, 10000);
    return () => clearInterval(interval);
  }, []);

  const sleep = (ms: number) => new Promise(r => setTimeout(r, ms));

  const emitWorkflow = async (type: string, content: string, progress?: number) => {
    await emit('workflow-message', { type, content, progress });
  };

  // ──────────────────────────────────────────────────────
  // 本地推理：直接使用已检测到的 Ollama 模型
  // ──────────────────────────────────────────────────────
  const handleRunLocal = async () => {
    if (isNegotiating || !selectedLocalModel) return;

    showRightPanel();
    setIsNegotiating(true);
    setNegotiationProgress(0);
    setPeers([]);

    setNegotiationPhase('连接本地 Ollama...');
    await emitWorkflow('info',
      `🔍 正在连接本地 Ollama 服务...\n\n选择模型: ${selectedLocalModel}`
    );
    setNegotiationProgress(20);

    try {
      const result = await invoke<{
        found: boolean;
        has_models?: boolean;
        inference_endpoint?: string;
        model_name?: string;
        all_models?: string[];
        summary?: string;
      }>('quick_start_local_inference');

      if (result.found && result.has_models) {
        // 使用用户选择的模型（而不是自动推荐的）
        const modelToUse = selectedLocalModel;

        // ── 预热模型：确保模型已加载到内存，避免首次对话 502 ──
        setNegotiationPhase('🔥 预热模型中，请稍候...');
        await emitWorkflow('info',
          `🔥 预热模型中...\n\n正在将 ${modelToUse} 加载到内存，首次加载可能需要 30-60 秒，请耐心等待。`
        );
        setNegotiationProgress(50);

        try {
          const warmupResult = await invoke<{ success: boolean; message?: string; status?: number }>(
            'warmup_local_model',
            { modelName: modelToUse }
          );
          if (warmupResult.success) {
            await emitWorkflow('info',
              `✅ 模型已加载到内存\n\n${warmupResult.message ?? modelToUse + ' 就绪'}`
            );
          } else {
            // 预热失败但不阻断流程（可能模型名变化或Ollama版本差异）
            await emitWorkflow('warning',
              `⚠️ 预热返回状态 ${warmupResult.status ?? '?'}，将继续尝试对话`
            );
          }
        } catch (warmupErr: any) {
          // 预热超时或网络问题，给出提示但继续
          await emitWorkflow('warning',
            `⚠️ 预热超时: ${warmupErr}\n\n继续启动，首次对话可能较慢`
          );
        }

        setNegotiationProgress(100);
        setNegotiationPhase('就绪！');

        const splitPlan: PeerDisplay[] = [{
          shortId: '本机',
          fullId: 'local',
          vram: 8,
          layers: [0, 27],
          status: 'ready',
          isLocal: true,
        }];
        setPeers(splitPlan);

        await emitWorkflow('success',
          `✅ 本地推理就绪\n\n模型: ${modelToUse}\n端点: ${result.inference_endpoint}\n\n已安装模型: ${result.all_models?.join(', ')}\n\n可以在下方开始对话 👇`
        );

        setActiveSession({
          modelName: modelToUse,          // 显示真实的本地模型名
          modelRepo: modelToUse,
          params: 'local',
          totalLayers: 28,
          splitPlan: splitPlan.map(p => ({
            shortId: p.shortId,
            fullId: p.fullId,
            vram: p.vram,
            layers: p.layers,
            isLocal: p.isLocal,
          })),
          isLocalOnly: true,
          activatedAt: new Date(),
          inferenceEndpoint: result.inference_endpoint,
          localModelName: modelToUse,    // 确保和 modelName 一致
        });
      } else {
        // Ollama 状态变了，重新检查
        await checkOllama();
        await emitWorkflow('warning',
          `⚠️ Ollama 状态已改变\n\n${result.found ? '请先安装模型：ollama pull qwen2.5:1.5b' : 'Ollama 已停止，请重新启动'}`
        );
      }
    } catch (e: any) {
      await emitWorkflow('error', `❌ 连接失败: ${e}`);
    }

    await sleep(800);
    setIsNegotiating(false);
    setNegotiationPhase('');
    setNegotiationProgress(0);
  };

  // ──────────────────────────────────────────────────────
  // 分布式推理：通过 AI 代理配置大模型
  // ──────────────────────────────────────────────────────
  const handleRunDistributed = async () => {
    if (isNegotiating) return;

    showRightPanel();
    setIsNegotiating(true);
    setNegotiationProgress(0);
    setPeers([]);

    const model = selectedDistModel;

    // 先检测本地 Ollama 快速路径
    setNegotiationPhase('检测本地推理服务...');
    await emitWorkflow('info',
      `🔍 检测推理服务...\n\n目标模型: ${model.name} (${model.params})\n所需显存: ~${model.vramNeeded}GB`
    );
    setNegotiationProgress(15);

    // 对于分布式大模型，跳过本地快速路径直接使用 AI 代理
    setNegotiationPhase('AI 代理配置环境...');
    await emitWorkflow('info',
      `🤖 启动 AI 代理自动配置...\n\n目标模型: ${model.name} (${model.params})\n\n⚠️ 此步骤需要在「设置」中配置外部 API Key\n\nAI 代理将自动：选择最适合本机的模型 → 安装 Ollama → 拉取模型 → 启动服务`
    );
    setNegotiationProgress(20);

    let progressTick = 20;
    const progressTimer = setInterval(() => {
      progressTick = Math.min(progressTick + 1.5, 85);
      setNegotiationProgress(progressTick);
    }, 2000);

    let agentResult: {
      success: boolean;
      inference_endpoint?: string;
      model_name?: string;
      summary?: string;
      message?: string;
    } = { success: false, message: '未知错误' };

    try {
      agentResult = await invoke<typeof agentResult>('run_ai_agent_setup', {
        userModelHint: `${model.name} — 参数量: ${model.params}, HuggingFace repo: ${model.hfRepo}`,
      });
    } catch (e: any) {
      const errMsg = String(e);
      if (errMsg.includes('需要先配置外部 API')) {
        agentResult = {
          success: false,
          message: '❌ 需要先在「设置 → 外部 API」中添加 API Key，AI 代理才能自动配置推理环境。\n\n或者手动安装：\n  ollama pull qwen2.5:1.5b',
        };
      } else {
        agentResult = { success: false, message: errMsg };
      }
    } finally {
      clearInterval(progressTimer);
    }

    const splitPlan: PeerDisplay[] = [{
      shortId: '本机',
      fullId: 'local',
      vram: 8,
      layers: [0, model.layers - 1],
      status: agentResult.success ? 'ready' : 'negotiating',
      isLocal: true,
    }];
    setPeers(splitPlan);

    if (agentResult.success) {
      setNegotiationProgress(100);
      setNegotiationPhase('就绪！');
      await emitWorkflow('success',
        `✅ 本机推理就绪\n\n${agentResult.summary ?? '模型配置完成'}\n\n推理端点: ${agentResult.inference_endpoint}\n模型: ${agentResult.model_name}\n\n可以在下方输入框开始对话 👇`
      );
      setActiveSession({
        modelName: agentResult.model_name ?? model.name,
        modelRepo: model.hfRepo,
        params: model.params,
        totalLayers: model.layers,
        splitPlan: splitPlan.map(p => ({
          shortId: p.shortId,
          fullId: p.fullId,
          vram: p.vram,
          layers: p.layers,
          isLocal: p.isLocal,
        })),
        isLocalOnly: true,
        activatedAt: new Date(),
        inferenceEndpoint: agentResult.inference_endpoint,
        localModelName: agentResult.model_name,
      });
    } else {
      setNegotiationProgress(0);
      setNegotiationPhase('配置失败');
      await emitWorkflow('error',
        `❌ 配置失败\n\n${agentResult.message ?? '未知错误'}\n\n您仍可使用已配置的外部 API 进行对话。`
      );
    }

    await sleep(1500);
    setIsNegotiating(false);
    setNegotiationPhase('');
    setNegotiationProgress(0);
  };

  // ── 渲染本地模型列表 ──
  const renderLocalModels = () => {
    if (ollamaStatus === 'checking') {
      return (
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, py: 1 }}>
          <CircularProgress size={14} />
          <Typography variant="caption" color="text.secondary">检测 Ollama...</Typography>
        </Box>
      );
    }

    if (ollamaStatus === 'stopped') {
      return (
        <Box sx={{
          py: 1, px: 1.5,
          borderRadius: 1,
          background: alpha(theme.palette.warning.main, 0.08),
          border: `1px solid ${alpha(theme.palette.warning.main, 0.2)}`,
        }}>
          <Typography variant="caption" color="warning.main" sx={{ display: 'block', mb: 0.5 }}>
            ⚠️ Ollama 未运行
          </Typography>
          <Typography variant="caption" color="text.secondary" sx={{ fontFamily: 'monospace', fontSize: '0.7rem', display: 'block' }}>
            OLLAMA_NUM_GPU=0 ollama serve &
          </Typography>
        </Box>
      );
    }

    if (ollamaStatus === 'no-models') {
      return (
        <Box sx={{
          py: 1, px: 1.5,
          borderRadius: 1,
          background: alpha(theme.palette.info.main, 0.08),
          border: `1px solid ${alpha(theme.palette.info.main, 0.2)}`,
        }}>
          <Typography variant="caption" color="info.main" sx={{ display: 'block', mb: 0.5 }}>
            Ollama 运行中，但没有模型
          </Typography>
          <Typography variant="caption" color="text.secondary" sx={{ fontFamily: 'monospace', fontSize: '0.7rem', display: 'block' }}>
            ollama pull qwen2.5:1.5b
          </Typography>
        </Box>
      );
    }

    // running + has models
    return (
      <FormControl fullWidth size="small">
        <Select
          value={selectedLocalModel}
          onChange={(e) => setSelectedLocalModel(e.target.value)}
          disabled={isNegotiating}
          sx={{ fontSize: '0.875rem' }}
        >
          {ollamaModels.map((m) => (
            <MenuItem key={m} value={m}>
              <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, width: '100%' }}>
                <SmartToyIcon sx={{ fontSize: 14, color: 'success.main' }} />
                <Typography variant="body2" sx={{ fontFamily: 'monospace', fontSize: '0.8rem' }}>
                  {m}
                </Typography>
                <Chip
                  label="本地"
                  size="small"
                  color="success"
                  variant="outlined"
                  sx={{ height: 14, fontSize: '0.6rem', ml: 'auto', '& .MuiChip-label': { px: 0.5 } }}
                />
              </Box>
            </MenuItem>
          ))}
        </Select>
      </FormControl>
    );
  };

  return (
    <Box sx={{ width: '100%' }}>
      <Card
        sx={{
          background: alpha(theme.palette.background.paper, 0.9),
          backdropFilter: 'blur(10px)',
          border: `1px solid ${theme.palette.divider}`,
          borderRadius: 1,
        }}
      >
        <CardContent sx={{ p: 1.5, '&:last-child': { pb: 1.5 } }}>

          {/* Mode Tabs */}
          <Box sx={{ display: 'flex', alignItems: 'center', mb: 1.2, gap: 1 }}>
            <Tabs
              value={modeTab}
              onChange={(_, v) => setModeTab(v)}
              sx={{
                minHeight: 28,
                flex: 1,
                '& .MuiTab-root': { minHeight: 28, py: 0.5, fontSize: '0.75rem', minWidth: 0, px: 1.5 },
                '& .MuiTabs-indicator': { height: 2 },
              }}
            >
              <Tab
                value="local"
                label="本地推理"
                icon={<SmartToyIcon sx={{ fontSize: 13 }} />}
                iconPosition="start"
              />
              <Tab
                value="distributed"
                label="分布式"
                icon={<AccountTreeIcon sx={{ fontSize: 13 }} />}
                iconPosition="start"
              />
            </Tabs>

            {/* Node count badge for distributed tab */}
            {modeTab === 'distributed' && (
              <Tooltip title={`Workers 网络中注册的节点数：${registeredNodeCount}`}>
                <Box sx={{
                  display: 'flex', alignItems: 'center', gap: 0.5,
                  px: 1, py: 0.3, borderRadius: 1,
                  background: alpha(
                    registeredNodeCount > 0 ? theme.palette.success.main : theme.palette.text.disabled,
                    0.1
                  ),
                  border: `1px solid ${alpha(
                    registeredNodeCount > 0 ? theme.palette.success.main : theme.palette.divider,
                    0.3
                  )}`,
                  cursor: 'default',
                }}>
                  <HubIcon sx={{ fontSize: 12, color: registeredNodeCount > 0 ? 'success.main' : 'text.disabled' }} />
                  <Typography variant="caption" sx={{
                    color: registeredNodeCount > 0 ? 'success.main' : 'text.disabled',
                    fontWeight: 600, fontSize: '0.7rem',
                  }}>
                    {registeredNodeCount} 节点
                  </Typography>
                </Box>
              </Tooltip>
            )}

            {/* Refresh button for local tab */}
            {modeTab === 'local' && (
              <Tooltip title="重新检测 Ollama">
                <IconButton
                  size="small"
                  onClick={checkOllama}
                  disabled={ollamaStatus === 'checking' || isNegotiating}
                  sx={{ p: 0.5 }}
                >
                  {ollamaStatus === 'checking'
                    ? <CircularProgress size={12} />
                    : <RefreshIcon sx={{ fontSize: 14 }} />
                  }
                </IconButton>
              </Tooltip>
            )}
          </Box>

          {/* ── 本地推理 Tab ── */}
          {modeTab === 'local' && (
            <Box>
              <Box sx={{ display: 'flex', gap: 1, alignItems: 'center' }}>
                <Box sx={{ flex: 1, minWidth: 0 }}>
                  {renderLocalModels()}
                </Box>
                <Button
                  variant="contained"
                  startIcon={isNegotiating ? <CircularProgress size={14} color="inherit" /> : <PlayArrowIcon />}
                  onClick={handleRunLocal}
                  disabled={isNegotiating || ollamaStatus !== 'running' || !selectedLocalModel}
                  size="small"
                  sx={{ px: 1.5, py: 0.75, fontSize: '0.8rem', minWidth: '72px', whiteSpace: 'nowrap' }}
                >
                  {isNegotiating ? '连接中' : '运行'}
                </Button>
              </Box>

              {/* Ollama status indicator */}
              <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.5, mt: 0.8 }}>
                <Box sx={{
                  width: 6, height: 6, borderRadius: '50%',
                  background:
                    ollamaStatus === 'running' ? theme.palette.success.main :
                    ollamaStatus === 'checking' ? theme.palette.warning.main :
                    theme.palette.error.main,
                }} />
                <Typography variant="caption" color="text.secondary" sx={{ fontSize: '0.7rem' }}>
                  {ollamaStatus === 'running'
                    ? `Ollama 运行中 · ${ollamaModels.length} 个模型`
                    : ollamaStatus === 'checking'
                    ? '检测中...'
                    : ollamaStatus === 'no-models'
                    ? 'Ollama 运行中，无模型'
                    : 'Ollama 未运行'}
                </Typography>
                {ollamaStatus === 'running' && selectedLocalModel && (
                  <Chip
                    label={selectedLocalModel}
                    size="small"
                    color="success"
                    variant="outlined"
                    sx={{ height: 14, fontSize: '0.6rem', ml: 'auto', fontFamily: 'monospace', '& .MuiChip-label': { px: 0.5 } }}
                  />
                )}
              </Box>
            </Box>
          )}

          {/* ── 分布式推理 Tab ── */}
          {modeTab === 'distributed' && (
            <Box>
              <Box sx={{ display: 'flex', gap: 1, alignItems: 'center' }}>
                <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, flex: 1, minWidth: 0 }}>
                  <Typography variant="caption" sx={{ color: 'text.secondary', whiteSpace: 'nowrap' }}>
                    模型
                  </Typography>
                  <FormControl fullWidth size="small">
                    <Select
                      value={selectedDistModelId}
                      onChange={(e) => setSelectedDistModelId(e.target.value)}
                      disabled={isNegotiating}
                      sx={{ fontSize: '0.875rem' }}
                    >
                      {DISTRIBUTED_MODELS.map((m) => (
                        <MenuItem key={m.id} value={m.id}>
                          <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, width: '100%' }}>
                            <span>{m.name}</span>
                            <Chip
                              label={m.params}
                              size="small"
                              sx={{ height: 16, fontSize: '0.65rem', ml: 'auto' }}
                              color={m.vramNeeded > 100 ? 'error' : m.vramNeeded > 30 ? 'warning' : 'success'}
                            />
                          </Box>
                        </MenuItem>
                      ))}
                    </Select>
                  </FormControl>
                </Box>

                <Button
                  variant="contained"
                  startIcon={isNegotiating ? <CircularProgress size={14} color="inherit" /> : <PlayArrowIcon />}
                  onClick={handleRunDistributed}
                  disabled={isNegotiating}
                  size="small"
                  color={registeredNodeCount >= selectedDistModel.minNodes ? 'primary' : 'warning'}
                  sx={{ px: 1.5, py: 0.75, fontSize: '0.8rem', minWidth: '80px', whiteSpace: 'nowrap' }}
                >
                  {isNegotiating ? '协商中' : '运行'}
                </Button>
              </Box>

              {/* Model info row */}
              <Box sx={{ display: 'flex', gap: 1, mt: 0.8, alignItems: 'center', flexWrap: 'wrap' }}>
                <MemoryIcon sx={{ fontSize: 12, color: 'text.secondary' }} />
                <Typography variant="caption" color="text.secondary">
                  需要 ~{selectedDistModel.vramNeeded}GB 显存 · {selectedDistModel.layers} layers
                </Typography>
                {registeredNodeCount < selectedDistModel.minNodes && (
                  <Chip
                    label={`需要 ${selectedDistModel.minNodes} 节点，当前 ${registeredNodeCount}`}
                    size="small"
                    color="warning"
                    variant="outlined"
                    sx={{ height: 14, fontSize: '0.6rem', '& .MuiChip-label': { px: 0.5 } }}
                  />
                )}
              </Box>
            </Box>
          )}

          {/* ── 进度显示（两个 Tab 共用）── */}
          {isNegotiating && (
            <Box sx={{ mt: 1 }}>
              <Box sx={{ display: 'flex', justifyContent: 'space-between', mb: 0.5 }}>
                <Typography variant="caption" color="primary.main">
                  <DeviceHubIcon sx={{ fontSize: 12, mr: 0.5, verticalAlign: 'middle' }} />
                  {negotiationPhase}
                </Typography>
                <Typography variant="caption" color="text.secondary">
                  {negotiationProgress}%
                </Typography>
              </Box>
              <LinearProgress
                variant="determinate"
                value={negotiationProgress}
                sx={{ height: 4, borderRadius: 2 }}
              />

              {peers.length > 0 && (
                <Box sx={{ mt: 1, display: 'flex', gap: 0.5, flexWrap: 'wrap' }}>
                  {peers.map((peer) => (
                    <Chip
                      key={peer.fullId}
                      size="small"
                      icon={<HubIcon style={{ fontSize: 12 }} />}
                      label={`${peer.shortId}`}
                      color={
                        peer.status === 'ready' ? 'success' :
                        peer.status === 'loading' ? 'primary' : 'default'
                      }
                      variant={peer.status === 'negotiating' ? 'outlined' : 'filled'}
                      sx={{ height: 20, fontSize: '0.65rem' }}
                    />
                  ))}
                </Box>
              )}
            </Box>
          )}
        </CardContent>
      </Card>
    </Box>
  );
};
