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
        md.push_str("allowed-tools: Task, Bash, Read\n");
        md.push_str("---\n\n");
        md.push_str(&format!("# {} Skill\n\n", skill.name));
        md.push_str(&format!("{}\n\n", description));
        md.push_str("## Input\n\n");
        md.push_str("**$PROMPT**\n\n");
        md.push_str("## Task\n\n");
        md.push_str(&format!("Call the `{}` function with the input.\n\n", skill.function));
        md.push_str("**TODO**: Implement skill logic using compiled workers\n");

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
