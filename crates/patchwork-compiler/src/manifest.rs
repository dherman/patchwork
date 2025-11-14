/// Plugin manifest generation for Claude Code plugins
///
/// Generates the directory structure and files needed for a Claude Code plugin:
/// - .claude-plugin/plugin.json
/// - commands/*.md
/// - skills/*/SKILL.md

use std::collections::HashMap;
use serde::{Serialize, Deserialize};

/// Plugin manifest data extracted from trait annotations
#[derive(Debug, Clone)]
pub struct PluginManifest {
    /// Plugin name (from trait name)
    pub name: String,
    /// Plugin description (from trait doc comments)
    pub description: Option<String>,
    /// Skills extracted from @skill annotations
    pub skills: Vec<SkillEntry>,
    /// Commands extracted from @command annotations
    pub commands: Vec<CommandEntry>,
}

/// A skill entry point
#[derive(Debug, Clone)]
pub struct SkillEntry {
    /// Skill name (from annotation argument)
    pub name: String,
    /// Function name in generated JS
    pub function: String,
    /// Description (from method doc comments)
    pub description: Option<String>,
    /// Method parameters
    pub params: Vec<String>,
}

/// A command entry point
#[derive(Debug, Clone)]
pub struct CommandEntry {
    /// Command name (from annotation argument)
    pub name: String,
    /// Skill name to invoke (if paired with @skill)
    pub skill: Option<String>,
    /// Function name in generated JS
    pub function: String,
    /// Description (from method doc comments)
    pub description: Option<String>,
}

/// JSON structure for .claude-plugin/plugin.json
#[derive(Debug, Serialize, Deserialize)]
struct PluginJson {
    name: String,
    version: String,
    description: String,
    author: AuthorInfo,
}

#[derive(Debug, Serialize, Deserialize)]
struct AuthorInfo {
    name: String,
}

impl PluginManifest {
    pub fn new(name: String) -> Self {
        Self {
            name,
            description: None,
            skills: Vec::new(),
            commands: Vec::new(),
        }
    }

    /// Generate plugin.json content
    pub fn generate_plugin_json(&self) -> String {
        let plugin = PluginJson {
            name: self.name.clone(),
            version: "0.1.0".to_string(),
            description: self.description.clone()
                .unwrap_or_else(|| format!("{} plugin", self.name)),
            author: AuthorInfo {
                name: "Patchwork Compiler".to_string(),
            },
        };

        serde_json::to_string_pretty(&plugin).unwrap()
    }

    /// Generate command markdown for a command entry
    pub fn generate_command_md(&self, command: &CommandEntry) -> String {
        let description = command.description.as_deref()
            .unwrap_or("Generated command");

        let mut md = String::new();
        md.push_str("---\n");
        md.push_str(&format!("description: {}\n", description));

        // If command has a paired skill, invoke it
        if let Some(skill_name) = &command.skill {
            md.push_str("allowed-tools: Skill\n");
            md.push_str("---\n\n");
            md.push_str(&format!("# {}\n\n", command.name));
            md.push_str(&format!("{}\n\n", description));
            md.push_str("## Task\n\n");
            md.push_str(&format!("Invoke the skill:\n\n```\nSkill(command: \"{}:{}\")\n```\n\n",
                self.name, skill_name));
            md.push_str("Pass the user's arguments (`$ARGUMENTS`) as the prompt to the skill.\n");
        } else {
            // Direct invocation (for future use)
            md.push_str("---\n\n");
            md.push_str(&format!("# {}\n\n", command.name));
            md.push_str(&format!("{}\n\n", description));
            md.push_str("**TODO**: Implement command logic\n");
        }

        md
    }

    /// Generate skill markdown for a skill entry
    pub fn generate_skill_md(&self, skill: &SkillEntry) -> String {
        let description = skill.description.as_deref()
            .unwrap_or("Generated skill");

        let mut md = String::new();
        md.push_str("---\n");
        md.push_str(&format!("name: {}\n", skill.name));
        md.push_str(&format!("description: {}\n", description));
        md.push_str("allowed-tools: All\n");
        md.push_str("---\n\n");
        md.push_str(&format!("# {} Skill\n\n", skill.name));
        md.push_str(&format!("{}\n\n", description));

        md.push_str("## Input\n\n");
        md.push_str("**$PROMPT**\n\n");

        md.push_str("## Setup\n\n");
        md.push_str("Create a session and spawn the code process:\n\n");
        md.push_str("```bash\n");
        md.push_str("# Create session directory\n");
        md.push_str("SESSION_ID=\"patchwork-$(date +%Y%m%d-%H%M%S)\"\n");
        md.push_str("WORK_DIR=\"/tmp/$SESSION_ID\"\n");
        md.push_str("mkdir -p \"$WORK_DIR/mailboxes\"\n");
        md.push_str("\n");
        md.push_str("# Spawn code process with stdio IPC\n");
        md.push_str(&format!("node ./workers/{}.js <<EOF &\n", skill.function));
        md.push_str("{\n");
        md.push_str("  \"type\": \"session\",\n");
        md.push_str("  \"sessionId\": \"$SESSION_ID\",\n");
        md.push_str("  \"timestamp\": \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\",\n");
        md.push_str("  \"workDir\": \"$WORK_DIR\",\n");
        md.push_str("  \"input\": \"$PROMPT\"\n");
        md.push_str("}\n");
        md.push_str("EOF\n");
        md.push_str("\n");
        md.push_str("CODE_PID=$!\n");
        md.push_str("```\n\n");

        md.push_str("## Message Handling\n\n");
        md.push_str("The code process may send IPC requests. Monitor stdout and respond:\n\n");
        md.push_str("```bash\n");
        md.push_str("# Read messages from code process\n");
        md.push_str("while IFS= read -r line; do\n");
        md.push_str("  MSG_TYPE=$(echo \"$line\" | jq -r '.type')\n");
        md.push_str("  \n");
        md.push_str("  case \"$MSG_TYPE\" in\n");
        md.push_str("    \"executePrompt\")\n");
        md.push_str("      # Extract skill name and bindings\n");
        md.push_str("      SKILL_NAME=$(echo \"$line\" | jq -r '.skill')\n");
        md.push_str("      BINDINGS=$(echo \"$line\" | jq -c '.bindings')\n");
        md.push_str("      \n");
        md.push_str("      # Invoke the skill and get result\n");
        md.push_str("      # (Use Skill tool or direct prompt execution)\n");
        md.push_str("      RESULT=$(invoke_skill \"$SKILL_NAME\" \"$BINDINGS\")\n");
        md.push_str("      \n");
        md.push_str("      # Send result back to code process\n");
        md.push_str("      echo \"{\\\"type\\\":\\\"promptResult\\\",\\\"value\\\":$RESULT}\" >&3\n");
        md.push_str("      ;;\n");
        md.push_str("    \n");
        md.push_str("    \"delegate\")\n");
        md.push_str("      # Extract worker configs\n");
        md.push_str("      SESSION_ID=$(echo \"$line\" | jq -r '.sessionId')\n");
        md.push_str("      WORK_DIR=$(echo \"$line\" | jq -r '.workDir')\n");
        md.push_str("      WORKERS=$(echo \"$line\" | jq -c '.workers')\n");
        md.push_str("      \n");
        md.push_str("      # Spawn workers as Task subagents\n");
        md.push_str("      # Each worker runs as a separate code process\n");
        md.push_str("      # Wait for all to complete and aggregate results\n");
        md.push_str("      RESULTS=$(spawn_workers \"$WORKERS\" \"$SESSION_ID\" \"$WORK_DIR\")\n");
        md.push_str("      \n");
        md.push_str("      # Send results back\n");
        md.push_str("      echo \"{\\\"type\\\":\\\"delegateComplete\\\",\\\"results\\\":$RESULTS}\" >&3\n");
        md.push_str("      ;;\n");
        md.push_str("    \n");
        md.push_str("    \"error\")\n");
        md.push_str("      # Code process encountered an error\n");
        md.push_str("      ERROR_MSG=$(echo \"$line\" | jq -r '.message')\n");
        md.push_str("      echo \"Error from code process: $ERROR_MSG\"\n");
        md.push_str("      exit 1\n");
        md.push_str("      ;;\n");
        md.push_str("  esac\n");
        md.push_str("done < <(tail -f /proc/$CODE_PID/fd/1)\n");
        md.push_str("```\n\n");

        md.push_str("**Note**: The actual implementation should use proper IPC mechanisms. ");
        md.push_str("This pseudocode illustrates the message handling pattern.\n");

        md
    }

    /// Get all files that should be generated
    /// Returns: (relative_path, content) pairs
    pub fn get_files(&self) -> HashMap<String, String> {
        let mut files = HashMap::new();

        // plugin.json
        files.insert(
            ".claude-plugin/plugin.json".to_string(),
            self.generate_plugin_json(),
        );

        // Command markdown files
        for command in &self.commands {
            let path = format!("commands/{}.md", command.name);
            files.insert(path, self.generate_command_md(command));
        }

        // Skill markdown files
        for skill in &self.skills {
            let path = format!("skills/{}/SKILL.md", skill.name);
            files.insert(path, self.generate_skill_md(skill));
        }

        files
    }
}
