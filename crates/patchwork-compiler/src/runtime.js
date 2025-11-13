/**
 * Patchwork Runtime Library
 *
 * Provides the runtime infrastructure for compiled Patchwork programs.
 * This file is automatically emitted by the Patchwork compiler.
 */

import { spawn } from 'child_process';
import { promisify } from 'util';
import { watch } from 'fs';
import { writeFile, readFile, access } from 'fs/promises';

/**
 * Mailbox for worker message passing
 *
 * Provides FIFO message queue with blocking receive.
 * Integrates with session failure detection to abort on worker failures.
 */
export class Mailbox {
  constructor(name, session) {
    this.name = name;
    this.session = session;
    this.queue = [];
    this.waiters = [];
  }

  /**
   * Send a message to this mailbox
   *
   * @param {any} message - The message to send (will be JSON serialized)
   * @throws {Error} - If the session has failed
   */
  async send(message) {
    // Check if session has failed before sending
    await this.session.checkFailed();

    // Clone the message to ensure isolation between workers
    const cloned = JSON.parse(JSON.stringify(message));

    // If there's a waiter, resolve it immediately
    if (this.waiters.length > 0) {
      const waiter = this.waiters.shift();
      clearTimeout(waiter.timeoutId);
      waiter.resolve(cloned);
    } else {
      // Otherwise, queue the message
      this.queue.push(cloned);
    }
  }

  /**
   * Receive a message from this mailbox
   *
   * Blocks until a message is available, session fails, or timeout is reached.
   *
   * @param {number} timeout - Timeout in milliseconds (optional)
   * @returns {Promise<any>} - The received message
   * @throws {Error} - If timeout is reached or session fails before a message arrives
   */
  async receive(timeout) {
    // Check if session has already failed
    await this.session.checkFailed();

    // If there's a queued message, return it immediately
    if (this.queue.length > 0) {
      return this.queue.shift();
    }

    // Race between: message arrival, session failure, and timeout
    const promises = [
      // Message arrival
      new Promise((resolve, reject) => {
        const waiter = { resolve, reject, timeoutId: null };
        this.waiters.push(waiter);
      }),

      // Session failure detection (via fs.watch)
      this.session.failurePromise.catch(err => {
        // Clean up any waiters when session fails
        this.waiters.forEach(w => {
          if (w.timeoutId) clearTimeout(w.timeoutId);
        });
        this.waiters = [];
        throw err;
      })
    ];

    // Add timeout if specified
    if (timeout !== undefined && timeout !== null) {
      promises.push(
        new Promise((_, reject) =>
          setTimeout(() => {
            reject(new Error(`Mailbox receive timeout after ${timeout}ms`));
          }, timeout)
        )
      );
    }

    return Promise.race(promises);
  }
}

/**
 * Mailroom manages all mailboxes for a session
 *
 * Provides lazy mailbox creation via property access.
 */
export class Mailroom {
  constructor(session) {
    this.session = session;
    this.mailboxes = new Map();

    // Return a proxy that creates mailboxes on-demand
    return new Proxy(this, {
      get(target, prop) {
        // Allow access to internal methods/properties
        if (prop === 'session' || prop === 'mailboxes' || typeof target[prop] === 'function') {
          return target[prop];
        }

        // Lazy mailbox creation
        if (!target.mailboxes.has(prop)) {
          target.mailboxes.set(prop, new Mailbox(prop, target.session));
        }
        return target.mailboxes.get(prop);
      }
    });
  }
}

/**
 * Session context available to all workers
 *
 * Provides session state and coordinates failure detection across workers.
 * Uses filesystem-based failure tracking that works across processes.
 */
export class SessionContext {
  constructor(id, timestamp, dir) {
    this.id = id;
    this.timestamp = timestamp;
    this.dir = dir;
    this.failureFile = `${dir}/.failed`;
    this.failureWatcher = null;
    this.failurePromise = null;

    // Mailroom for worker message passing
    this.mailbox = new Mailroom(this);

    // Set up failure detection
    this.setupFailureWatch();
  }

  /**
   * Set up filesystem watcher for session failure detection
   *
   * Creates a promise that rejects when .failed file is created.
   * This allows mailbox operations to race against session failure.
   */
  setupFailureWatch() {
    this.failurePromise = new Promise((_resolve, reject) => {
      // First check if .failed already exists (session may have failed before we joined)
      access(this.failureFile)
        .then(() => {
          // File exists - session already failed
          return readFile(this.failureFile, 'utf-8');
        })
        .then(content => {
          const failureInfo = JSON.parse(content);
          reject(new Error(`Session ${this.id} failed: ${failureInfo.error}`));
        })
        .catch(err => {
          // File doesn't exist yet - set up watcher
          if (err.code !== 'ENOENT') {
            // Some other error reading the file
            console.error('Error checking failure file:', err);
          }

          // Watch the session directory for .failed file creation
          this.failureWatcher = watch(this.dir, (_eventType, filename) => {
            if (filename === '.failed') {
              readFile(this.failureFile, 'utf-8')
                .then(content => {
                  const failureInfo = JSON.parse(content);
                  reject(new Error(`Session ${this.id} failed: ${failureInfo.error}`));
                })
                .catch(readErr => {
                  reject(new Error(`Session ${this.id} failed but could not read error details: ${readErr.message}`));
                });
            }
          });
        });
    });
  }

  /**
   * Mark this session as failed
   *
   * Writes .failed file to notify all workers in this session.
   * This is called when a worker throws an error.
   *
   * @param {Error} error - The error that caused the failure
   */
  async markFailed(error) {
    const failureInfo = {
      timestamp: new Date().toISOString(),
      error: error.message,
      stack: error.stack
    };

    try {
      await writeFile(this.failureFile, JSON.stringify(failureInfo, null, 2));
    } catch (writeErr) {
      // If we can't write the failure file, log it
      console.error('Failed to write session failure file:', writeErr);
    }
  }

  /**
   * Check if session has failed
   *
   * Checks for existence of .failed file synchronously.
   * Throws if session has failed.
   *
   * @throws {Error} - If the session has failed
   */
  async checkFailed() {
    try {
      // Check if .failed file exists
      await access(this.failureFile);

      // File exists - read it and throw
      const content = await readFile(this.failureFile, 'utf-8');
      const failureInfo = JSON.parse(content);
      throw new Error(`Session ${this.id} failed: ${failureInfo.error}`);
    } catch (err) {
      // If error is about session failure, re-throw it
      if (err.message && err.message.includes('Session')) {
        throw err;
      }
      // Otherwise it's just ENOENT (file doesn't exist) - session is fine
    }
  }

  /**
   * Clean up resources
   *
   * Should be called when session completes (success or failure).
   */
  cleanup() {
    if (this.failureWatcher) {
      this.failureWatcher.close();
      this.failureWatcher = null;
    }
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

// Export as $shell for generated code
export { shell as $shell };

/**
 * Execute a shell pipe (cmd1 | cmd2)
 *
 * Connects stdout of first command to stdin of second command.
 *
 * @param {Array<string>} commands - Array of command strings to pipe together
 * @param {Object} options - Execution options
 * @returns {Promise<string>} - The output of the final command if capture=true
 */
export async function $shellPipe(commands, options = {}) {
  // Join commands with pipe operator and execute as single shell command
  const pipeCmd = commands.join(' | ');
  return shell(pipeCmd, options);
}

/**
 * Execute shell commands with && operator (cmd1 && cmd2)
 *
 * Executes second command only if first succeeds.
 *
 * @param {Array<string>} commands - Array of command strings to chain
 * @param {Object} options - Execution options
 * @returns {Promise<string>} - The output if capture=true, otherwise empty string
 */
export async function $shellAnd(commands, options = {}) {
  const andCmd = commands.join(' && ');
  return shell(andCmd, options);
}

/**
 * Execute shell commands with || operator (cmd1 || cmd2)
 *
 * Executes second command only if first fails.
 *
 * @param {Array<string>} commands - Array of command strings to chain
 * @param {Object} options - Execution options
 * @returns {Promise<string>} - The output if capture=true, otherwise empty string
 */
export async function $shellOr(commands, options = {}) {
  const orCmd = commands.join(' || ');
  return shell(orCmd, options);
}

/**
 * Execute shell command with redirection (cmd > file)
 *
 * @param {string} command - The command string to execute
 * @param {string} operator - The redirection operator ('>', '>>', '<', '2>', '2>&1')
 * @param {string} target - The file path or descriptor for redirection
 * @param {Object} options - Execution options
 * @returns {Promise<string>} - Empty string (redirections don't capture)
 */
export async function $shellRedirect(command, operator, target, options = {}) {
  // Build the full command with redirection
  const redirectCmd = `${command} ${operator} ${target}`;
  return shell(redirectCmd, { ...options, capture: false });
}

/**
 * IPC Message types for prompt execution
 * (Mock implementation - full IPC transport to be implemented later)
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
 * Sends IPC request with template ID and variable bindings.
 * Currently a mock implementation - full IPC transport to be implemented.
 *
 * @param {SessionContext} session - The session context
 * @param {string} templateId - The prompt template ID (e.g., 'think_0')
 * @param {Object} bindings - Variable bindings to interpolate into the template
 * @returns {Promise<any>} - The result from the agent (structure depends on prompt type)
 */
export async function executePrompt(session, templateId, bindings) {
  // TODO: Implement full IPC transport for agent communication

  console.log(`[Patchwork Runtime] executePrompt: ${templateId}`);
  console.log(`[Patchwork Runtime] Session: ${session.id}`);
  console.log(`[Patchwork Runtime] Bindings:`, bindings);

  // Mock response placeholder
  return {
    success: true,
    message: `Mock response for ${templateId}`,
  };
}

/**
 * Delegate work to a group of workers (fork/join pattern)
 *
 * Spawns multiple workers in parallel and waits for all to complete.
 * If any worker fails, the entire session fails and all pending operations abort.
 *
 * This implements fork/join semantics:
 * - All workers start in parallel
 * - All workers must succeed for delegation to succeed
 * - If any worker fails, session is marked failed and other workers abort
 *
 * @param {SessionContext} session - The session context
 * @param {Array<Promise>} workers - Array of worker promises to execute
 * @returns {Promise<Array>} - Array of results from each worker (in same order)
 * @throws {Error} - If any worker fails
 */
export async function delegate(session, workers) {
  try {
    // Wait for all workers to complete
    // Promise.all will reject if any worker rejects
    const results = await Promise.all(workers);

    // All workers succeeded - return results
    return results;
  } catch (error) {
    // One or more workers failed - mark session as failed
    await session.markFailed(error);

    // Re-throw to propagate the error
    throw error;
  } finally {
    // Clean up session resources
    session.cleanup();
  }
}
