use anyhow::{bail, Result};

/// Validate S3 bucket name according to S3 naming rules
pub fn validate_bucket_name(name: &str) -> Result<()> {
    if name.len() < 3 || name.len() > 63 {
        bail!("Bucket name must be between 3 and 63 characters long");
    }
    
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        bail!("Bucket name can only contain lowercase letters, numbers, and hyphens");
    }
    
    if name.starts_with('-') || name.ends_with('-') {
        bail!("Bucket name cannot start or end with a hyphen");
    }
    
    if name.starts_with("xn--") {
        bail!("Bucket name cannot start with 'xn--'");
    }
    
    if name.ends_with("-s3alias") {
        bail!("Bucket name cannot end with '-s3alias'");
    }
    
    if name.contains("..") {
        bail!("Bucket name cannot contain consecutive periods");
    }
    
    Ok(())
}

/// Validate index name
pub fn validate_index_name(name: &str) -> Result<()> {
    if name.is_empty() || name.len() > 255 {
        bail!("Index name must be between 1 and 255 characters");
    }
    
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        bail!("Index name can only contain alphanumeric characters, hyphens, and underscores");
    }
    
    Ok(())
}

/// Validate vector dimensions
pub fn validate_dimensions(dimensions: u32) -> Result<()> {
    if dimensions == 0 || dimensions > 4096 {
        bail!("Vector dimensions must be between 1 and 4096");
    }
    Ok(())
}

/// Validate top-k value for queries
pub fn validate_top_k(top_k: u32) -> Result<()> {
    if top_k == 0 || top_k > 30 {
        bail!("Top-k must be between 1 and 30 (preview limitation)");
    }
    Ok(())
}

/// Validate AWS region is supported for S3 Vectors preview
pub fn validate_region(region: &str) -> Result<()> {
    // S3 Vectors is currently in preview and only available in specific regions
    // Based on AWS documentation, these are the confirmed preview regions
    const SUPPORTED_REGIONS: &[&str] = &[
        "us-east-1",
        "us-west-2",
    ];
    
    if !SUPPORTED_REGIONS.contains(&region) {
        bail!(
            "S3 Vectors preview is only available in: {}. Please use one of these regions.",
            SUPPORTED_REGIONS.join(", ")
        );
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_bucket_name_validation() {
        assert!(validate_bucket_name("my-vector-bucket").is_ok());
        assert!(validate_bucket_name("123").is_ok());
        
        assert!(validate_bucket_name("ab").is_err()); // too short
        assert!(validate_bucket_name(&"a".repeat(64)).is_err()); // too long
        assert!(validate_bucket_name("My-Bucket").is_err()); // uppercase
        assert!(validate_bucket_name("-bucket").is_err()); // starts with hyphen
        assert!(validate_bucket_name("bucket-").is_err()); // ends with hyphen
        assert!(validate_bucket_name("bucket..name").is_err()); // consecutive periods
    }
    
    #[test]
    fn test_index_name_validation() {
        assert!(validate_index_name("my_index_123").is_ok());
        assert!(validate_index_name("index-name").is_ok());
        
        assert!(validate_index_name("").is_err()); // empty
        assert!(validate_index_name(&"a".repeat(256)).is_err()); // too long
        assert!(validate_index_name("index name").is_err()); // contains space
    }
    
    #[test]
    fn test_dimension_validation() {
        assert!(validate_dimensions(128).is_ok());
        assert!(validate_dimensions(4096).is_ok());
        
        assert!(validate_dimensions(0).is_err());
        assert!(validate_dimensions(4097).is_err());
    }
}