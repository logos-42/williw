import React from 'react';
import {
  Box,
  IconButton,
  Tooltip,
  useTheme,
  alpha,
} from '@mui/material';
import SettingsIcon from '@mui/icons-material/Settings';

interface SettingsButtonProps {
  onClick: () => void;
}

export const SettingsButton: React.FC<SettingsButtonProps> = ({ onClick }) => {
  const theme = useTheme();

  return (
    <Box
      sx={{
        position: 'absolute',
        top: 80,
        left: 16,
        zIndex: 10,
      }}
    >
      <Tooltip title="设置">
        <IconButton
          onClick={onClick}
          sx={{
            width: 48,
            height: 48,
            background: alpha(theme.palette.background.paper, 0.9),
            backdropFilter: 'blur(10px)',
            border: `1px solid ${theme.palette.divider}`,
            borderRadius: 2,
            '&:hover': {
              background: alpha(theme.palette.background.paper, 1),
              transform: 'scale(1.05)',
            },
            transition: 'all 0.2s ease',
          }}
        >
          <SettingsIcon sx={{ fontSize: 24 }} />
        </IconButton>
      </Tooltip>
    </Box>
  );
};
