import React, { useState } from 'react';
import {
  Box,
  Button,
  Card,
  CardContent,
  Typography,
  Grid,
  Chip,
  Alert,
  AlertTitle,
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  TextField,
  useTheme,
  alpha,
  CircularProgress,
  IconButton,
  Tooltip,
} from '@mui/material';
import RefreshIcon from '@mui/icons-material/Refresh';
import PlayArrowIcon from '@mui/icons-material/PlayArrow';
import StopIcon from '@mui/icons-material/Stop';
import CheckCircleIcon from '@mui/icons-material/CheckCircle';
import SettingsIcon from '@mui/icons-material/Settings';
import NetworkCheckIcon from '@mui/icons-material/NetworkCheck';
import AutoFixHighIcon from '@mui/icons-material/AutoFixHigh';
import TerminalIcon from '@mui/icons-material/Terminal';
import { useAutonomousCommand } from '../hooks/useAutonomousCommand';

export const AutonomousCommandPanel: React.FC = () => {
  const theme = useTheme();
  const [dialogOpen, setDialogOpen] = useState(false);
  const [customCommand, setCustomCommand] = useState('');
  const [customDescription, setCustomDescription] = useState('');

  const {
    result,
    isLoading,
    error,
    startOllama,
    stopOllama,
    checkService,
    diagnoseNetwork,
    runSelfHealing,
    executeCommand,
    clearResult,
  } = useAutonomousCommand();

  const handleQuickAction = (action: string) => {
    switch (action) {
      case 'start-ollama':
        startOllama();
        break;
      case 'stop-ollama':
        stopOllama();
        break;
      case 'check-ollama':
        checkService('ollama');
        break;
      case 'self-heal':
        runSelfHealing();
        break;
      case 'network-check':
        diagnoseNetwork('8.8.8.8');
        break;
    }
  };

  const handleCustomCommand = () => {
    if (customCommand.trim()) {
      executeCommand({
        type: 'Custom',
        command: customCommand.trim(),
        description: customDescription.trim() || '自定义命令',
      });
      setDialogOpen(false);
      setCustomCommand('');
      setCustomDescription('');
    }
  };

  return (
    <Box sx={{ p: 2 }}>
      <Typography variant="h6" sx={{ mb: 2, fontWeight: 600 }}>
        <TerminalIcon sx={{ mr: 1, verticalAlign: 'middle' }} />
        自主命令面板
      </Typography>

      {/* 快捷操作按钮 */}
      <Grid container spacing={1.5} sx={{ mb: 2 }}>
        <Grid size={{ xs: 6, sm: 'auto' }}>
          <Tooltip title="启动 Ollama 服务">
            <Button
              variant="contained"
              startIcon={isLoading ? <CircularProgress size={16} /> : <PlayArrowIcon />}
              onClick={() => handleQuickAction('start-ollama')}
              disabled={isLoading}
              fullWidth
              sx={{ minWidth: 120 }}
            >
              启动 Ollama
            </Button>
          </Tooltip>
        </Grid>

        <Grid size={{ xs: 6, sm: 'auto' }}>
          <Tooltip title="停止 Ollama 服务">
            <Button
              variant="outlined"
              startIcon={<StopIcon />}
              onClick={() => handleQuickAction('stop-ollama')}
              disabled={isLoading}
              color="warning"
              fullWidth
              sx={{ minWidth: 120 }}
            >
              停止 Ollama
            </Button>
          </Tooltip>
        </Grid>

        <Grid size={{ xs: 12, sm: 'auto' }}>
          <Tooltip title="检查 Ollama 状态">
            <Button
              variant="outlined"
              startIcon={<CheckCircleIcon />}
              onClick={() => handleQuickAction('check-ollama')}
              disabled={isLoading}
              fullWidth
              sx={{ minWidth: 120 }}
            >
              检查状态
            </Button>
          </Tooltip>
        </Grid>

        <Grid size={{ xs: 12, sm: 'auto' }}>
          <Tooltip title="执行自愈流程">
            <Button
              variant="contained"
              startIcon={<AutoFixHighIcon />}
              onClick={() => handleQuickAction('self-heal')}
              disabled={isLoading}
              color="info"
              fullWidth
              sx={{ minWidth: 120 }}
            >
              自愈
            </Button>
          </Tooltip>
        </Grid>

        <Grid size={{ xs: 12, sm: 'auto' }}>
          <Tooltip title="网络诊断">
            <Button
              variant="outlined"
              startIcon={<NetworkCheckIcon />}
              onClick={() => handleQuickAction('network-check')}
              disabled={isLoading}
              fullWidth
              sx={{ minWidth: 120 }}
            >
              网络诊断
            </Button>
          </Tooltip>
        </Grid>

        <Grid size={{ xs: 12, sm: 'auto' }}>
          <Tooltip title="执行自定义命令">
            <Button
              variant="outlined"
              startIcon={<SettingsIcon />}
              onClick={() => setDialogOpen(true)}
              disabled={isLoading}
              fullWidth
              sx={{ minWidth: 120 }}
            >
              自定义
            </Button>
          </Tooltip>
        </Grid>
      </Grid>

      {/* 结果显示 */}
      {(result || error) && (
        <Card
          sx={{
            background: alpha(theme.palette.background.paper, 0.9),
            backdropFilter: 'blur(10px)',
            border: `1px solid ${theme.palette.divider}`,
          }}
        >
          <CardContent>
            <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', mb: 1 }}>
              <Typography variant="subtitle2" sx={{ fontWeight: 600 }}>
                执行结果
              </Typography>
              <IconButton size="small" onClick={clearResult}>
                <RefreshIcon sx={{ fontSize: 16 }} />
              </IconButton>
            </Box>

            {error ? (
              <Alert severity="error" sx={{ mb: 2 }}>
                <AlertTitle>错误</AlertTitle>
                {error}
              </Alert>
            ) : result ? (
              <>
                <Alert
                  severity={result.success ? 'success' : 'warning'}
                  sx={{ mb: 2 }}
                >
                  <AlertTitle>
                    {result.success ? '成功' : '警告'}
                  </AlertTitle>
                  {result.message}
                </Alert>

                {result.stdout && (
                  <Box
                    sx={{
                      p: 1.5,
                      borderRadius: 1,
                      background: alpha(theme.palette.info.main, 0.05),
                      border: `1px solid ${alpha(theme.palette.info.main, 0.2)}`,
                      mb: 1,
                    }}
                  >
                    <Typography
                      variant="caption"
                      sx={{
                        fontFamily: 'monospace',
                        fontSize: '0.75rem',
                        whiteSpace: 'pre-wrap',
                        wordBreak: 'break-all',
                      }}
                    >
                      {result.stdout}
                    </Typography>
                  </Box>
                )}

                {result.stderr && (
                  <Box
                    sx={{
                      p: 1.5,
                      borderRadius: 1,
                      background: alpha(theme.palette.warning.main, 0.05),
                      border: `1px solid ${alpha(theme.palette.warning.main, 0.2)}`,
                    }}
                  >
                    <Typography
                      variant="caption"
                      sx={{
                        fontFamily: 'monospace',
                        fontSize: '0.75rem',
                        whiteSpace: 'pre-wrap',
                        wordBreak: 'break-all',
                        color: theme.palette.warning.light,
                      }}
                    >
                      {result.stderr}
                    </Typography>
                  </Box>
                )}

                {result.exit_code !== null && result.exit_code !== undefined && (
                  <Chip
                    label={`退出码：${result.exit_code}`}
                    size="small"
                    color={result.exit_code === 0 ? 'success' : 'warning'}
                    sx={{ mt: 1 }}
                  />
                )}
              </>
            ) : null}
          </CardContent>
        </Card>
      )}

      {/* 自定义命令对话框 */}
      <Dialog
        open={dialogOpen}
        onClose={() => setDialogOpen(false)}
        maxWidth="sm"
        fullWidth
      >
        <DialogTitle>执行自定义命令</DialogTitle>
        <DialogContent>
          <Alert severity="warning" sx={{ mb: 2 }}>
            <AlertTitle>警告</AlertTitle>
            自定义命令将直接在系统上执行，请确保您了解命令的作用。危险操作可能导致系统损坏。
          </Alert>

          <TextField
            fullWidth
            label="命令描述"
            placeholder="例如：检查磁盘空间"
            value={customDescription}
            onChange={(e) => setCustomDescription(e.target.value)}
            sx={{ mb: 2 }}
          />

          <TextField
            fullWidth
            label="Shell 命令"
            placeholder="例如：df -h"
            value={customCommand}
            onChange={(e) => setCustomCommand(e.target.value)}
            multiline
            rows={3}
            sx={{ fontFamily: 'monospace' }}
          />
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setDialogOpen(false)}>取消</Button>
          <Button
            onClick={handleCustomCommand}
            variant="contained"
            disabled={!customCommand.trim() || isLoading}
          >
            执行
          </Button>
        </DialogActions>
      </Dialog>
    </Box>
  );
};
