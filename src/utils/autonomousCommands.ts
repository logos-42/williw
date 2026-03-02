import { invoke } from '@tauri-apps/api/core';

export interface AutonomousCommandResult {
  success: boolean;
  stdout: string;
  stderr: string;
  exit_code: number | null;
  message: string;
}

export type AutonomousCommandType =
  | { type: 'StartOllama'; gpu_limit?: number }
  | { type: 'StopOllama' }
  | { type: 'CheckService'; service_name: string }
  | { type: 'KillProcess'; process_name: string }
  | { type: 'CheckDiskSpace'; path?: string }
  | { type: 'CleanupTemp'; directory: string; max_age_days?: number }
  | { type: 'NetworkDiagnose'; target: string }
  | { type: 'Custom'; command: string; description: string };

/**
 * 执行自主命令
 * 
 * @param command 命令对象
 * @param requireConfirmation 是否需要用户确认（暂未实现）
 */
export async function executeAutonomousCommand(
  command: AutonomousCommandType,
  requireConfirmation: boolean = false
): Promise<AutonomousCommandResult> {
  // 将前端命令类型转换为 Rust 枚举格式
  const rustCommand = (() => {
    switch (command.type) {
      case 'StartOllama':
        return {
          StartOllama: { gpu_limit: command.gpu_limit }
        };
      case 'StopOllama':
        return {
          StopOllama: {}
        };
      case 'CheckService':
        return {
          CheckService: { service_name: command.service_name }
        };
      case 'KillProcess':
        return {
          KillProcess: { process_name: command.process_name }
        };
      case 'CheckDiskSpace':
        return {
          CheckDiskSpace: { path: command.path }
        };
      case 'CleanupTemp':
        return {
          CleanupTemp: { 
            directory: command.directory,
            max_age_days: command.max_age_days
          }
        };
      case 'NetworkDiagnose':
        return {
          NetworkDiagnose: { target: command.target }
        };
      case 'Custom':
        return {
          Custom: { 
            command: command.command,
            description: command.description
          }
        };
      default:
        throw new Error(`未知命令类型：${command}`);
    }
  })();

  return await invoke<AutonomousCommandResult>(
    'execute_autonomous_command',
    {
      command: rustCommand,
      requireConfirmation
    }
  );
}

/**
 * 执行自愈流程
 * 
 * 自动检测并修复常见问题：
 * - Ollama 服务未运行 → 自动启动
 * - 进程卡死 → 自动清理
 */
export async function executeSelfHealing(): Promise<any> {
  return await invoke('execute_self_healing');
}

/**
 * 快捷命令：启动 Ollama
 */
export async function startOllama(gpuLimit?: number): Promise<AutonomousCommandResult> {
  return executeAutonomousCommand({ type: 'StartOllama', gpu_limit: gpuLimit });
}

/**
 * 快捷命令：停止 Ollama
 */
export async function stopOllama(): Promise<AutonomousCommandResult> {
  return executeAutonomousCommand({ type: 'StopOllama' });
}

/**
 * 快捷命令：检查服务状态
 */
export async function checkServiceStatus(serviceName: string): Promise<AutonomousCommandResult> {
  return executeAutonomousCommand({ type: 'CheckService', service_name: serviceName });
}

/**
 * 快捷命令：网络诊断
 */
export async function diagnoseNetwork(target: string = '8.8.8.8'): Promise<AutonomousCommandResult> {
  return executeAutonomousCommand({ type: 'NetworkDiagnose', target });
}
