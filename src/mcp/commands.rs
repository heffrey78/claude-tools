use crate::errors::Result;
use crate::mcp::{ClaudeConfig, ClaudeMcpServer, MaskedServerDisplay};
use anyhow::Context;
use std::collections::HashMap;

/// Execute MCP add command
pub fn execute_mcp_add(
    name: String,
    command: String,
    args: Vec<String>,
    env: Vec<String>,
    global: bool,
    project: Option<String>,
    verbose: bool,
) -> Result<()> {
    if verbose {
        eprintln!("Loading Claude configuration...");
    }

    // Load configuration
    let mut config = ClaudeConfig::load()?;

    // Parse environment variables
    let mut env_map = HashMap::new();
    for env_str in env {
        if let Some((key, value)) = env_str.split_once('=') {
            env_map.insert(key.to_string(), value.to_string());
        } else {
            return Err(anyhow::anyhow!(
                "Invalid environment variable format: {}. Use KEY=VALUE",
                env_str
            )
            .into());
        }
    }

    // Create server configuration
    let server = ClaudeMcpServer {
        server_type: "stdio".to_string(),
        command,
        args,
        env: env_map,
    };

    // Add server
    if global {
        if verbose {
            eprintln!("Adding global MCP server '{}'...", name);
        }
        config.add_global_server(name.clone(), server);
    } else {
        let project_path = project
            .or_else(|| {
                std::env::current_dir()
                    .ok()
                    .map(|p| p.to_string_lossy().to_string())
            })
            .ok_or_else(|| anyhow::anyhow!("Could not determine project path"))?;

        if verbose {
            eprintln!(
                "Adding MCP server '{}' to project '{}'...",
                name, project_path
            );
        }
        config.add_project_server(&project_path, name.clone(), server);
    }

    // Save configuration
    config.save()?;

    println!("✅ Successfully added MCP server '{}'", name);
    if global {
        println!("   Scope: Global (available to all projects)");
    } else {
        println!("   Scope: Project-specific");
    }

    Ok(())
}

/// Execute MCP remove command
pub fn execute_mcp_remove(
    name: String,
    global: bool,
    project: Option<String>,
    verbose: bool,
) -> Result<()> {
    if verbose {
        eprintln!("Loading Claude configuration...");
    }

    // Load configuration
    let mut config = ClaudeConfig::load()?;

    // Remove server
    let removed = if global {
        if verbose {
            eprintln!("Removing global MCP server '{}'...", name);
        }
        config.remove_global_server(&name)
    } else {
        let project_path = project
            .or_else(|| {
                std::env::current_dir()
                    .ok()
                    .map(|p| p.to_string_lossy().to_string())
            })
            .ok_or_else(|| anyhow::anyhow!("Could not determine project path"))?;

        if verbose {
            eprintln!(
                "Removing MCP server '{}' from project '{}'...",
                name, project_path
            );
        }
        config.remove_project_server(&project_path, &name)
    };

    if removed.is_none() {
        return Err(anyhow::anyhow!("Server '{}' not found", name).into());
    }

    // Save configuration
    config.save()?;

    println!("✅ Successfully removed MCP server '{}'", name);

    Ok(())
}

/// Execute MCP update command
pub fn execute_mcp_update(
    name: String,
    command: Option<String>,
    args: Option<Vec<String>>,
    env: Option<Vec<String>>,
    global: bool,
    project: Option<String>,
    verbose: bool,
) -> Result<()> {
    if verbose {
        eprintln!("Loading Claude configuration...");
    }

    // Load configuration
    let mut config = ClaudeConfig::load()?;

    // Determine project path if needed
    let project_path = if !global {
        Some(
            project
                .or_else(|| {
                    std::env::current_dir()
                        .ok()
                        .map(|p| p.to_string_lossy().to_string())
                })
                .ok_or_else(|| anyhow::anyhow!("Could not determine project path"))?,
        )
    } else {
        None
    };

    // Update fields
    {
        // Get the server to update
        let server = if global {
            config.mcp_servers.get_mut(&name)
        } else {
            config
                .projects
                .get_mut(project_path.as_ref().unwrap())
                .and_then(|p| p.mcp_servers.get_mut(&name))
        }
        .ok_or_else(|| anyhow::anyhow!("Server '{}' not found", name))?;

        if let Some(new_command) = command {
            server.command = new_command;
        }

        if let Some(new_args) = args {
            server.args = new_args;
        }

        if let Some(env_updates) = env {
            for env_str in env_updates {
                if let Some((key, value)) = env_str.split_once('=') {
                    server.env.insert(key.to_string(), value.to_string());
                } else {
                    return Err(anyhow::anyhow!(
                        "Invalid environment variable format: {}. Use KEY=VALUE",
                        env_str
                    )
                    .into());
                }
            }
        }
    }

    // Save configuration
    config.save()?;

    println!("✅ Successfully updated MCP server '{}'", name);

    // Display updated configuration
    let server = if global {
        config.mcp_servers.get(&name)
    } else {
        config
            .projects
            .get(project_path.as_ref().unwrap())
            .and_then(|p| p.mcp_servers.get(&name))
    }
    .unwrap(); // Safe to unwrap since we just confirmed it exists

    let display = MaskedServerDisplay::new(&name, server);
    println!("\nUpdated configuration:");
    println!("{}", display.display());

    Ok(())
}

/// List Claude Code MCP servers
pub fn list_claude_servers(verbose: bool) -> Result<()> {
    // Load Claude configuration
    let config =
        ClaudeConfig::load().context("Failed to load Claude configuration from ~/.claude.json")?;

    // Get current project path
    let current_project = std::env::current_dir()
        .ok()
        .map(|p| p.to_string_lossy().to_string());

    println!("🔍 Claude Code MCP Servers");
    println!();

    // Display global servers
    if !config.mcp_servers.is_empty() {
        println!("📋 Global Servers (available to all projects):");
        for (name, server) in &config.mcp_servers {
            let display = MaskedServerDisplay::new(name, server);
            println!("  {}", display.display());
            println!();
        }
    }

    // Display current project servers if applicable
    if let Some(ref project_path) = current_project {
        if let Some(project) = config.projects.get(project_path) {
            if !project.mcp_servers.is_empty() {
                println!("📁 Project Servers ({}):", project_path);
                for (name, server) in &project.mcp_servers {
                    let display = MaskedServerDisplay::new(name, server);
                    println!("  {}", display.display());
                    println!();
                }
            }
        }
    }

    // Summary
    let global_count = config.mcp_servers.len();
    let project_count = current_project
        .as_ref()
        .and_then(|p| config.projects.get(p))
        .map(|p| p.mcp_servers.len())
        .unwrap_or(0);

    println!(
        "Total: {} global server(s), {} project server(s)",
        global_count, project_count
    );

    if verbose {
        println!("\nConfiguration file: ~/.claude.json");
        if let Some(ref project) = current_project {
            println!("Current project: {}", project);
        }
    }

    Ok(())
}
