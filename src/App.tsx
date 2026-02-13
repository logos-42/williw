import React from 'react';
import { ThemeProvider, createTheme } from '@mui/material/styles';
import CssBaseline from '@mui/material/CssBaseline';
import { AppLayout } from './components/AppLayout';

console.log('=== App.tsx loading ===')

const darkTheme = createTheme({
  palette: {
    mode: 'dark',
    primary: {
      main: '#ffffff',
      contrastText: '#000000',
    },
    secondary: {
      main: '#888888',
    },
    background: {
      default: '#000000',
      paper: '#0a0a0a',
    },
    text: {
      primary: '#ffffff',
      secondary: '#aaaaaa',
    },
    divider: '#333333',
  },
  components: {
    MuiButton: {
      styleOverrides: {
        root: {
          borderColor: '#ffffff',
          '&.Mui-contained': {
            backgroundColor: '#ffffff',
            color: '#000000',
            '&:hover': {
              backgroundColor: '#e0e0e0',
            },
          },
          '&.Mui-outlined': {
            borderColor: '#ffffff',
            color: '#ffffff',
            '&:hover': {
              borderColor: '#ffffff',
              backgroundColor: 'rgba(255, 255, 255, 0.1)',
            },
          },
        },
      },
    },
    MuiChip: {
      styleOverrides: {
        root: {
          borderColor: '#444444',
        },
      },
    },
    MuiTextField: {
      styleOverrides: {
        root: {
          '& .MuiOutlinedInput-root': {
            '& fieldset': {
              borderColor: '#333333',
            },
            '&:hover fieldset': {
              borderColor: '#555555',
            },
          },
        },
      },
    },
  },
});

// 错误边界组件
class ErrorBoundary extends React.Component<{children: React.ReactNode}, {hasError: boolean, error: string}> {
  constructor(props: {children: React.ReactNode}) {
    super(props);
    this.state = { hasError: false, error: '' };
  }
  
  static getDerivedStateFromError(error: Error) {
    console.error('=== ErrorBoundary caught error ===', error);
    return { hasError: true, error: error.message };
  }
  
  componentDidCatch(error: Error, errorInfo: React.ErrorInfo) {
    console.error('=== ErrorBoundary details ===', error, errorInfo);
  }
  
  render() {
    if (this.state.hasError) {
      return (
        <div style={{color: 'red', padding: '20px'}}>
          <h1>Rendering Error:</h1>
          <pre>{this.state.error}</pre>
        </div>
      );
    }
    return this.props.children;
  }
}

function App() {
  console.log('=== App rendering ===')
  return (
    <ErrorBoundary>
      <ThemeProvider theme={darkTheme}>
        <CssBaseline />
        <AppLayout />
      </ThemeProvider>
    </ErrorBoundary>
  );
}

export default App;
