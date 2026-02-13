import React, { useState, useEffect } from 'react';
import {
  Box,
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  Button,
  TextField,
  Typography,
  Grid,
  Tabs,
  Tab,
  Chip,
  CircularProgress,
  useTheme,
  alpha,
} from '@mui/material';
import { invoke } from '@tauri-apps/api/core';
import { ExternalApiForm, ApiList, type ExternalApiConfig } from './settings/mod';

interface SettingsPanelProps {
  onClose: () => void;
}

interface ApiKey {
  id: string;
  name: string;
  key: string;
  provider: string;
  created_at: string;
}

interface DeployStatus {
  pythonInstalled: boolean;
  dependenciesInstalled: boolean;
  modelDownloaded: boolean;
  serverRunning: boolean;
}

export const SettingsPanel: React.FC<SettingsPanelProps> = ({ onClose }) => {
  const theme = useTheme();
  const [tabValue, setTabValue] = useState(0);
  const [apiKeys, setApiKeys] = useState<ApiKey[]>([]);
  const [selectedKey, setSelectedKey] = useState<ApiKey | null>(null);
  const [showApiDialog, setShowApiDialog] = useState(false);
  const [newApiName, setNewApiName] = useState('');
  const [showCreateDialog, setShowCreateDialog] = useState(false);

  // 外部 API 配置状态
  const [externalApis, setExternalApis] = useState<ExternalApiConfig[]>([]);
  const [editingApiId, setEditingApiId] = useState<string | null>(null);
  const [, setShowExternalApiForm] = useState(false);

  // AI 自动部署状态
  const [deployStatus, setDeployStatus] = useState<DeployStatus>({
    pythonInstalled: false,
    dependenciesInstalled: false,
    modelDownloaded: false,
    serverRunning: false,
  });
  const [isDeploying, setIsDeploying] = useState(false);
  const [deployProgress, setDeployProgress] = useState('');

  useEffect(() => {
    loadApiKeys();
    loadExternalApis();
    checkDeployStatus();
  }, []);

  const loadApiKeys = async () => {
    try {
      const keys = await invoke<ApiKey[]>('get_api_keys');
      setApiKeys(keys);
    } catch (error) {
      console.error('Error loading API keys:', error);
    }
  };

  const handleCreateApi = async () => {
    if (!newApiName.trim()) {
      alert('请输入 API 名称');
      return;
    }

    try {
      const newKey = await invoke<ApiKey>('create_api_key', { name: newApiName });
      setApiKeys([...apiKeys, newKey]);
      setNewApiName('');
      setShowCreateDialog(false);
      setSelectedKey(newKey);
      setShowApiDialog(true);
    } catch (error) {
      console.error('Error creating API key:', error);
    }
  };

  const handleDeleteApi = async (id: string) => {
    try {
      await invoke('delete_api_key', { id });
      setApiKeys(apiKeys.filter((k) => k.id !== id));
      if (selectedKey?.id === id) {
        setSelectedKey(null);
      }
    } catch (error) {
      console.error('Error deleting API key:', error);
    }
  };

  const handleUpdateApiName = async (id: string, newName: string) => {
    try {
      await invoke('update_api_key_name', { id, newName });
      setApiKeys(apiKeys.map((k) => (k.id === id ? { ...k, name: newName } : k)));
    } catch (error) {
      console.error('Error updating API key name:', error);
    }
  };

  const [editingKeyId, setEditingKeyId] = useState<string | null>(null);
  const [editingName, setEditingName] = useState('');

  const startEditing = (key: ApiKey) => {
    setEditingKeyId(key.id);
    setEditingName(key.name);
  };

  const saveEditing = async (id: string) => {
    if (editingName.trim()) {
      await handleUpdateApiName(id, editingName);
    }
    setEditingKeyId(null);
    setEditingName('');
  };

  const cancelEditing = () => {
    setEditingKeyId(null);
    setEditingName('');
  };

  // ============ 外部 API 配置 ============

  const loadExternalApis = async () => {
    try {
      const apis = await invoke<ExternalApiConfig[]>('get_external_apis');
      setExternalApis(apis);
    } catch (error) {
      console.error('Error loading external APIs:', error);
    }
  };

  const handleSaveExternalApi = async (config: Partial<ExternalApiConfig>) => {
    try {
      if (editingApiId) {
        // 编辑模式：先删除旧的，再保存新的
        await invoke('delete_external_api', { id: editingApiId });
      }
      await invoke('save_external_api', { config });
      await loadExternalApis();
      setShowExternalApiForm(false);
      setEditingApiId(null);
    } catch (error) {
      console.error('Error saving external API:', error);
    }
  };

  const handleDeleteExternalApi = async (id: string) => {
    try {
      await invoke('delete_external_api', { id });
      setExternalApis(externalApis.filter((api) => api.id !== id));
    } catch (error) {
      console.error('Error deleting external API:', error);
    }
  };

  const handleToggleExternalApi = async (id: string, enabled: boolean) => {
    try {
      await invoke('toggle_external_api', { id, enabled });
      setExternalApis(externalApis.map((api) => (api.id === id ? { ...api, enabled } : api)));
    } catch (error) {
      console.error('Error toggling external API:', error);
    }
  };

  const checkDeployStatus = async () => {
    try {
      const status = await invoke<DeployStatus>('check_deploy_status');
      setDeployStatus(status);
    } catch (error) {
      console.error('Error checking deploy status:', error);
    }
  };

  const handleAutoDeploy = async () => {
    setIsDeploying(true);
    setDeployProgress('开始自动部署...');

    try {
      // 步骤 1: 检查 Python 环境
      setDeployProgress('正在检查 Python 环境...');
      const pythonOk = await invoke<boolean>('check_python');
      if (!pythonOk) {
        setDeployProgress('未找到 Python，请先安装 Python 3.8+');
        setIsDeploying(false);
        return;
      }
      setDeployStatus((prev) => ({ ...prev, pythonInstalled: true }));

      // 步骤 2: 安装依赖
      setDeployProgress('正在安装 Python 依赖...');
      await invoke('install_dependencies');
      setDeployStatus((prev) => ({ ...prev, dependenciesInstalled: true }));

      // 步骤 3: 下载模型
      setDeployProgress('正在下载默认模型...');
      await invoke('download_default_model');
      setDeployStatus((prev) => ({ ...prev, modelDownloaded: true }));

      // 步骤 4: 启动服务器
      setDeployProgress('正在启动 GPU 服务器...');
      await invoke('start_gpu_server');
      setDeployStatus((prev) => ({ ...prev, serverRunning: true }));

      setDeployProgress('部署完成！GPU 服务器已启动');
    } catch (error) {
      console.error('Error during auto deploy:', error);
      setDeployProgress(`部署失败: ${error}`);
    } finally {
      setIsDeploying(false);
    }
  };

  return (
    <Dialog
      open={true}
      onClose={onClose}
      maxWidth="md"
      fullWidth
      PaperProps={{
        sx: {
          background: alpha(theme.palette.background.paper, 0.95),
          backdropFilter: 'blur(20px)',
          border: `1px solid ${theme.palette.divider}`,
          height: '600px',
          maxHeight: '600px',
        },
      }}
    >
      <DialogTitle>
        <Typography component="span" sx={{ fontSize: '1.25rem', fontWeight: 500 }}>
          设置
        </Typography>
      </DialogTitle>

      <DialogContent dividers sx={{ p: 0, display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
        <Tabs
          value={tabValue}
          onChange={(_, newValue) => setTabValue(newValue)}
          sx={{ borderBottom: 1, borderColor: 'divider', px: 2, flexShrink: 0 }}
        >
          <Tab label="API 密钥" />
          <Tab label="外部 API" />
          <Tab label="AI 部署" />
        </Tabs>

        {/* Tab 内容容器 - 固定高度，内部滚动 */}
        <Box sx={{ 
          flex: 1,
          overflow: 'auto',
          overflowX: 'hidden',
        }}>
          {/* Tab 0: API 密钥管理 */}
          {tabValue === 0 && (
            <Box sx={{ p: 3 }}>
            <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', mb: 2 }}>
              <Typography variant="h6">API 密钥管理</Typography>
              <Button variant="contained" color="primary" onClick={() => setShowCreateDialog(true)}>
                创建新密钥
              </Button>
            </Box>
            <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
              管理您的 API 密钥，用于访问去中心化训练服务
            </Typography>

            {apiKeys.length > 0 ? (
              <Box sx={{ display: 'flex', flexDirection: 'column', gap: 1 }}>
                {apiKeys.map((apiKey) => (
                  <Box
                    key={apiKey.id}
                    sx={{
                      p: 2,
                      backgroundColor: 'rgba(255, 255, 255, 0.05)',
                      borderRadius: 1,
                      display: 'flex',
                      justifyContent: 'space-between',
                      alignItems: 'center',
                    }}
                  >
                    <Box>
                      {editingKeyId === apiKey.id ? (
                        <Box sx={{ display: 'flex', gap: 1, alignItems: 'center' }}>
                          <TextField
                            size="small"
                            value={editingName}
                            onChange={(e) => setEditingName(e.target.value)}
                            placeholder="输入新名称"
                          />
                          <Button size="small" onClick={() => saveEditing(apiKey.id)}>
                            保存
                          </Button>
                          <Button size="small" onClick={cancelEditing}>
                            取消
                          </Button>
                        </Box>
                      ) : (
                        <>
                          <Typography variant="body2" sx={{ fontWeight: 500 }}>
                            {apiKey.name}
                          </Typography>
                          <Typography variant="caption" color="text.secondary">
                            提供商: {apiKey.provider} | 创建时间: {apiKey.created_at}
                          </Typography>
                        </>
                      )}
                    </Box>
                    <Box sx={{ display: 'flex', gap: 1 }}>
                      <Button
                        size="small"
                        variant="text"
                        onClick={() => {
                          setSelectedKey(apiKey);
                          setShowApiDialog(true);
                        }}
                      >
                        查看
                      </Button>
                      {editingKeyId !== apiKey.id && (
                        <Button size="small" variant="text" onClick={() => startEditing(apiKey)}>
                          重命名
                        </Button>
                      )}
                      <Button
                        size="small"
                        variant="text"
                        color="error"
                        onClick={() => handleDeleteApi(apiKey.id)}
                      >
                        删除
                      </Button>
                    </Box>
                  </Box>
                ))}
              </Box>
            ) : (
              <Typography variant="body2" color="text.secondary">
                暂无 API 密钥，点击上方按钮创建
              </Typography>
            )}
          </Box>
        )}

        {/* Tab 1: 外部 API 配置 */}
        {tabValue === 1 && (
          <Grid container spacing={3} sx={{ p: 3 }}>
            <Grid size={{ xs: 12 }}>
              <Typography variant="h6" gutterBottom>
                外部 API 配置
              </Typography>
              <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
                添加和管理外部 AI 服务提供商的 API 配置
              </Typography>

              {/* 内联表单 - 直接显示 */}
              <ExternalApiForm
                editingApi={editingApiId ? externalApis.find((a) => a.id === editingApiId) || null : null}
                onSave={handleSaveExternalApi}
                onCancel={() => {
                  setShowExternalApiForm(false);
                  setEditingApiId(null);
                }}
              />

              {/* 已配置列表 */}
              <Typography variant="subtitle2" sx={{ mb: 1 }}>
                已配置 ({externalApis.length})
              </Typography>
              <ApiList
                apis={externalApis}
                editingId={editingApiId}
                onEdit={(api: ExternalApiConfig) => {
                  setEditingApiId(api.id);
                  setShowExternalApiForm(true);
                }}
                onToggle={handleToggleExternalApi}
                onDelete={handleDeleteExternalApi}
              />
            </Grid>
          </Grid>
        )}

        {/* Tab 2: AI 自动部署 */}
        {tabValue === 2 && (
          <Grid container spacing={3} sx={{ p: 3 }}>
            <Grid size={{ xs: 12 }}>
              <Typography variant="h6" gutterBottom>
                AI 自动部署
              </Typography>
              <Typography variant="body2" color="text.secondary" sx={{ mb: 3 }}>
                一键自动部署 GPU 推理服务器环境，让 AI 完成所有配置工作
              </Typography>

              {/* 部署状态 */}
              <Box sx={{ mb: 3 }}>
                <Typography variant="subtitle2" sx={{ mb: 1 }}>
                  部署状态:
                </Typography>
                <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 1 }}>
                  <Chip
                    label={`Python: ${deployStatus.pythonInstalled ? '✓' : '✗'}`}
                    color={deployStatus.pythonInstalled ? 'success' : 'default'}
                    variant="outlined"
                  />
                  <Chip
                    label={`依赖: ${deployStatus.dependenciesInstalled ? '✓' : '✗'}`}
                    color={deployStatus.dependenciesInstalled ? 'success' : 'default'}
                    variant="outlined"
                  />
                  <Chip
                    label={`模型: ${deployStatus.modelDownloaded ? '✓' : '✗'}`}
                    color={deployStatus.modelDownloaded ? 'success' : 'default'}
                    variant="outlined"
                  />
                  <Chip
                    label={`服务器: ${deployStatus.serverRunning ? '✓' : '✗'}`}
                    color={deployStatus.serverRunning ? 'success' : 'default'}
                    variant="outlined"
                  />
                </Box>
              </Box>

              {/* 部署进度 */}
              {deployProgress && (
                <Box sx={{ mb: 3, p: 2, backgroundColor: 'rgba(255, 255, 255, 0.05)', borderRadius: 1 }}>
                  <Box sx={{ display: 'flex', alignItems: 'center', gap: 2 }}>
                    {isDeploying && <CircularProgress size={20} sx={{ color: '#ffffff' }} />}
                    <Typography variant="body2" sx={{ color: isDeploying ? '#ffffff' : '#cccccc' }}>
                      {deployProgress}
                    </Typography>
                  </Box>
                </Box>
              )}

              {/* 自动部署按钮 */}
              <Button
                variant="contained"
                color="primary"
                size="large"
                onClick={handleAutoDeploy}
                disabled={isDeploying}
                startIcon={isDeploying ? <CircularProgress size={20} color="inherit" /> : undefined}
                sx={{ mb: 2 }}
              >
                {isDeploying ? '部署中...' : '开始自动部署'}
              </Button>

              {/* 手动启动服务器 */}
              <Box sx={{ mt: 3 }}>
                <Typography variant="subtitle2" sx={{ mb: 1 }}>
                  手动操作:
                </Typography>
                <Box sx={{ display: 'flex', gap: 1, flexWrap: 'wrap' }}>
                  <Button
                    variant="outlined"
                    onClick={async () => {
                      await invoke('start_gpu_server');
                      await checkDeployStatus();
                    }}
                  >
                    启动服务器
                  </Button>
                  <Button
                    variant="outlined"
                    onClick={async () => {
                      await invoke('stop_gpu_server');
                      await checkDeployStatus();
                    }}
                  >
                    停止服务器
                  </Button>
                  <Button variant="outlined" onClick={checkDeployStatus}>
                    刷新状态
                  </Button>
                  <Button
                    variant="outlined"
                    color="secondary"
                    onClick={async () => {
                      try {
                        const result = await invoke<string>('test_workflow_event');
                        console.log('Test workflow event:', result);
                        alert('测试已启动，请查看右侧对话框');
                      } catch (error) {
                        console.error('Test failed:', error);
                        alert('测试失败: ' + error);
                      }
                    }}
                  >
                    测试工作流事件
                  </Button>
                </Box>
              </Box>

              {/* 帮助信息 */}
              <Box sx={{ mt: 3, p: 2, backgroundColor: 'rgba(255, 193, 7, 0.1)', borderRadius: 1 }}>
                <Typography variant="body2" color="text.secondary">
                  <strong>提示:</strong> 如果自动部署失败，请确保:
                </Typography>
                <Typography variant="body2" color="text.secondary" component="ul" sx={{ mb: 0, pl: 2 }}>
                  <li>已安装 Python 3.8+</li>
                  <li>网络连接正常（需要下载模型）</li>
                  <li>有足够的磁盘空间（需要约 5GB）</li>
                </Typography>
              </Box>
            </Grid>
          </Grid>
        )}
        </Box>
      </DialogContent>

      <DialogActions>
        <Button onClick={onClose}>关闭</Button>
      </DialogActions>

      {/* 创建 API 对话框 */}
      <Dialog
        open={showCreateDialog}
        onClose={() => setShowCreateDialog(false)}
        maxWidth="sm"
        fullWidth
        PaperProps={{
          sx: {
            background: alpha(theme.palette.background.paper, 0.95),
            backdropFilter: 'blur(20px)',
            border: `1px solid ${theme.palette.divider}`,
          },
        }}
      >
        <DialogTitle>
          <Typography variant="h6" component="span">
            创建 API 密钥
          </Typography>
        </DialogTitle>
        <DialogContent>
          <TextField
            fullWidth
            label="API 名称"
            placeholder="请输入 API 名称（如：生产环境、测试环境）"
            value={newApiName}
            onChange={(e) => setNewApiName(e.target.value)}
            helperText="为您的 API 密钥起一个易于识别的名称"
            sx={{ mt: 1 }}
          />
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setShowCreateDialog(false)}>取消</Button>
          <Button onClick={handleCreateApi} variant="contained" disabled={!newApiName.trim()}>
            创建
          </Button>
        </DialogActions>
      </Dialog>

      {/* API 密钥显示对话框 */}
      <Dialog
        open={showApiDialog}
        onClose={() => setShowApiDialog(false)}
        maxWidth="sm"
        fullWidth
        PaperProps={{
          sx: {
            background: alpha(theme.palette.background.paper, 0.95),
            backdropFilter: 'blur(20px)',
            border: `1px solid ${theme.palette.divider}`,
          },
        }}
      >
        <DialogTitle>
          <Typography variant="h6" component="span">
            API 密钥 - {selectedKey?.name}
          </Typography>
        </DialogTitle>
        <DialogContent>
          <TextField
            fullWidth
            multiline
            rows={4}
            value={selectedKey?.key || ''}
            InputProps={{
              readOnly: true,
            }}
            helperText="请妥善保管您的 API 密钥"
          />
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setShowApiDialog(false)}>关闭</Button>
          <Button
            onClick={() => {
              if (selectedKey?.key) {
                navigator.clipboard.writeText(selectedKey.key);
                console.log('API key copied to clipboard');
              }
            }}
            variant="contained"
          >
            复制
          </Button>
        </DialogActions>
      </Dialog>
    </Dialog>
  );
};
