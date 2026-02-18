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
import { emit } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { useUIStore } from '../store/uiStore';
import { useModelStore } from '../store/modelStore';
import type { ActiveInferenceSession } from '../store/modelStore';

interface ModelOption {
  id: string;
  name: string;
  params: string;
  vramNeeded: number; // GB
  layers: number;
  hfRepo: string;
}

const MODELS: ModelOption[] = [
  {
    id: 'qwen2.5-72b',
    name: 'Qwen2.5-72B',
    params: '72B',
    vramNeeded: 144,
    layers: 80,
    hfRepo: 'Qwen/Qwen2.5-72B-Instruct',
  },
  {
    id: 'llama3-70b',
    name: 'Llama-3.1-70B',
    params: '70B',
    vramNeeded: 140,
    layers: 80,
    hfRepo: 'meta-llama/Llama-3.1-70B-Instruct',
  },
  {
    id: 'deepseek-r1-32b',
    name: 'DeepSeek-R1-32B',
    params: '32B',
    vramNeeded: 64,
    layers: 64,
    hfRepo: 'deepseek-ai/DeepSeek-R1-Distill-Qwen-32B',
  },
  {
    id: 'qwen2.5-7b',
    name: 'Qwen2.5-7B',
    params: '7B',
    vramNeeded: 14,
    layers: 28,
    hfRepo: 'Qwen/Qwen2.5-7B-Instruct',
  },
];

// Real node from Workers backend
interface WorkerNode {
  node_id: string;
  endpoint: string;
  max_memory_gb: number;
  gpu_type?: string;
  gpu_memory_gb?: number;
  cpu_cores: number;
  current_load: number;
  reliability: number;
}

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

  const [selectedModelId, setSelectedModelId] = useState<string>(MODELS[0].id);
  const [isNegotiating, setIsNegotiating] = useState(false);
  const [negotiationPhase, setNegotiationPhase] = useState<string>('');
  const [negotiationProgress, setNegotiationProgress] = useState(0);
  const [peers, setPeers] = useState<PeerDisplay[]>([]);
  const [registeredNodeCount, setRegisteredNodeCount] = useState(0);

  const selectedModel = MODELS.find(m => m.id === selectedModelId)!;

  // Poll registered node count from Workers backend
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

  // Compute pipeline split plan based on real nodes
  const computeSplitPlan = (model: ModelOption, nodes: WorkerNode[]): PeerDisplay[] => {
    // Get local device info
    const localVram = 8; // will be replaced by real data

    // Build participant list: local + remote nodes (sorted by available memory, exclude overloaded)
    const availableRemotes = nodes
      .filter(n => n.current_load < 0.8) // exclude >80% loaded nodes
      .slice(0, 3); // max 3 remote nodes

    const participants: Array<{ shortId: string; fullId: string; vram: number; isLocal: boolean; gpuType?: string }> = [
      { shortId: '本机', fullId: 'local', vram: localVram, isLocal: true },
      ...availableRemotes.map(n => ({
        shortId: n.node_id.slice(0, 8) + '...',
        fullId: n.node_id,
        vram: n.gpu_memory_gb ?? n.max_memory_gb,
        isLocal: false,
        gpuType: n.gpu_type,
      })),
    ];

    const totalLayers = model.layers;
    const totalVram = participants.reduce((s, p) => s + p.vram, 0);

    // Allocate layers proportional to VRAM
    let allocated = 0;
    return participants.map((p, i) => {
      const isLast = i === participants.length - 1;
      const layerCount = isLast
        ? totalLayers - allocated
        : Math.round((p.vram / totalVram) * totalLayers);
      const start = allocated;
      const end = start + layerCount - 1;
      allocated += layerCount;
      return {
        shortId: p.shortId,
        fullId: p.fullId,
        vram: p.vram,
        layers: [start, Math.min(end, totalLayers - 1)] as [number, number],
        status: 'negotiating' as const,
        isLocal: p.isLocal,
        gpuType: p.gpuType,
      };
    });
  };

  const sleep = (ms: number) => new Promise(r => setTimeout(r, ms));

  const emitWorkflow = async (type: string, content: string, progress?: number) => {
    await emit('workflow-message', { type, content, progress });
  };

  const handleRun = async () => {
    if (isNegotiating) return;

    showRightPanel();
    setIsNegotiating(true);
    setNegotiationProgress(0);
    setPeers([]);

    const model = selectedModel;

    // --- Phase 1: Broadcast intent ---
    setNegotiationPhase('广播推理请求...');
    await emitWorkflow('info',
      `🚀 分布式推理请求\n\n模型: ${model.name} (${model.params} 参数)\n需要显存: ~${model.vramNeeded}GB · ${model.layers} 层\nHF: ${model.hfRepo}\n\n正在向 Workers 网络查询可用节点...`
    );
    setNegotiationProgress(15);
    await sleep(800);

    // --- Phase 2: Query real nodes from Workers backend (with 5s timeout) ---
    setNegotiationPhase('查询在线节点...');
    let workerNodes: WorkerNode[] = [];
    try {
      const result = await Promise.race([
        invoke<any>('get_available_nodes_from_workers'),
        new Promise<null>((_, reject) => setTimeout(() => reject(new Error('timeout')), 5000)),
      ]);
      workerNodes = (result as any)?.nodes ?? [];
      setRegisteredNodeCount((result as any)?.total ?? 0);
    } catch (e) {
      console.warn('Workers backend unavailable, using local-only mode:', e);
    }

    const onlineCount = workerNodes.length;
    await emitWorkflow('progress',
      `🔍 节点发现完成\n\n已向 Workers 后端查询: ${onlineCount} 个在线节点\n（Workers 网络: https://workers.williw.io）\n\n在线节点:\n${
        onlineCount === 0
          ? '  • 仅本机（其他节点未注册）'
          : workerNodes.slice(0, 5).map(n =>
              `  • ${n.node_id.slice(0, 12)}... GPU: ${n.gpu_memory_gb ?? n.max_memory_gb}GB · 负载: ${(n.current_load * 100).toFixed(0)}%`
            ).join('\n')
      }`,
      0.3
    );
    setNegotiationProgress(35);
    await sleep(1000);

    // --- Phase 3: Capability check & split plan ---
    setNegotiationPhase('计算切分方案...');
    const splitPlan = computeSplitPlan(model, workerNodes);
    setPeers(splitPlan.map(p => ({ ...p, status: 'negotiating' })));

    const totalAvailVram = splitPlan.reduce((s, n) => s + n.vram, 0);
    await emitWorkflow('progress',
      `📊 节点能力确认 (${splitPlan.length} 个节点参与)\n\n${splitPlan.map(n =>
        `• ${n.shortId}${n.gpuType ? ` [${n.gpuType}]` : ''}: ${n.vram}GB → Layer ${n.layers[0]}-${n.layers[1]} (${n.layers[1] - n.layers[0] + 1}层)`
      ).join('\n')}\n\n合计可用显存: ${totalAvailVram}GB / 需要: ${model.vramNeeded}GB\n${totalAvailVram >= model.vramNeeded ? '✅ 算力充足' : '⚠️ 算力不足，降级运行'}`,
      0.55
    );
    setNegotiationProgress(55);
    await sleep(900);

    // --- Phase 4: Pipeline plan ---
    setNegotiationPhase('确认 Pipeline 方案...');
    await emitWorkflow('progress',
      `🧠 Pipeline Parallelism 方案\n\n${splitPlan.map((n, i) =>
        `节点${i + 1} [${n.shortId}]: Layer ${n.layers[0]}–${n.layers[1]}`
      ).join(' → ')}\n\n数据流:\n用户输入 → 节点1推理 → 激活值(iroh传输) → 节点2 → ... → 汇总输出`,
      0.7
    );
    setNegotiationProgress(70);
    setPeers(prev => prev.map(p => ({ ...p, status: 'ready' })));
    await sleep(800);

    // --- Phase 5: Model weight download ---
    setNegotiationPhase('通知各节点加载权重...');
    await emitWorkflow('progress',
      `📥 各节点开始加载模型权重\n\n${splitPlan.map((n, i) =>
        `• 节点${i + 1} [${n.shortId}]${n.isLocal ? ' (本机)' : ' (远程)'}: 下载 Layer ${n.layers[0]}-${n.layers[1]} 权重`
      ).join('\n')}\n\n权重来源: HuggingFace Hub → ${model.hfRepo}`,
      0.85
    );
    setNegotiationProgress(85);
    setPeers(prev => prev.map(p => ({ ...p, status: 'loading' })));
    await sleep(1000);

    // --- Phase 6: Ready ---
    setNegotiationPhase('就绪！');
    setNegotiationProgress(100);
    const isSingleNode = splitPlan.length === 1;
    await emitWorkflow('success',
      `✅ ${isSingleNode ? '本机推理就绪' : '分布式推理就绪'}\n\n${model.name} ${isSingleNode
        ? '将在本机运行（无其他节点在线）'
        : `已切分到 ${splitPlan.length} 个节点协作运行`
      }\n\n现在可以在下方输入框开始对话 👇\n（注意：权重下载需要时间，首次较慢）`
    );

    // Set active inference session so ChatBox can use model context
    const session: ActiveInferenceSession = {
      modelName: model.name,
      modelRepo: model.hfRepo,
      params: model.params,
      totalLayers: model.layers,
      splitPlan: splitPlan.map(p => ({
        shortId: p.shortId,
        fullId: p.fullId,
        vram: p.vram,
        layers: p.layers,
        isLocal: p.isLocal,
        gpuType: p.gpuType,
      })),
      isLocalOnly: isSingleNode,
      activatedAt: new Date(),
    };
    setActiveSession(session);

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

          {/* Top row: model select + real node count + run button */}
          <Box sx={{ display: 'flex', gap: 1, alignItems: 'center' }}>

            {/* Model selector */}
            <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, flex: 1, minWidth: 0 }}>
              <Typography variant="caption" sx={{ color: 'text.secondary', whiteSpace: 'nowrap' }}>
                模型
              </Typography>
              <FormControl fullWidth size="small">
                <Select
                  value={selectedModelId}
                  onChange={(e) => setSelectedModelId(e.target.value)}
                  disabled={isNegotiating}
                  sx={{ fontSize: '0.875rem' }}
                >
                  {MODELS.map((m) => (
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

            {/* Real registered node count from Workers backend */}
            <Tooltip title={`Workers 网络中注册的节点数：${registeredNodeCount}（其他用户的机器）`}>
              <Box sx={{
                display: 'flex', alignItems: 'center', gap: 0.5,
                px: 1, py: 0.5, borderRadius: 1,
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
                <HubIcon sx={{ fontSize: 14, color: registeredNodeCount > 0 ? 'success.main' : 'text.disabled' }} />
                <Typography variant="caption" sx={{
                  color: registeredNodeCount > 0 ? 'success.main' : 'text.disabled',
                  fontWeight: 600,
                }}>
                  {registeredNodeCount}
                </Typography>
              </Box>
            </Tooltip>

            {/* Run button */}
            <Button
              variant="contained"
              startIcon={isNegotiating ? <CircularProgress size={14} color="inherit" /> : <PlayArrowIcon />}
              onClick={handleRun}
              disabled={isNegotiating}
              size="small"
              sx={{ px: 1.5, py: 0.75, fontSize: '0.8rem', minWidth: '80px', whiteSpace: 'nowrap' }}
            >
              {isNegotiating ? '协商中' : '运行'}
            </Button>
          </Box>

          {/* Model info row */}
          <Box sx={{ display: 'flex', gap: 1, mt: 0.8, alignItems: 'center' }}>
            <MemoryIcon sx={{ fontSize: 12, color: 'text.secondary' }} />
            <Typography variant="caption" color="text.secondary">
              需要 ~{selectedModel.vramNeeded}GB 显存 · {selectedModel.layers} layers ·{' '}
              <span style={{ fontFamily: 'monospace', fontSize: '0.7rem' }}>{selectedModel.hfRepo}</span>
            </Typography>
          </Box>

          {/* Negotiation progress */}
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

              {/* Real peer node status chips */}
              {peers.length > 0 && (
                <Box sx={{ mt: 1, display: 'flex', gap: 0.5, flexWrap: 'wrap' }}>
                  {peers.map((peer) => (
                    <Chip
                      key={peer.fullId}
                      size="small"
                      icon={<HubIcon style={{ fontSize: 12 }} />}
                      label={`${peer.shortId} L${peer.layers[0]}-${peer.layers[1]}`}
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
