use crate::S3VectorsClient;
use anyhow::{Context, Result};
use colored::*;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Password, Select};
use std::fmt::Write;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

#[derive(Debug, clap::Args)]
pub struct InitCommand;

impl InitCommand {
    /// Execute init command from CLI
    pub async fn execute(&self) -> Result<()> {
        println!("{}", "\nWelcome to S3 Vectors Setup! 🚀\n".cyan().bold());

        // Run the interactive setup
        self.execute_interactive().await?;

        Ok(())
    }

    /// Execute init command from interactive mode, returning configured client
    pub async fn execute_interactive(&self) -> Result<Option<S3VectorsClient>> {
        // Check current configuration
        let has_env_creds = std::env::var("AWS_ACCESS_KEY_ID").is_ok()
            && std::env::var("AWS_SECRET_ACCESS_KEY").is_ok();
        let current_region =
            std::env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".to_string());

        println!("{}", "Current configuration:".yellow());
        if has_env_creds {
            println!("  {} Credentials found in environment", "✓".green());
        } else {
            println!("  {} No credentials found", "✗".red());
        }
        println!("  {} Region: {}", "✓".green(), current_region);
        println!();

        // Ask how to configure
        let options = vec![
            "Enter AWS access keys",
            "Use existing AWS profile",
            "Show environment variable setup",
            "Skip (I'll configure manually)",
        ];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("How would you like to configure credentials?")
            .items(&options)
            .default(0)
            .interact()
            .context("Failed to get credential type selection")?;

        match selection {
            0 => self.setup_access_keys().await,
            1 => self.setup_profile().await,
            2 => {
                self.show_env_setup();
                Ok(None)
            }
            3 => {
                println!("\n{}", "Skipping credential setup.".yellow());
                println!("You can configure credentials later using one of these methods:");
                println!(
                    "  • Set AWS_ACCESS_KEY_ID and AWS_SECRET_ACCESS_KEY environment variables"
                );
                println!("  • Create ~/.aws/credentials file");
                println!("  • Run 's3-vectors init' again\n");
                Ok(None)
            }
            _ => unreachable!(),
        }
    }

    async fn setup_access_keys(&self) -> Result<Option<S3VectorsClient>> {
        loop {
            println!("\n{}", "Enter your AWS credentials:".cyan());

            let access_key_id: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("AWS Access Key ID")
                .validate_with(|input: &String| -> Result<(), &str> {
                    if input.starts_with("AKIA") && input.len() == 20 {
                        Ok(())
                    } else {
                        Err("Invalid Access Key ID format (should start with AKIA and be 20 characters)")
                    }
                })
                .interact_text()?;

            let secret_access_key: String = Password::with_theme(&ColorfulTheme::default())
                .with_prompt("AWS Secret Access Key")
                .interact()
                .context("Failed to get secret access key input")?;

            let session_token: Option<String> = {
                let token: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("AWS Session Token (optional, press Enter to skip)")
                    .allow_empty(true)
                    .interact_text()?;
                if token.is_empty() {
                    None
                } else {
                    Some(token)
                }
            };

            // Select region
            let region = self.select_region().await?;

            // Create client for testing
            let client = S3VectorsClient::with_credentials(
                &region,
                access_key_id.clone(),
                secret_access_key.clone(),
                session_token.clone(),
            );

            // Test credentials
            println!("\n{}", "Testing credentials...".yellow());
            if self.test_credentials(&client).await {
                println!("{} Successfully authenticated!\n", "✓".green());

                // Ask where to save
                let save_option = self.ask_save_location()?;

                match save_option {
                    SaveOption::AwsCredentials(profile_name) => {
                        self.save_to_aws_credentials(
                            &profile_name,
                            &access_key_id,
                            &secret_access_key,
                            session_token.as_deref(),
                            &region,
                        )?;
                        println!(
                            "\n{} Configuration saved to ~/.aws/credentials",
                            "✓".green()
                        );
                        println!(
                            "You can now use S3 Vectors with: {}",
                            format!("s3-vectors --profile {profile_name}").cyan()
                        );
                    }
                    SaveOption::Environment => {
                        self.show_export_commands(
                            &access_key_id,
                            &secret_access_key,
                            session_token.as_deref(),
                            &region,
                        );
                    }
                    SaveOption::NoSave => {
                        println!("\n{}", "Credentials validated but not saved.".yellow());
                    }
                }

                return Ok(Some(client));
            } else {
                println!(
                    "{} Authentication failed. Please check your credentials.",
                    "✗".red()
                );

                if !Confirm::with_theme(&ColorfulTheme::default())
                    .with_prompt("Would you like to try again?")
                    .interact()
                    .context("Failed to get retry confirmation")?
                {
                    return Ok(None);
                }
                // Loop continues if user wants to try again
            }
        }
    }

    async fn setup_profile(&self) -> Result<Option<S3VectorsClient>> {
        let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Cannot find home directory"))?;
        let creds_path = home.join(".aws/credentials");

        if !creds_path.exists() {
            println!(
                "\n{} No AWS credentials file found at: {:?}",
                "⚠".yellow(),
                creds_path
            );
            println!("Would you like to create one with access keys instead?");

            if Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt("Create new credentials?")
                .interact()
                .context("Failed to get credential creation confirmation")?
            {
                return self.setup_access_keys().await;
            } else {
                return Ok(None);
            }
        }

        // List available profiles
        let profiles = self.list_aws_profiles(&creds_path)?;
        if profiles.is_empty() {
            println!(
                "\n{} No profiles found in AWS credentials file.",
                "⚠".yellow()
            );
            return self.setup_access_keys().await;
        }

        println!("\n{}", "Available AWS profiles:".cyan());
        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select a profile")
            .items(&profiles)
            .interact()
            .context("Failed to get profile selection")?;

        let profile_name = &profiles[selection];
        let region = self.select_region().await?;

        // Try to create client with profile
        match S3VectorsClient::from_profile(profile_name, &region) {
            Ok(client) => {
                println!("\n{} Using profile: {}", "✓".green(), profile_name);
                Ok(Some(client))
            }
            Err(e) => {
                println!("\n{} Failed to load profile: {}", "✗".red(), e);
                Ok(None)
            }
        }
    }

    async fn select_region(&self) -> Result<String> {
        let regions = vec![
            "us-east-1",
            "us-west-2",
            "eu-west-1",
            "eu-central-1",
            "ap-southeast-1",
            "ap-northeast-1",
            "Enter custom region",
        ];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select AWS region")
            .items(&regions)
            .default(0)
            .interact()
            .context("Failed to get region selection")?;

        if selection == regions.len() - 1 {
            Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Enter region")
                .interact_text()
                .map_err(|e| anyhow::anyhow!("Failed to read input: {}", e))
        } else {
            Ok(regions[selection].to_string())
        }
    }

    async fn test_credentials(&self, client: &S3VectorsClient) -> bool {
        // Try to list buckets as a simple test
        match client.list_buckets().await {
            Ok(_) => true,
            Err(e) => {
                tracing::debug!("Credential test failed: {}", e);
                false
            }
        }
    }

    fn ask_save_location(&self) -> Result<SaveOption> {
        let options = vec![
            "AWS credentials file (~/.aws/credentials) [Recommended]",
            "Environment variables (show export commands)",
            "Don't save (one-time use)",
        ];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Where to save credentials?")
            .items(&options)
            .default(0)
            .interact()
            .context("Failed to get user selection")?;

        match selection {
            0 => {
                let profile_name: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Profile name")
                    .default("default".to_string())
                    .interact_text()
                    .context("Failed to get profile name input")?;
                Ok(SaveOption::AwsCredentials(profile_name))
            }
            1 => Ok(SaveOption::Environment),
            2 => Ok(SaveOption::NoSave),
            _ => unreachable!(),
        }
    }

    fn save_to_aws_credentials(
        &self,
        profile_name: &str,
        access_key_id: &str,
        secret_access_key: &str,
        session_token: Option<&str>,
        region: &str,
    ) -> Result<()> {
        let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Cannot find home directory"))?;
        let aws_dir = home.join(".aws");
        let creds_path = aws_dir.join("credentials");
        let config_path = aws_dir.join("config");

        // Create .aws directory if it doesn't exist
        if !aws_dir.exists() {
            fs::create_dir_all(&aws_dir).context("Failed to create .aws directory")?;
        }

        // Read existing credentials if any
        let mut creds_content = if creds_path.exists() {
            fs::read_to_string(&creds_path).context("Failed to read existing credentials file")?
        } else {
            String::new()
        };

        // Remove existing profile if it exists
        let profile_header = format!("[{profile_name}]");
        if let Some(start) = creds_content.find(&profile_header) {
            let end = creds_content[start..]
                .find("\n[")
                .map(|i| start + i)
                .unwrap_or(creds_content.len());
            creds_content.replace_range(start..end, "");
        }

        // Append new profile
        if !creds_content.is_empty() && !creds_content.ends_with('\n') {
            creds_content.push('\n');
        }

        writeln!(creds_content, "[{profile_name}]")?;
        writeln!(creds_content, "aws_access_key_id = {access_key_id}")?;
        writeln!(creds_content, "aws_secret_access_key = {secret_access_key}")?;
        if let Some(token) = session_token {
            writeln!(creds_content, "aws_session_token = {token}")?;
        }
        writeln!(creds_content)?;

        // Write credentials file
        fs::write(&creds_path, creds_content).context("Failed to write credentials file")?;

        // Set permissions to 600
        #[cfg(unix)]
        {
            let metadata = fs::metadata(&creds_path)?;
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o600);
            fs::set_permissions(&creds_path, permissions)?;
        }

        // Update config file with region
        let mut config_content = if config_path.exists() {
            fs::read_to_string(&config_path).context("Failed to read existing config file")?
        } else {
            String::new()
        };

        let config_header = if profile_name == "default" {
            "[default]".to_string()
        } else {
            format!("[profile {profile_name}]")
        };

        // Remove existing profile config if it exists
        if let Some(start) = config_content.find(&config_header) {
            let end = config_content[start..]
                .find("\n[")
                .map(|i| start + i)
                .unwrap_or(config_content.len());
            config_content.replace_range(start..end, "");
        }

        // Append new config
        if !config_content.is_empty() && !config_content.ends_with('\n') {
            config_content.push('\n');
        }

        writeln!(config_content, "{config_header}")?;
        writeln!(config_content, "region = {region}")?;
        writeln!(config_content)?;

        // Write config file
        fs::write(&config_path, config_content).context("Failed to write config file")?;

        Ok(())
    }

    fn show_env_setup(&self) {
        println!(
            "\n{}",
            "To configure S3 Vectors using environment variables:".cyan()
        );
        println!("\nAdd these to your shell profile (~/.bashrc, ~/.zshrc, etc.):\n");
        println!("{}", "# S3 Vectors Configuration".green());
        println!("export AWS_ACCESS_KEY_ID=\"your-access-key-here\"");
        println!("export AWS_SECRET_ACCESS_KEY=\"your-secret-key-here\"");
        println!("export AWS_REGION=\"us-east-1\"");
        println!("# Optional: for temporary credentials");
        println!("export AWS_SESSION_TOKEN=\"your-session-token-here\"");
        println!("\n{}", "Then reload your shell or run:".yellow());
        println!("source ~/.bashrc  # or ~/.zshrc\n");
    }

    fn show_export_commands(
        &self,
        access_key: &str,
        secret_key: &str,
        session_token: Option<&str>,
        region: &str,
    ) {
        println!(
            "\n{}",
            "To use these credentials in your current shell:".cyan()
        );
        println!("\n{}", "Copy and run these commands:".yellow());
        println!();
        println!("export AWS_ACCESS_KEY_ID=\"{access_key}\"");
        println!("export AWS_SECRET_ACCESS_KEY=\"{secret_key}\"");
        if let Some(token) = session_token {
            println!("export AWS_SESSION_TOKEN=\"{token}\"");
        }
        println!("export AWS_REGION=\"{region}\"");
        println!();
    }

    fn list_aws_profiles(&self, creds_path: &PathBuf) -> Result<Vec<String>> {
        let content = fs::read_to_string(creds_path).context("Failed to read credentials file")?;

        let mut profiles = Vec::new();
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with('[') && line.ends_with(']') {
                let profile = line[1..line.len() - 1].to_string();
                profiles.push(profile);
            }
        }

        Ok(profiles)
    }
}

enum SaveOption {
    AwsCredentials(String), // profile name
    Environment,
    NoSave,
}
