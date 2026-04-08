import { useState, useCallback } from 'react';
import {
  executeAutonomousCommand,
  executeSelfHealing,
  AutonomousCommandResult,
  AutonomousCommandType,
} from '../utils/autonomousCommands';

interface UseAutonomousCommandReturn {
  result: AutonomousCommandResult | null;
  isLoading: boolean;
  error: string | null;
  executeCommand: (command: AutonomousCommandType, requireConfirmation?: boolean) => Promise<void>;
  checkService: (serviceName: string) => Promise<void>;
  diagnoseNetwork: (target: string) => Promise<void>;
  runSelfHealing: () => Promise<void>;
  clearResult: () => void;
}

/**
 * React Hook for executing autonomous commands
 */
export function useAutonomousCommand(): UseAutonomousCommandReturn {
  const [result, setResult] = useState<AutonomousCommandResult | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const executeCommand = useCallback(async (
    command: AutonomousCommandType,
    requireConfirmation: boolean = false
  ) => {
    setIsLoading(true);
    setError(null);

    try {
      const cmdResult = await executeAutonomousCommand(command, requireConfirmation);
      setResult(cmdResult);
    } catch (err: any) {
      setError(err.message || String(err));
      setResult(null);
    } finally {
      setIsLoading(false);
    }
  }, []);

  const checkService = useCallback(async (serviceName: string) => {
    setIsLoading(true);
    setError(null);

    try {
      const cmdResult = await executeAutonomousCommand({ type: 'CheckService', service_name: serviceName });
      setResult(cmdResult);
    } catch (err: any) {
      setError(err.message || String(err));
      setResult(null);
    } finally {
      setIsLoading(false);
    }
  }, []);

  const diagnoseNetwork = useCallback(async (target: string) => {
    setIsLoading(true);
    setError(null);

    try {
      const cmdResult = await executeAutonomousCommand({ type: 'NetworkDiagnose', target });
      setResult(cmdResult);
    } catch (err: any) {
      setError(err.message || String(err));
      setResult(null);
    } finally {
      setIsLoading(false);
    }
  }, []);

  const runSelfHealing = useCallback(async () => {
    setIsLoading(true);
    setError(null);

    try {
      const cmdResult = await executeSelfHealing();
      setResult(cmdResult as AutonomousCommandResult);
    } catch (err: any) {
      setError(err.message || String(err));
      setResult(null);
    } finally {
      setIsLoading(false);
    }
  }, []);

  const clearResult = useCallback(() => {
    setResult(null);
    setError(null);
  }, []);

  return {
    result,
    isLoading,
    error,
    executeCommand,
    checkService,
    diagnoseNetwork,
    runSelfHealing,
    clearResult,
  };
}
