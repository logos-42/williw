import React, { useState } from 'react';
import {
  Box,
  Typography,
  TextField,
  Button,
  Grid,
  Card,
  CardContent,
  CardActions,
  Chip,
  Alert,
  InputAdornment,
  IconButton,
  Tab,
  Tabs,
} from '@mui/material';
import {
  Store as StoreIcon,
  AccountBalanceWallet,
  Visibility,
  VisibilityOff,
  Cloud,
  Code,
  Security,
} from '@mui/icons-material';
import { useWalletStore } from '../store/walletStore';

interface ApiProduct {
  id: string;
  name: string;
  provider: string;
  description: string;
  price: number;
  unit: string;
  category: 'llm' | 'vision' | 'embedding';
  rating: number;
  users: number;
}

interface ComputeProvider {
  id: string;
  name: string;
  gpuModel: string;
  vram: string;
  pricePerHour: number;
  location: string;
  status: 'online' | 'offline' | 'busy';
  uptime: number;
}

const mockApiProducts: ApiProduct[] = [
  {
    id: '1',
    name: 'LLaMA 2.5',
    provider: 'Meta',
    description: '最新开源大模型，支持长上下文，高性能',
    price: 0.008,
    unit: '1K tokens',
    category: 'llm',
    rating: 4.8,
    users: 15200,
  },
  {
    id: '2',
    name: 'Qwen 3.5',
    provider: 'Alibaba',
    description: '阿里最新模型，中文任务表现优异',
    price: 0.006,
    unit: '1K tokens',
    category: 'llm',
    rating: 4.9,
    users: 12800,
  },
  {
    id: '3',
    name: 'GLM-5',
    provider: 'Zhipu',
    description: '智谱最新ChatGLM，多轮对话能力强',
    price: 0.007,
    unit: '1K tokens',
    category: 'llm',
    rating: 4.7,
    users: 9500,
  },
  {
    id: '4',
    name: 'MiniMax-M2',
    provider: 'MiniMax',
    description: 'MiniMax最新模型，长文本处理能力强',
    price: 0.005,
    unit: '1K tokens',
    category: 'llm',
    rating: 4.6,
    users: 7200,
  },
  {
    id: '5',
    name: 'DeepSeek-V3',
    provider: 'DeepSeek',
    description: '最新MoE模型，代码能力突出',
    price: 0.006,
    unit: '1K tokens',
    category: 'llm',
    rating: 4.8,
    users: 8900,
  },
  {
    id: '6',
    name: 'FLUX.1-schnell',
    provider: 'BlackForest',
    description: '开源图像生成模型，高质量快速',
    price: 0.002,
    unit: '1张图',
    category: 'vision',
    rating: 4.6,
    users: 4200,
  },
  {
    id: '7',
    name: 'bge-m3',
    provider: 'BAAI',
    description: '文本向量化模型，支持多语言',
    price: 0.0001,
    unit: '1K tokens',
    category: 'embedding',
    rating: 4.5,
    users: 3100,
  },
];

const mockComputeProviders: ComputeProvider[] = [
  {
    id: '1',
    name: 'GPU Farm Alpha',
    gpuModel: 'NVIDIA A100',
    vram: '80GB',
    pricePerHour: 2.5,
    location: 'US-East',
    status: 'online',
    uptime: 99.5,
  },
  {
    id: '2',
    name: '算力集群 Beta',
    gpuModel: 'NVIDIA H100',
    vram: '80GB',
    pricePerHour: 3.0,
    location: 'Singapore',
    status: 'online',
    uptime: 98.8,
  },
  {
    id: '3',
    name: '云端算力 Gamma',
    gpuModel: 'NVIDIA RTX 4090',
    vram: '24GB',
    pricePerHour: 1.2,
    location: 'Europe',
    status: 'busy',
    uptime: 97.2,
  },
];

export const Marketplace: React.FC = () => {
  const { walletAddress, isConnected, setWalletAddress } = useWalletStore();
  const [walletInput, setWalletInput] = useState('');
  const [showWalletInput, setShowWalletInput] = useState(false);
  const [showPassword, setShowPassword] = useState(false);
  const [marketTab, setMarketTab] = useState(0);

  const handleConnectWallet = () => {
    if (walletInput.trim()) {
      setWalletAddress(walletInput.trim());
      setShowWalletInput(false);
      setWalletInput('');
    }
  };

  const handleDisconnect = () => {
    setWalletAddress('');
  };

  const formatAddress = (addr: string) => {
    if (addr.length <= 12) return addr;
    return `${addr.slice(0, 6)}...${addr.slice(-4)}`;
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'online':
        return 'success';
      case 'busy':
        return 'warning';
      default:
        return 'default';
    }
  };

  return (
    <Box sx={{ height: '100%', display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
      {/* 头部 */}
      <Box
        sx={{
          p: 3,
          borderBottom: 1,
          borderColor: 'divider',
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          flexShrink: 0,
        }}
      >
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 2 }}>
          <StoreIcon sx={{ fontSize: 32, color: 'primary.main' }} />
          <Typography variant="h5" sx={{ fontWeight: 600 }}>
            API 市场
          </Typography>
        </Box>

        {isConnected ? (
          <Box sx={{ display: 'flex', alignItems: 'center', gap: 2 }}>
            <Chip
              icon={<AccountBalanceWallet />}
              label={formatAddress(walletAddress)}
              variant="outlined"
              color="primary"
            />
            <Button size="small" variant="text" onClick={handleDisconnect}>
              断开
            </Button>
          </Box>
        ) : (
          <Button
            variant="outlined"
            startIcon={<AccountBalanceWallet />}
            onClick={() => setShowWalletInput(!showWalletInput)}
          >
            连接钱包
          </Button>
        )}
      </Box>

      {/* 钱包输入区 */}
      {showWalletInput && !isConnected && (
        <Box
          sx={{
            p: 2,
            borderBottom: 1,
            borderColor: 'divider',
            backgroundColor: 'rgba(255, 193, 7, 0.1)',
          }}
        >
          <Box sx={{ display: 'flex', gap: 2, alignItems: 'center' }}>
            <TextField
              size="small"
              placeholder="输入加密钱包地址 (可选)"
              value={walletInput}
              onChange={(e) => setWalletInput(e.target.value)}
              sx={{ flex: 1, maxWidth: 400 }}
              InputProps={{
                endAdornment: (
                  <InputAdornment position="end">
                    <IconButton
                      onClick={() => setShowPassword(!showPassword)}
                      edge="end"
                    >
                      {showPassword ? <VisibilityOff /> : <Visibility />}
                    </IconButton>
                  </InputAdornment>
                ),
              }}
            />
            <Button variant="contained" onClick={handleConnectWallet} disabled={!walletInput.trim()}>
              绑定钱包
            </Button>
            <Button variant="text" onClick={() => setShowWalletInput(false)}>
              跳过
            </Button>
          </Box>
          <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mt: 1 }}>
            绑定钱包后可购买 API 服务和发布算力资源，不绑定也可浏览
          </Typography>
        </Box>
      )}

      {/* 未连接钱包的提醒 */}
      {!isConnected && !showWalletInput && (
        <Alert severity="info" sx={{ m: 2 }}>
          您当前未连接加密钱包。连接钱包后可以：
          <ul style={{ margin: '8px 0 0 20px', padding: 0 }}>
            <li>购买 API 服务</li>
            <li>出租您的算力获取收益</li>
            <li>签署算力分享合约</li>
            <li>提供模型给其他用户</li>
          </ul>
        </Alert>
      )}

      {/* Tab 切换 */}
      <Box sx={{ borderBottom: 1, borderColor: 'divider', px: 2, flexShrink: 0 }}>
        <Tabs value={marketTab} onChange={(_, v) => setMarketTab(v)}>
          <Tab icon={<Code />} label="API 市场" iconPosition="start" />
          <Tab icon={<Cloud />} label="算力服务" iconPosition="start" />
        </Tabs>
      </Box>

      {/* 内容区域 */}
      <Box sx={{ flex: 1, overflow: 'auto', p: 3 }}>
        {marketTab === 0 && (
          <Grid container spacing={3}>
            {mockApiProducts.map((product) => (
              <Grid size={{ xs: 12, sm: 6, md: 4 }} key={product.id}>
                <Card
                  sx={{
                    height: '100%',
                    display: 'flex',
                    flexDirection: 'column',
                    transition: 'transform 0.2s',
                    '&:hover': { transform: 'translateY(-4px)' },
                  }}
                >
                  <CardContent sx={{ flex: 1 }}>
                    <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', mb: 1 }}>
                      <Typography variant="h6" sx={{ fontWeight: 600 }}>
                        {product.name}
                      </Typography>
                      <Chip label={product.category.toUpperCase()} size="small" variant="outlined" />
                    </Box>
                    <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
                      {product.provider}
                    </Typography>
                    <Typography variant="body2" sx={{ mb: 2, minHeight: 40 }}>
                      {product.description}
                    </Typography>
                    <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                      <Typography variant="h6" color="primary">
                        ${product.price}/{product.unit}
                      </Typography>
                      <Box sx={{ textAlign: 'right' }}>
                        <Typography variant="caption" display="block">
                          ⭐ {product.rating}
                        </Typography>
                        <Typography variant="caption" color="text.secondary">
                          {product.users.toLocaleString()} 用户
                        </Typography>
                      </Box>
                    </Box>
                  </CardContent>
                  <CardActions sx={{ p: 2, pt: 0 }}>
                    <Button
                      fullWidth
                      variant="contained"
                      onClick={() => alert(`购买 ${product.name} - 需要连接钱包`)}
                    >
                      {isConnected ? '立即购买' : '连接钱包购买'}
                    </Button>
                  </CardActions>
                </Card>
              </Grid>
            ))}
          </Grid>
        )}

        {marketTab === 1 && (
          <>
            <Typography variant="h6" sx={{ mb: 2 }}>
              可用的算力提供商
            </Typography>
            <Typography variant="body2" color="text.secondary" sx={{ mb: 3 }}>
              租用GPU算力，运行您的AI模型，或签署算力分享合约
            </Typography>
            <Grid container spacing={3}>
              {mockComputeProviders.map((provider) => (
                <Grid size={{ xs: 12, sm: 6, md: 4 }} key={provider.id}>
                  <Card>
                    <CardContent>
                      <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', mb: 2 }}>
                        <Typography variant="h6" sx={{ fontWeight: 600 }}>
                          {provider.name}
                        </Typography>
                        <Chip
                          label={provider.status}
                          size="small"
                          color={getStatusColor(provider.status) as any}
                          variant="outlined"
                        />
                      </Box>
                      <Box sx={{ display: 'flex', flexDirection: 'column', gap: 1, mb: 2 }}>
                        <Typography variant="body2">
                          <strong>GPU:</strong> {provider.gpuModel}
                        </Typography>
                        <Typography variant="body2">
                          <strong>显存:</strong> {provider.vram}
                        </Typography>
                        <Typography variant="body2">
                          <strong>位置:</strong> {provider.location}
                        </Typography>
                        <Typography variant="body2">
                          <strong>可用率:</strong> {provider.uptime}%
                        </Typography>
                      </Box>
                      <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', mt: 2 }}>
                        <Typography variant="h6" color="primary">
                          ${provider.pricePerHour}/小时
                        </Typography>
                      </Box>
                    </CardContent>
                    <CardActions sx={{ p: 2, pt: 0 }}>
                      <Button
                        fullWidth
                        variant="contained"
                        disabled={provider.status === 'offline'}
                        onClick={() => alert(`租用 ${provider.name} - 需要连接钱包`)}
                      >
                        {isConnected ? '租用算力' : '连接钱包租用'}
                      </Button>
                    </CardActions>
                  </Card>
                </Grid>
              ))}
            </Grid>

            {/* 提供算力服务区域 */}
            <Box sx={{ mt: 4, p: 3, border: 1, borderColor: 'divider', borderRadius: 2 }}>
              <Box sx={{ display: 'flex', alignItems: 'center', gap: 2, mb: 2 }}>
                <Security color="primary" />
                <Typography variant="h6">成为算力提供商</Typography>
              </Box>
              <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
                共享您的GPU资源，通过智能合约自动结算收益。提供您的模型给其他用户使用。
              </Typography>
              <Box sx={{ display: 'flex', gap: 2 }}>
                <Button
                  variant="outlined"
                  disabled={!isConnected}
                  onClick={() => alert('注册成为算力提供商 - 需要连接钱包')}
                >
                  注册算力节点
                </Button>
                <Button
                  variant="outlined"
                  disabled={!isConnected}
                  onClick={() => alert('发布模型服务 - 需要连接钱包')}
                >
                  发布我的模型
                </Button>
              </Box>
              {!isConnected && (
                <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mt: 1 }}>
                  连接钱包后可开始分享您的算力和模型
                </Typography>
              )}
            </Box>
          </>
        )}
      </Box>
    </Box>
  );
};