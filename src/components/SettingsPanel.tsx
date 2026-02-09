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
  useTheme,
  alpha,
} from '@mui/material';
import { invoke } from '@tauri-apps/api/core';

interface SettingsPanelProps {
  onClose: () => void;
}

interface ApiKey {
  id: string;
  name: string;
  key: string;
  created_at: string;
}

export const SettingsPanel: React.FC<SettingsPanelProps> = ({ onClose }) => {
  const theme = useTheme();
  const [apiKeys, setApiKeys] = useState<ApiKey[]>([]);
  const [selectedKey, setSelectedKey] = useState<ApiKey | null>(null);
  const [showApiDialog, setShowApiDialog] = useState(false);
  const [newApiName, setNewApiName] = useState('');
  const [showCreateDialog, setShowCreateDialog] = useState(false);

  useEffect(() => {
    loadApiKeys();
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
      setApiKeys(apiKeys.filter(k => k.id !== id));
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
      setApiKeys(apiKeys.map(k => k.id === id ? { ...k, name: newName } : k));
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
        },
      }}
    >
      <DialogTitle>
        <Typography variant="h6" component="span">设置</Typography>
      </DialogTitle>

      <DialogContent>
        <Grid container spacing={3}>
          {/* API 管理 */}
          <Grid item xs={12}>
            <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', mb: 2 }}>
              <Typography variant="h6" gutterBottom>
                API 管理
              </Typography>
              <Button
                variant="contained"
                color="primary"
                onClick={() => setShowCreateDialog(true)}
              >
                创建新 API
              </Button>
            </Box>
            
            {/* API 列表 */}
            {apiKeys.length > 0 ? (
              <Box sx={{ display: 'flex', flexDirection: 'column', gap: 1 }}>
                {apiKeys.map((key) => (
                  <Box
                    key={key.id}
                    sx={{
                      p: 2,
                      backgroundColor: 'rgba(255, 255, 255, 0.05)',
                      borderRadius: 1,
                      display: 'flex',
                      justifyContent: 'space-between',
                      alignItems: 'center',
                    }}
                  >
                    <Box sx={{ flex: 1 }}>
                      {editingKeyId === key.id ? (
                        <TextField
                          size="small"
                          value={editingName}
                          onChange={(e) => setEditingName(e.target.value)}
                          sx={{ mb: 1 }}
                        />
                      ) : (
                        <Typography variant="body2" sx={{ fontWeight: 500, mb: 0.5 }}>
                          {key.name}
                        </Typography>
                      )}
                      <Typography variant="caption" color="text.secondary">
                        {key.key.substring(0, 16)}...{key.key.substring(key.key.length - 4)}
                      </Typography>
                      <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mt: 0.5 }}>
                        创建时间: {new Date(key.created_at).toLocaleDateString('zh-CN')}
                      </Typography>
                    </Box>
                    <Box sx={{ display: 'flex', gap: 1 }}>
                      {editingKeyId === key.id ? (
                        <>
                          <Button
                            size="small"
                            variant="text"
                            onClick={() => saveEditing(key.id)}
                          >
                            保存
                          </Button>
                          <Button
                            size="small"
                            variant="text"
                            onClick={cancelEditing}
                          >
                            取消
                          </Button>
                        </>
                      ) : (
                        <>
                          <Button
                            size="small"
                            variant="text"
                            onClick={() => startEditing(key)}
                          >
                            重命名
                          </Button>
                          <Button
                            size="small"
                            variant="text"
                            onClick={() => {
                              setSelectedKey(key);
                              setShowApiDialog(true);
                            }}
                          >
                            查看
                          </Button>
                          <Button
                            size="small"
                            variant="text"
                            color="error"
                            onClick={() => handleDeleteApi(key.id)}
                          >
                            删除
                          </Button>
                        </>
                      )}
                    </Box>
                  </Box>
                ))}
              </Box>
            ) : (
              <Typography variant="body2" color="text.secondary">
                暂无 API 密钥，点击上方按钮创建
              </Typography>
            )}
          </Grid>
        </Grid>
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
          <Typography variant="h6" component="span">创建 API 密钥</Typography>
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
          <Button
            onClick={handleCreateApi}
            variant="contained"
            disabled={!newApiName.trim()}
          >
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
          <Typography variant="h6" component="span">API 密钥 - {selectedKey?.name}</Typography>
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
