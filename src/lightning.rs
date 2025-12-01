use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

/// Lightning service for managing payments
///
/// Note: This uses a simplified LNURL-withdraw implementation
/// The actual withdrawal is triggered when someone scans the NFC tag
/// which contains a URL to our /api/lnurlw/{location_id} endpoint
pub struct LightningService {
    // In a full implementation, this would hold a Blitzi client
    // For now, we'll use a simplified approach
}

impl LightningService {
    pub fn new() -> Result<Self> {
        Ok(Self {})
    }

    /// Generate a unique secret for a location's LNURL-w
    pub fn generate_lnurlw_secret() -> String {
        uuid::Uuid::new_v4().to_string()
    }

    /// Create an invoice for withdrawal
    /// In production, this would use blitzi to generate actual Lightning invoices
    pub async fn create_withdrawal_invoice(
        &self,
        amount_sats: i64,
        description: &str,
    ) -> Result<String> {
        // TODO: Implement actual Lightning invoice creation with blitzi
        // For now, return a mock invoice
        Ok(format!(
            "lnbc{}1mock{}",
            amount_sats,
            uuid::Uuid::new_v4().to_string().replace("-", "")
        ))
    }

    /// Pay an invoice (send sats to user)
    /// In production, this would use blitzi to actually pay the invoice
    pub async fn pay_invoice(&self, invoice: &str) -> Result<()> {
        // TODO: Implement actual payment with blitzi
        tracing::info!("Would pay invoice: {}", invoice);
        Ok(())
    }
}

/// LNURL-withdraw response as per LUD-03 spec
#[derive(Debug, Serialize, Deserialize)]
pub struct LnurlWithdrawResponse {
    pub tag: String, // "withdrawRequest"
    pub callback: String, // URL to call with user's invoice
    #[serde(rename = "k1")]
    pub secret: String, // Secret to verify the request
    #[serde(rename = "minWithdrawable")]
    pub min_withdrawable: i64, // millisatoshis
    #[serde(rename = "maxWithdrawable")]
    pub max_withdrawable: i64, // millisatoshis
    #[serde(rename = "defaultDescription")]
    pub default_description: String,
}

impl LnurlWithdrawResponse {
    pub fn new(
        callback_url: String,
        secret: String,
        available_sats: i64,
        location_name: &str,
    ) -> Self {
        let msats = available_sats * 1000;
        Self {
            tag: "withdrawRequest".to_string(),
            callback: callback_url,
            secret,
            min_withdrawable: 1000, // 1 sat minimum
            max_withdrawable: msats,
            default_description: format!("SatShunt treasure from {}", location_name),
        }
    }
}

/// Request from Lightning wallet to execute withdrawal
#[derive(Debug, Deserialize)]
pub struct LnurlWithdrawCallback {
    #[serde(rename = "k1")]
    pub secret: String,
    pub pr: String, // Payment request (invoice) from user's wallet
}

/// Response to withdrawal callback
#[derive(Debug, Serialize)]
pub struct LnurlCallbackResponse {
    pub status: String, // "OK" or "ERROR"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl LnurlCallbackResponse {
    pub fn ok() -> Self {
        Self {
            status: "OK".to_string(),
            reason: None,
        }
    }

    pub fn error(reason: impl Into<String>) -> Self {
        Self {
            status: "ERROR".to_string(),
            reason: Some(reason.into()),
        }
    }
}
