import React, { useEffect, useState, useRef } from 'react';
import {
  Box,
  Card,
  CardContent,
  Grid,
  Typography,
  LinearProgress,
  alpha,
  Tooltip,
  IconButton,
  Chip,
} from '@mui/material';
import ContentCopyIcon from '@mui/icons-material/ContentCopy';
import HubIcon from '@mui/icons-material/Hub';
import StarIcon from '@mui/icons-material/Star';
import ComputerIcon from '@mui/icons-material/Computer';
import PeopleIcon from '@mui/icons-material/People';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export const TrainingDashboard: React.FC = () => {
  const [nodeInfo, setNodeInfo] = useState<any>(null);
  const [connectedPeers, setConnectedPeers] = useState<any[]>([]);
  const [deviceInfo, setDeviceInfo] = useState<any>(null);
  const [contributionPoints, setContributionPoints] = useState(0);
  const [sessionStartTime] = useState(Date.now());
  const [copied, setCopied] = useState(false);
  const contributionRef = useRef(0);

  // 每分钟节点在线+1分，每次tick+0.1分（模拟贡献积分）
  useEffect(() => {
    const interval = setInterval(() => {
      setContributionPoints(prev => {
        const newVal = prev + 1;
        contributionRef.current = newVal;
        return newVal;
      });
    }, 60000); // 每分钟+1分
    return () => clearInterval(interval);
  }, []);

  useEffect(() => {
    const loadNodeInfo = async () => {
      try {
        const info = await invoke<any>('get_node_info');
        setNodeInfo(info);
        // 节点在线时，每次获取到tick_counter变化就加分
        if (info?.is_running && info?.tick_counter > 0) {
          setContributionPoints(prev => Math.max(prev, Math.floor(info.tick_counter * 0.1)));
        }
      } catch {
        setNodeInfo(null);
      }
    };

    const loadPeers = async () => {
      try {
        const peers = await invoke<any[]>('get_connected_peers');
        setConnectedPeers(peers || []);
      } catch {
        setConnectedPeers([]);
      }
    };

    const loadDeviceInfo = async () => {
      try {
        const info = await invoke<any>('get_device_info');
        setDeviceInfo(info);
      } catch {
        setDeviceInfo(null);
      }
    };

    loadNodeInfo();
    loadPeers();
    loadDeviceInfo();

    const nodeInterval = setInterval(loadNodeInfo, 10000);
    const peersInterval = setInterval(loadPeers, 30000);
    const deviceInterval = setInterval(loadDeviceInfo, 60000);

    let unlistenFn: any = null;
    listen('device_info_refresh', () => loadDeviceInfo()).then(fn => { unlistenFn = fn; });

    return () => {
      clearInterval(nodeInterval);
      clearInterval(peersInterval);
      clearInterval(deviceInterval);
      if (unlistenFn) unlistenFn();
    };
  }, []);

  const handleCopyNodeId = () => {
    if (nodeInfo?.id) {
      navigator.clipboard.writeText(nodeInfo.id);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  const onlineMinutes = Math.floor((Date.now() - sessionStartTime) / 60000);

  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
      {/* 节点状态卡片 — 最重要，放最上面 */}
      <Card sx={{
        background: alpha(nodeInfo?.is_running ? '#4caf50' : '#666', 0.08),
        border: `1px solid ${alpha(nodeInfo?.is_running ? '#4caf50' : '#666', 0.25)}`,
        borderRadius: 1,
      }}>
        <CardContent sx={{ p: 2, '&:last-child': { pb: 2 } }}>
          <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', mb: 1.5 }}>
            <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
              <HubIcon sx={{ color: nodeInfo?.is_running ? 'success.main' : 'text.disabled', fontSize: 20 }} />
              <Typography variant="subtitle2">P2P 节点</Typography>
            </Box>
            <Chip
              label={nodeInfo?.is_running ? '运行中' : '启动中...'}
              size="small"
              sx={{
                background: alpha(nodeInfo?.is_running ? '#4caf50' : '#ff9800', 0.15),
                color: nodeInfo?.is_running ? 'success.main' : 'warning.main',
                border: `1px solid ${alpha(nodeInfo?.is_running ? '#4caf50' : '#ff9800', 0.3)}`,
                fontSize: '0.7rem',
                height: 22,
              }}
            />
          </Box>

          {nodeInfo?.id ? (
            <Box sx={{
              display: 'flex', alignItems: 'center', gap: 0.5,
              p: 1, borderRadius: 1,
              background: alpha('#ffffff', 0.04),
              border: `1px solid ${alpha('#ffffff', 0.06)}`,
            }}>
              <Typography variant="caption" sx={{
                fontFamily: 'monospace', fontSize: '0.7rem',
                color: 'text.secondary', flex: 1, overflow: 'hidden',
                textOverflow: 'ellipsis', whiteSpace: 'nowrap',
              }}>
                {nodeInfo.id.slice(0, 20)}...
              </Typography>
              <Tooltip title={copied ? '已复制！' : '复制节点 ID'} placement="top">
                <IconButton size="small" onClick={handleCopyNodeId} sx={{ p: 0.3 }}>
                  <ContentCopyIcon sx={{ fontSize: 13, color: copied ? 'success.main' : 'text.secondary' }} />
                </IconButton>
              </Tooltip>
            </Box>
          ) : (
            <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
              <LinearProgress sx={{ flex: 1, height: 3, borderRadius: 2 }} />
              <Typography variant="caption" color="text.secondary" sx={{ fontSize: '0.7rem' }}>
                正在连接...
              </Typography>
            </Box>
          )}

          {/* 网络信息行 */}
          <Box sx={{ display: 'flex', gap: 2, mt: 1.5 }}>
            <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.5 }}>
              <PeopleIcon sx={{ fontSize: 14, color: 'text.secondary' }} />
              <Typography variant="caption" color="text.secondary">
                {connectedPeers.length} 个节点
              </Typography>
            </Box>
            <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.5 }}>
              <Typography variant="caption" color="text.secondary">
                在线 {onlineMinutes > 0 ? `${onlineMinutes} 分钟` : '不到 1 分钟'}
              </Typography>
            </Box>
          </Box>
        </CardContent>
      </Card>

      {/* 贡献积分卡片 */}
      <Card sx={{
        background: alpha('#ffd700', 0.06),
        border: `1px solid ${alpha('#ffd700', 0.2)}`,
        borderRadius: 1,
      }}>
        <CardContent sx={{ p: 2, '&:last-child': { pb: 2 } }}>
          <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 1 }}>
            <StarIcon sx={{ color: '#ffd700', fontSize: 20 }} />
            <Typography variant="subtitle2">贡献积分</Typography>
          </Box>
          <Box sx={{ display: 'flex', alignItems: 'baseline', gap: 1 }}>
            <Typography variant="h3" sx={{ color: '#ffd700', fontWeight: 700, lineHeight: 1 }}>
              {contributionPoints}
            </Typography>
            <Typography variant="caption" color="text.secondary">分</Typography>
          </Box>
          <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mt: 0.5 }}>
            节点在线时间 · 网络贡献 · 任务处理
          </Typography>
          <LinearProgress
            variant="determinate"
            value={Math.min((contributionPoints % 100), 100)}
            sx={{
              mt: 1, height: 4, borderRadius: 2,
              background: alpha('#ffd700', 0.1),
              '& .MuiLinearProgress-bar': { background: '#ffd700' },
            }}
          />
          <Typography variant="caption" color="text.secondary" sx={{ fontSize: '0.65rem' }}>
            下一级别还需 {100 - (contributionPoints % 100)} 分
          </Typography>
        </CardContent>
      </Card>

      {/* 设备资源卡片 */}
      <Card sx={{
        background: alpha('#ffffff', 0.03),
        border: `1px solid ${alpha('#ffffff', 0.08)}`,
        borderRadius: 1,
      }}>
        <CardContent sx={{ p: 2, '&:last-child': { pb: 2 } }}>
          <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 1.5 }}>
            <ComputerIcon sx={{ color: 'text.secondary', fontSize: 18 }} />
            <Typography variant="subtitle2">设备资源</Typography>
          </Box>
          {deviceInfo ? (
            <Grid container spacing={1.5}>
              <Grid size={{ xs: 6 }}>
                <Typography variant="caption" color="text.secondary" display="block">CPU</Typography>
                <Typography variant="body2" fontWeight={500}>
                  {deviceInfo.cpu_cores} 核
                  {deviceInfo.cpu_usage != null && (
                    <Typography component="span" variant="caption" color="text.secondary" sx={{ ml: 0.5 }}>
                      {deviceInfo.cpu_usage.toFixed(0)}%
                    </Typography>
                  )}
                </Typography>
              </Grid>
              <Grid size={{ xs: 6 }}>
                <Typography variant="caption" color="text.secondary" display="block">内存</Typography>
                <Typography variant="body2" fontWeight={500}>
                  {deviceInfo.total_memory_gb?.toFixed(1)} GB
                </Typography>
              </Grid>
              <Grid size={{ xs: 12 }}>
                <Typography variant="caption" color="text.secondary" display="block">GPU</Typography>
                <Typography variant="body2" fontWeight={500}>
                  {deviceInfo.gpu_type || '无独显'}
                  {deviceInfo.gpu_usage != null && (
                    <Typography component="span" variant="caption" color="text.secondary" sx={{ ml: 0.5 }}>
                      {deviceInfo.gpu_usage.toFixed(0)}%
                    </Typography>
                  )}
                </Typography>
              </Grid>
            </Grid>
          ) : (
            <Typography variant="caption" color="text.secondary">加载中...</Typography>
          )}
        </CardContent>
      </Card>

      {/* 连接的 Peer 节点（折叠展示） */}
      {connectedPeers.length > 0 && (
        <Card sx={{
          background: alpha('#ffffff', 0.02),
          border: `1px solid ${alpha('#ffffff', 0.06)}`,
          borderRadius: 1,
        }}>
          <CardContent sx={{ p: 2, '&:last-child': { pb: 2 } }}>
            <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 1 }}>
              已连接的节点（{connectedPeers.length}）
            </Typography>
            <Box sx={{ display: 'flex', flexDirection: 'column', gap: 0.5 }}>
              {connectedPeers.slice(0, 3).map((peer, i) => (
                <Box key={i} sx={{
                  display: 'flex', alignItems: 'center', gap: 1,
                  p: 0.75, borderRadius: 0.5,
                  background: alpha('#ffffff', 0.03),
                }}>
                  <Box sx={{
                    width: 6, height: 6, borderRadius: '50%',
                    background: peer.type === 'primary' ? '#4caf50' : '#ff9800',
                  }} />
                  <Typography variant="caption" sx={{ fontFamily: 'monospace', color: 'text.secondary', flex: 1 }}>
                    {peer.id?.slice(0, 16)}...
                  </Typography>
                  <Typography variant="caption" sx={{ color: 'text.disabled', fontSize: '0.65rem' }}>
                    {peer.type === 'primary' ? '主' : '备'}
                  </Typography>
                </Box>
              ))}
              {connectedPeers.length > 3 && (
                <Typography variant="caption" color="text.secondary" sx={{ fontSize: '0.65rem', textAlign: 'center' }}>
                  +{connectedPeers.length - 3} 个节点
                </Typography>
              )}
            </Box>
          </CardContent>
        </Card>
      )}
    </Box>
  );
};
