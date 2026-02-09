import React, { useEffect, useState } from 'react';
import {
  Box,
  Card,
  CardContent,
  Grid,
  Typography,
  LinearProgress,
  useTheme,
  alpha,
  IconButton,
} from '@mui/material';
import ExpandMoreIcon from '@mui/icons-material/ExpandMore';
import ExpandLessIcon from '@mui/icons-material/ExpandLess';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { TrainingStatus, DeviceInfo } from '../types';

// 可折叠卡片组件
interface CollapsibleCardProps {
  title: string;
  children: React.ReactNode;
  defaultCollapsed?: boolean;
}

const CollapsibleCard: React.FC<CollapsibleCardProps> = ({ title, children, defaultCollapsed = false }) => {
  const [collapsed, setCollapsed] = useState(defaultCollapsed);

  return (
    <Card
      onClick={() => collapsed && setCollapsed(false)}
      sx={{
        background: alpha('#000000', 0.6),
        backdropFilter: 'blur(10px)',
        border: '1px solid rgba(255, 255, 255, 0.1)',
        cursor: collapsed ? 'pointer' : 'default',
      }}
    >
      <CardContent sx={{ p: 2 }}>
        <Box
          sx={{
            display: 'flex',
            justifyContent: 'space-between',
            alignItems: 'center',
            mb: collapsed ? 0 : 1,
          }}
        >
          <Typography variant="subtitle2" sx={{ fontSize: '0.875rem' }}>
            {title}
          </Typography>
          <IconButton
            size="small"
            onClick={(e) => {
              e.stopPropagation();
              setCollapsed(!collapsed);
            }}
            sx={{
              color: 'text.secondary',
              '&:hover': {
                color: 'text.primary',
              },
            }}
          >
            {collapsed ? <ExpandMoreIcon /> : <ExpandLessIcon />}
          </IconButton>
        </Box>
        {!collapsed && children}
      </CardContent>
    </Card>
  );
};

export const TrainingDashboard: React.FC = () => {
  const theme = useTheme();
  const [trainingStatus, setTrainingStatus] = useState<TrainingStatus | null>(null);
  const [deviceInfo, setDeviceInfo] = useState<DeviceInfo | null>(null);
  const [nodeInfo, setNodeInfo] = useState<any>(null);
  const [connectedPeers, setConnectedPeers] = useState<any[]>([]);

  useEffect(() => {
    const loadTrainingStatus = async () => {
      try {
        const status = await invoke<TrainingStatus>('get_training_stats');
        setTrainingStatus(status);
      } catch (error) {
        console.error('Error loading training status:', error);
      }
    };

    const loadNodeInfo = async () => {
      try {
        const info = await invoke<any>('get_node_info');
        setNodeInfo(info);
      } catch (error) {
        console.error('Error loading node info:', error);
        setNodeInfo(null);
      }
    };

    const loadConnectedPeers = async () => {
      try {
        const peers = await invoke<any>('get_connected_peers');
        setConnectedPeers(peers);
      } catch (error) {
        console.error('Error loading connected peers:', error);
        setConnectedPeers([]);
      }
    };

    const loadDeviceInfo = async () => {
      try {
        const info = await invoke<DeviceInfo>('get_device_info');
        setDeviceInfo(info);
      } catch (error) {
        console.error('Error loading device info:', error);
        setDeviceInfo(null);
      }
    };

    // Load initial data
    loadTrainingStatus();
    loadNodeInfo();
    loadConnectedPeers();
    loadDeviceInfo();

    const statusInterval = setInterval(() => {
      loadTrainingStatus();
    }, 5000);

    const nodeInterval = setInterval(() => {
      loadNodeInfo();
    }, 10000);

    const peersInterval = setInterval(() => {
      loadConnectedPeers();
    }, 60000);

    const deviceInterval = setInterval(() => {
      loadDeviceInfo();
    }, 60000);

    let unlistenFn: any = null;
    
    const setupEventListener = async () => {
      try {
        unlistenFn = await listen('device_info_refresh', () => {
          loadDeviceInfo();
        });
      } catch (error) {
        console.warn('Event listener setup failed, using polling only:', error);
      }
    };

    setupEventListener();

    return () => {
      clearInterval(statusInterval);
      clearInterval(nodeInterval);
      clearInterval(peersInterval);
      clearInterval(deviceInterval);
      if (unlistenFn) {
        unlistenFn();
      }
    };
  }, []);

  return (
    <Grid container spacing={2}>
      {/* 训练状态卡片 */}
      <Grid item xs={12} md={6}>
        <CollapsibleCard title="训练状态">
          {trainingStatus ? (
            <Box>
              <Box sx={{ mb: 2 }}>
                <Box sx={{ display: 'flex', justifyContent: 'space-between', mb: 1 }}>
                  <Typography variant="body2" color="text.secondary">
                    训练进度
                  </Typography>
                  <Typography variant="body2" color="text.secondary">
                    {trainingStatus.current_epoch} / {trainingStatus.total_epochs}
                  </Typography>
                </Box>
                <LinearProgress
                  variant="determinate"
                  value={(trainingStatus.current_epoch / trainingStatus.total_epochs) * 100}
                  sx={{
                    height: 8,
                    borderRadius: 4,
                    background: alpha(theme.palette.primary.main, 0.1),
                  }}
                />
              </Box>

              <Grid container spacing={2}>
                <Grid item xs={6}>
                  <Typography variant="body2" color="text.secondary" gutterBottom>
                    准确率
                  </Typography>
                  <Typography variant="h4" color="primary">
                    {(trainingStatus.accuracy * 100).toFixed(1)}%
                  </Typography>
                </Grid>
                <Grid item xs={6}>
                  <Typography variant="body2" color="text.secondary" gutterBottom>
                    损失
                  </Typography>
                  <Typography variant="h4" color="secondary">
                    {trainingStatus.loss.toFixed(4)}
                  </Typography>
                </Grid>

                <Grid item xs={6}>
                  <Typography variant="body2" color="text.secondary" gutterBottom>
                    处理样本数
                  </Typography>
                  <Typography variant="h5">
                    {trainingStatus.samples_processed.toLocaleString()}
                  </Typography>
                </Grid>
              </Grid>
            </Box>
          ) : (
            <Typography color="text.secondary">加载中...</Typography>
          )}
        </CollapsibleCard>
      </Grid>

      {/* 设备信息卡片 */}
      <Grid item xs={12} md={6}>
        <CollapsibleCard title="设备信息">
          {deviceInfo ? (
            <Grid container spacing={1.5}>
              <Grid item xs={6}>
                <Typography variant="caption" color="text.secondary" display="block">
                  GPU类型
                </Typography>
                <Typography variant="body2" sx={{ fontWeight: 500 }}>
                  {deviceInfo.gpu_type || '未检测到'}
                </Typography>
              </Grid>
              <Grid item xs={6}>
                <Typography variant="caption" color="text.secondary" display="block">
                  GPU使用率
                </Typography>
                <Typography variant="body2" sx={{ fontWeight: 500 }}>
                  {deviceInfo.gpu_usage != null ? `${deviceInfo.gpu_usage.toFixed(1)}%` : 'N/A'}
                </Typography>
              </Grid>
              <Grid item xs={6}>
                <Typography variant="caption" color="text.secondary" display="block">
                  GPU内存
                </Typography>
                <Typography variant="body2" sx={{ fontWeight: 500 }}>
                  {deviceInfo.gpu_memory_used != null && deviceInfo.gpu_memory_total != null ?
                    `${deviceInfo.gpu_memory_used.toFixed(1)}/${deviceInfo.gpu_memory_total.toFixed(1)} GB` : 'N/A'}
                </Typography>
              </Grid>
              <Grid item xs={6}>
                <Typography variant="caption" color="text.secondary" display="block">
                  CPU核心
                </Typography>
                <Typography variant="body2" sx={{ fontWeight: 500 }}>
                  {deviceInfo.cpu_cores}核
                </Typography>
              </Grid>
              <Grid item xs={6}>
                <Typography variant="caption" color="text.secondary" display="block">
                  总内存
                </Typography>
                <Typography variant="body2" sx={{ fontWeight: 500 }}>
                  {deviceInfo.total_memory_gb.toFixed(1)}GB
                </Typography>
              </Grid>
              {deviceInfo.battery_level != null && (
                <Grid item xs={6}>
                  <Typography variant="caption" color="text.secondary" display="block">
                    电池
                  </Typography>
                  <Typography variant="body2" sx={{ fontWeight: 500 }}>
                    {deviceInfo.battery_level.toFixed(0)}%
                  </Typography>
                </Grid>
              )}
            </Grid>
          ) : (
            <Typography color="text.secondary" variant="body2">加载中...</Typography>
          )}
        </CollapsibleCard>
      </Grid>

      {/* 节点信息卡片 */}
      <Grid item xs={12}>
        <CollapsibleCard title="节点信息">
          {nodeInfo ? (
            <Grid container spacing={2}>
              <Grid item xs={12} md={6}>
                <Box
                  sx={{
                    p: 1.5,
                    borderRadius: 1,
                    background: alpha(nodeInfo.is_running ? theme.palette.success.main : theme.palette.grey[700], 0.1),
                    textAlign: 'center',
                  }}
                >
                  <Typography variant="h6" color={nodeInfo.is_running ? 'success.main' : 'text.secondary'} sx={{ fontSize: '1.2rem' }}>
                    {nodeInfo.is_running ? '运行中' : '已停止'}
                  </Typography>
                  <Typography variant="caption" color="text.secondary" sx={{ mt: 0.5, display: 'block' }}>
                    节点状态
                  </Typography>
                  {nodeInfo.id && (
                    <Typography variant="caption" color="text.secondary" sx={{ mt: 0.5, display: 'block', fontFamily: 'monospace' }}>
                      ID: {nodeInfo.id.slice(0, 16)}...
                    </Typography>
                  )}
                </Box>
              </Grid>

              <Grid item xs={12} md={6}>
                <Box
                  sx={{
                    p: 1.5,
                    borderRadius: 1,
                    background: alpha(theme.palette.info.main, 0.1),
                    textAlign: 'center',
                  }}
                >
                  <Typography variant="h6" color="info.main" sx={{ fontSize: '1.2rem' }}>
                    {nodeInfo.tick_counter || 0}
                  </Typography>
                  <Typography variant="caption" color="text.secondary" sx={{ mt: 0.5, display: 'block' }}>
                    训练周期
                  </Typography>
                </Box>
              </Grid>

              {nodeInfo.device_capabilities && (
                <>
                  <Grid item xs={6}>
                    <Typography variant="caption" color="text.secondary" display="block">
                      内存
                    </Typography>
                    <Typography variant="body2" sx={{ fontWeight: 500 }}>
                      {nodeInfo.device_capabilities.max_memory_mb}MB
                    </Typography>
                  </Grid>
                  <Grid item xs={6}>
                    <Typography variant="caption" color="text.secondary" display="block">
                      CPU核心
                    </Typography>
                    <Typography variant="body2" sx={{ fontWeight: 500 }}>
                      {nodeInfo.device_capabilities.cpu_cores}核
                    </Typography>
                  </Grid>
                  <Grid item xs={6}>
                    <Typography variant="caption" color="text.secondary" display="block">
                      GPU
                    </Typography>
                    <Typography variant="body2" sx={{ fontWeight: 500 }}>
                      {nodeInfo.device_capabilities.has_gpu ? '有' : '无'}
                    </Typography>
                  </Grid>
                  <Grid item xs={6}>
                    <Typography variant="caption" color="text.secondary" display="block">
                      网络类型
                    </Typography>
                    <Typography variant="body2" sx={{ fontWeight: 500 }}>
                      {nodeInfo.device_capabilities.network_type || '未知'}
                    </Typography>
                  </Grid>
                </>
              )}
            </Grid>
          ) : (
            <Typography color="text.secondary" variant="body2">节点未启动</Typography>
          )}
        </CollapsibleCard>
      </Grid>

      {/* 连接节点卡片 */}
      <Grid item xs={12}>
        <CollapsibleCard title="连接节点" defaultCollapsed={true}>
          {connectedPeers.length > 0 ? (
            <Grid container spacing={1}>
              {connectedPeers.map((peer, index) => (
                <Grid item xs={12} sm={6} md={4} key={peer.id || index}>
                  <Box
                    sx={{
                      p: 1.5,
                      borderRadius: 1,
                      background: alpha(
                        peer.type === 'primary' ? theme.palette.success.main : theme.palette.warning.main,
                        0.1
                      ),
                      border: `1px solid ${
                        peer.type === 'primary' 
                          ? alpha(theme.palette.success.main, 0.3)
                          : alpha(theme.palette.warning.main, 0.3)
                      }`,
                    }}
                  >
                    <Typography variant="body2" sx={{ fontWeight: 500, fontFamily: 'monospace' }}>
                      {peer.id ? `${peer.id.slice(0, 12)}...` : 'Unknown'}
                    </Typography>
                    <Typography variant="caption" color="text.secondary" display="block">
                      类型: {peer.type === 'primary' ? '主节点' : '备份节点'}
                    </Typography>
                    <Typography variant="caption" color="text.secondary" display="block">
                      相似度: {peer.similarity?.toFixed(3) || 'N/A'}
                    </Typography>
                    <Typography variant="caption" color="text.secondary" display="block">
                      地理亲和: {peer.geo_affinity?.toFixed(3) || 'N/A'}
                    </Typography>
                  </Box>
                </Grid>
              ))}
            </Grid>
          ) : (
            <Typography color="text.secondary" variant="body2">暂无连接节点</Typography>
          )}
        </CollapsibleCard>
      </Grid>
    </Grid>
  );
};
