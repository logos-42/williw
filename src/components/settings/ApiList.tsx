import React from 'react';
import {
  Box,
  Button,
  Typography,
  Chip,
} from '@mui/material';
import { ExternalApiConfig, type ProviderType } from './ExternalApiForm';

interface ApiListProps {
  apis: ExternalApiConfig[];
  editingId: string | null;
  onEdit: (api: ExternalApiConfig) => void;
  onToggle: (id: string, enabled: boolean) => void;
  onDelete: (id: string) => void;
}

export const ApiList: React.FC<ApiListProps> = ({
  apis,
  editingId,
  onEdit,
  onToggle,
  onDelete,
}) => {
  const getChipColor = (provider: ProviderType): 'primary' | 'success' | 'secondary' | 'warning' | 'info' | 'default' => {
    switch (provider) {
      case 'deepseek':
        return 'primary';
      case 'openai':
        return 'success';
      case 'anthropic':
        return 'secondary';
      case 'google':
        return 'info';
      case 'nvidia':
        return 'warning';
      default:
        return 'default';
    }
  };

  if (apis.length === 0) {
    return (
      <Typography variant="body2" color="text.secondary">
        暂无配置，填写上方表单添加新配置
      </Typography>
    );
  }

  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', gap: 1 }}>
      {apis.map((api) => (
        <Box
          key={api.id}
          sx={{
            p: 2,
            backgroundColor: 'rgba(255, 255, 255, 0.03)',
            borderRadius: 1,
            display: 'flex',
            justifyContent: 'space-between',
            alignItems: 'center',
            opacity: api.enabled ? 1 : 0.6,
            border: editingId === api.id ? '1px solid #1976d2' : '1px solid transparent',
          }}
        >
          <Box sx={{ flex: 1 }}>
            <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 0.5 }}>
              <Typography variant="body2" sx={{ fontWeight: 500 }}>
                {api.provider} - {api.model}
              </Typography>
              <Chip
                label={api.provider}
                size="small"
                color={getChipColor(api.provider)}
              />
              {api.enabled ? (
                <Chip label="已启用" size="small" color="success" />
              ) : (
                <Chip label="已禁用" size="small" />
              )}
            </Box>
            <Typography variant="caption" color="text.secondary">
              模型: {api.model} | {api.base_url}
            </Typography>
          </Box>
          <Box sx={{ display: 'flex', gap: 0.5 }}>
            <Button
              size="small"
              variant="text"
              onClick={() => onEdit(api)}
            >
              编辑
            </Button>
            <Button
              size="small"
              variant="text"
              color={api.enabled ? 'warning' : 'success'}
              onClick={() => onToggle(api.id, !api.enabled)}
            >
              {api.enabled ? '禁用' : '启用'}
            </Button>
            <Button
              size="small"
              variant="text"
              color="error"
              onClick={() => onDelete(api.id)}
            >
              删除
            </Button>
          </Box>
        </Box>
      ))}
    </Box>
  );
};
