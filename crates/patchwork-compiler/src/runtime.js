/**
 * Patchwork Runtime Library
 *
 * Provides the runtime infrastructure for compiled Patchwork programs.
 * This file is automatically emitted by the Patchwork compiler.
 */

import { spawn } from 'child_process';
import { promisify } from 'util';

/**
 * Session context available to all workers
 */
export class SessionContext {
  constructor(id, timestamp, dir) {
    this.id = id;
    this.timestamp = timestamp;
    this.dir = dir;
  }
}

/**
 * Shell command execution options
 */
class ShellOptions {
  constructor(options = {}) {
    this.capture = options.capture ?? false;
    this.cwd = options.cwd ?? process.cwd();
  }
}

/**
 * Execute a shell command
 *
 * @param {string} command - The command string to execute
 * @param {Object} options - Execution options
 * @param {boolean} options.capture - Whether to capture and return stdout
 * @param {string} options.cwd - Working directory for command execution
 * @returns {Promise<string>} - The stdout output if capture=true, otherwise empty string
 */
export async function shell(command, options = {}) {
  const opts = new ShellOptions(options);

  return new Promise((resolve, reject) => {
    const child = spawn('sh', ['-c', command], {
      cwd: opts.cwd,
      stdio: opts.capture ? ['ignore', 'pipe', 'pipe'] : ['ignore', 'inherit', 'inherit']
    });

    if (opts.capture) {
      let stdout = '';
      let stderr = '';

      child.stdout.on('data', (data) => {
        stdout += data.toString();
      });

      child.stderr.on('data', (data) => {
        stderr += data.toString();
      });

      child.on('close', (code) => {
        if (code !== 0) {
          reject(new Error(`Command failed with exit code ${code}: ${stderr}`));
        } else {
          resolve(stdout.trimEnd());
        }
      });

      child.on('error', (err) => {
        reject(err);
      });
    } else {
      child.on('close', (code) => {
        if (code !== 0) {
          reject(new Error(`Command failed with exit code ${code}`));
        } else {
          resolve('');
        }
      });

      child.on('error', (err) => {
        reject(err);
      });
    }
  });
}

/**
 * IPC Message types for prompt execution
 * (Phase 3: scaffolding only, full implementation in Phase 11)
 */
export class IpcMessage {
  constructor(type, data) {
    this.type = type;
    this.data = data;
  }
}

export class ThinkRequest extends IpcMessage {
  constructor(templateId, bindings) {
    super('ThinkRequest', { templateId, bindings });
  }
}

export class ThinkResponse extends IpcMessage {
  constructor(result) {
    super('ThinkResponse', { result });
  }
}

export class AskRequest extends IpcMessage {
  constructor(templateId, bindings) {
    super('AskRequest', { templateId, bindings });
  }
}

export class AskResponse extends IpcMessage {
  constructor(result) {
    super('AskResponse', { result });
  }
}

/**
 * Execute a prompt block (think or ask)
 *
 * Phase 4: Sends IPC request with template ID and variable bindings.
 * Phase 11: Full IPC implementation with actual agent communication.
 *
 * @param {SessionContext} session - The session context
 * @param {string} templateId - The prompt template ID (e.g., 'think_0')
 * @param {Object} bindings - Variable bindings to interpolate into the template
 * @returns {Promise<any>} - The result from the agent (structure depends on prompt type)
 */
export async function executePrompt(session, templateId, bindings) {
  // Phase 4: Mock implementation that just returns a placeholder
  // Phase 11 will implement the full IPC transport

  console.log(`[Patchwork Runtime] executePrompt: ${templateId}`);
  console.log(`[Patchwork Runtime] Session: ${session.id}`);
  console.log(`[Patchwork Runtime] Bindings:`, bindings);

  // Return a mock response for now
  // In Phase 11, this will send an IPC message and await the response
  return {
    success: true,
    message: `Mock response for ${templateId}`,
  };
}
