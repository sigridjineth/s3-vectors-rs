use crate::cli::{Commands, OutputFormat};
use crate::S3VectorsClient;
use anyhow::Result;
use clap::Parser;
use colored::*;
use dialoguer::{theme::ColorfulTheme, Input};

const ASCII_BANNER: &str = r#"
╔═══════════════════════════════════════════════════════════════════════════════╗
║                                                                               ║
║    ███████╗██████╗    ██╗   ██╗███████╗ ██████╗████████╗ ██████╗ ██████╗      ║
║    ██╔════╝╚════██╗   ██║   ██║██╔════╝██╔════╝╚══██╔══╝██╔═══██╗██╔══██╗     ║
║    ███████╗ █████╔╝   ██║   ██║█████╗  ██║        ██║   ██║   ██║██████╔╝     ║
║    ╚════██║ ╚═══██╗   ╚██╗ ██╔╝██╔══╝  ██║        ██║   ██║   ██║██╔══██╗     ║
║    ███████║██████╔╝    ╚████╔╝ ███████╗╚██████╗   ██║   ╚██████╔╝██║  ██║     ║
║    ╚══════╝╚═════╝      ╚═══╝  ╚══════╝ ╚═════╝   ╚═╝    ╚═════╝ ╚═╝  ╚═╝     ║
║                                                                               ║
║                    AWS S3 Vectors CLI by @sigridjineth                        ║
║                                                                               ║
╚═══════════════════════════════════════════════════════════════════════════════╝
"#;

pub struct InteractiveMode {
    client: S3VectorsClient,
    output_format: OutputFormat,
    verbose: bool,
}

impl InteractiveMode {
    pub fn new(client: S3VectorsClient, output_format: OutputFormat, verbose: bool) -> Self {
        Self {
            client,
            output_format,
            verbose,
        }
    }

    pub async fn run(mut self) -> Result<()> {
        self.display_banner();
        self.display_tips();

        loop {
            let input = Input::<String>::with_theme(&ColorfulTheme::default())
                .with_prompt("s3-vectors>")
                .interact_text()?;

            let input = input.trim();

            // Handle special commands
            match input {
                "" => continue,
                "exit" | "quit" | "/exit" | "/quit" => {
                    println!("Goodbye!");
                    break;
                }
                "help" | "/help" | "?" => {
                    self.display_help();
                    continue;
                }
                "clear" | "/clear" => {
                    self.clear_screen();
                    continue;
                }
                "init" => {
                    // Handle init command specially
                    let init_cmd = crate::cli::init::InitCommand;
                    match init_cmd.execute_interactive().await {
                        Ok(Some(new_client)) => {
                            self.client = new_client;
                            println!("\n{} Credentials configured successfully! You can now use S3 Vectors commands.\n", "✓".green());
                        }
                        Ok(None) => {
                            println!("\n{} Init cancelled or skipped.\n", "ℹ".yellow());
                        }
                        Err(e) => {
                            eprintln!("{} Failed to initialize: {}", "Error:".red(), e);
                        }
                    }
                    continue;
                }
                _ => {}
            }

            // Parse and execute command
            if let Err(e) = self.execute_command(input).await {
                eprintln!("{} {}", "Error:".red(), e);
            }
        }

        Ok(())
    }

    fn display_banner(&self) {
        println!("{}", ASCII_BANNER.cyan());
        println!();
    }

    fn display_tips(&self) {
        println!(
            "{}",
            "  Welcome to the unofficial S3 Vectors interactive CLI mode!".green()
        );
        println!();
        println!("{}", "  Quick Start:".yellow().bold());
        println!("  ╭─────────────────────────────────────────────────────────────────────╮");
        println!(
            "  │ • Create a bucket:  {:<47} │",
            "bucket create my-vectors".cyan()
        );
        println!(
            "  │ • Query buckets:    {:<47} │",
            "bucket query prod --status active".cyan()
        );
        println!(
            "  │ • Create an index:  {:<47} │",
            "index create my-bucket my-index -d 384".cyan()
        );
        println!(
            "  │ • Query indexes:    {:<47} │",
            "index list my-bucket --query \"embeddings\"".cyan()
        );
        println!(
            "  │ • Add vectors:      {:<47} │",
            "vector put my-bucket my-index key1 -d 0.1,0.2".cyan()
        );
        println!(
            "  │ • Search vectors:   {:<47} │",
            "vector query my-bucket my-index -q 0.1,0.2".cyan()
        );
        println!(
            "  │ • RAG ingest:       {:<47} │",
            "rag ingest --directory ./docs".cyan()
        );
        println!(
            "  │ • RAG query:        {:<47} │",
            "rag query 'What is S3 Vectors?'".cyan()
        );
        println!("  ╰─────────────────────────────────────────────────────────────────────╯");
        println!();
        println!(
            "  {} Type {} for available commands or {} to quit.",
            "💡".yellow(),
            "help".green().bold(),
            "exit".red().bold()
        );
        println!();
    }

    fn display_help(&self) {
        println!();
        println!(
            "{}",
            "╔══════════════════════════════════════════════════════════════════════════════╗"
                .blue()
        );
        println!(
            "{}",
            "║                            AVAILABLE COMMANDS                                ║"
                .blue()
        );
        println!(
            "{}",
            "╠══════════════════════════════════════════════════════════════════════════════╣"
                .blue()
        );
        println!(
            "{}",
            "║                                                                              ║"
                .blue()
        );
        println!(
            "{} {} {:<60} {}",
            "║".blue(),
            "bucket".cyan().bold(),
            "- Manage vector buckets",
            "║".blue()
        );
        println!(
            "{} {:<72} {}",
            "║".blue(),
            "       create, list, query, get, delete",
            "║".blue()
        );
        println!(
            "{}",
            "║                                                                              ║"
                .blue()
        );
        println!(
            "{} {} {:<60} {}",
            "║".blue(),
            "index ".cyan().bold(),
            "- Manage vector indexes",
            "║".blue()
        );
        println!(
            "{} {:<72} {}",
            "║".blue(),
            "       create, list [--query \"search\"], get, delete",
            "║".blue()
        );
        println!(
            "{}",
            "║                                                                              ║"
                .blue()
        );
        println!(
            "{} {} {:<60} {}",
            "║".blue(),
            "vector".cyan().bold(),
            "- Manage vectors",
            "║".blue()
        );
        println!(
            "{} {:<72} {}",
            "║".blue(),
            "       put, get, list, delete, query",
            "║".blue()
        );
        println!(
            "{}",
            "║                                                                              ║"
                .blue()
        );
        println!(
            "{} {} {:<60} {}",
            "║".blue(),
            "policy".cyan().bold(),
            "- Manage bucket policies",
            "║".blue()
        );
        println!(
            "{} {:<72} {}",
            "║".blue(),
            "       put, get, delete",
            "║".blue()
        );
        println!(
            "{}",
            "║                                                                              ║"
                .blue()
        );
        println!(
            "{} {} {:<64} {}",
            "║".blue(),
            "rag   ".cyan().bold(),
            "- RAG operations",
            "║".blue()
        );
        println!(
            "{} {:<72} {}",
            "║".blue(),
            "       init, ingest, query, interactive",
            "║".blue()
        );
        println!(
            "{}",
            "║                                                                              ║"
                .blue()
        );
        println!(
            "{}",
            "╠══════════════════════════════════════════════════════════════════════════════╣"
                .blue()
        );
        println!(
            "{}",
            "║                            SPECIAL COMMANDS                                  ║"
                .blue()
        );
        println!(
            "{}",
            "╠══════════════════════════════════════════════════════════════════════════════╣"
                .blue()
        );
        println!(
            "{}",
            "║                                                                              ║"
                .blue()
        );
        println!(
            "{} {:<20} {:<57} {}",
            "║".blue(),
            "init".yellow(),
            "- Configure AWS credentials",
            "║".blue()
        );
        println!(
            "{} {:<20} {:<57} {}",
            "║".blue(),
            "install-models".yellow(),
            "- Download ML models for RAG",
            "║".blue()
        );
        println!(
            "{} {:<20} {:<57} {}",
            "║".blue(),
            "help, /help, ?".yellow(),
            "- Show this help",
            "║".blue()
        );
        println!(
            "{} {:<20} {:<57} {}",
            "║".blue(),
            "clear, /clear".yellow(),
            "- Clear the screen",
            "║".blue()
        );
        println!(
            "{} {:<20} {:<57} {}",
            "║".blue(),
            "exit, quit".yellow(),
            "- Exit interactive mode",
            "║".blue()
        );
        println!(
            "{}",
            "║                                                                              ║"
                .blue()
        );
        println!(
            "{}",
            "╚══════════════════════════════════════════════════════════════════════════════╝"
                .blue()
        );
        println!();
    }

    fn clear_screen(&self) {
        print!("\x1B[2J\x1B[1;1H");
    }

    /// Parse command arguments handling quoted strings properly
    fn parse_command_args(&self, input: &str) -> Result<Vec<String>> {
        let mut args = Vec::new();
        let mut current_arg = String::new();
        let mut in_quotes = false;
        let chars = input.chars().peekable();
        let mut escape_next = false;

        for ch in chars {
            if escape_next {
                // If we're escaping, just add the character as-is
                current_arg.push(ch);
                escape_next = false;
                continue;
            }

            match ch {
                '\\' if in_quotes => {
                    // Start escape sequence
                    escape_next = true;
                    current_arg.push(ch); // Keep the backslash
                }
                '"' => {
                    in_quotes = !in_quotes;
                    // Don't include the quotes in the argument
                }
                ' ' if !in_quotes => {
                    if !current_arg.is_empty() {
                        args.push(current_arg);
                        current_arg = String::new();
                    }
                }
                _ => {
                    current_arg.push(ch);
                }
            }
        }

        // Push the last argument if any
        if !current_arg.is_empty() {
            args.push(current_arg);
        }

        // Check for unclosed quotes
        if in_quotes {
            return Err(anyhow::anyhow!("Unclosed quote in command"));
        }

        Ok(args)
    }

    async fn execute_command(&self, input: &str) -> Result<()> {
        // Prepend "s3-vectors" to make it parseable by clap
        let args_str = format!("s3-vectors {input}");
        let args = self.parse_command_args(&args_str)?;

        // Parse the command using a temporary CLI struct for commands only
        #[derive(Parser)]
        struct TempCli {
            #[command(subcommand)]
            command: Commands,
        }

        match TempCli::try_parse_from(args.iter().map(|s| s.as_str())) {
            Ok(parsed) => {
                // Execute the command
                match parsed.command {
                    Commands::Init(cmd) => {
                        cmd.execute().await?;
                    }
                    Commands::InstallModels(cmd) => {
                        cmd.execute().await?;
                    }
                    Commands::Bucket(cmd) => cmd.execute(&self.client, self.output_format).await?,
                    Commands::Index(cmd) => cmd.execute(&self.client, self.output_format).await?,
                    Commands::Vector(cmd) => cmd.execute(&self.client, self.output_format).await?,
                    Commands::Policy(cmd) => cmd.execute(&self.client, self.output_format).await?,
                    Commands::Rag(cmd) => cmd.execute(&self.client, self.output_format).await?,
                }
            }
            Err(e) => {
                // Show a more user-friendly error for interactive mode
                eprintln!(
                    "{} Invalid command. Type 'help' for available commands.",
                    "Error:".red()
                );
                if self.verbose {
                    eprintln!("Details: {e}");
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::bucket::*;
    use crate::cli::index::*;
    use crate::cli::policy::*;
    use crate::cli::vector::*;

    #[test]
    fn test_interactive_mode_creation() {
        let client = S3VectorsClient::new("us-east-1");
        let interactive = InteractiveMode::new(client, OutputFormat::Table, false);
        assert_eq!(interactive.output_format as i32, OutputFormat::Table as i32);
        assert!(!interactive.verbose);
    }

    #[test]
    fn test_parse_bucket_list_command() {
        // Test that we can parse "bucket list" in interactive mode
        let args = "s3-vectors bucket list";
        let args: Vec<&str> = args.split_whitespace().collect();

        #[derive(Parser)]
        struct TempCli {
            #[command(subcommand)]
            command: Commands,
        }

        let parsed = TempCli::try_parse_from(&args).unwrap();
        match parsed.command {
            Commands::Bucket(BucketCommand {
                command: BucketSubcommands::List { .. },
            }) => {}
            _ => panic!("Expected bucket list command"),
        }
    }

    #[test]
    fn test_parse_command_args() {
        let interactive = InteractiveMode::new(
            S3VectorsClient::from_env().unwrap(),
            OutputFormat::Table,
            false,
        );

        // Test simple args
        let args = interactive
            .parse_command_args("s3-vectors bucket list")
            .unwrap();
        assert_eq!(args, vec!["s3-vectors", "bucket", "list"]);

        // Test quoted args
        let args = interactive
            .parse_command_args("s3-vectors index list my-bucket --query \"how many apples\"")
            .unwrap();
        assert_eq!(
            args,
            vec![
                "s3-vectors",
                "index",
                "list",
                "my-bucket",
                "--query",
                "how many apples"
            ]
        );

        // Test multiple quoted args (escape sequences are preserved but quotes are removed)
        let args = interactive
            .parse_command_args(
                "s3-vectors vector put bucket index key -m \"{\\\"key\\\": \\\"value\\\"}\"",
            )
            .unwrap();
        assert_eq!(args.len(), 8);
        assert_eq!(args[7], "{\\\"key\\\": \\\"value\\\"}");

        // Test unclosed quote error
        let result = interactive.parse_command_args("s3-vectors index list --query \"unclosed");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_index_create_command() {
        // Test parsing "index create my-bucket my-index -d 384"
        let args = "s3-vectors index create my-bucket my-index -d 384";
        let args: Vec<&str> = args.split_whitespace().collect();

        #[derive(Parser)]
        struct TempCli {
            #[command(subcommand)]
            command: Commands,
        }

        let parsed = TempCli::try_parse_from(&args).unwrap();
        match parsed.command {
            Commands::Index(IndexCommand {
                command:
                    IndexSubcommands::Create {
                        bucket,
                        name,
                        dimensions,
                        ..
                    },
            }) => {
                assert_eq!(bucket, "my-bucket");
                assert_eq!(name, "my-index");
                assert_eq!(dimensions, 384);
            }
            _ => panic!("Expected index create command"),
        }
    }

    #[test]
    fn test_parse_index_list_with_query() {
        // Test parsing "index list my-bucket --query \"document embeddings\""
        let interactive = InteractiveMode::new(
            S3VectorsClient::from_env().unwrap(),
            OutputFormat::Table,
            false,
        );
        let args = interactive
            .parse_command_args("s3-vectors index list my-bucket --query \"document embeddings\"")
            .unwrap();

        #[derive(Parser)]
        struct TempCli {
            #[command(subcommand)]
            command: Commands,
        }

        let parsed = TempCli::try_parse_from(args.iter().map(|s| s.as_str())).unwrap();
        match parsed.command {
            Commands::Index(IndexCommand {
                command: IndexSubcommands::List { bucket, query, .. },
            }) => {
                assert_eq!(bucket, "my-bucket");
                assert_eq!(query.unwrap(), "document embeddings");
            }
            _ => panic!("Expected index list command with query"),
        }
    }

    #[test]
    fn test_parse_vector_query_command() {
        // Test parsing complex query command
        let args = "s3-vectors vector query my-bucket my-index -q 0.1,0.2,0.3 -t 10";
        let args: Vec<&str> = args.split_whitespace().collect();

        #[derive(Parser)]
        struct TempCli {
            #[command(subcommand)]
            command: Commands,
        }

        let parsed = TempCli::try_parse_from(&args).unwrap();
        match parsed.command {
            Commands::Vector(VectorCommand {
                command:
                    VectorSubcommands::Query {
                        bucket,
                        index,
                        vector,
                        top_k,
                        ..
                    },
            }) => {
                assert_eq!(bucket, "my-bucket");
                assert_eq!(index, "my-index");
                assert_eq!(vector, "0.1,0.2,0.3");
                assert_eq!(top_k, 10);
            }
            _ => panic!("Expected vector query command"),
        }
    }

    #[test]
    fn test_invalid_command_handling() {
        // Test that invalid commands are handled gracefully
        let args = "s3-vectors invalid command";
        let args: Vec<&str> = args.split_whitespace().collect();

        #[derive(Parser)]
        struct TempCli {
            #[command(subcommand)]
            command: Commands,
        }

        let result = TempCli::try_parse_from(&args);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_policy_put_command() {
        // Test parsing "policy put my-bucket --file policy.json"
        let args = "s3-vectors policy put my-bucket --file policy.json";
        let args: Vec<&str> = args.split_whitespace().collect();

        #[derive(Parser)]
        struct TempCli {
            #[command(subcommand)]
            command: Commands,
        }

        let parsed = TempCli::try_parse_from(&args).unwrap();
        match parsed.command {
            Commands::Policy(PolicyCommand {
                command:
                    PolicySubcommands::Put {
                        bucket,
                        policy,
                        file,
                    },
            }) => {
                assert_eq!(bucket, "my-bucket");
                assert!(policy.is_none());
                assert_eq!(file, Some("policy.json".to_string()));
            }
            _ => panic!("Expected policy put command"),
        }
    }

    #[test]
    fn test_parse_vector_put_with_metadata() {
        // Test parsing vector put with metadata
        let args = r#"s3-vectors vector put my-bucket my-index key1 -d 0.1,0.2,0.3 -m {"category":"test"}"#;
        let args: Vec<&str> = args.split_whitespace().collect();

        #[derive(Parser)]
        struct TempCli {
            #[command(subcommand)]
            command: Commands,
        }

        let parsed = TempCli::try_parse_from(&args).unwrap();
        match parsed.command {
            Commands::Vector(VectorCommand {
                command:
                    VectorSubcommands::Put {
                        bucket,
                        index,
                        key,
                        data,
                        metadata,
                        ..
                    },
            }) => {
                assert_eq!(bucket, "my-bucket");
                assert_eq!(index, "my-index");
                assert_eq!(key, "key1");
                assert_eq!(data, "0.1,0.2,0.3");
                assert_eq!(metadata, Some(r#"{"category":"test"}"#.to_string()));
            }
            _ => panic!("Expected vector put command"),
        }
    }

    #[test]
    fn test_parse_bucket_delete_force() {
        // Test parsing bucket delete with force flag
        let args = "s3-vectors bucket delete my-bucket --force";
        let args: Vec<&str> = args.split_whitespace().collect();

        #[derive(Parser)]
        struct TempCli {
            #[command(subcommand)]
            command: Commands,
        }

        let parsed = TempCli::try_parse_from(&args).unwrap();
        match parsed.command {
            Commands::Bucket(BucketCommand {
                command: BucketSubcommands::Delete { name, force },
            }) => {
                assert_eq!(name, "my-bucket");
                assert!(force);
            }
            _ => panic!("Expected bucket delete command"),
        }
    }

    #[test]
    fn test_parse_bucket_query_command() {
        // Test parsing bucket query with multiple filters
        let args = "s3-vectors bucket query prod --status active --created-after yesterday";
        let args: Vec<&str> = args.split_whitespace().collect();

        #[derive(Parser)]
        struct TempCli {
            #[command(subcommand)]
            command: Commands,
        }

        let parsed = TempCli::try_parse_from(&args).unwrap();
        match parsed.command {
            Commands::Bucket(BucketCommand {
                command:
                    BucketSubcommands::Query {
                        pattern,
                        status,
                        created_after,
                        ..
                    },
            }) => {
                assert_eq!(pattern, Some("prod".to_string()));
                assert_eq!(status, Some(crate::types::BucketStatus::Active));
                assert_eq!(created_after, Some("yesterday".to_string()));
            }
            _ => panic!("Expected bucket query command"),
        }
    }
}
