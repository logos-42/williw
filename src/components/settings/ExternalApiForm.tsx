import React, { useState } from 'react';
import {
  Box,
  Button,
  TextField,
  Typography,
  Grid,
  Select,
  MenuItem,
  FormControl,
  InputLabel,
  CircularProgress,
} from '@mui/material';
import { invoke } from '@tauri-apps/api/core';

export type ProviderType = 
  | 'openai'
  | 'deepseek'
  | 'anthropic'
  | 'glm'
  | 'kimichat'
  | 'minimax'
  | 'qwen'
  | 'nvidia'
  | 'openrouter'
  | 'google'
  | 'vercel'
  | 'groq'
  | 'perplexity'
  | 'custom';

export interface ExternalApiConfig {
  id: string;
  name?: string;
  provider: ProviderType;
  base_url: string;
  api_key: string;
  model: string;
  enabled: boolean;
}

interface ExternalApiFormProps {
  editingApi: ExternalApiConfig | null;
  onSave: (config: Partial<ExternalApiConfig>) => Promise<void>;
  onCancel: () => void;
}

const PROVIDER_CONFIGS: Record<ProviderType, { baseUrl: string; model: string }> = {
  openai: { baseUrl: 'https://api.openai.com/v1', model: 'gpt-5.3' },
  deepseek: { baseUrl: 'https://api.deepseek.com/v1', model: 'deepseek-chat' },
  anthropic: { baseUrl: 'https://api.anthropic.com', model: 'claude-4.6-sonnet-20240229' },
  glm: { baseUrl: 'https://open.bigmodel.cn/api/paas/v4', model: 'glm-4.7' },
  kimichat: { baseUrl: 'https://api.moonshot.cn/v1', model: 'kimi k2.5 thinking' },
  minimax: { baseUrl: 'https://api.minimax.chat/v1', model: 'abab6.5s-chat' },
  qwen: { baseUrl: 'https://dashscope.aliyuncs.com/compatible-mode/v1', model: 'qwen-turbo' },
  nvidia: { baseUrl: 'https://api.nvidia.com/v1', model: 'mistralai/mixtral-8x7b-instruct-v0.1' },
  openrouter: { baseUrl: 'https://openrouter.ai/api/v1', model: 'anthropic/claude-3-haiku' },
  google: { baseUrl: 'https://generativelanguage.googleapis.com/v1beta', model: 'gemini-1.5-flash' },
  vercel: { baseUrl: 'https://api.vercel.com/v1', model: 'claude-3-5-sonnet' },
  groq: { baseUrl: 'https://api.groq.com/openai/v1', model: 'llama3-8b-8192' },
  perplexity: { baseUrl: 'https://api.perplexity.ai', model: 'sonar' },
  custom: { baseUrl: '', model: '' },
};

export const ExternalApiForm: React.FC<ExternalApiFormProps> = ({
  editingApi,
  onSave,
  onCancel,
}) => {
  const [formData, setFormData] = useState<Partial<ExternalApiConfig>>(
    editingApi || {
      provider: 'deepseek',
      base_url: 'https://api.deepseek.com/v1',
      model: 'deepseek-chat',
      enabled: true,
    }
  );
  const [testingApi, setTestingApi] = useState(false);
  const [testResult, setTestResult] = useState<{
    success: boolean;
    message: string;
  } | null>(null);

  const handleProviderChange = (provider: ExternalApiConfig['provider']) => {
    const config = PROVIDER_CONFIGS[provider];
    setFormData((prev) => ({
      ...prev,
      provider,
      base_url: config.baseUrl,
      model: config.model,
    }));
  };

  const handleTestApi = async () => {
    if (!formData.api_key?.trim() || !formData.base_url?.trim()) {
      setTestResult({
        success: false,
        message: '请填写 API 密钥和 Base URL',
      });
      return;
    }

    setTestingApi(true);
    setTestResult(null);

    try {
      const result = await invoke<{
        success: boolean;
        message: string;
        response: string;
      }>('test_external_api', {
        provider: formData.provider,
        apiKey: formData.api_key,
        baseUrl: formData.base_url,
        model: formData.model || 'default',
      });

      setTestResult({
        success: result.success,
        message: result.success
          ? `✓ API 连接成功！${result.response ? ': ' + result.response.substring(0, 100) : ''}`
          : `✗ API 返回错误: ${result.message}`,
      });
    } catch (error: any) {
      setTestResult({
        success: false,
        message: `✗ 连接失败: ${error.toString()}`,
      });
    } finally {
      setTestingApi(false);
    }
  };

  const handleSave = async () => {
    if (!formData.api_key?.trim()) {
      alert('请填写 API 密钥');
      return;
    }
    await onSave(formData);
  };

  return (
    <Box
      sx={{
        p: 3,
        backgroundColor: 'rgba(255, 255, 255, 0.05)',
        borderRadius: 2,
        mb: 3,
      }}
    >
      <Typography variant="subtitle1" sx={{ mb: 2, fontWeight: 500 }}>
        {editingApi ? '编辑配置' : '添加新配置'}
      </Typography>

      <Grid container spacing={2}>

        <Grid size={{ xs: 12, sm: 6 }}>
          <FormControl fullWidth size="small">
            <InputLabel>服务提供商</InputLabel>
            <Select
              value={formData.provider || 'deepseek'}
              label="服务提供商"
              onChange={(e) => handleProviderChange(e.target.value as ProviderType)}
            >
              <MenuItem value="openai">OpenAI</MenuItem>
              <MenuItem value="deepseek">DeepSeek</MenuItem>
              <MenuItem value="anthropic">Anthropic Claude</MenuItem>
              <MenuItem value="glm">GLM (智谱AI)</MenuItem>
              <MenuItem value="kimichat">Kimichat (月之暗面)</MenuItem>
              <MenuItem value="minimax">Minimax (海螺AI)</MenuItem>
              <MenuItem value="qwen">Qwen (通义千问)</MenuItem>
              <MenuItem value="nvidia">NVIDIA</MenuItem>
              <MenuItem value="openrouter">OpenRouter</MenuItem>
              <MenuItem value="google">Google Gemini</MenuItem>
              <MenuItem value="vercel">Vercel AI SDK</MenuItem>
              <MenuItem value="groq">Groq</MenuItem>
              <MenuItem value="perplexity">Perplexity</MenuItem>
              <MenuItem value="custom">自定义 API</MenuItem>
            </Select>
          </FormControl>
        </Grid>

        <Grid size={{ xs: 12 }}>
          <TextField
            fullWidth
            size="small"
            label="API 密钥"
            placeholder="请填入您的 API 密钥"
            type="password"
            value={formData.api_key || ''}
            onChange={(e) => setFormData((prev) => ({ ...prev, api_key: e.target.value }))}
          />
        </Grid>

        <Grid size={{ xs: 12, sm: 6 }}>
          <TextField
            fullWidth
            size="small"
            label="API Base URL"
            placeholder="https://api.deepseek.com"
            value={formData.base_url || ''}
            onChange={(e) => setFormData((prev) => ({ ...prev, base_url: e.target.value }))}
          />
        </Grid>

        <Grid size={{ xs: 12, sm: 6 }}>
          <TextField
            fullWidth
            size="small"
            label="模型名称"
            placeholder="deepseek-chat"
            value={formData.model || ''}
            onChange={(e) => setFormData((prev) => ({ ...prev, model: e.target.value }))}
          />
        </Grid>
      </Grid>

      {/* 测试结果显示 */}
      {testResult && (
        <Box
          sx={{
            mt: 2,
            p: 2,
            borderRadius: 1,
            backgroundColor: testResult.success
              ? 'rgba(255, 255, 255, 0.05)'
              : 'rgba(255, 255, 255, 0.05)',
            border: `1px solid ${testResult.success ? '#ffffff' : '#666666'}`,
          }}
        >
          <Typography variant="body2" sx={{ color: testResult.success ? '#ffffff' : '#888888' }}>
            {testResult.message}
          </Typography>
        </Box>
      )}

      {/* 操作按钮 */}
      <Box sx={{ display: 'flex', gap: 1, mt: 2 }}>
        <Button
          variant="outlined"
          color="primary"
          onClick={handleTestApi}
          disabled={testingApi || !formData.api_key?.trim()}
          startIcon={testingApi ? <CircularProgress size={16} /> : undefined}
        >
          {testingApi ? '测试中...' : '测试连接'}
        </Button>

        <Button
          variant="contained"
          color="primary"
          onClick={handleSave}
          disabled={!formData.api_key?.trim()}
        >
          {editingApi ? '保存配置' : '添加配置'}
        </Button>

        <Button variant="text" onClick={onCancel}>
          取消
        </Button>
      </Box>
    </Box>
  );
};
