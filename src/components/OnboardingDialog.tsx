import React, { useState } from 'react';
import {
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  Box,
  Button,
  Typography,
  TextField,
  MenuItem,
  Select,
  FormControl,
  InputLabel,
  Stepper,
  Step,
  StepLabel,
  alpha,
  CircularProgress,
  Alert,
} from '@mui/material';
import HubIcon from '@mui/icons-material/Hub';
import SmartToyIcon from '@mui/icons-material/SmartToy';
import CheckCircleIcon from '@mui/icons-material/CheckCircle';
import { invoke } from '@tauri-apps/api/core';

interface OnboardingDialogProps {
  open: boolean;
  onClose: () => void;
}

const PROVIDERS = [
  { value: 'deepseek', label: 'DeepSeek', baseUrl: 'https://api.deepseek.com/v1', model: 'deepseek-chat', hint: '推荐 · 价格低 · 速度快' },
  { value: 'openai', label: 'OpenAI', baseUrl: 'https://api.openai.com/v1', model: 'gpt-4o-mini', hint: '全球最流行' },
  { value: 'anthropic', label: 'Anthropic Claude', baseUrl: 'https://api.anthropic.com/v1', model: 'claude-3-haiku-20240307', hint: '高质量回复' },
  { value: 'qwen', label: '通义千问 (Qwen)', baseUrl: 'https://dashscope.aliyuncs.com/compatible-mode/v1', model: 'qwen-turbo', hint: '阿里云 · 国内推荐' },
  { value: 'glm', label: '智谱 GLM', baseUrl: 'https://open.bigmodel.cn/api/paas/v4', model: 'glm-4-flash', hint: '智谱 AI · 国内推荐' },
  { value: 'groq', label: 'Groq', baseUrl: 'https://api.groq.com/openai/v1', model: 'llama3-8b-8192', hint: '超快推理速度' },
  { value: 'custom', label: '自定义', baseUrl: '', model: '', hint: '任何 OpenAI 兼容接口' },
];

const steps = ['欢迎', '配置 AI', '完成'];

export const OnboardingDialog: React.FC<OnboardingDialogProps> = ({ open, onClose }) => {
  const [activeStep, setActiveStep] = useState(0);
  const [provider, setProvider] = useState('deepseek');
  const [apiKey, setApiKey] = useState('');
  const [baseUrl, setBaseUrl] = useState('https://api.deepseek.com/v1');
  const [model, setModel] = useState('deepseek-chat');
  const [isTesting, setIsTesting] = useState(false);
  const [testResult, setTestResult] = useState<{ success: boolean; message: string } | null>(null);
  const [isSaving, setIsSaving] = useState(false);

  const selectedProvider = PROVIDERS.find(p => p.value === provider);

  const handleProviderChange = (newProvider: string) => {
    setProvider(newProvider);
    const p = PROVIDERS.find(p => p.value === newProvider);
    if (p && p.value !== 'custom') {
      setBaseUrl(p.baseUrl);
      setModel(p.model);
    }
    setTestResult(null);
  };

  const handleTestApi = async () => {
    if (!apiKey.trim()) {
      setTestResult({ success: false, message: '请先输入 API Key' });
      return;
    }
    setIsTesting(true);
    setTestResult(null);
    try {
      const result = await invoke<{ success: boolean; message: string }>('test_external_api', {
        provider,
        apiKey,
        baseUrl,
        model,
      });
      setTestResult(result);
    } catch (error: any) {
      setTestResult({ success: false, message: error.toString() });
    } finally {
      setIsTesting(false);
    }
  };

  const handleSave = async () => {
    if (!apiKey.trim()) return;
    setIsSaving(true);
    try {
      await invoke('save_external_api', {
        config: {
          name: `${selectedProvider?.label || provider}`,
          provider,
          base_url: baseUrl,
          api_key: apiKey,
          model,
          enabled: true,
        },
      });
      setActiveStep(2);
    } catch (error: any) {
      setTestResult({ success: false, message: `保存失败: ${error}` });
    } finally {
      setIsSaving(false);
    }
  };

  const handleSkip = () => {
    onClose();
  };

  const handleFinish = () => {
    onClose();
  };

  return (
    <Dialog
      open={open}
      maxWidth="sm"
      fullWidth
      disableEscapeKeyDown
      PaperProps={{
        sx: {
          background: '#0a0a0a',
          border: `1px solid ${alpha('#ffffff', 0.1)}`,
          borderRadius: 2,
        },
      }}
    >
      {/* 顶部步骤条 */}
      <Box sx={{ px: 3, pt: 3, pb: 1 }}>
        <Stepper activeStep={activeStep} alternativeLabel>
          {steps.map((label) => (
            <Step key={label}>
              <StepLabel>{label}</StepLabel>
            </Step>
          ))}
        </Stepper>
      </Box>

      {/* 步骤 0: 欢迎页 */}
      {activeStep === 0 && (
        <>
          <DialogContent sx={{ textAlign: 'center', py: 4 }}>
            <Box sx={{ mb: 3 }}>
              <HubIcon sx={{ fontSize: 64, color: 'primary.main', opacity: 0.8 }} />
            </Box>
            <Typography variant="h5" gutterBottom fontWeight={600}>
              欢迎使用 Williw
            </Typography>
            <Typography variant="body1" color="text.secondary" sx={{ mb: 3, lineHeight: 1.8 }}>
              Williw 是一个去中心化 AI 平台。<br />
              你的设备将成为 P2P 网络中的一个节点，<br />
              同时你可以使用 AI 进行对话。
            </Typography>

            <Box sx={{ display: 'flex', gap: 2, justifyContent: 'center', flexWrap: 'wrap' }}>
              {[
                { icon: '🌐', title: 'P2P 节点', desc: '自动加入去中心化网络' },
                { icon: '🤖', title: 'AI 对话', desc: '使用你的 API 与 AI 对话' },
                { icon: '⭐', title: '贡献积分', desc: '贡献算力获得积分' },
              ].map((item) => (
                <Box
                  key={item.title}
                  sx={{
                    p: 2,
                    borderRadius: 1,
                    border: `1px solid ${alpha('#ffffff', 0.1)}`,
                    textAlign: 'center',
                    minWidth: 120,
                    flex: 1,
                  }}
                >
                  <Typography variant="h4" sx={{ mb: 0.5 }}>{item.icon}</Typography>
                  <Typography variant="body2" fontWeight={600}>{item.title}</Typography>
                  <Typography variant="caption" color="text.secondary">{item.desc}</Typography>
                </Box>
              ))}
            </Box>
          </DialogContent>
          <DialogActions sx={{ px: 3, pb: 3, justifyContent: 'space-between' }}>
            <Button color="inherit" onClick={handleSkip} sx={{ color: 'text.secondary' }}>
              跳过配置
            </Button>
            <Button variant="contained" onClick={() => setActiveStep(1)} sx={{ px: 4 }}>
              开始配置
            </Button>
          </DialogActions>
        </>
      )}

      {/* 步骤 1: 配置 AI API */}
      {activeStep === 1 && (
        <>
          <DialogTitle sx={{ pb: 0 }}>
            <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
              <SmartToyIcon sx={{ color: 'primary.main' }} />
              <Typography variant="h6">配置 AI 服务</Typography>
            </Box>
            <Typography variant="caption" color="text.secondary">
              Williw 本身不提供 AI 算力，请填入你自己的 API Key
            </Typography>
          </DialogTitle>
          <DialogContent sx={{ pt: 2 }}>
            {/* 服务商选择 */}
            <FormControl fullWidth size="small" sx={{ mb: 2 }}>
              <InputLabel>AI 服务商</InputLabel>
              <Select
                value={provider}
                label="AI 服务商"
                onChange={(e) => handleProviderChange(e.target.value)}
              >
                {PROVIDERS.map((p) => (
                  <MenuItem key={p.value} value={p.value}>
                    <Box>
                      <Typography variant="body2">{p.label}</Typography>
                      <Typography variant="caption" color="text.secondary">{p.hint}</Typography>
                    </Box>
                  </MenuItem>
                ))}
              </Select>
            </FormControl>

            {/* API Key 输入 */}
            <TextField
              fullWidth
              size="small"
              label="API Key"
              type="password"
              placeholder={`输入你的 ${selectedProvider?.label || 'API'} Key`}
              value={apiKey}
              onChange={(e) => {
                setApiKey(e.target.value);
                setTestResult(null);
              }}
              sx={{ mb: 2 }}
              helperText={
                provider === 'deepseek' ? '前往 platform.deepseek.com 获取 API Key' :
                provider === 'openai' ? '前往 platform.openai.com 获取 API Key' :
                provider === 'qwen' ? '前往 dashscope.console.aliyun.com 获取 API Key' :
                provider === 'glm' ? '前往 open.bigmodel.cn 获取 API Key' :
                '请输入你的 API Key'
              }
            />

            {/* 自定义时显示 BaseURL 和 Model */}
            {provider === 'custom' && (
              <>
                <TextField
                  fullWidth
                  size="small"
                  label="API Base URL"
                  placeholder="https://your-api.com/v1"
                  value={baseUrl}
                  onChange={(e) => setBaseUrl(e.target.value)}
                  sx={{ mb: 2 }}
                />
                <TextField
                  fullWidth
                  size="small"
                  label="模型名称"
                  placeholder="gpt-4o-mini"
                  value={model}
                  onChange={(e) => setModel(e.target.value)}
                  sx={{ mb: 2 }}
                />
              </>
            )}

            {/* 测试结果 */}
            {testResult && (
              <Alert
                severity={testResult.success ? 'success' : 'error'}
                sx={{ mb: 2 }}
              >
                {testResult.message}
              </Alert>
            )}

            {/* 测试按钮 */}
            <Button
              variant="outlined"
              onClick={handleTestApi}
              disabled={isTesting || !apiKey.trim()}
              startIcon={isTesting ? <CircularProgress size={16} /> : undefined}
              fullWidth
              sx={{ mb: 1 }}
            >
              {isTesting ? '测试中...' : '测试连接'}
            </Button>
          </DialogContent>
          <DialogActions sx={{ px: 3, pb: 3, justifyContent: 'space-between' }}>
            <Button color="inherit" onClick={() => setActiveStep(0)}>
              上一步
            </Button>
            <Box sx={{ display: 'flex', gap: 1 }}>
              <Button color="inherit" onClick={handleSkip} sx={{ color: 'text.secondary' }}>
                跳过
              </Button>
              <Button
                variant="contained"
                onClick={handleSave}
                disabled={isSaving || !apiKey.trim()}
                startIcon={isSaving ? <CircularProgress size={16} color="inherit" /> : undefined}
              >
                {isSaving ? '保存中...' : '保存并继续'}
              </Button>
            </Box>
          </DialogActions>
        </>
      )}

      {/* 步骤 2: 完成 */}
      {activeStep === 2 && (
        <>
          <DialogContent sx={{ textAlign: 'center', py: 5 }}>
            <CheckCircleIcon sx={{ fontSize: 72, color: 'success.main', mb: 2 }} />
            <Typography variant="h5" gutterBottom fontWeight={600}>
              配置完成！
            </Typography>
            <Typography variant="body1" color="text.secondary" sx={{ mb: 2, lineHeight: 1.8 }}>
              你的节点已自动加入 P2P 网络。<br />
              现在可以在右侧开始 AI 对话了。
            </Typography>
            <Box
              sx={{
                display: 'inline-block',
                p: 2,
                borderRadius: 1,
                background: alpha('#4caf50', 0.1),
                border: `1px solid ${alpha('#4caf50', 0.3)}`,
              }}
            >
              <Typography variant="body2" color="success.light">
                ✅ AI API 已配置<br />
                ✅ P2P 节点运行中<br />
                ✅ 准备好对话了
              </Typography>
            </Box>
          </DialogContent>
          <DialogActions sx={{ px: 3, pb: 3, justifyContent: 'center' }}>
            <Button variant="contained" onClick={handleFinish} sx={{ px: 6 }}>
              开始使用
            </Button>
          </DialogActions>
        </>
      )}
    </Dialog>
  );
};
