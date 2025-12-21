use anyhow::Result;
use async_trait::async_trait;
use blitzi::{Amount, Blitzi, BlitziBuilder};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Trait for Lightning Network operations
/// Allows mocking in tests where Blitzi (which requires live funds) cannot be used
#[async_trait]
pub trait Lightning: Send + Sync {
    /// Create a Lightning invoice for receiving payment
    async fn create_invoice(&self, amount_sats: u64, description: &str) -> Result<String>;

    /// Pay an invoice (send sats to user)
    async fn pay_invoice(&self, invoice: &str) -> Result<()>;

    /// Wait for an invoice to be paid
    async fn await_payment(&self, invoice: &str) -> Result<()>;
}

/// Lightning service for managing payments (production implementation using Blitzi)
pub struct LightningService {
    client: Blitzi,
}

impl LightningService {
    pub async fn new(data_dir: &Path) -> Result<Self> {
        let client = BlitziBuilder::default().datadir(data_dir).build().await?;
        tracing::info!(
            "Blitzi Lightning client initialized with data dir: {}",
            data_dir.display()
        );
        Ok(Self { client })
    }

    /// Generate a unique secret for a location's LNURL-w
    pub fn generate_lnurlw_secret() -> String {
        uuid::Uuid::new_v4().to_string()
    }
}

#[async_trait]
impl Lightning for LightningService {
    async fn create_invoice(&self, amount_sats: u64, description: &str) -> Result<String> {
        let amount = Amount::from_sats(amount_sats);
        let invoice = self.client.lightning_invoice(amount, description).await?;
        tracing::info!("Created invoice for {} sats: {}", amount_sats, description);
        Ok(invoice.to_string())
    }

    async fn pay_invoice(&self, invoice: &str) -> Result<()> {
        let bolt11 = invoice
            .parse()
            .map_err(|e| anyhow::anyhow!("Invalid invoice format: {}", e))?;

        tracing::info!("Paying invoice: {}", invoice);
        let preimage = self.client.pay(&bolt11).await?;
        tracing::info!(
            "Invoice paid successfully, preimage: {}",
            hex::encode(preimage)
        );
        Ok(())
    }

    async fn await_payment(&self, invoice: &str) -> Result<()> {
        let invoice_obj = invoice
            .parse()
            .map_err(|e| anyhow::anyhow!("Invalid invoice format: {}", e))?;

        self.client.await_incoming_payment(&invoice_obj).await?;
        tracing::info!("Payment received for invoice");
        Ok(())
    }
}

/// Mock Lightning service for testing (does not require Blitzi or live funds)
#[derive(Default)]
pub struct MockLightning {
    /// If set, pay_invoice will return this error
    pub pay_error: Option<String>,
    /// If set, await_payment will return this error
    pub await_error: Option<String>,
}

impl MockLightning {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a MockLightning that fails on pay_invoice
    #[allow(dead_code)]
    pub fn with_pay_error(error: impl Into<String>) -> Self {
        Self {
            pay_error: Some(error.into()),
            await_error: None,
        }
    }
}

#[async_trait]
impl Lightning for MockLightning {
    async fn create_invoice(&self, amount_sats: u64, description: &str) -> Result<String> {
        // Return a fake invoice format for testing
        Ok(format!("lnbc{}n1mock{}", amount_sats, description.len()))
    }

    async fn pay_invoice(&self, invoice: &str) -> Result<()> {
        if let Some(ref err) = self.pay_error {
            return Err(anyhow::anyhow!("{}", err));
        }
        tracing::info!("MockLightning: Simulated payment for invoice: {}", invoice);
        Ok(())
    }

    async fn await_payment(&self, invoice: &str) -> Result<()> {
        if let Some(ref err) = self.await_error {
            return Err(anyhow::anyhow!("{}", err));
        }
        tracing::info!(
            "MockLightning: Simulated payment received for invoice: {}",
            invoice
        );
        Ok(())
    }
}

/// LNURL-withdraw response as per LUD-03 spec
#[derive(Debug, Serialize, Deserialize)]
pub struct LnurlWithdrawResponse {
    pub tag: String,      // "withdrawRequest"
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
            min_withdrawable: msats, // Must withdraw all sats
            max_withdrawable: msats,
            default_description: format!("SatsHunt treasure from {}", location_name),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lnurl_withdraw_response() {
        let response = LnurlWithdrawResponse::new(
            "https://example.com/callback".to_string(),
            "secret123".to_string(),
            100, // 100 sats
            "Test Location",
        );

        assert_eq!(response.tag, "withdrawRequest");
        assert_eq!(response.callback, "https://example.com/callback");
        assert_eq!(response.secret, "secret123");
        assert_eq!(response.min_withdrawable, 100_000); // 100 sats = 100,000 msats
        assert_eq!(response.max_withdrawable, 100_000);
        assert_eq!(
            response.default_description,
            "SatsHunt treasure from Test Location"
        );
    }

    #[test]
    fn test_lnurl_withdraw_response_zero_sats() {
        let response = LnurlWithdrawResponse::new(
            "https://example.com/callback".to_string(),
            "secret".to_string(),
            0,
            "Empty",
        );

        assert_eq!(response.min_withdrawable, 0);
        assert_eq!(response.max_withdrawable, 0);
    }

    #[test]
    fn test_lnurl_callback_response_ok() {
        let response = LnurlCallbackResponse::ok();

        assert_eq!(response.status, "OK");
        assert!(response.reason.is_none());

        // Test JSON serialization
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"status\":\"OK\""));
        assert!(!json.contains("reason")); // reason should be skipped when None
    }

    #[test]
    fn test_lnurl_callback_response_error() {
        let response = LnurlCallbackResponse::error("Invalid secret");

        assert_eq!(response.status, "ERROR");
        assert_eq!(response.reason, Some("Invalid secret".to_string()));

        // Test JSON serialization
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"status\":\"ERROR\""));
        assert!(json.contains("\"reason\":\"Invalid secret\""));
    }

    #[test]
    fn test_generate_lnurlw_secret() {
        let secret1 = LightningService::generate_lnurlw_secret();
        let secret2 = LightningService::generate_lnurlw_secret();

        // Secrets should be UUIDs (36 chars with hyphens)
        assert_eq!(secret1.len(), 36);
        assert_eq!(secret2.len(), 36);

        // Secrets should be unique
        assert_ne!(secret1, secret2);
    }

    #[tokio::test]
    async fn test_mock_lightning_create_invoice() {
        let mock = MockLightning::new();
        let invoice = mock.create_invoice(1000, "test").await.unwrap();

        assert!(invoice.starts_with("lnbc"));
        assert!(invoice.contains("1000"));
    }

    #[tokio::test]
    async fn test_mock_lightning_pay_invoice_success() {
        let mock = MockLightning::new();
        let result = mock.pay_invoice("lnbc1000n1fake").await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_lightning_pay_invoice_error() {
        let mock = MockLightning::with_pay_error("Payment failed");
        let result = mock.pay_invoice("lnbc1000n1fake").await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Payment failed"));
    }

    #[tokio::test]
    async fn test_mock_lightning_await_payment_success() {
        let mock = MockLightning::new();
        let result = mock.await_payment("lnbc1000n1fake").await;

        assert!(result.is_ok());
    }
}
