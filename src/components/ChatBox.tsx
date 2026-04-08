import React, { useState, useRef, useEffect } from 'react';
import {
  Box,
  Button,
  Card,
  CardContent,
  Chip,
  TextField,
  IconButton,
  Typography,
  List,
  ListItem,
  ListItemText,
  useTheme,
  alpha,
  CircularProgress,
  LinearProgress,
} from '@mui/material';
import SendIcon from '@mui/icons-material/Send';
import SmartToyIcon from '@mui/icons-material/SmartToy';
import PersonIcon from '@mui/icons-material/Person';
import AutoFixHighIcon from '@mui/icons-material/AutoFixHigh';
import CheckCircleIcon from '@mui/icons-material/CheckCircle';
import ErrorIcon from '@mui/icons-material/Error';
import InfoIcon from '@mui/icons-material/Info';
import SettingsIcon from '@mui/icons-material/Settings';
import HubIcon from '@mui/icons-material/Hub';
import AccountTreeIcon from '@mui/icons-material/AccountTree';
import CloseIcon from '@mui/icons-material/Close';
import { useModelStore } from '../store/modelStore';
import { useWorkflowStore } from '../store/workflowStore';
import { useTrainingStore } from '../store/trainingStore';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';

interface ChatMessage {
  id: string;
  content: string;
  sender: 'user' | 'assistant' | 'workflow';
  timestamp: Date;
  workflowType?: 'info' | 'progress' | 'success' | 'error' | 'warning';
  workflowProgress?: number;
}

interface ChatBoxProps {
  expanded?: boolean;
  onExpand?: () => void;
}

export const ChatBox: React.FC<ChatBoxProps> = ({ expanded = false, onExpand }) => {
  const theme = useTheme();
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [inputText, setInputText] = useState('');
  const [isAiThinking, setIsAiThinking] = useState(false);
  const [hasApiConfig, setHasApiConfig] = useState<boolean | null>(null);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const { addMessage } = useWorkflowStore();
  const { openSettings } = useTrainingStore();
  const { activeSession, clearActiveSession } = useModelStore();

  const scrollToBottom = () => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  };

  useEffect(() => {
    scrollToBottom();
  }, [messages]);

  // 检查是否有配置的外部 API
  useEffect(() => {
    const checkApiConfig = async () => {
      try {
        const apis = await invoke<any[]>('get_external_apis');
        const enabled = apis.filter((api: any) => api.enabled && api.api_key);
        setHasApiConfig(enabled.length > 0);
      } catch {
        setHasApiConfig(false);
      }
    };
    checkApiConfig();
    // 每 5 秒检查一次（用户可能在设置中配置了API）
    const interval = setInterval(checkApiConfig, 5000);
    return () => clearInterval(interval);
  }, []);

  // 监听工作流消息事件
  useEffect(() => {
    let unlistenFn: any = null;

    const setupWorkflowListeners = async () => {
      try {
        unlistenFn = await listen('workflow-message', (event: any) => {
          const { type, content, progress } = event.payload as {
            type: string;
            content: string;
            progress?: number;
          };

          const workflowMessage: ChatMessage = {
            id: `workflow-${Date.now()}-${Math.random()}`,
            content,
            sender: 'workflow',
            timestamp: new Date(),
            workflowType: type as any,
            workflowProgress: progress,
          };

          setMessages(prev => [...prev, workflowMessage]);
          addMessage({
            type: type as any,
            content,
            timestamp: new Date(),
          });
        });
      } catch (error) {
        console.error('❌ [ChatBox] Failed to setup workflow listeners:', error);
      }
    };

    setupWorkflowListeners();

    return () => {
      if (unlistenFn) {
        unlistenFn();
      }
    };
  }, [addMessage]);

  const handleSendMessage = async () => {
    if (!inputText.trim() || isAiThinking) return;

    const userMessage: ChatMessage = {
      id: Date.now().toString(),
      content: inputText.trim(),
      sender: 'user',
      timestamp: new Date(),
    };

    setMessages(prev => [...prev, userMessage]);
    const currentInput = inputText.trim();
    setInputText('');

    if (messages.filter(m => m.sender === 'user').length === 0) {
      onExpand?.();
    }

    setIsAiThinking(true);

    const thinkingMessage: ChatMessage = {
      id: `thinking-${Date.now()}`,
      content: activeSession
        ? `${activeSession.modelName} 推理中...`
        : '正在思考...',
      sender: 'assistant',
      timestamp: new Date(),
    };
    setMessages(prev => [...prev, thinkingMessage]);

    try {
      let result: { success: boolean; message: string };

      if (activeSession?.inferenceEndpoint) {
        // AI agent configured a local endpoint — route directly there
        result = await invoke<{ success: boolean; message: string }>('chat_with_local_endpoint', {
          message: currentInput,
          endpoint: activeSession.inferenceEndpoint,
          modelName: activeSession.localModelName ?? activeSession.modelName,
          systemPrompt: `你是 ${activeSession.modelName}，一个运行在本机的大语言模型。请用中文回复。`,
        });
      } else if (activeSession) {
        // No local endpoint yet — fall back to external API with model context in system prompt
        result = await invoke<{ success: boolean; message: string }>('chat_with_distributed_model', {
          message: currentInput,
          modelName: activeSession.modelName,
          modelRepo: activeSession.modelRepo,
          params: activeSession.params,
          totalLayers: activeSession.totalLayers,
          isLocalOnly: activeSession.isLocalOnly,
          nodeCount: activeSession.splitPlan.length,
        });
      } else {
        // No active session: use plain external API
        result = await invoke<{ success: boolean; message: string }>('chat_with_external_api', {
          message: currentInput,
        });
      }

      setMessages(prev => {
        const filtered = prev.filter(msg => msg.id !== thinkingMessage.id);
        return [...filtered, {
          id: `ai-${Date.now()}`,
          content: result.success
            ? result.message
            : '抱歉，AI 无法回复。请在设置中配置外部 API（如 OpenAI、DeepSeek）。',
          sender: 'assistant',
          timestamp: new Date(),
        }];
      });

    } catch (error: any) {
      console.error('AI 推理失败:', error);
      const errStr = String(error?.message ?? error ?? '');

      // 根据错误类型和当前模式给出不同的提示
      let errorContent: string;

      if (activeSession?.inferenceEndpoint) {
        // 本地推理模式下的错误
        if (errStr.includes('502') || errStr.includes('Bad Gateway')) {
          errorContent =
            '⚠️ 本地模型加载失败（502 Bad Gateway）\n\n可能原因：\n' +
            '• 推理服务正在加载模型（首次需要 5-10 秒）\n\n' +
            '请等待片刻后再次发送消息。';
        } else if (errStr.includes('Connection refused') || errStr.includes('请求失败')) {
          errorContent =
            '⚠️ 无法连接到本地推理服务\n\n' +
            '推理服务可能已停止运行。请重新启动。';
        } else {
          errorContent = `本地推理出错：${errStr}\n\n端点: ${activeSession.inferenceEndpoint}\n模型: ${activeSession.localModelName ?? activeSession.modelName}`;
        }
      } else if (activeSession) {
        // 分布式/外部API模式
        errorContent = `推理失败：${errStr}`;
      } else if (!hasApiConfig) {
        errorContent = '请先在右上角「设置」中配置 AI API（OpenAI、DeepSeek 等），然后才能开始对话。';
      } else {
        errorContent = `请求失败：${errStr}。请检查 API 配置是否正确。`;
      }

      setMessages(prev => {
        const filtered = prev.filter(msg => msg.id !== thinkingMessage.id);
        return [...filtered, {
          id: `error-${Date.now()}`,
          content: errorContent,
          sender: 'assistant',
          timestamp: new Date(),
        }];
      });
    } finally {
      setIsAiThinking(false);
    }
  };

  const handleKeyPress = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSendMessage();
    }
  };

  return (
    <Box
      sx={{
        width: '100%',
        height: '100%',
        display: 'flex',
        flexDirection: 'column',
      }}
    >
      <Card
        sx={{
          background: alpha(theme.palette.background.paper, 0.9),
          backdropFilter: 'blur(10px)',
          border: `1px solid ${theme.palette.divider}`,
          borderRadius: 1,
          flex: 1,
          display: 'flex',
          flexDirection: 'column',
          transition: 'all 0.3s ease',
          minHeight: expanded ? 500 : 300,
        }}
      >
        <CardContent sx={{ p: 1.5, flex: 1, display: 'flex', flexDirection: 'column', minHeight: 0 }}>
          {/* 活跃推理会话横幅：显示当前激活的模型 */}
          {activeSession && (
            <Box
              sx={{
                mb: 1,
                px: 1.5,
                py: 0.75,
                borderRadius: 1,
                background: alpha(theme.palette.primary.main, 0.08),
                border: `1px solid ${alpha(theme.palette.primary.main, 0.25)}`,
                display: 'flex',
                alignItems: 'center',
                gap: 1,
              }}
            >
              {activeSession.isLocalOnly
                ? <SmartToyIcon sx={{ fontSize: 14, color: 'primary.main', flexShrink: 0 }} />
                : <AccountTreeIcon sx={{ fontSize: 14, color: 'primary.main', flexShrink: 0 }} />
              }
              <Box sx={{ flex: 1, minWidth: 0 }}>
                <Typography variant="caption" sx={{ color: 'primary.main', fontWeight: 600 }}>
                  {activeSession.modelName}
                </Typography>
                <Typography variant="caption" sx={{ color: 'text.secondary', ml: 1 }}>
                  {activeSession.isLocalOnly
                    ? '本机运行'
                    : `分布式 · ${activeSession.splitPlan.length} 节点`
                  }
                </Typography>
                {!activeSession.isLocalOnly && (
                  <Box sx={{ display: 'flex', gap: 0.5, mt: 0.3, flexWrap: 'wrap' }}>
                    {activeSession.splitPlan.map((n) => (
                      <Chip
                        key={n.fullId}
                        size="small"
                        icon={<HubIcon style={{ fontSize: 10 }} />}
                        label={`${n.shortId} L${n.layers[0]}-${n.layers[1]}`}
                        sx={{ height: 16, fontSize: '0.6rem', '& .MuiChip-label': { px: 0.5 } }}
                        color={n.isLocal ? 'primary' : 'default'}
                        variant="outlined"
                      />
                    ))}
                  </Box>
                )}
              </Box>
              <IconButton
                size="small"
                onClick={clearActiveSession}
                sx={{ p: 0.3, color: 'text.disabled', '&:hover': { color: 'text.secondary' } }}
              >
                <CloseIcon sx={{ fontSize: 14 }} />
              </IconButton>
            </Box>
          )}

          {/* 未配置 API 的提示横幅 */}
          {hasApiConfig === false && (
            <Box
              sx={{
                mb: 1.5,
                p: 1.5,
                borderRadius: 1,
                background: alpha(theme.palette.warning.main, 0.1),
                border: `1px solid ${alpha(theme.palette.warning.main, 0.3)}`,
                display: 'flex',
                alignItems: 'center',
                gap: 1,
              }}
            >
              <InfoIcon sx={{ fontSize: 16, color: 'warning.main', flexShrink: 0 }} />
              <Typography variant="caption" sx={{ flex: 1, color: 'warning.light' }}>
                请先配置 AI API 才能对话
              </Typography>
              <Button
                size="small"
                variant="outlined"
                color="warning"
                startIcon={<SettingsIcon sx={{ fontSize: 14 }} />}
                onClick={openSettings}
                sx={{ fontSize: '0.7rem', py: 0.3, px: 1, borderColor: 'warning.main', color: 'warning.main' }}
              >
                去配置
              </Button>
            </Box>
          )}

          <Box sx={{ flex: 1, overflow: 'auto', mb: 1.5, minHeight: 0 }}>
            {messages.length === 0 ? (
              <Box sx={{
                display: 'flex',
                flexDirection: 'column',
                alignItems: 'center',
                justifyContent: 'center',
                height: '100%',
                gap: 1,
                color: 'text.secondary',
              }}>
                <SmartToyIcon sx={{ fontSize: 40, opacity: 0.3 }} />
                <Typography variant="body2" sx={{ opacity: 0.6 }}>
                  {activeSession
                    ? `${activeSession.modelName} 已就绪，开始对话...`
                    : hasApiConfig
                    ? '开始对话...'
                    : '配置 API 后即可开始对话'}
                </Typography>
              </Box>
            ) : (
              <List sx={{ p: 0 }}>
                {messages.map((message) => (
                  <ListItem
                    key={message.id}
                    sx={{
                      py: 0.5,
                      px: 0,
                      alignItems: 'flex-start',
                      flexDirection: message.sender === 'user' ? 'row-reverse' : 'row',
                    }}
                  >
                    <Box
                      sx={{
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        width: 24,
                        height: 24,
                        borderRadius: '50%',
                        mr: message.sender === 'user' ? 0 : 1,
                        ml: message.sender === 'user' ? 1 : 0,
                        background: message.sender === 'user'
                          ? alpha(theme.palette.primary.main, 0.2)
                          : message.sender === 'workflow'
                          ? alpha(theme.palette.info.main, 0.2)
                          : alpha(theme.palette.secondary.main, 0.2),
                        color: message.sender === 'user'
                          ? theme.palette.primary.main
                          : message.sender === 'workflow'
                          ? theme.palette.info.main
                          : theme.palette.secondary.main,
                        flexShrink: 0,
                      }}
                    >
                      {message.sender === 'user' ? (
                        <PersonIcon sx={{ fontSize: 16 }} />
                      ) : message.sender === 'workflow' ? (
                        <AutoFixHighIcon sx={{ fontSize: 16 }} />
                      ) : (
                        <SmartToyIcon sx={{ fontSize: 16 }} />
                      )}
                    </Box>

                    <ListItemText
                      primary={
                        <Box
                          sx={{
                            p: 1,
                            borderRadius: 1,
                            background: message.sender === 'user'
                              ? alpha(theme.palette.primary.main, 0.1)
                              : message.sender === 'workflow'
                              ? alpha(
                                  message.workflowType === 'error'
                                    ? theme.palette.error.main
                                    : message.workflowType === 'success'
                                    ? theme.palette.success.main
                                    : theme.palette.info.main,
                                  0.1
                                )
                              : alpha(theme.palette.secondary.main, 0.1),
                            border: `1px solid ${alpha(theme.palette.divider, 0.3)}`,
                            maxWidth: '80%',
                            display: 'inline-block',
                          }}
                        >
                          {message.sender === 'workflow' && (
                            <>
                              {message.workflowType === 'success' && (
                                <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.5, mb: 0.5 }}>
                                  <CheckCircleIcon sx={{ fontSize: 14, color: 'success.main' }} />
                                  <Typography variant="caption" color="success.main">完成</Typography>
                                </Box>
                              )}
                              {message.workflowType === 'error' && (
                                <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.5, mb: 0.5 }}>
                                  <ErrorIcon sx={{ fontSize: 14, color: 'error.main' }} />
                                  <Typography variant="caption" color="error.main">错误</Typography>
                                </Box>
                              )}
                              {message.workflowType === 'progress' && (
                                <>
                                  <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.5, mb: 0.5 }}>
                                    <AutoFixHighIcon sx={{ fontSize: 14, color: 'info.main' }} />
                                    <Typography variant="caption" color="info.main">AI 工作流</Typography>
                                  </Box>
                                  {message.workflowProgress !== undefined && (
                                    <LinearProgress
                                      variant="determinate"
                                      value={message.workflowProgress * 100}
                                      sx={{ height: 3, borderRadius: 2, mt: 0.5, background: alpha(theme.palette.info.main, 0.1) }}
                                    />
                                  )}
                                </>
                              )}
                              {message.workflowType === 'info' && (
                                <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.5, mb: 0.5 }}>
                                  <InfoIcon sx={{ fontSize: 14, color: 'info.main' }} />
                                  <Typography variant="caption" color="info.main">信息</Typography>
                                </Box>
                              )}
                            </>
                          )}
                          <Typography
                            variant="body2"
                            sx={{ fontSize: '0.875rem', whiteSpace: 'pre-wrap', wordBreak: 'break-word' }}
                          >
                            {message.content}
                          </Typography>
                        </Box>
                      }
                      secondary={
                        <Typography
                          variant="caption"
                          sx={{
                            display: 'block',
                            mt: 0.3,
                            textAlign: message.sender === 'user' ? 'right' : 'left',
                            color: 'text.secondary',
                          }}
                        >
                          {message.timestamp.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
                        </Typography>
                      }
                      sx={{ mx: 0 }}
                    />
                  </ListItem>
                ))}
                <div ref={messagesEndRef} />
              </List>
            )}
          </Box>

          {/* 输入框 */}
          <Box sx={{ display: 'flex', gap: 1 }}>
            <TextField
              fullWidth
              size="small"
              placeholder={
                isAiThinking
                  ? (activeSession ? `${activeSession.modelName} 推理中...` : '正在思考...')
                  : hasApiConfig === false
                  ? '请先配置 AI API...'
                  : activeSession
                  ? `向 ${activeSession.modelName} 发送消息...`
                  : '输入消息（Enter 发送）'
              }
              value={inputText}
              onChange={(e) => setInputText(e.target.value)}
              onKeyPress={handleKeyPress}
              multiline
              maxRows={3}
              disabled={isAiThinking}
              sx={{
                '& .MuiOutlinedInput-root': {
                  fontSize: '0.875rem',
                  fieldset: { borderColor: alpha(theme.palette.divider, 0.5) },
                },
              }}
            />
            <IconButton
              size="small"
              onClick={handleSendMessage}
              disabled={!inputText.trim() || isAiThinking}
              sx={{
                background: (inputText.trim() && !isAiThinking)
                  ? alpha(theme.palette.primary.main, 0.2)
                  : 'transparent',
                color: (inputText.trim() && !isAiThinking)
                  ? theme.palette.primary.main
                  : theme.palette.text.disabled,
                alignSelf: 'flex-end',
                mb: 0.5,
              }}
            >
              {isAiThinking ? <CircularProgress size={18} /> : <SendIcon sx={{ fontSize: 18 }} />}
            </IconButton>
          </Box>
        </CardContent>
      </Card>
    </Box>
  );
};
