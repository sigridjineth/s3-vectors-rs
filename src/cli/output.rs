use crate::cli::OutputFormat;
use anyhow::Result;
use serde::Serialize;
use tabled::{Table, Tabled};

pub trait OutputFormatter {
    fn format_output<T: Serialize>(&self, data: T, format: OutputFormat) -> Result<String>;
    fn format_table<T: Tabled>(&self, data: Vec<T>) -> String;
}

pub struct DefaultFormatter;

impl OutputFormatter for DefaultFormatter {
    fn format_output<T: Serialize>(&self, data: T, format: OutputFormat) -> Result<String> {
        match format {
            OutputFormat::Json => {
                serde_json::to_string_pretty(&data).map_err(Into::into)
            }
            OutputFormat::Yaml => {
                serde_yaml::to_string(&data).map_err(Into::into)
            }
            OutputFormat::Table => {
                // For table output, we'll need specific implementations
                // This is a fallback to JSON
                serde_json::to_string_pretty(&data).map_err(Into::into)
            }
        }
    }
    
    fn format_table<T: Tabled>(&self, data: Vec<T>) -> String {
        if data.is_empty() {
            return "No data found".to_string();
        }
        Table::new(data).to_string()
    }
}

pub fn print_output<T: Serialize>(data: T, format: OutputFormat) -> Result<()> {
    let formatter = DefaultFormatter;
    let output = formatter.format_output(data, format)?;
    println!("{}", output);
    Ok(())
}

pub fn print_table<T: Tabled>(data: Vec<T>) -> Result<()> {
    let formatter = DefaultFormatter;
    println!("{}", formatter.format_table(data));
    Ok(())
}