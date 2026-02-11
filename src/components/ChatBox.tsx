import React, { useState, useRef, useEffect } from 'react';
import {
  Box,
  Button,
  Card,
  CardContent,
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
import BugReportIcon from '@mui/icons-material/BugReport';
import PlayArrowIcon from '@mui/icons-material/PlayArrow';
import StopIcon from '@mui/icons-material/Stop';
import { useModelStore } from '../store/modelStore';
import { useWorkflowStore } from '../store/workflowStore';
import { runInference, InferenceRequest } from '../services/inferenceService';
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
  const [isWorkflowRunning, setIsWorkflowRunning] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const { inferenceResult, isInferenceLoading } = useModelStore();
  const { status, addMessage } = useWorkflowStore();

  const scrollToBottom = () => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  };

  useEffect(() => {
    scrollToBottom();
  }, [messages]);

  // 监听推理结果，自动添加到聊天框（仅用于ModelSelector的初始推理）
  useEffect(() => {
    if (inferenceResult && !isInferenceLoading && messages.length === 0) {
      const assistantMessage: ChatMessage = {
        id: `inference-${Date.now()}`,
        content: `模型已准备就绪！\n\n请求ID: ${inferenceResult.request_id || 'N/A'}\n分配节点数: ${inferenceResult.selected_nodes?.length || 0}\n预计总时间: ${inferenceResult.estimated_total_time || 0}ms\n\n${inferenceResult.result || '模型已加载，可以开始对话了！'}`,
        sender: 'assistant',
        timestamp: new Date(),
      };
      setMessages(prev => [...prev, assistantMessage]);
    }
  }, [inferenceResult, isInferenceLoading, messages.length]);

  // 监听推理开始，显示加载消息（仅用于ModelSelector的初始推理）
  useEffect(() => {
    if (isInferenceLoading && messages.length === 0) {
      const loadingMessage: ChatMessage = {
        id: `loading-${Date.now()}`,
        content: '正在加载AI模型，请稍候...',
        sender: 'assistant',
        timestamp: new Date(),
      };
      setMessages(prev => [...prev, loadingMessage]);
    }
  }, [isInferenceLoading, messages.length]);

  // 监听工作流消息事件
  useEffect(() => {
    console.log('🔧 [ChatBox] Setting up workflow listeners...');
    let unlistenFn: any = null;

    const setupWorkflowListeners = async () => {
      try {
        console.log('🔧 [ChatBox] Calling listen()...');
        // 监听工作流消息
        unlistenFn = await listen('workflow-message', (event: any) => {
          console.log('📨 [ChatBox] Received workflow-message event:', event);
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

          console.log('💬 [ChatBox] Adding workflow message:', workflowMessage);
          setMessages(prev => [...prev, workflowMessage]);
          addMessage({
            type: type as any,
            content,
            timestamp: new Date(),
          });
        });

        console.log('✅ [ChatBox] Workflow listeners registered successfully');
      } catch (error) {
        console.error('❌ [ChatBox] Failed to setup workflow listeners:', error);
      }
    };

    setupWorkflowListeners();

    return () => {
      console.log('🔧 [ChatBox] Cleaning up workflow listeners...');
      if (unlistenFn) {
        unlistenFn();
      }
    };
  }, [addMessage]);

  // 监听工作流完成状态
  useEffect(() => {
    if (status.isCompleted && messages.length > 0) {
      // 工作流完成后，自动添加欢迎消息
      setTimeout(() => {
        const welcomeMessage: ChatMessage = {
          id: `welcome-${Date.now()}`,
          content: '🎉 现在可以开始与AI对话了！请输入您的问题或任务。',
          sender: 'assistant',
          timestamp: new Date(),
        };
        setMessages(prev => [...prev, welcomeMessage]);
      }, 2000);
    }
  }, [status.isCompleted, messages.length]);

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

    // 只在首次发送消息时展开聊天框
    if (messages.length === 0) {
      onExpand?.();
    }

    // 显示AI思考状态
    setIsAiThinking(true);
    
    // 添加思考中的消息
    const thinkingMessage: ChatMessage = {
      id: `thinking-${Date.now()}`,
      content: 'AI正在思考...',
      sender: 'assistant',
      timestamp: new Date(),
    };
    setMessages(prev => [...prev, thinkingMessage]);

    try {
      // 首先尝试使用外部 API
      let result: { result?: string; error?: string } | null = null;
      let useExternalApi = false;
      
      try {
        const externalResult = await invoke<{ success: boolean; message: string }>('chat_with_external_api', {
          message: currentInput,
        });
        if (externalResult.success) {
          result = { result: externalResult.message };
          useExternalApi = true;
        }
      } catch (externalError) {
        console.log('外部 API 不可用，回退到本地 GPU:', externalError);
      }
      
      // 如果外部 API 失败或未配置，使用本地 GPU
      if (!useExternalApi) {
        const modelPath = 'D:\\AI\\去中心化训练\\test_models\\models--LiquidAI--LFM2.5-1.2B-Thinking\\snapshots\\1c9725ba97f047b37bcf53e44e9133ccf1f79333';
        
        const inferenceRequest: InferenceRequest = {
          model_path: modelPath,
          input_text: currentInput,
          max_length: 150
        };

        result = await runInference(inferenceRequest);
      }

      // 移除思考中的消息并添加AI回复
      setMessages(prev => {
        const filtered = prev.filter(msg => msg.id !== thinkingMessage.id);
        return [...filtered, {
          id: `ai-${Date.now()}`,
          content: result?.result || '抱歉，我无法生成回复。',
          sender: 'assistant',
          timestamp: new Date(),
        }];
      });

    } catch (error: any) {
      console.error('AI推理失败:', error);
      
      // 移除思考中的消息并添加错误消息
      setMessages(prev => {
        const filtered = prev.filter(msg => msg.id !== thinkingMessage.id);
        return [...filtered, {
          id: `error-${Date.now()}`,
          content: `抱歉，出现了错误：${error.message || '未知错误'}。请确保GPU服务器正在运行或已配置外部 API。`,
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
          <>
            <Box sx={{ flex: 1, overflow: 'auto', mb: 1.5, minHeight: 0 }}>
            {messages.length === 0 ? (
              <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%', color: 'text.secondary' }}>
                <Typography variant="body2">开始对话...</Typography>
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
                            maxWidth: '70%',
                          }}
                        >
                          {message.sender === 'workflow' && (
                            <>
                              {message.workflowType === 'success' && (
                                <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.5, mb: 0.5 }}>
                                  <CheckCircleIcon sx={{ fontSize: 16, color: 'success.main' }} />
                                  <Typography variant="caption" color="success.main">
                                    完成
                                  </Typography>
                                </Box>
                              )}
                              {message.workflowType === 'error' && (
                                <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.5, mb: 0.5 }}>
                                  <ErrorIcon sx={{ fontSize: 16, color: 'error.main' }} />
                                  <Typography variant="caption" color="error.main">
                                    错误
                                  </Typography>
                                </Box>
                              )}
                              {message.workflowType === 'progress' && (
                                <>
                                  <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.5, mb: 0.5 }}>
                                    <AutoFixHighIcon sx={{ fontSize: 16, color: 'info.main' }} />
                                    <Typography variant="caption" color="info.main">
                                      AI工作流
                                    </Typography>
                                  </Box>
                                  {message.workflowProgress !== undefined && (
                                    <Box sx={{ mt: 0.5 }}>
                                      <LinearProgress
                                        variant="determinate"
                                        value={message.workflowProgress * 100}
                                        sx={{
                                          height: 4,
                                          borderRadius: 2,
                                          background: alpha(theme.palette.info.main, 0.1),
                                        }}
                                      />
                                    </Box>
                                  )}
                                </>
                              )}
                              {message.workflowType === 'info' && (
                                <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.5, mb: 0.5 }}>
                                  <InfoIcon sx={{ fontSize: 16, color: 'info.main' }} />
                                  <Typography variant="caption" color="info.main">
                                    信息
                                  </Typography>
                                </Box>
                              )}
                            </>
                          )}
                          <Typography
                            variant="body2"
                            sx={{
                              fontSize: '0.875rem',
                              whiteSpace: 'pre-wrap',
                              wordBreak: 'break-word',
                            }}
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
                            mt: 0.5,
                            textAlign: message.sender === 'user' ? 'right' : 'left',
                            color: 'text.secondary',
                          }}
                        >
                          {message.timestamp.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
                        </Typography>
                      }
                      sx={{
                        mx: 0,
                        '& .MuiListItemText-primary': { mb: 0.5 },
                      }}
                    />
                  </ListItem>
                ))}
                <div ref={messagesEndRef} />
                </List>
              )}
            </Box>

            {/* 测试按钮区域 */}
            <Box sx={{ display: 'flex', gap: 1, mb: 1 }}>
              <Button
                size="small"
                variant="outlined"
                startIcon={<BugReportIcon />}
                onClick={async () => {
                  console.log('🧪 [Test] Manually adding test message...');
                  // 直接添加消息到对话框（不通过事件系统）
                  const testMessage: ChatMessage = {
                    id: `test-${Date.now()}`,
                    content: '🧪 本地测试消息：如果看到这个消息，说明ChatBox渲染正常',
                    sender: 'workflow',
                    timestamp: new Date(),
                    workflowType: 'info',
                  };
                  setMessages(prev => [...prev, testMessage]);
                  
                  // 尝试通过emit发送事件（用于测试Tauri事件系统）
                  try {
                    const { emit } = await import('@tauri-apps/api/event');
                    await emit('workflow-message', {
                      type: 'progress',
                      content: '🧪 通过emit发送的测试消息',
                      progress: 0.5,
                    });
                    console.log('✅ [Test] Event emitted successfully');
                  } catch (e) {
                    console.error('❌ [Test] Failed to emit event:', e);
                  }
                }}
              >
                测试消息
              </Button>
              <Button
                size="small"
                variant="outlined"
                onClick={async () => {
                  try {
                    const result = await invoke<string>('test_workflow_event');
                    console.log('🧪 [Test] Backend test result:', result);
                  } catch (e) {
                    console.error('❌ [Test] Backend test failed:', e);
                  }
                }}
              >
                测试后端事件
              </Button>
            </Box>

            <Box sx={{ display: 'flex', gap: 1 }}>
            <TextField
              fullWidth
              size="small"
              placeholder={isAiThinking ? "AI正在思考..." : "输入消息..."}
              value={inputText}
              onChange={(e) => setInputText(e.target.value)}
              onKeyPress={handleKeyPress}
              multiline
              maxRows={2}
              disabled={isAiThinking}
              sx={{
                '& .MuiOutlinedInput-root': {
                  fontSize: '0.875rem',
                  fieldset: {
                    borderColor: alpha(theme.palette.divider, 0.5),
                  },
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
              }}
            >
              {isAiThinking ? <CircularProgress size={20} /> : <SendIcon sx={{ fontSize: 20 }} />}
            </IconButton>
            </Box>
          </>
        </CardContent>
      </Card>
    </Box>
  );
};