use crate::cli::output::{print_output, print_table};
use crate::cli::OutputFormat;
use crate::types::BucketStatus;
use crate::S3VectorsClient;
use anyhow::Result;
use chrono::{DateTime, NaiveDate, Utc};
use clap::{Args, Subcommand, ValueEnum};
use serde::Serialize;
use std::str::FromStr;
use tabled::Tabled;

// API limits
const MAX_LIST_RESULTS: u32 = 500; // AWS S3 Vectors API maximum

#[derive(Args, Debug)]
pub struct BucketCommand {
    #[command(subcommand)]
    pub command: BucketSubcommands,
}

#[derive(Subcommand, Debug)]
pub enum BucketSubcommands {
    #[command(about = "Create a new vector bucket")]
    Create {
        #[arg(help = "Name of the vector bucket")]
        name: String,

        #[arg(long, help = "KMS key ID for encryption")]
        kms_key_id: Option<String>,

        #[arg(long, help = "Tags in key=value format", value_delimiter = ',')]
        tags: Option<Vec<String>>,
    },

    #[command(about = "List vector buckets")]
    List {
        #[arg(short, long, help = "Maximum number of results", default_value = "100")]
        max_results: u32,

        #[arg(long, help = "Prefix to filter bucket names")]
        prefix: Option<String>,
    },

    #[command(about = "Query vector buckets with advanced filtering")]
    Query {
        #[arg(help = "Bucket name prefix to search for (e.g., 'prod' matches 'prod-vectors')")]
        pattern: Option<String>,

        #[arg(long, help = "Filter buckets containing this text in the name")]
        name_contains: Option<String>,

        #[arg(long, help = "Filter buckets with names starting with prefix")]
        name_prefix: Option<String>,

        #[arg(long, help = "Filter buckets with names ending with suffix")]
        name_suffix: Option<String>,

        #[arg(long, help = "Filter by bucket status", value_enum)]
        status: Option<BucketStatus>,

        #[arg(
            long,
            help = "Filter buckets created after date (YYYY-MM-DD or relative like 'yesterday')"
        )]
        created_after: Option<String>,

        #[arg(
            long,
            help = "Filter buckets created before date (YYYY-MM-DD or relative)"
        )]
        created_before: Option<String>,

        #[arg(long, help = "Filter only encrypted buckets")]
        encrypted: bool,

        #[arg(
            long,
            help = "Sort results by field",
            value_enum,
            default_value = "name"
        )]
        sort_by: BucketSortField,

        #[arg(long, help = "Sort order", value_enum, default_value = "asc")]
        sort_order: SortOrder,

        #[arg(long, help = "Maximum number of results to display")]
        limit: Option<usize>,
    },

    #[command(about = "Get vector bucket details")]
    Get {
        #[arg(help = "Name of the vector bucket")]
        name: String,
    },

    #[command(about = "Delete a vector bucket")]
    Delete {
        #[arg(help = "Name of the vector bucket")]
        name: String,

        #[arg(long, help = "Skip confirmation prompt")]
        force: bool,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum BucketSortField {
    Name,
    Created,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum SortOrder {
    Asc,
    Desc,
}

#[derive(Serialize, Tabled)]
struct BucketInfo {
    name: String,
    status: String,
    created_at: String,
    region: String,
}

struct BucketQueryParams<'a> {
    pattern: Option<&'a str>,
    name_contains: Option<&'a str>,
    name_prefix: Option<&'a str>,
    name_suffix: Option<&'a str>,
    status_filter: Option<&'a BucketStatus>,
    created_after: Option<&'a str>,
    created_before: Option<&'a str>,
    encrypted_only: bool,
    sort_by: BucketSortField,
    sort_order: SortOrder,
    limit: Option<usize>,
}

impl BucketCommand {
    pub async fn execute(
        &self,
        client: &S3VectorsClient,
        output_format: OutputFormat,
    ) -> Result<()> {
        match &self.command {
            BucketSubcommands::Create {
                name,
                kms_key_id,
                tags,
            } => {
                self.create_bucket(
                    client,
                    name,
                    kms_key_id.as_deref(),
                    tags.as_ref(),
                    output_format,
                )
                .await
            }
            BucketSubcommands::List {
                max_results,
                prefix,
            } => {
                self.list_buckets(client, *max_results, prefix.as_deref(), output_format)
                    .await
            }
            BucketSubcommands::Query {
                pattern,
                name_contains,
                name_prefix,
                name_suffix,
                status,
                created_after,
                created_before,
                encrypted,
                sort_by,
                sort_order,
                limit,
            } => {
                let params = BucketQueryParams {
                    pattern: pattern.as_deref(),
                    name_contains: name_contains.as_deref(),
                    name_prefix: name_prefix.as_deref(),
                    name_suffix: name_suffix.as_deref(),
                    status_filter: status.as_ref(),
                    created_after: created_after.as_deref(),
                    created_before: created_before.as_deref(),
                    encrypted_only: *encrypted,
                    sort_by: *sort_by,
                    sort_order: *sort_order,
                    limit: *limit,
                };
                self.query_buckets(client, params, output_format).await
            }
            BucketSubcommands::Get { name } => self.get_bucket(client, name, output_format).await,
            BucketSubcommands::Delete { name, force } => {
                self.delete_bucket(client, name, *force, output_format)
                    .await
            }
        }
    }

    async fn create_bucket(
        &self,
        client: &S3VectorsClient,
        name: &str,
        _kms_key_id: Option<&str>,
        _tags: Option<&Vec<String>>,
        output_format: OutputFormat,
    ) -> Result<()> {
        let bucket = client.create_vector_bucket(name).await?;

        match output_format {
            OutputFormat::Table => {
                println!("✓ Vector bucket created successfully");
                println!("Name: {}", bucket.vector_bucket_name);
                println!("Status: {:?}", bucket.status);
                println!("ARN: {}", bucket.vector_bucket_arn);
            }
            _ => print_output(&bucket, output_format)?,
        }

        Ok(())
    }

    async fn list_buckets(
        &self,
        client: &S3VectorsClient,
        max_results: u32,
        prefix: Option<&str>,
        output_format: OutputFormat,
    ) -> Result<()> {
        let response = client
            .list_vector_buckets(Some(max_results), None, prefix.map(|s| s.to_string()))
            .await?;

        match output_format {
            OutputFormat::Table => {
                let buckets: Vec<BucketInfo> = response
                    .buckets
                    .iter()
                    .map(|b| BucketInfo {
                        name: b.vector_bucket_name.clone(),
                        status: format!("{:?}", b.status),
                        created_at: chrono::DateTime::from_timestamp(b.creation_time as i64, 0)
                            .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
                            .unwrap_or_default(),
                        region: client.region().to_string(),
                    })
                    .collect();

                print_table(buckets)?;
            }
            _ => print_output(&response, output_format)?,
        }

        Ok(())
    }

    async fn get_bucket(
        &self,
        client: &S3VectorsClient,
        name: &str,
        output_format: OutputFormat,
    ) -> Result<()> {
        let bucket = client.describe_vector_bucket(name).await?;

        match output_format {
            OutputFormat::Table => {
                println!("Vector Bucket Details:");
                println!("  Name: {}", bucket.vector_bucket_name);
                println!("  ARN: {}", bucket.vector_bucket_arn);
                println!("  Status: {:?}", bucket.status);
                if let Some(created) =
                    chrono::DateTime::from_timestamp(bucket.creation_time as i64, 0)
                {
                    println!("  Created: {}", created.format("%Y-%m-%d %H:%M:%S"));
                }
                if let Some(encryption) = &bucket.encryption_configuration {
                    if let Some(sse_type) = &encryption.sse_type {
                        println!("  Encryption: {sse_type}");
                    }
                }
            }
            _ => print_output(&bucket, output_format)?,
        }

        Ok(())
    }

    async fn delete_bucket(
        &self,
        client: &S3VectorsClient,
        name: &str,
        force: bool,
        output_format: OutputFormat,
    ) -> Result<()> {
        if !force {
            use dialoguer::Confirm;
            let proceed = Confirm::new()
                .with_prompt(format!("Are you sure you want to delete bucket '{name}'?"))
                .default(false)
                .interact()?;

            if !proceed {
                println!("Operation cancelled");
                return Ok(());
            }
        }

        client.delete_vector_bucket(name).await?;

        match output_format {
            OutputFormat::Table => {
                println!("✓ Vector bucket '{name}' deleted successfully");
            }
            _ => {
                let result = serde_json::json!({
                    "status": "success",
                    "message": format!("Vector bucket '{}' deleted", name)
                });
                print_output(&result, output_format)?;
            }
        }

        Ok(())
    }

    async fn query_buckets(
        &self,
        client: &S3VectorsClient,
        params: BucketQueryParams<'_>,
        output_format: OutputFormat,
    ) -> Result<()> {
        // Determine if we can use API-level prefix filtering
        let api_prefix = if params.pattern.is_some()
            && params.name_contains.is_none()
            && params.name_prefix.is_none()
            && params.name_suffix.is_none()
        {
            // Simple pattern search - use as prefix by default
            params.pattern
        } else if params.name_prefix.is_some()
            && params.pattern.is_none()
            && params.name_contains.is_none()
            && params.name_suffix.is_none()
        {
            // Only prefix filter specified
            params.name_prefix
        } else {
            None
        };

        // Fetch all buckets with pagination
        let mut all_buckets = Vec::new();
        let mut next_token = None;
        let mut page_count = 0;

        // Show progress if fetching many buckets
        if api_prefix.is_none() && output_format == OutputFormat::Table {
            print!("Fetching buckets");
            use std::io::{self, Write};
            let _ = io::stdout().flush(); // Best effort flush, ignore errors
        }

        loop {
            let response = client
                .list_vector_buckets(
                    Some(MAX_LIST_RESULTS),
                    next_token.clone(),
                    api_prefix.map(|s| s.to_string()),
                )
                .await?;

            all_buckets.extend(response.buckets);
            page_count += 1;

            // Show progress for large lists
            if page_count > 1 && output_format == OutputFormat::Table {
                print!(".");
                use std::io::{self, Write};
                let _ = io::stdout().flush(); // Best effort flush, ignore errors
            }

            match response.next_token {
                Some(token) => next_token = Some(token),
                None => break,
            }
        }

        // Complete the progress line
        if api_prefix.is_none() && page_count > 1 && output_format == OutputFormat::Table {
            println!(" done! ({} buckets)", all_buckets.len());
        }

        // Apply client-side filters
        let mut filtered_buckets = all_buckets;

        // Name filtering (if not already done server-side)
        if api_prefix.is_none() {
            if let Some(p) = params.pattern {
                // Pattern uses prefix matching by default (more intuitive for bucket names)
                filtered_buckets.retain(|b| b.vector_bucket_name.starts_with(p));
            }
            if let Some(contains) = params.name_contains {
                filtered_buckets.retain(|b| b.vector_bucket_name.contains(contains));
            }
            if let Some(prefix) = params.name_prefix {
                filtered_buckets.retain(|b| b.vector_bucket_name.starts_with(prefix));
            }
        }
        if let Some(suffix) = params.name_suffix {
            filtered_buckets.retain(|b| b.vector_bucket_name.ends_with(suffix));
        }

        // Status filtering
        if let Some(status) = params.status_filter {
            filtered_buckets.retain(|b| b.status.as_ref() == Some(status));
        }

        // Date filtering
        if let Some(after_str) = params.created_after {
            match parse_date(after_str) {
                Ok(after_date) => {
                    filtered_buckets.retain(|b| {
                        DateTime::from_timestamp(b.creation_time as i64, 0)
                            .map(|dt| dt >= after_date)
                            .unwrap_or(false)
                    });
                }
                Err(e) => {
                    eprintln!("Warning: Invalid date format for --created-after '{after_str}': {e}. Supported formats: YYYY-MM-DD, 'today', 'yesterday', 'N days ago'");
                }
            }
        }
        if let Some(before_str) = params.created_before {
            match parse_date(before_str) {
                Ok(before_date) => {
                    filtered_buckets.retain(|b| {
                        DateTime::from_timestamp(b.creation_time as i64, 0)
                            .map(|dt| dt <= before_date)
                            .unwrap_or(false)
                    });
                }
                Err(e) => {
                    eprintln!("Warning: Invalid date format for --created-before '{before_str}': {e}. Supported formats: YYYY-MM-DD, 'today', 'yesterday', 'N days ago'");
                }
            }
        }

        // Encryption filtering
        if params.encrypted_only {
            filtered_buckets.retain(|b| b.encryption_configuration.is_some());
        }

        // Sort results
        match params.sort_by {
            BucketSortField::Name => {
                filtered_buckets.sort_by(|a, b| match params.sort_order {
                    SortOrder::Asc => a.vector_bucket_name.cmp(&b.vector_bucket_name),
                    SortOrder::Desc => b.vector_bucket_name.cmp(&a.vector_bucket_name),
                });
            }
            BucketSortField::Created => {
                filtered_buckets.sort_by(|a, b| match params.sort_order {
                    SortOrder::Asc => a
                        .creation_time
                        .partial_cmp(&b.creation_time)
                        .unwrap_or(std::cmp::Ordering::Equal),
                    SortOrder::Desc => b
                        .creation_time
                        .partial_cmp(&a.creation_time)
                        .unwrap_or(std::cmp::Ordering::Equal),
                });
            }
        }

        // Apply limit
        if let Some(limit) = params.limit {
            filtered_buckets.truncate(limit);
        }

        // Output results
        match output_format {
            OutputFormat::Table => {
                if filtered_buckets.is_empty() {
                    println!("No buckets found matching the query criteria.");
                    return Ok(());
                }

                // Show summary
                let total = filtered_buckets.len();
                let active = filtered_buckets
                    .iter()
                    .filter(|b| b.status == Some(BucketStatus::Active))
                    .count();
                let creating = filtered_buckets
                    .iter()
                    .filter(|b| b.status == Some(BucketStatus::Creating))
                    .count();
                let failed = filtered_buckets
                    .iter()
                    .filter(|b| b.status == Some(BucketStatus::Failed))
                    .count();

                println!(
                    "Found {} bucket{} ({} active, {} creating, {} failed)\n",
                    total,
                    if total == 1 { "" } else { "s" },
                    active,
                    creating,
                    failed
                );

                let buckets: Vec<BucketInfo> = filtered_buckets
                    .iter()
                    .map(|b| BucketInfo {
                        name: b.vector_bucket_name.clone(),
                        status: format_status(&b.status),
                        created_at: format_relative_time(b.creation_time),
                        region: client.region().to_string(),
                    })
                    .collect();

                print_table(buckets)?;
            }
            _ => print_output(&filtered_buckets, output_format)?,
        }

        Ok(())
    }
}

fn parse_date(date_str: &str) -> Result<DateTime<Utc>> {
    // Try parsing as ISO date first
    if let Ok(date) = NaiveDate::from_str(date_str) {
        return date
            .and_hms_opt(0, 0, 0)
            .map(|dt| Ok(dt.and_utc()))
            .unwrap_or_else(|| Err(anyhow::anyhow!("Invalid date: {}", date_str)));
    }

    // Handle relative dates
    let now = Utc::now();
    match date_str.to_lowercase().as_str() {
        "today" => now
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .map(|dt| Ok(dt.and_utc()))
            .unwrap_or_else(|| Err(anyhow::anyhow!("Invalid time calculation for today"))),
        "yesterday" => (now - chrono::Duration::days(1))
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .map(|dt| Ok(dt.and_utc()))
            .unwrap_or_else(|| Err(anyhow::anyhow!("Invalid time calculation for yesterday"))),
        "last week" | "lastweek" => Ok(now - chrono::Duration::weeks(1)),
        "last month" | "lastmonth" => Ok(now - chrono::Duration::days(30)),
        s if s.ends_with(" days ago") => {
            let days = s.trim_end_matches(" days ago").parse::<i64>()?;
            Ok(now - chrono::Duration::days(days))
        }
        s if s.ends_with(" weeks ago") => {
            let weeks = s.trim_end_matches(" weeks ago").parse::<i64>()?;
            Ok(now - chrono::Duration::weeks(weeks))
        }
        _ => Err(anyhow::anyhow!("Invalid date format: {}", date_str)),
    }
}

fn format_status(status: &Option<BucketStatus>) -> String {
    use colored::*;
    match status {
        Some(BucketStatus::Active) => "Active".green().to_string(),
        Some(BucketStatus::Creating) => "Creating".yellow().to_string(),
        Some(BucketStatus::Deleting) => "Deleting".yellow().to_string(),
        Some(BucketStatus::Failed) => "Failed".red().to_string(),
        None => "Unknown".dimmed().to_string(),
    }
}

fn format_relative_time(timestamp: f64) -> String {
    if let Some(dt) = DateTime::from_timestamp(timestamp as i64, 0) {
        let now = Utc::now();
        let duration = now.signed_duration_since(dt);

        if duration.num_days() == 0 {
            if duration.num_hours() == 0 {
                format!("{} minutes ago", duration.num_minutes())
            } else {
                format!("{} hours ago", duration.num_hours())
            }
        } else if duration.num_days() == 1 {
            "yesterday".to_string()
        } else if duration.num_days() < 7 {
            format!("{} days ago", duration.num_days())
        } else if duration.num_weeks() < 4 {
            format!("{} weeks ago", duration.num_weeks())
        } else {
            dt.format("%Y-%m-%d").to_string()
        }
    } else {
        "unknown".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser, Debug)]
    struct TestCli {
        #[command(subcommand)]
        command: BucketSubcommands,
    }

    #[test]
    fn test_parse_create_command() {
        let args = vec!["test", "create", "my-bucket"];
        let cli = TestCli::parse_from(args);

        match cli.command {
            BucketSubcommands::Create {
                name,
                kms_key_id,
                tags,
            } => {
                assert_eq!(name, "my-bucket");
                assert!(kms_key_id.is_none());
                assert!(tags.is_none());
            }
            _ => panic!("Expected Create command"),
        }
    }

    #[test]
    fn test_parse_create_with_options() {
        let args = vec![
            "test",
            "create",
            "my-bucket",
            "--kms-key-id",
            "key123",
            "--tags",
            "env=prod,team=data",
        ];
        let cli = TestCli::parse_from(args);

        match cli.command {
            BucketSubcommands::Create {
                name,
                kms_key_id,
                tags,
            } => {
                assert_eq!(name, "my-bucket");
                assert_eq!(kms_key_id, Some("key123".to_string()));
                assert_eq!(
                    tags,
                    Some(vec!["env=prod".to_string(), "team=data".to_string()])
                );
            }
            _ => panic!("Expected Create command"),
        }
    }

    #[test]
    fn test_parse_list_command() {
        let args = vec!["test", "list"];
        let cli = TestCli::parse_from(args);

        match cli.command {
            BucketSubcommands::List {
                max_results,
                prefix,
            } => {
                assert_eq!(max_results, 100); // default value
                assert!(prefix.is_none());
            }
            _ => panic!("Expected List command"),
        }
    }

    #[test]
    fn test_parse_get_command() {
        let args = vec!["test", "get", "my-bucket"];
        let cli = TestCli::parse_from(args);

        match cli.command {
            BucketSubcommands::Get { name } => {
                assert_eq!(name, "my-bucket");
            }
            _ => panic!("Expected Get command"),
        }
    }

    #[test]
    fn test_parse_delete_command() {
        let args = vec!["test", "delete", "my-bucket", "--force"];
        let cli = TestCli::parse_from(args);

        match cli.command {
            BucketSubcommands::Delete { name, force } => {
                assert_eq!(name, "my-bucket");
                assert!(force);
            }
            _ => panic!("Expected Delete command"),
        }
    }

    #[test]
    fn test_parse_query_simple() {
        let args = vec!["test", "query", "prod"];
        let cli = TestCli::parse_from(args);

        match cli.command {
            BucketSubcommands::Query { pattern, .. } => {
                assert_eq!(pattern, Some("prod".to_string()));
            }
            _ => panic!("Expected Query command"),
        }
    }

    #[test]
    fn test_parse_query_with_filters() {
        let args = vec![
            "test",
            "query",
            "--name-contains",
            "vec",
            "--status",
            "active",
        ];
        let cli = TestCli::parse_from(args);

        match cli.command {
            BucketSubcommands::Query {
                pattern,
                name_contains,
                status,
                ..
            } => {
                assert_eq!(pattern, None);
                assert_eq!(name_contains, Some("vec".to_string()));
                assert_eq!(status, Some(BucketStatus::Active));
            }
            _ => panic!("Expected Query command"),
        }
    }

    #[test]
    fn test_parse_query_with_date_filter() {
        let args = vec![
            "test",
            "query",
            "--created-after",
            "2024-01-01",
            "--sort-by",
            "created",
        ];
        let cli = TestCli::parse_from(args);

        match cli.command {
            BucketSubcommands::Query {
                created_after,
                sort_by,
                ..
            } => {
                assert_eq!(created_after, Some("2024-01-01".to_string()));
                assert!(matches!(sort_by, BucketSortField::Created));
            }
            _ => panic!("Expected Query command"),
        }
    }

    #[test]
    fn test_parse_date() {
        // Test ISO date
        let date = parse_date("2024-01-15").expect("Test date should parse");
        assert_eq!(date.format("%Y-%m-%d").to_string(), "2024-01-15");

        // Test relative dates
        let today = parse_date("today").expect("Today should parse");
        assert_eq!(today.date_naive(), Utc::now().date_naive());

        // Test "days ago" format
        let five_days = parse_date("5 days ago").expect("Relative date should parse");
        let expected = Utc::now() - chrono::Duration::days(5);
        assert_eq!(five_days.date_naive(), expected.date_naive());
    }

    #[test]
    fn test_format_status() {
        assert!(format_status(&Some(BucketStatus::Active)).contains("Active"));
        assert!(format_status(&Some(BucketStatus::Creating)).contains("Creating"));
        assert!(format_status(&Some(BucketStatus::Failed)).contains("Failed"));
        assert!(format_status(&None).contains("Unknown"));
    }

    #[test]
    fn test_format_relative_time() {
        let now_timestamp = Utc::now().timestamp() as f64;
        assert!(format_relative_time(now_timestamp).contains("minutes ago"));

        let yesterday = (Utc::now() - chrono::Duration::days(1)).timestamp() as f64;
        assert_eq!(format_relative_time(yesterday), "yesterday");

        let five_days_ago = (Utc::now() - chrono::Duration::days(5)).timestamp() as f64;
        assert!(format_relative_time(five_days_ago).contains("days ago"));

        let month_ago = (Utc::now() - chrono::Duration::days(30)).timestamp() as f64;
        // Should show actual date for older timestamps
        assert!(format_relative_time(month_ago).contains("-"));
    }
}
