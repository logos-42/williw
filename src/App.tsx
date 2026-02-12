import React from 'react';
import { ThemeProvider, createTheme } from '@mui/material/styles';
import CssBaseline from '@mui/material/CssBaseline';
import { AppLayout } from './components/AppLayout';

console.log('=== App.tsx loading ===')

const darkTheme = createTheme({
  palette: {
    mode: 'dark',
    primary: {
      main: '#2196f3',
    },
    secondary: {
      main: '#ff4081',
    },
    background: {
      default: '#000000',
      paper: '#121212',
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
