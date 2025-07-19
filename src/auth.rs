use anyhow::Result;
use chrono::Utc;
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone, Debug)]
pub struct AwsV4Signer {
    access_key_id: String,
    secret_access_key: String,
    session_token: Option<String>,
    region: String,
}

impl AwsV4Signer {
    pub fn new(
        access_key_id: String,
        secret_access_key: String,
        session_token: Option<String>,
        region: String,
    ) -> Self {
        Self {
            access_key_id,
            secret_access_key,
            session_token,
            region,
        }
    }

    pub async fn sign_request(
        &self,
        method: &str,
        url: &str,
        headers: std::collections::HashMap<String, String>,
        payload: &[u8],
    ) -> Result<std::collections::HashMap<String, String>> {
        let now = Utc::now();
        let date_stamp = now.format("%Y%m%d").to_string();
        let time_stamp = now.format("%Y%m%dT%H%M%SZ").to_string();

        // Extract host from URL
        let url_parsed = url::Url::parse(url).map_err(|e| anyhow::anyhow!("Failed to parse URL: {}", e))?;
        let host = match url_parsed.port() {
            Some(port) => format!("{}:{}", url_parsed.host_str()
                .ok_or_else(|| anyhow::anyhow!("URL has no host"))?, port),
            None => url_parsed.host_str()
                .ok_or_else(|| anyhow::anyhow!("URL has no host"))?
                .to_string(),
        };
        
        // Build headers map
        let mut signed_headers = headers;
        signed_headers.insert("host".to_string(), host);
        signed_headers.insert("x-amz-date".to_string(), time_stamp.clone());
        
        if let Some(token) = &self.session_token {
            signed_headers.insert("x-amz-security-token".to_string(), token.clone());
        }

        // Calculate payload hash
        let payload_hash = hex::encode(Sha256::digest(payload));
        signed_headers.insert("x-amz-content-sha256".to_string(), payload_hash.clone());

        // Extract URI from URL
        let uri = url_parsed.path().to_string();

        // Create canonical request
        let canonical_headers = self.create_canonical_headers_map(&signed_headers);
        let signed_headers_str = self.get_signed_headers_map(&signed_headers);
        
        let canonical_request = format!(
            "{}\n{}\n{}\n{}\n{}\n{}",
            method,
            uri,
            "", // query string
            canonical_headers,
            signed_headers_str,
            payload_hash
        );

        // Create string to sign
        let request_hash = hex::encode(Sha256::digest(canonical_request.as_bytes()));
        let credential_scope = format!("{}/{}/s3vectors/aws4_request", date_stamp, self.region);
        let string_to_sign = format!(
            "AWS4-HMAC-SHA256\n{}\n{}\n{}",
            time_stamp, credential_scope, request_hash
        );

        // Calculate signature
        let signature = self.calculate_signature(&date_stamp, &string_to_sign)?;

        // Create authorization header
        let auth_header = format!(
            "AWS4-HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
            self.access_key_id, credential_scope, signed_headers_str, signature
        );

        signed_headers.insert("authorization".to_string(), auth_header);

        Ok(signed_headers)
    }

    fn create_canonical_headers_map(&self, headers: &std::collections::HashMap<String, String>) -> String {
        let mut canonical = Vec::new();
        for (key, value) in headers {
            let key_str = key.to_lowercase();
            canonical.push(format!("{}:{}", key_str, value.trim()));
        }
        canonical.sort();
        canonical.join("\n") + "\n"
    }

    fn get_signed_headers_map(&self, headers: &std::collections::HashMap<String, String>) -> String {
        let mut signed = Vec::new();
        for (key, _) in headers {
            signed.push(key.to_lowercase());
        }
        signed.sort();
        signed.join(";")
    }

    fn calculate_signature(&self, date_stamp: &str, string_to_sign: &str) -> Result<String> {
        let k_secret = format!("AWS4{}", self.secret_access_key);
        let k_date = sign(k_secret.as_bytes(), date_stamp.as_bytes())?;
        let k_region = sign(&k_date, self.region.as_bytes())?;
        let k_service = sign(&k_region, b"s3vectors")?;
        let k_signing = sign(&k_service, b"aws4_request")?;
        let signature = sign(&k_signing, string_to_sign.as_bytes())?;
        
        Ok(hex::encode(signature))
    }
}

fn sign(key: &[u8], msg: &[u8]) -> Result<Vec<u8>> {
    let mut mac = HmacSha256::new_from_slice(key)
        .map_err(|e| anyhow::anyhow!("Failed to create HMAC: {}", e))?;
    mac.update(msg);
    Ok(mac.finalize().into_bytes().to_vec())
}