use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde_json::Value;
use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "echo")]
#[command(about = "Cursor Multi-Agent Rules System CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize repository with Cursor Multi-Agent Rules System
    Init {
        /// Overwrite existing files
        #[arg(long)]
        force: bool,
        /// Skip creating AGENT.md even if it's missing
        #[arg(long)]
        skip_agent: bool,
        /// Always create AGENT.md from template (overwrites existing)
        #[arg(long)]
        with_agent: bool,
        /// Skip installing git hooks (hooks are installed by default)
        #[arg(long)]
        skip_hooks: bool,
        /// Explicitly install git hooks (redundant, hooks install by default)
        #[arg(long)]
        install_hooks: bool,
    },
    /// Initialize flowmates configuration directory
    InitFlowmatesConfig {
        /// Specify repository path (default: auto-detect from git root)
        #[arg(long)]
        path: Option<String>,
        /// Specify config filename (default: config.json)
        #[arg(long, default_value = "config.json")]
        file: String,
        /// Overwrite existing config file even if it already exists
        #[arg(long)]
        force: bool,
        /// Show what would be created without making changes
        #[arg(long)]
        dry_run: bool,
        /// Skip git repository validation (allow non-git directories)
        #[arg(long)]
        no_validate: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init {
            force,
            skip_agent,
            with_agent,
            skip_hooks,
            install_hooks,
        } => init_command(force, skip_agent, with_agent, skip_hooks, install_hooks),
        Commands::InitFlowmatesConfig {
            path,
            file,
            force,
            dry_run,
            no_validate,
        } => init_flowmates_config_command(path, file, force, dry_run, no_validate),
    }
}

struct InitReport {
    copied_rules: Vec<String>,
    skipped_rules: Vec<String>,
    copied_templates: Vec<String>,
    skipped_templates: Vec<String>,
    created_dirs: Vec<String>,
    copied_scripts: Vec<String>,
    skipped_scripts: Vec<String>,
    errors: Vec<String>,
    warnings: Vec<String>,
    agent_created: bool,
    gitignore_action: Option<String>, // "created", "added", "skipped"
    hook_action: Option<String>,      // "installed", "updated", "skipped", "not_found", "not_git"
    source_used: Option<String>,      // "flowmates" or "cursor"
}

fn init_command(
    force: bool,
    skip_agent: bool,
    with_agent: bool,
    skip_hooks: bool,
    _install_hooks: bool, // Redundant, kept for backward compatibility
) -> Result<()> {
    let mut report = InitReport {
        copied_rules: Vec::new(),
        skipped_rules: Vec::new(),
        copied_templates: Vec::new(),
        skipped_templates: Vec::new(),
        created_dirs: Vec::new(),
        copied_scripts: Vec::new(),
        skipped_scripts: Vec::new(),
        errors: Vec::new(),
        warnings: Vec::new(),
        agent_created: false,
        gitignore_action: None,
        hook_action: None,
        source_used: None,
    };

    let home_dir =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;

    // Step 1: Discover source location
    let source_info = discover_source_location(&home_dir, &mut report)?;
    if source_info.is_none() {
        anyhow::bail!("Both flowmates repo and ~/.cursor/ unavailable. Please run 'init-flowmates-config' or 'sync-cursor' first.");
    }
    let source_info = source_info.unwrap();

    // Step 2: Create rules directory and copy rules
    if let Err(e) = copy_rules(
        &source_info.rules_path,
        &PathBuf::from(".cursor/rules/"),
        force,
        &mut report,
    ) {
        report.errors.push(format!("Error copying rules: {}", e));
    }

    // Step 3: Create issue workflow structure
    let project_name = detect_project_name()?;
    create_issue_workflow_structure(&project_name, &mut report)?;

    // Step 4: Copy issue templates
    if let Some(templates_source) = &source_info.templates_path {
        let templates_dest = PathBuf::from("issues/shared/templates/");
        if let Err(e) = copy_templates(templates_source, &templates_dest, force, &mut report) {
            report
                .warnings
                .push(format!("Error copying templates: {}", e));
        }
    }

    // Step 5: Copy scripts directory (optional, for git hooks)
    if let Some(scripts_source) = &source_info.scripts_path {
        if let Err(e) = copy_scripts(scripts_source, force, &mut report) {
            report
                .warnings
                .push(format!("Error copying scripts: {}", e));
        }
    }

    // Step 6: Ensure .cursor is in .gitignore
    ensure_cursor_in_gitignore(&mut report)?;

    // Step 7: Install git hooks (default behavior, skip if --skip-hooks)
    if !skip_hooks {
        install_git_hooks(force, &mut report);
    } else {
        report.hook_action = Some("skipped".to_string());
    }

    // Step 8: Optional: Create AGENT.md
    if !skip_agent {
        let agent_template = if source_info.is_flowmates {
            source_info
                .base_path
                .join("templates/AGENT_REPO.template.md")
        } else {
            home_dir.join(".cursor/templates/AGENT_REPO.template.md")
        };
        if with_agent || !Path::new("AGENT.md").exists() {
            if agent_template.exists() {
                if let Err(e) = create_agent_md(&agent_template, force || with_agent, &mut report) {
                    report
                        .warnings
                        .push(format!("Error creating AGENT.md: {}", e));
                }
            } else if with_agent {
                report.warnings.push(format!(
                    "AGENT template not found at: {}",
                    agent_template.display()
                ));
            }
        }
    }

    // Step 9: Validate setup
    validate_setup(&project_name, &mut report);

    // Step 10: Print summary report
    print_summary_report(&report, &project_name);

    if !report.errors.is_empty() {
        anyhow::bail!("Initialization completed with errors");
    }

    Ok(())
}

struct SourceInfo {
    base_path: PathBuf,
    rules_path: PathBuf,
    templates_path: Option<PathBuf>,
    scripts_path: Option<PathBuf>,
    is_flowmates: bool,
}

fn discover_source_location(
    home_dir: &Path,
    report: &mut InitReport,
) -> Result<Option<SourceInfo>> {
    // Try flowmates repo first
    let flowmates_config = home_dir.join(".flowmates/config.json");
    if flowmates_config.exists() {
        if let Ok(config_content) = fs::read_to_string(&flowmates_config) {
            if let Ok(config) = serde_json::from_str::<Value>(&config_content) {
                if let Some(repo_path_str) = config.get("repo_path").and_then(|v| v.as_str()) {
                    let repo_path = PathBuf::from(repo_path_str);
                    if validate_flowmates_repo(&repo_path) {
                        let rules_path = repo_path.join("rules");
                        let templates_path_primary = repo_path.join("issues/shared/templates");
                        let templates_path_fallback = repo_path.join("docs/issues/templates");
                        let templates_path = if templates_path_primary.exists() {
                            Some(templates_path_primary)
                        } else if templates_path_fallback.exists() {
                            Some(templates_path_fallback)
                        } else {
                            None
                        };
                        let scripts_path = repo_path.join("scripts");
                        let scripts_path = if scripts_path.exists() {
                            Some(scripts_path)
                        } else {
                            None
                        };

                        report.source_used = Some("flowmates".to_string());
                        return Ok(Some(SourceInfo {
                            base_path: repo_path,
                            rules_path,
                            templates_path,
                            scripts_path,
                            is_flowmates: true,
                        }));
                    } else {
                        report.warnings.push(format!(
                            "Flowmates repository path invalid: {}. Using ~/.cursor/ as fallback.",
                            repo_path.display()
                        ));
                    }
                }
            }
        }
    } else {
        report
            .warnings
            .push("Flowmates repository not configured. Using ~/.cursor/ as fallback.".to_string());
    }

    // Fallback to ~/.cursor/
    let cursor_rules = home_dir.join(".cursor/rules");
    if cursor_rules.exists() {
        let templates_path_primary = home_dir.join(".cursor/issues/shared/templates");
        let templates_path_fallback = home_dir.join(".cursor/docs/issues/templates");
        let templates_path = if templates_path_primary.exists() {
            Some(templates_path_primary)
        } else if templates_path_fallback.exists() {
            Some(templates_path_fallback)
        } else {
            None
        };

        report.source_used = Some("cursor".to_string());
        Ok(Some(SourceInfo {
            base_path: home_dir.join(".cursor"),
            rules_path: cursor_rules,
            templates_path,
            scripts_path: None, // Scripts not available in ~/.cursor/
            is_flowmates: false,
        }))
    } else {
        Ok(None)
    }
}

fn validate_flowmates_repo(repo_path: &Path) -> bool {
    if !repo_path.exists() {
        return false;
    }
    let rules_dir = repo_path.join("rules");
    if !rules_dir.exists() {
        return false;
    }
    // Check for .mdc files
    if let Ok(entries) = fs::read_dir(&rules_dir) {
        for entry in entries.flatten() {
            if entry.path().extension().and_then(|s| s.to_str()) == Some("mdc") {
                return true;
            }
        }
    }
    false
}

fn copy_rules(source: &Path, dest: &Path, force: bool, report: &mut InitReport) -> Result<()> {
    fs::create_dir_all(dest)
        .with_context(|| format!("Failed to create directory: {}", dest.display()))?;

    let entries = fs::read_dir(source)
        .with_context(|| format!("Failed to read directory: {}", source.display()))?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("mdc") {
            let file_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| anyhow::anyhow!("Invalid file name"))?;
            let dest_path = dest.join(file_name);

            if dest_path.exists() && !force {
                report.skipped_rules.push(file_name.to_string());
            } else {
                let existed = dest_path.exists();
                fs::copy(&path, &dest_path)
                    .with_context(|| format!("Failed to copy: {}", path.display()))?;
                if existed {
                    report.copied_rules.push(format!("{} (updated)", file_name));
                } else {
                    report.copied_rules.push(file_name.to_string());
                }
            }
        }
    }

    Ok(())
}

fn detect_project_name() -> Result<String> {
    // Try to detect from git remote
    let output = std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            let url = String::from_utf8_lossy(&output.stdout);
            // Extract project name from git URL
            if let Some(name) = url
                .trim()
                .split('/')
                .last()
                .and_then(|s| s.strip_suffix(".git"))
            {
                return Ok(name.to_string());
            }
        }
    }

    // Fallback: try to detect from current directory name
    let current_dir = std::env::current_dir()?;
    if let Some(name) = current_dir.file_name().and_then(|n| n.to_str()) {
        // Special case for "flowmates" repo
        if name == "flowmates" {
            return Ok("flowmates".to_string());
        }
        return Ok(name.to_string());
    }

    Ok("project".to_string())
}

fn create_issue_workflow_structure(project_name: &str, report: &mut InitReport) -> Result<()> {
    let dirs = [
        format!("issues/{}/proposal", project_name),
        format!("issues/{}/todo", project_name),
        format!("issues/{}/in_progress", project_name),
        format!("issues/{}/done", project_name),
        "issues/shared/templates".to_string(),
    ];

    for dir in &dirs {
        if !Path::new(dir).exists() {
            fs::create_dir_all(dir)
                .with_context(|| format!("Failed to create directory: {}", dir))?;
            report.created_dirs.push(dir.clone());
        }
    }

    Ok(())
}

fn copy_templates(source: &Path, dest: &Path, force: bool, report: &mut InitReport) -> Result<()> {
    fs::create_dir_all(dest)
        .with_context(|| format!("Failed to create directory: {}", dest.display()))?;

    let entries = fs::read_dir(source)
        .with_context(|| format!("Failed to read directory: {}", source.display()))?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("md") {
            let file_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| anyhow::anyhow!("Invalid file name"))?;
            let dest_path = dest.join(file_name);

            if dest_path.exists() && !force {
                report.skipped_templates.push(file_name.to_string());
            } else {
                let existed = dest_path.exists();
                fs::copy(&path, &dest_path)
                    .with_context(|| format!("Failed to copy: {}", path.display()))?;
                if existed {
                    report
                        .copied_templates
                        .push(format!("{} (updated)", file_name));
                } else {
                    report.copied_templates.push(file_name.to_string());
                }
            }
        }
    }

    Ok(())
}

fn copy_scripts(scripts_source: &Path, force: bool, report: &mut InitReport) -> Result<()> {
    let scripts_dest = PathBuf::from("scripts");
    fs::create_dir_all(&scripts_dest)
        .with_context(|| format!("Failed to create directory: {}", scripts_dest.display()))?;

    let scripts_to_copy = [
        "pre-commit-hook",
        "validate-workflow-state.py",
        "pre-work-hook", // Optional
    ];

    for script_name in &scripts_to_copy {
        let source_path = scripts_source.join(script_name);
        if source_path.exists() {
            let dest_path = scripts_dest.join(script_name);
            if dest_path.exists() && !force {
                report.skipped_scripts.push(script_name.to_string());
            } else {
                let existed = dest_path.exists();
                fs::copy(&source_path, &dest_path)
                    .with_context(|| format!("Failed to copy: {}", source_path.display()))?;
                if existed {
                    report
                        .copied_scripts
                        .push(format!("{} (updated)", script_name));
                } else {
                    report.copied_scripts.push(script_name.to_string());
                }
            }
        }
    }

    Ok(())
}

fn ensure_cursor_in_gitignore(report: &mut InitReport) -> Result<()> {
    let gitignore_path = Path::new(".gitignore");

    if !gitignore_path.exists() {
        // Create .gitignore with .cursor entry
        let mut file =
            fs::File::create(gitignore_path).with_context(|| "Failed to create .gitignore")?;
        writeln!(file, "# Cursor agent state and cache")?;
        writeln!(file, ".cursor/")?;
        report.gitignore_action = Some("created".to_string());
        return Ok(());
    }

    // Check if .cursor entry already exists
    let content =
        fs::read_to_string(gitignore_path).with_context(|| "Failed to read .gitignore")?;

    let lines: Vec<&str> = content.lines().collect();
    let has_cursor_entry = lines.iter().any(|line| {
        let trimmed = line.trim();
        // Skip comments and empty lines
        if trimmed.is_empty() || trimmed.starts_with("#") {
            return false;
        }
        // Match exact .cursor or .cursor/ or patterns starting with .cursor (case-insensitive)
        trimmed.eq_ignore_ascii_case(".cursor")
            || trimmed.eq_ignore_ascii_case(".cursor/")
            || trimmed.to_lowercase().starts_with(".cursor")
    });

    if has_cursor_entry {
        report.gitignore_action = Some("skipped".to_string());
    } else {
        // Append .cursor entry
        let mut file = fs::OpenOptions::new()
            .append(true)
            .open(gitignore_path)
            .with_context(|| "Failed to open .gitignore for appending")?;
        writeln!(file, "\n# Cursor agent state and cache")?;
        writeln!(file, ".cursor/")?;
        report.gitignore_action = Some("added".to_string());
    }

    Ok(())
}

fn install_git_hooks(force: bool, report: &mut InitReport) {
    let hook_template = Path::new("scripts/pre-commit-hook");
    let git_hooks_dir = Path::new(".git/hooks");
    let hook_dest = git_hooks_dir.join("pre-commit");

    if !hook_template.exists() {
        report.hook_action = Some("not_found".to_string());
        return;
    }

    if !git_hooks_dir.exists() {
        report.hook_action = Some("not_git".to_string());
        return;
    }

    let hook_existed = hook_dest.exists();
    if hook_existed && !force {
        report.hook_action = Some("skipped".to_string());
        return;
    }

    // Copy hook
    if let Err(e) = fs::copy(hook_template, &hook_dest) {
        report
            .errors
            .push(format!("Failed to copy git hook: {}", e));
        return;
    }

    // Make hook executable
    if let Err(e) = fs::set_permissions(&hook_dest, fs::Permissions::from_mode(0o755)) {
        report
            .warnings
            .push(format!("Failed to make hook executable: {}", e));
    }

    if hook_existed {
        report.hook_action = Some("updated".to_string());
    } else {
        report.hook_action = Some("installed".to_string());
    }
}

fn create_agent_md(template_path: &Path, force: bool, report: &mut InitReport) -> Result<()> {
    let dest_path = Path::new("AGENT.md");

    if dest_path.exists() && !force {
        return Ok(());
    }

    let content = fs::read_to_string(template_path)
        .with_context(|| format!("Failed to read template: {}", template_path.display()))?;

    // Replace placeholders (basic implementation - can be enhanced)
    let project_name = detect_project_name().unwrap_or_else(|_| "project".to_string());
    let content = content.replace("{{PROJECT_NAME}}", &project_name);

    fs::write(dest_path, content)
        .with_context(|| format!("Failed to write: {}", dest_path.display()))?;

    report.agent_created = true;
    Ok(())
}

fn validate_setup(project_name: &str, report: &mut InitReport) {
    // Check rules directory
    let rules_dir = Path::new(".cursor/rules/");
    if !rules_dir.exists() {
        report
            .errors
            .push("Rules directory not found: .cursor/rules/".to_string());
    } else {
        let mdc_count = fs::read_dir(rules_dir)
            .ok()
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("mdc"))
                    .count()
            })
            .unwrap_or(0);
        if mdc_count == 0 {
            report
                .warnings
                .push("No .mdc rule files found in .cursor/rules/".to_string());
        }
    }

    // Check issue workflow directories
    let required_dirs = [
        format!("issues/{}/proposal", project_name),
        format!("issues/{}/todo", project_name),
        format!("issues/{}/in_progress", project_name),
        format!("issues/{}/done", project_name),
        "issues/shared/templates".to_string(),
    ];

    for dir in &required_dirs {
        if !Path::new(dir).exists() {
            report
                .errors
                .push(format!("Required directory missing: {}", dir));
        }
    }

    // Check templates
    let templates_dir = Path::new("issues/shared/templates/");
    if templates_dir.exists() {
        let md_count = fs::read_dir(templates_dir)
            .ok()
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("md"))
                    .count()
            })
            .unwrap_or(0);
        if md_count == 0 {
            report
                .warnings
                .push("No template files found in issues/shared/templates/".to_string());
        }
    }
}

fn print_summary_report(report: &InitReport, project_name: &str) {
    println!("\n=== Initialization Summary ===\n");

    // Source used
    if let Some(source) = &report.source_used {
        println!(
            "Source: {} repository",
            if source == "flowmates" {
                "Flowmates"
            } else {
                "~/.cursor/"
            }
        );
        println!();
    }

    if !report.created_dirs.is_empty() {
        println!("Created directories:");
        for dir in &report.created_dirs {
            println!("  ✓ {}", dir);
        }
        println!();
    }

    if !report.copied_rules.is_empty() {
        println!("Copied/Updated rules:");
        for rule in &report.copied_rules {
            println!("  ✓ {}", rule);
        }
        println!();
    }

    if !report.skipped_rules.is_empty() {
        println!("Skipped rules (already exist):");
        for rule in &report.skipped_rules {
            println!("  ⊘ {}", rule);
        }
        println!();
    }

    if !report.copied_templates.is_empty() {
        println!("Copied/Updated templates:");
        for template in &report.copied_templates {
            println!("  ✓ {}", template);
        }
        println!();
    }

    if !report.skipped_templates.is_empty() {
        println!("Skipped templates (already exist):");
        for template in &report.skipped_templates {
            println!("  ⊘ {}", template);
        }
        println!();
    }

    if !report.copied_scripts.is_empty() {
        println!("Copied/Updated scripts:");
        for script in &report.copied_scripts {
            println!("  ✓ {}", script);
        }
        println!();
    }

    if !report.skipped_scripts.is_empty() {
        println!("Skipped scripts (already exist):");
        for script in &report.skipped_scripts {
            println!("  ⊘ {}", script);
        }
        println!();
    }

    if let Some(action) = &report.gitignore_action {
        match action.as_str() {
            "created" => println!("✓ Created .gitignore with .cursor entry\n"),
            "added" => println!("✓ Added .cursor entry to .gitignore\n"),
            "skipped" => println!("⊘ .cursor already in .gitignore (skipped)\n"),
            _ => {}
        }
    }

    if let Some(action) = &report.hook_action {
        match action.as_str() {
            "installed" => println!("✅ Git pre-commit hook installed\n"),
            "updated" => println!("✅ Git pre-commit hook updated\n"),
            "skipped" => println!("⚠️  Git pre-commit hook already exists (skipped)\n"),
            "not_found" => println!("⚠️  scripts/pre-commit-hook not found. Run init from flowmates repo or ensure scripts/ directory is available.\n"),
            "not_git" => println!("⚠️  Not a git repository, skipping hook installation\n"),
            _ => {}
        }
    }

    if report.agent_created {
        println!("✓ Created AGENT.md from template\n");
    }

    if !report.warnings.is_empty() {
        println!("Warnings:");
        for warning in &report.warnings {
            println!("  ⚠ {}", warning);
        }
        println!();
    }

    if !report.errors.is_empty() {
        println!("Errors:");
        for error in &report.errors {
            println!("  ✗ {}", error);
        }
        println!();
    }

    println!("Project name detected: {}\n", project_name);

    if report.errors.is_empty() {
        println!("✓ Initialization completed successfully!");
        println!("\nNext steps:");
        println!("  - Run `load-context` to bootstrap repository context");
    } else {
        println!("⚠ Initialization completed with errors. Please review above.");
    }
}

fn init_flowmates_config_command(
    path: Option<String>,
    file: String,
    force: bool,
    dry_run: bool,
    no_validate: bool,
) -> Result<()> {
    // TODO: Implement init-flowmates-config command
    println!("init-flowmates-config command not yet implemented");
    println!("  path: {:?}", path);
    println!("  file: {}", file);
    println!("  force: {}", force);
    println!("  dry_run: {}", dry_run);
    println!("  no_validate: {}", no_validate);
    Ok(())
}
