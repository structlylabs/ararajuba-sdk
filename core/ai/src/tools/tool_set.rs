//! A set of tools available during generation.

use super::tool::ToolDef;
use std::collections::HashMap;

/// A collection of tools indexed by name.
#[derive(Debug, Default)]
pub struct ToolSet {
    tools: HashMap<String, ToolDef>,
}

impl ToolSet {
    /// Create a new empty tool set.
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Add a tool to the set.
    pub fn add(mut self, tool: ToolDef) -> Self {
        self.tools.insert(tool.name.clone(), tool);
        self
    }

    /// Get a tool by name.
    pub fn get(&self, name: &str) -> Option<&ToolDef> {
        self.tools.get(name)
    }

    /// Check if the tool set is empty.
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    /// Number of tools.
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Convert tools to the provider-level tool format.
    pub fn to_provider_tools(&self) -> Vec<ararajuba_provider::language_model::v4::tool::Tool> {
        self.tools
            .values()
            .map(|t| {
                ararajuba_provider::language_model::v4::tool::Tool::Function(
                    ararajuba_provider::language_model::v4::tool::FunctionTool {
                        name: t.name.clone(),
                        description: t.description.clone(),
                        input_schema: t.input_schema.clone(),
                        input_examples: None,
                        strict: None,
                        provider_options: None,
                    },
                )
            })
            .collect()
    }

    /// Iterate over tools.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &ToolDef)> {
        self.tools.iter()
    }

    /// Add a tool by reference (clones the `ToolDef` — cheap because closures
    /// are `Arc`-wrapped).
    pub fn add_ref(mut self, tool: &ToolDef) -> Self {
        self.tools.insert(tool.name.clone(), tool.clone());
        self
    }
}
