# Patchwork Runtime Design: End-to-End Integration

## Overview

This document describes the complete runtime architecture for Patchwork plugins running in Claude Code. It covers bidirectional IPC between prompt and code processes, filesystem-based mailboxes for worker communication, and plugin manifest generation.

## Architecture

### Dual-Process Model

Every agent (main skill entry point or worker subagent) runs with **two processes**:

1. **Prompt Process** - Claude agent executing markdown instructions
2. **Code Process** - Node.js executing compiled Patchwork JavaScript

**Key difference from Claude Code's model:**
- **Claude Code**: Starts with prompts (agent), can invoke code (via Bash/Task tools)
- **Patchwork**: Starts with code (compiled JS), can invoke prompts (via `think`/`ask` blocks)

### Communication Flows

```
┌─────────────────┐                    ┌─────────────────┐
│ Prompt Process  │                    │ Prompt Process  │
│  (Claude Agent) │                    │  (Claude Agent) │
└────────┬────────┘                    └────────┬────────┘
         │                                      │
         │ stdio IPC                            │ stdio IPC
         │ (bidirectional)                      │ (bidirectional)
         │                                      │
┌────────▼────────┐                    ┌────────▼────────┐
│  Code Process   │                    │  Code Process   │
│   (Node.js)     │                    │   (Node.js)     │
└────────┬────────┘                    └────────┬────────┘
         │                                      │
         │                                      │
         └──────────────────┬───────────────────┘
                            │
                    Filesystem Mailboxes
                    (session.dir/mailboxes/)
```

**Communication mechanisms:**
1. **Code → Prompt**: stdio IPC for `executePrompt()` and `delegate()`
2. **Prompt → Code**: stdio IPC for responses
3. **Worker ↔ Worker**: Filesystem-based mailboxes

## Design Decisions

### Decision 1: Prompt Block Compilation Strategy

**Question**: How should `think { }` and `ask { }` blocks be executed?

**Options Considered:**
- **A) "Eval" approach**: Runtime instantiates template, sends full prompt text as instructions
- **B) "Compilation" approach**: Pre-compile each block to a separate skill document

**Decision: Compilation Approach (Option B)**

**Rationale:**
1. **Consistency**: Workers are compiled to JS modules, prompts should be compiled to skill documents
2. **Debuggability**: Generated skills are visible in filesystem, can inspect what LLM receives
3. **Reusability**: Prompt blocks become callable skills that can be invoked multiple times
4. **Better architecture**: Mirrors the worker→subagent pattern
5. **Fits Claude Code model**: Skills are the natural unit for prompt execution

**Implementation:**
```patchwork
worker example(name) {
    think {
        Say hello to ${name}
    }
}
```

Compiles to:
- `workers/example.js` - Compiled worker code
- `skills/example_think_0/SKILL.md` - Compiled prompt block
- Worker calls: `await executePrompt("example_think_0", {name})`
- Prompt process invokes skill with variable bindings

**Naming convention:**
- Skills: `{worker_name}_think_{index}`, `{worker_name}_ask_{index}`
- Allows multiple think/ask blocks per worker
- Globally unique within plugin

### Decision 2: Mailbox Implementation

**Question**: How should filesystem-based mailboxes avoid race conditions when multiple senders write simultaneously?

**Problem:**
```javascript
// RACE CONDITION - Both processes append to same file
processA: fs.appendFileSync('narrator.jsonl', msg1 + '\n');
processB: fs.appendFileSync('narrator.jsonl', msg2 + '\n');
// Result: Corrupted, interleaved JSON
```

**Options Considered:**
- **A) Single JSONL file per mailbox**: Simple but has race condition
- **B) Directory per mailbox with atomic file writes**: No locking needed
- **C) Use sidechat MCP**: Reuse existing Claude Code mechanism

**Decision: Directory per mailbox (Option B)**

**Rationale:**
1. **Correctness**: Atomic file creation eliminates race conditions
2. **Portability**: Works outside Claude Code
3. **Debuggability**: Can inspect individual messages in filesystem
4. **Persistence**: Survives crashes, can replay messages
5. **Consistency**: Aligns with `.failed` file approach for session failure tracking

**Filesystem structure:**
```
/tmp/historian-20251024-120316/          # session.dir
  mailboxes/
    narrator/
      1730000001234-12345.json          # timestamp_ms-sender_pid.json
      1730000001456-12346.json
      1730000002789-12347.json
    scribe/
      1730000001567-12345.json
    analyst/
      1730000003890-12348.json
  .failed                                 # Session failure marker
```

**Message file format:**
```json
{
  "from": "analyst",
  "to": "narrator",
  "timestamp": "2025-01-24T12:03:16.234Z",
  "payload": {
    "type": "commit_plan",
    "commits": [...]
  }
}
```

**Filename format:** `${timestamp_ms}-${sender_pid}.json`
- **timestamp_ms**: Milliseconds since epoch (13 digits)
- **sender_pid**: Node.js process ID
- **Properties**:
  - Lexicographically sortable (FIFO ordering)
  - No collisions (PID prevents same-timestamp conflicts)
  - Atomic creation (fs.writeFileSync creates file atomically)

**Send algorithm:**
```javascript
async send(message) {
  const filename = `${Date.now()}-${process.pid}.json`;
  const filepath = `${session.dir}/mailboxes/${this.name}/${filename}`;
  await writeFile(filepath, JSON.stringify(message, null, 2));
}
```

**Receive algorithm:**
```javascript
async receive(timeout) {
  // List all messages in mailbox directory
  const files = await readdir(`${session.dir}/mailboxes/${this.name}`);

  // Sort by filename (timestamp ordering)
  files.sort();

  // Read and remove oldest message
  if (files.length > 0) {
    const filepath = `${session.dir}/mailboxes/${this.name}/${files[0]}`;
    const content = await readFile(filepath, 'utf-8');
    await unlink(filepath);  // Remove after reading
    return JSON.parse(content);
  }

  // If no messages, watch for new files (with timeout)
  return watchForNewMessage(timeout);
}
```

**Alternative considered:** Timestamp-based filenames could theoretically have ordering issues if system clocks differ between processes. This is acceptable for MVP because:
- All workers run on same machine (same clock)
- Millisecond precision makes collisions extremely unlikely
- PID prevents same-timestamp conflicts
- If ordering issues emerge, can add sequence numbers later

## IPC Protocol

### Stdio-Based IPC

Both processes communicate via stdio using newline-delimited JSON:

**Message format:**
```
{type}|{payload}\n
```

**Code → Prompt messages:**
```json
{"type": "executePrompt", "skill": "example_think_0", "bindings": {"name": "Claude"}}
{"type": "delegate", "workers": ["analyst", "narrator", "scribe"], "sessionId": "..."}
```

**Prompt → Code responses:**
```json
{"type": "promptResult", "value": {...}}
{"type": "delegateComplete", "results": [{...}, {...}, {...}]}
{"type": "error", "message": "..."}
```

### Helper Scripts

**1. code-process-init.js**
- Spawned by prompt process at agent startup
- Sets up stdio IPC handlers
- Loads compiled worker module
- Calls worker's `main(session)` function
- Handles graceful shutdown

**2. prompt-ipc-helper.sh** (may not be needed)
- If needed: Helper for code to send messages to prompt
- Alternative: Direct stdio write from Node.js

## Plugin Manifest Generation

### Skill Entry Points

**SKILL.md template:**
```markdown
---
name: {skill_name}
description: {description}
allowed-tools: Task, Bash, Read
---

# {Skill Name} Skill

{Description from trait method}

## Input

**$PROMPT**

## Setup

Spawn the code process and establish IPC:

```bash
# Create session directory
SESSION_ID="historian-$(date +%Y%m%d-%H%M%S)"
WORK_DIR="/tmp/$SESSION_ID"
mkdir -p "$WORK_DIR/mailboxes"

# Spawn code process with stdio IPC
node ./code-process-init.js {skill_function} <<EOF
{
  "sessionId": "$SESSION_ID",
  "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "workDir": "$WORK_DIR",
  "input": "$PROMPT"
}
EOF
```

The code process will execute and may send IPC messages back for prompt execution or subagent delegation.

## Message Handling

[Generated message handling loop for responding to code process IPC requests]
```

### Worker Agent Markdown

**agents/{worker_name}.md template:**
```markdown
---
name: {worker_name}
description: {description}
model: inherit
color: {color}
---

# {Worker Name} Agent

{Description from worker definition}

## Input

Your prompt contains session information:
```
Session ID: {session_id}
Work directory: {work_dir}
{Additional parameters}
```

## Setup and Execution

Extract session information and spawn code process:

```bash
# Extract from input
SESSION_ID="..."
WORK_DIR="..."

# Ensure mailbox directory exists
mkdir -p "$WORK_DIR/mailboxes/{worker_name}"

# Spawn code process
node ./code-process-init.js {worker_name} <<EOF
{
  "sessionId": "$SESSION_ID",
  "timestamp": "...",
  "workDir": "$WORK_DIR"
}
EOF
```

The code process will execute the worker logic and handle IPC for prompts and mailbox communication.
```

## Runtime Implementation

### executePrompt()

**Current (mocked):**
```javascript
export async function executePrompt(session, templateId, bindings) {
  console.log(`[Mock] executePrompt: ${templateId}`);
  return { success: true, message: `Mock response for ${templateId}` };
}
```

**Full implementation:**
```javascript
export async function executePrompt(session, skillName, bindings) {
  // Send IPC message to prompt process via stdout
  const request = {
    type: "executePrompt",
    skill: skillName,
    bindings: bindings
  };

  process.stdout.write(JSON.stringify(request) + '\n');

  // Wait for response from prompt process via stdin
  const response = await readLineFromStdin();
  const result = JSON.parse(response);

  if (result.type === "error") {
    throw new Error(result.message);
  }

  return result.value;
}
```

### delegate()

**Current (no spawning):**
```javascript
export async function delegate(session, workers) {
  try {
    const results = await Promise.all(workers);
    return results;
  } catch (error) {
    await session.markFailed(error);
    throw error;
  } finally {
    session.cleanup();
  }
}
```

**Full implementation:**
```javascript
export async function delegate(session, workerConfigs) {
  try {
    // Send IPC message to prompt process to spawn workers via Task tool
    const request = {
      type: "delegate",
      sessionId: session.id,
      workDir: session.dir,
      workers: workerConfigs.map(w => ({
        name: w.name,
        params: w.params
      }))
    };

    process.stdout.write(JSON.stringify(request) + '\n');

    // Wait for all workers to complete
    const response = await readLineFromStdin();
    const result = JSON.parse(response);

    if (result.type === "error") {
      throw new Error(result.message);
    }

    return result.results;
  } catch (error) {
    await session.markFailed(error);
    throw error;
  } finally {
    session.cleanup();
  }
}
```

### Mailbox

**Current (in-memory):**
```javascript
export class Mailbox {
  constructor(name, session) {
    this.name = name;
    this.session = session;
    this.queue = [];  // In-memory - doesn't work across processes!
    this.waiters = [];
  }
  // ...
}
```

**Full implementation:**
```javascript
export class Mailbox {
  constructor(name, session) {
    this.name = name;
    this.session = session;
    this.mailboxDir = `${session.dir}/mailboxes/${name}`;

    // Ensure mailbox directory exists
    mkdirSync(this.mailboxDir, { recursive: true });
  }

  async send(message) {
    await this.session.checkFailed();

    // Create message file with timestamp-PID naming
    const filename = `${Date.now()}-${process.pid}.json`;
    const filepath = `${this.mailboxDir}/${filename}`;

    const messageEnvelope = {
      from: "sender",  // TODO: Track sender identity
      to: this.name,
      timestamp: new Date().toISOString(),
      payload: message
    };

    await writeFile(filepath, JSON.stringify(messageEnvelope, null, 2));
  }

  async receive(timeout) {
    await this.session.checkFailed();

    // Try to read existing message
    const files = await readdir(this.mailboxDir);
    files.sort();  // Lexicographic sort = FIFO order

    if (files.length > 0) {
      const filepath = `${this.mailboxDir}/${files[0]}`;
      const content = await readFile(filepath, 'utf-8');
      await unlink(filepath);  // Remove after reading

      const envelope = JSON.parse(content);
      return envelope.payload;
    }

    // No messages yet - watch for new files
    return this.watchForMessage(timeout);
  }

  async watchForMessage(timeout) {
    return new Promise((resolve, reject) => {
      const watcher = watch(this.mailboxDir, async (eventType, filename) => {
        if (eventType === 'rename' && filename) {
          // New file created
          try {
            const filepath = `${this.mailboxDir}/${filename}`;
            const content = await readFile(filepath, 'utf-8');
            await unlink(filepath);

            const envelope = JSON.parse(content);
            watcher.close();
            clearTimeout(timer);
            resolve(envelope.payload);
          } catch (err) {
            // File might have been deleted by another receiver
            // Continue watching
          }
        }
      });

      // Timeout
      const timer = setTimeout(() => {
        watcher.close();
        reject(new Error(`Mailbox receive timeout after ${timeout}ms`));
      }, timeout);

      // Also race against session failure
      this.session.failurePromise.catch(err => {
        watcher.close();
        clearTimeout(timer);
        reject(err);
      });
    });
  }
}
```

## Compilation Updates

### Prompt Block Compilation

**Input (Patchwork):**
```patchwork
worker example(name) {
    var greeting = "Hello"

    think {
        ${greeting}, ${name}! Please analyze this request.
    }
}
```

**Output:**

**1. Compiled worker (workers/example.js):**
```javascript
export async function example(session, name) {
  let greeting = "Hello";

  // Execute think block via IPC
  const result = await executePrompt(
    session,
    "example_think_0",
    { greeting, name }
  );

  return result;
}
```

**2. Compiled skill (skills/example_think_0/SKILL.md):**
```markdown
---
name: example_think_0
description: Think block from worker example
allowed-tools: All
---

# Example Think Block

## Input

Variable bindings:
- `greeting`: $BINDING_greeting
- `name`: $BINDING_name

## Task

${greeting}, ${name}! Please analyze this request.

## Output

Return your analysis result as structured data.
```

**Codegen updates needed:**
- Detect think/ask blocks during codegen
- Generate skill documents for each block
- Replace block with `executePrompt()` call
- Pass variable bindings from lexical scope

## Testing Strategy

### Unit Tests

1. **Mailbox tests**: Verify filesystem-based send/receive works across processes
2. **IPC tests**: Mock stdio and test executePrompt/delegate message formats
3. **Compilation tests**: Verify think/ask blocks generate correct skill documents

### Integration Tests

**Test setup:**
1. Compile a simple Patchwork plugin to temp directory
2. Use `claude` CLI to invoke the skill
3. Verify the code process executes correctly
4. Check session directory structure

**Example test:**
```rust
#[test]
fn test_simple_think_block() {
    // Compile simple plugin
    let plugin = compile_plugin("
        trait Test: Agent {
            @skill test
            fun test(input: string) {
                think {
                    Analyze: ${input}
                }
            }
        }
    ");

    // Write to temp directory
    let temp_dir = TempDir::new();
    write_plugin_files(&temp_dir, plugin);

    // Invoke via claude CLI
    let output = Command::new("claude")
        .arg("skill")
        .arg("test:test")
        .arg("hello world")
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to execute claude");

    assert!(output.status.success());
}
```

### End-to-End Test

**Historian plugin test:**
1. Compile full historian example
2. Create test git repository with changes
3. Invoke `/historian:narrate` via `claude` CLI
4. Verify commit rewriting succeeds
5. Check generated commits are correct

## Implementation Phases

### Filesystem Mailboxes
- Update Mailbox class to use directory structure
- Add tests for cross-process communication
- Verify FIFO ordering and race condition handling

### Prompt Compilation
- Update codegen to detect think/ask blocks
- Generate skill documents for each block
- Add skill to manifest generation
- Test variable binding capture

### IPC Infrastructure
- Implement code-process-init.js helper
- Update executePrompt() with stdio IPC
- Update delegate() with Task spawning
- Add stdin reading helpers

### Manifest Updates
- Update SKILL.md generation with code process spawning
- Update agent .md generation with code process spawning
- Add IPC message handling loops

### Integration Testing
- Write integration test framework
- Test simple plugin end-to-end
- Test historian plugin end-to-end
- Verify all pieces work together

## Success Criteria

Implementation is complete when:
- [ ] Filesystem mailboxes work correctly across processes
- [ ] Think/ask blocks compile to skill documents
- [ ] Skills spawn code processes with stdio IPC
- [ ] Workers spawn as subagents via Task tool
- [ ] executePrompt() sends/receives messages correctly
- [ ] delegate() spawns workers and waits for completion
- [ ] Simple test plugin runs successfully in Claude Code
- [ ] Historian plugin runs successfully and rewrites commits

## Future Improvements

**Not in current scope, but worth noting:**

1. **Sequence numbers**: Add explicit sequence numbers to mailbox files to guarantee ordering even if clocks differ
2. **Message acknowledgment**: Track which messages have been processed to enable replay/recovery
3. **Mailbox cleanup**: Remove old/processed messages after session completes
4. **IPC timeout handling**: Better error messages when IPC fails
5. **Skill caching**: Cache compiled skills to avoid regeneration
6. **Multi-level prompts**: Support nested think/ask blocks
7. **Prompt state**: Allow prompts to maintain state across invocations
8. **Alternative IPC**: Consider unix sockets or HTTP for better reliability
