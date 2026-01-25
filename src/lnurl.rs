//! LNURL protocol implementation.
//!
//! This module handles:
//! - Resolving Lightning Addresses (user@domain.com format) to BOLT11 invoices (LUD-16)
//! - Encoding URLs to LNURL bech32 format (LUD-01)

use bech32::{Bech32, Hrp};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur during LN address resolution
#[derive(Debug, Error)]
pub enum LnurlError {
    #[error("Invalid LN address format: {0}")]
    InvalidFormat(String),

    #[error("Failed to resolve LN address: {0}")]
    ResolutionFailed(String),

    #[error("Amount {amount} msats is out of range ({min}-{max} msats)")]
    AmountOutOfRange { amount: i64, min: i64, max: i64 },

    #[error("Invalid LNURL-pay response: {0}")]
    InvalidResponse(String),

    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),
}

/// LNURL-pay metadata response (LUD-06)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LnurlPayResponse {
    /// Callback URL to request invoice
    pub callback: String,

    /// Minimum amount in millisatoshis
    #[serde(rename = "minSendable")]
    pub min_sendable: i64,

    /// Maximum amount in millisatoshis
    #[serde(rename = "maxSendable")]
    pub max_sendable: i64,

    /// Metadata as JSON string (contains description)
    pub metadata: String,

    /// Tag indicating this is a pay request
    pub tag: String,

    /// Optional comment allowed length
    #[serde(rename = "commentAllowed", default)]
    pub comment_allowed: Option<i64>,
}

/// Response from LNURL-pay callback with invoice
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LnurlPayCallbackResponse {
    /// BOLT11 payment request (invoice)
    pub pr: String,

    /// Optional routes for payment
    #[serde(default)]
    pub routes: Vec<serde_json::Value>,

    /// Optional success action
    #[serde(rename = "successAction", default)]
    pub success_action: Option<serde_json::Value>,
}

/// LNURL error response
#[derive(Debug, Clone, Deserialize)]
pub struct LnurlErrorResponse {
    pub status: String,
    pub reason: String,
}

/// Parse a Lightning Address into user and domain parts.
///
/// Lightning addresses follow the format: user@domain.com
fn parse_ln_address(address: &str) -> Result<(String, String), LnurlError> {
    let address = address.trim().to_lowercase();
    let parts: Vec<&str> = address.split('@').collect();

    if parts.len() != 2 {
        return Err(LnurlError::InvalidFormat(
            "must be in format user@domain".to_string(),
        ));
    }

    let user = parts[0];
    let domain = parts[1];

    if user.is_empty() {
        return Err(LnurlError::InvalidFormat("user part is empty".to_string()));
    }

    if domain.is_empty() || !domain.contains('.') {
        return Err(LnurlError::InvalidFormat(
            "domain must be a valid hostname".to_string(),
        ));
    }

    Ok((user.to_string(), domain.to_string()))
}

/// Resolve a Lightning Address to its LNURL-pay metadata.
///
/// This fetches the LNURL-pay endpoint at:
/// `https://{domain}/.well-known/lnurlp/{user}`
pub async fn resolve_ln_address(address: &str) -> Result<LnurlPayResponse, LnurlError> {
    let (user, domain) = parse_ln_address(address)?;

    let url = format!("https://{}/.well-known/lnurlp/{}", domain, user);

    tracing::info!("Resolving LN address {}@{} via {}", user, domain, url);

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(LnurlError::ResolutionFailed(format!(
            "HTTP {}: {}",
            status, body
        )));
    }

    let lnurl_pay: LnurlPayResponse = response
        .json()
        .await
        .map_err(|e| LnurlError::InvalidResponse(format!("failed to parse response: {}", e)))?;

    if lnurl_pay.tag != "payRequest" {
        return Err(LnurlError::InvalidResponse(format!(
            "expected tag 'payRequest', got '{}'",
            lnurl_pay.tag
        )));
    }

    Ok(lnurl_pay)
}

/// Get a BOLT11 invoice from the LNURL-pay callback.
///
/// Calls the callback URL with the specified amount to receive an invoice.
pub async fn get_invoice(callback_url: &str, amount_msats: i64) -> Result<String, LnurlError> {
    // Parse the callback URL and add the amount parameter
    let url = if callback_url.contains('?') {
        format!("{}&amount={}", callback_url, amount_msats)
    } else {
        format!("{}?amount={}", callback_url, amount_msats)
    };

    tracing::info!("Requesting invoice from callback: {}", url);

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        // Try to parse as LNURL error response
        if let Ok(error_response) = serde_json::from_str::<LnurlErrorResponse>(&body) {
            return Err(LnurlError::ResolutionFailed(error_response.reason));
        }

        return Err(LnurlError::ResolutionFailed(format!(
            "HTTP {}: {}",
            status, body
        )));
    }

    let callback_response: LnurlPayCallbackResponse = response.json().await.map_err(|e| {
        LnurlError::InvalidResponse(format!("failed to parse callback response: {}", e))
    })?;

    if callback_response.pr.is_empty() {
        return Err(LnurlError::InvalidResponse(
            "empty payment request in response".to_string(),
        ));
    }

    Ok(callback_response.pr)
}

/// Resolve a Lightning Address and get an invoice for the specified amount.
///
/// This is a convenience function that combines `resolve_ln_address` and `get_invoice`.
pub async fn get_invoice_for_ln_address(
    address: &str,
    amount_msats: i64,
) -> Result<String, LnurlError> {
    let lnurl_pay = resolve_ln_address(address).await?;

    // Validate amount is within range
    if amount_msats < lnurl_pay.min_sendable || amount_msats > lnurl_pay.max_sendable {
        return Err(LnurlError::AmountOutOfRange {
            amount: amount_msats,
            min: lnurl_pay.min_sendable,
            max: lnurl_pay.max_sendable,
        });
    }

    get_invoice(&lnurl_pay.callback, amount_msats).await
}

/// Encode a URL as an LNURL bech32 string (LUD-01).
///
/// LNURL encoding uses bech32 (not bech32m) with the "lnurl" HRP (human-readable part).
/// The resulting string is uppercase for better QR code compatibility.
pub fn encode_lnurl(url: &str) -> Result<String, LnurlError> {
    let hrp = Hrp::parse("lnurl").map_err(|e| LnurlError::InvalidFormat(e.to_string()))?;
    let encoded = bech32::encode::<Bech32>(hrp, url.as_bytes())
        .map_err(|e| LnurlError::InvalidFormat(e.to_string()))?;
    Ok(encoded.to_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ln_address_valid() {
        let address = "satoshi@bitcoin.org";
        let result = parse_ln_address(address);
        assert!(result.is_ok());
        let (user, domain) = result.unwrap();
        assert_eq!(&user, "satoshi");
        assert_eq!(&domain, "bitcoin.org");
    }

    #[test]
    fn test_parse_ln_address_with_subdomain() {
        let address = "user@pay.wallet.com";
        let result = parse_ln_address(address);
        assert!(result.is_ok());
        let (user, domain) = result.unwrap();
        assert_eq!(&user, "user");
        assert_eq!(&domain, "pay.wallet.com");
    }

    #[test]
    fn test_parse_ln_address_no_at() {
        let result = parse_ln_address("invalid");
        assert!(matches!(result, Err(LnurlError::InvalidFormat(_))));
    }

    #[test]
    fn test_parse_ln_address_multiple_at() {
        let result = parse_ln_address("user@domain@extra");
        assert!(matches!(result, Err(LnurlError::InvalidFormat(_))));
    }

    #[test]
    fn test_parse_ln_address_empty_user() {
        let result = parse_ln_address("@domain.com");
        assert!(matches!(result, Err(LnurlError::InvalidFormat(_))));
    }

    #[test]
    fn test_parse_ln_address_invalid_domain() {
        let result = parse_ln_address("user@localhost");
        assert!(matches!(result, Err(LnurlError::InvalidFormat(_))));
    }

    #[test]
    fn test_parse_ln_address_trims_whitespace() {
        let result = parse_ln_address("  user@domain.com  ");
        assert!(result.is_ok());
    }

    #[test]
    fn test_encode_lnurl() {
        // Test encoding a URL to LNURL format
        let url = "https://service.com/api/lnurl";
        let result = encode_lnurl(url);
        assert!(result.is_ok());
        let lnurl = result.unwrap();

        // LNURL should start with "LNURL1" (uppercase)
        assert!(lnurl.starts_with("LNURL1"));

        // Verify it can be decoded back using standard bech32 (not bech32m)
        let hrp = Hrp::parse("lnurl").unwrap();
        let (decoded_hrp, decoded_data) =
            bech32::decode(&lnurl.to_lowercase()).expect("should decode with bech32");
        assert_eq!(decoded_hrp, hrp);
        let decoded_url = String::from_utf8(decoded_data).expect("should be valid utf8");
        assert_eq!(decoded_url, url);
    }
}
