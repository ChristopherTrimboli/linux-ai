//! `la` — the Linux AI Companion command line.
//!
//! Usage:
//!   la "what is using port 8080?"      one-shot prompt
//!   cat file | la "summarize this"      pipe stdin into the prompt
//!   la chat                             interactive REPL
//!   la providers | la models            inspect configuration
//!   la auth <provider>                  store an API key in the OS keyring

use std::io::{IsTerminal, Read, Write};
use std::sync::Arc;

use clap::{Parser, Subcommand};
use la_core::tools::{ApprovalDecision, Approver};
use la_core::{Agent, AgentEvent, Config, Store};
use tokio::sync::mpsc;

#[derive(Parser)]
#[command(name = "la", version, about = "AI companion for your Linux desktop")]
struct Cli {
    /// Provider to use (overrides config default).
    #[arg(short, long, global = true)]
    provider: Option<String>,
    /// Model to use (overrides config default).
    #[arg(short, long, global = true)]
    model: Option<String>,
    /// Auto-approve all tool calls (skip prompts).
    #[arg(short = 'y', long, global = true)]
    yes: bool,

    #[command(subcommand)]
    command: Option<Command>,

    /// One-shot prompt (used when no subcommand is given).
    #[arg(trailing_var_arg = true)]
    prompt: Vec<String>,
}

#[derive(Subcommand)]
enum Command {
    /// Start an interactive chat session.
    Chat,
    /// Print the config file path and contents.
    Config,
    /// List configured providers and whether a key is set.
    Providers,
    /// List known models for a provider.
    Models {
        /// Provider name (defaults to the configured default).
        provider: Option<String>,
    },
    /// Store an API key for a provider in the OS keyring.
    Auth {
        /// Provider name, e.g. `anthropic` or `openai`.
        provider: String,
    },
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "warn".into()),
        )
        .with_writer(std::io::stderr)
        .init();

    if let Err(e) = run().await {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

async fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let mut config = Config::load()?;
    if cli.yes {
        config.tools.auto_approve = true;
    }

    match cli.command {
        Some(Command::Config) => cmd_config(&config),
        Some(Command::Providers) => cmd_providers(&config),
        Some(Command::Models { provider }) => cmd_models(&config, provider.as_deref()),
        Some(Command::Auth { provider }) => cmd_auth(&config, &provider),
        Some(Command::Chat) => {
            chat_repl(config, cli.provider.as_deref(), cli.model.as_deref()).await
        }
        None => {
            let mut prompt = cli.prompt.join(" ");
            let piped = read_piped_stdin();
            if let Some(piped) = piped {
                if prompt.is_empty() {
                    prompt = piped;
                } else {
                    prompt = format!("{prompt}\n\n{piped}");
                }
            }
            if prompt.trim().is_empty() {
                chat_repl(config, cli.provider.as_deref(), cli.model.as_deref()).await
            } else {
                one_shot(config, cli.provider.as_deref(), cli.model.as_deref(), &prompt).await
            }
        }
    }
}

fn read_piped_stdin() -> Option<String> {
    let stdin = std::io::stdin();
    if stdin.is_terminal() {
        return None;
    }
    let mut buf = String::new();
    if stdin.lock().read_to_string(&mut buf).is_ok() && !buf.trim().is_empty() {
        Some(buf)
    } else {
        None
    }
}

fn cmd_config(config: &Config) -> anyhow::Result<()> {
    println!("config file: {}", Config::config_path()?.display());
    println!("db file:     {}", Store::data_path()?.display());
    println!("\n{}", toml::to_string_pretty(config)?);
    Ok(())
}

fn cmd_providers(config: &Config) -> anyhow::Result<()> {
    println!("default provider: {}", config.default_provider);
    println!("default model:    {}\n", config.default_model);
    for (name, pc) in &config.providers {
        let key = la_core::secrets::resolve_api_key(name, pc);
        let status = if key.is_some() { "key set" } else { "no key" };
        println!(
            "{name:12} {:9} {:35} [{status}]",
            format!("{:?}", pc.kind).to_lowercase(),
            pc.effective_base_url(),
        );
    }
    Ok(())
}

fn cmd_models(config: &Config, provider: Option<&str>) -> anyhow::Result<()> {
    let name = provider.unwrap_or(&config.default_provider);
    let pc = config.provider(name)?;
    println!("models for {name}:");
    for m in &pc.models {
        println!("  {m}");
    }
    Ok(())
}

fn cmd_auth(config: &Config, provider: &str) -> anyhow::Result<()> {
    config.provider(provider)?; // validate it exists
    print!("Enter API key for '{provider}' (input is visible): ");
    std::io::stdout().flush()?;
    let mut key = String::new();
    std::io::stdin().read_line(&mut key)?;
    let key = key.trim();
    if key.is_empty() {
        anyhow::bail!("no key entered");
    }
    la_core::secrets::store_api_key(provider, key)?;
    println!("stored key for '{provider}' in the OS keyring.");
    Ok(())
}

/// Interactive y/N approval prompt for mutating tools.
fn cli_approver() -> Approver {
    Arc::new(|req| {
        Box::pin(async move {
            let approved = tokio::task::spawn_blocking(move || {
                print!("\n  \u{2192} allow `{}`? [y/N] ", req.summary);
                std::io::stdout().flush().ok();
                let mut line = String::new();
                std::io::stdin().read_line(&mut line).ok();
                line.trim().eq_ignore_ascii_case("y")
            })
            .await
            .unwrap_or(false);
            if approved {
                ApprovalDecision::Approve
            } else {
                ApprovalDecision::Deny
            }
        })
    })
}

async fn drive_turn(agent: Arc<Agent>, conversation_id: String, input: String) {
    let (tx, mut rx) = mpsc::unbounded_channel::<AgentEvent>();
    let approver = cli_approver();
    let agent2 = agent.clone();
    let convo = conversation_id.clone();
    let handle = tokio::spawn(async move {
        let _ = agent2.run_turn(&convo, &input, approver, tx).await;
    });

    let mut stdout = std::io::stdout();
    while let Some(event) = rx.recv().await {
        match event {
            AgentEvent::TextDelta(t) => {
                print!("{t}");
                let _ = stdout.flush();
            }
            AgentEvent::ToolStarted { name, summary, .. } => {
                print!("\n\x1b[2m[{name}] {summary}\x1b[0m\n");
                let _ = stdout.flush();
            }
            AgentEvent::ToolCompleted {
                name,
                output,
                is_error,
                ..
            } => {
                let first = output.lines().next().unwrap_or("");
                let mark = if is_error { "\u{2717}" } else { "\u{2713}" };
                println!("\x1b[2m  {mark} {name}: {first}\x1b[0m");
            }
            AgentEvent::ApprovalRequired { .. } => {}
            AgentEvent::Usage(_) => {}
            AgentEvent::Done => {
                println!();
                break;
            }
            AgentEvent::Error(e) => {
                eprintln!("\n\x1b[31merror: {e}\x1b[0m");
                break;
            }
        }
    }
    let _ = handle.await;
}

async fn one_shot(
    config: Config,
    provider: Option<&str>,
    model: Option<&str>,
    prompt: &str,
) -> anyhow::Result<()> {
    let store = Store::open_default()?;
    let agent = Arc::new(Agent::from_config(&config, store.clone(), provider, model)?);
    let convo = store.create_conversation(&truncate_title(prompt))?;
    drive_turn(agent, convo.id, prompt.to_string()).await;
    Ok(())
}

async fn chat_repl(
    config: Config,
    provider: Option<&str>,
    model: Option<&str>,
) -> anyhow::Result<()> {
    let store = Store::open_default()?;
    let agent = Arc::new(Agent::from_config(&config, store.clone(), provider, model)?);
    let convo = store.create_conversation("chat")?;

    println!(
        "Linux AI Companion — provider: {}, model: {}",
        provider.unwrap_or(&config.default_provider),
        model.unwrap_or(&config.default_model),
    );
    println!("Type your message. Ctrl-D or 'exit' to quit.\n");

    let mut rl = rustyline::DefaultEditor::new()?;
    let mut first = true;
    loop {
        match rl.readline("\x1b[36myou>\x1b[0m ") {
            Ok(line) => {
                let line = line.trim().to_string();
                if line.is_empty() {
                    continue;
                }
                if line == "exit" || line == "quit" {
                    break;
                }
                let _ = rl.add_history_entry(line.as_str());
                if first {
                    let _ = store.rename_conversation(&convo.id, &truncate_title(&line));
                    first = false;
                }
                drive_turn(agent.clone(), convo.id.clone(), line).await;
            }
            Err(rustyline::error::ReadlineError::Interrupted) => continue,
            Err(rustyline::error::ReadlineError::Eof) => break,
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}

fn truncate_title(s: &str) -> String {
    let t: String = s.chars().take(60).collect();
    t.replace('\n', " ")
}
