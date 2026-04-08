import React, { useState, useEffect } from 'react';
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
} from '@mui/material';
import PlayArrowIcon from '@mui/icons-material/PlayArrow';
import HubIcon from '@mui/icons-material/Hub';
import MemoryIcon from '@mui/icons-material/Memory';
import DeviceHubIcon from '@mui/icons-material/DeviceHub';
import AccountTreeIcon from '@mui/icons-material/AccountTree';
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

export const ModelSelector: React.FC = () => {
  const theme = useTheme();
  const { showRightPanel } = useUIStore();
  const { setActiveSession } = useModelStore();

  // ── Distributed mode state ──
  const [selectedDistModelId, setSelectedDistModelId] = useState<string>(DISTRIBUTED_MODELS[0].id);
  const [registeredNodeCount, setRegisteredNodeCount] = useState(0);

  // ── Shared state ──
  const [isNegotiating, setIsNegotiating] = useState(false);
  const [negotiationPhase, setNegotiationPhase] = useState<string>('');
  const [negotiationProgress, setNegotiationProgress] = useState(0);
  const [peers, setPeers] = useState<PeerDisplay[]>([]);

  const selectedDistModel = DISTRIBUTED_MODELS.find(m => m.id === selectedDistModelId)!;

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
  // 分布式推理：通过 AI 代理配置大模型
  // ──────────────────────────────────────────────────────
  const handleRunDistributed = async () => {
    if (isNegotiating) return;

    showRightPanel();
    setIsNegotiating(true);
    setNegotiationProgress(0);
    setPeers([]);

    const model = selectedDistModel;

    setNegotiationPhase('AI 代理配置环境...');
    await emitWorkflow('info',
      `🤖 启动 AI 代理自动配置...\n\n目标模型: ${model.name} (${model.params})\n\n⚠️ 此步骤需要在「设置」中配置外部 API Key\n\nAI 代理将自动配置推理环境并启动服务`
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
          message: '❌ 需要先在「设置 → 外部 API」中添加 API Key，AI 代理才能自动配置推理环境。',
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

          {/* Header with node count */}
          <Box sx={{ display: 'flex', alignItems: 'center', mb: 1.2, gap: 1 }}>
            <Typography variant="caption" sx={{ fontWeight: 600, flex: 1 }}>
              <AccountTreeIcon sx={{ fontSize: 13, mr: 0.5, verticalAlign: 'middle' }} />
              分布式推理
            </Typography>

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
          </Box>

          {/* ── 分布式推理 ── */}
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

          {/* ── 进度显示 ── */}
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
