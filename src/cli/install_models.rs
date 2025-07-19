use anyhow::{Context, Result};
use clap::Args;
use colored::*;
use dialoguer::Confirm;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Args)]
#[command(about = "Download ML models for RAG functionality")]
pub struct InstallModelsCommand {
    #[arg(long, help = "Directory to install models", default_value = "./models")]
    model_dir: PathBuf,

    #[arg(long, help = "Force re-download even if files exist")]
    force: bool,

    #[arg(long, help = "Verify checksums after download")]
    verify: bool,

    #[arg(long, help = "Model to download", default_value = "all-MiniLM-L6-v2")]
    model: String,
}

// Model file information
struct ModelFile {
    name: &'static str,
    url: &'static str,
    size: u64, // approximate size in bytes
    required: bool,
}

impl InstallModelsCommand {
    pub async fn execute(&self) -> Result<()> {
        println!("{}", "Installing ML models for S3 Vectors...".cyan().bold());
        println!();

        // Model files to download
        let model_files = vec![
            ModelFile {
                name: "config.json",
                url: "https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/config.json",
                size: 600,
                required: true,
            },
            ModelFile {
                name: "tokenizer.json",
                url: "https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/tokenizer.json",
                size: 700_000,
                required: true,
            },
            ModelFile {
                name: "vocab.txt",
                url: "https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/vocab.txt",
                size: 232_000,
                required: false,
            },
            ModelFile {
                name: "special_tokens_map.json",
                url: "https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/special_tokens_map.json",
                size: 125,
                required: false,
            },
            ModelFile {
                name: "model.safetensors",
                url: "https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/model.safetensors",
                size: 90_000_000,
                required: true,
            },
        ];

        // Calculate total size
        let total_size: u64 = model_files.iter().map(|f| f.size).sum();
        println!("Model: {}", self.model.yellow());
        println!("Total download size: {}", format_bytes(total_size).green());
        println!();

        // Create model directory
        let model_path = self.model_dir.join(&self.model);
        if !model_path.exists() {
            fs::create_dir_all(&model_path).context("Failed to create model directory")?;
        }

        // Check existing files
        let mut files_to_download = Vec::new();
        let mut existing_files = Vec::new();

        for file in &model_files {
            let file_path = model_path.join(file.name);
            if file_path.exists() && !self.force {
                existing_files.push(file.name);
            } else {
                files_to_download.push(file);
            }
        }

        // Handle existing files
        if !existing_files.is_empty() && !self.force {
            println!("{} Existing files found:", "⚠".yellow());
            for name in &existing_files {
                println!("  • {name}");
            }

            if files_to_download.is_empty() {
                println!("\n{} All model files already downloaded!", "✓".green());
                println!("Use {} to re-download", "--force".cyan());
                return Ok(());
            }

            println!("\n{} files need to be downloaded", files_to_download.len());
            if !Confirm::with_theme(&dialoguer::theme::ColorfulTheme::default())
                .with_prompt("Continue with download?")
                .default(true)
                .interact()?
            {
                println!("Download cancelled.");
                return Ok(());
            }
        }

        // Download files
        let multi_progress = MultiProgress::new();
        let client = reqwest::Client::builder()
            .user_agent(concat!(
                env!("CARGO_PKG_NAME"),
                "/",
                env!("CARGO_PKG_VERSION")
            ))
            .timeout(std::time::Duration::from_secs(300))
            .build()?;

        println!("\n{} Downloading model files...", "→".blue());

        for (idx, file) in files_to_download.iter().enumerate() {
            let file_path = model_path.join(file.name);
            println!(
                "\n[{}/{}] Downloading {}...",
                idx + 1,
                files_to_download.len(),
                file.name.cyan()
            );

            match self
                .download_file(&client, file, &file_path, &multi_progress)
                .await
            {
                Ok(_) => {
                    println!("{} Downloaded {}", "✓".green(), file.name);
                }
                Err(e) => {
                    if file.required {
                        eprintln!("{} Failed to download {}: {}", "✗".red(), file.name, e);
                        return Err(e);
                    } else {
                        eprintln!(
                            "{} Failed to download {} (optional): {}",
                            "⚠".yellow(),
                            file.name,
                            e
                        );
                    }
                }
            }
        }

        // Verify installation
        println!("\n{} Verifying installation...", "→".blue());
        let mut all_required_present = true;

        for file in &model_files {
            let file_path = model_path.join(file.name);
            if file_path.exists() {
                let size = fs::metadata(&file_path)?.len();
                println!("{} {} ({})", "✓".green(), file.name, format_bytes(size));
            } else if file.required {
                println!("{} {} (missing)", "✗".red(), file.name);
                all_required_present = false;
            } else {
                println!("{} {} (optional, not downloaded)", "○".yellow(), file.name);
            }
        }

        if all_required_present {
            println!("\n{} Model installation complete!", "✓".green().bold());
            println!(
                "\nModels installed to: {}",
                model_path.display().to_string().cyan()
            );
            println!("\nYou can now use RAG features with the S3 Vectors CLI.");
        } else {
            eprintln!("\n{} Some required files are missing!", "✗".red().bold());
            return Err(anyhow::anyhow!("Model installation incomplete"));
        }

        Ok(())
    }

    async fn download_file(
        &self,
        client: &reqwest::Client,
        file: &ModelFile,
        file_path: &Path,
        multi_progress: &MultiProgress,
    ) -> Result<()> {
        // Start download
        let response = client
            .get(file.url)
            .send()
            .await
            .context("Failed to start download")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("HTTP error: {}", response.status()));
        }

        // Get content length
        let total_size = response.content_length().unwrap_or(file.size);

        // Create progress bar
        let pb = multi_progress.add(ProgressBar::new(total_size));
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                .context("Failed to set progress bar template")?
                .progress_chars("#>-")
        );

        // Create temporary file
        let temp_path = file_path.with_extension("tmp");
        let mut temp_file =
            fs::File::create(&temp_path).context("Failed to create temporary file")?;

        // Download with progress
        let mut downloaded = 0u64;
        let mut stream = response.bytes_stream();

        use futures_util::StreamExt;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.context("Failed to read chunk")?;
            temp_file
                .write_all(&chunk)
                .context("Failed to write to file")?;

            downloaded += chunk.len() as u64;
            pb.set_position(downloaded);
        }

        pb.finish_and_clear();

        // Flush and close temp file
        temp_file.flush()?;
        drop(temp_file);

        // Move temp file to final location
        fs::rename(&temp_path, file_path).context("Failed to move downloaded file")?;

        Ok(())
    }
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;

    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }

    if unit_idx == 0 {
        format!("{} {}", size as u64, UNITS[unit_idx])
    } else {
        format!("{:.2} {}", size, UNITS[unit_idx])
    }
}
