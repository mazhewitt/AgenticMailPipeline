//! Shared Gmail API client utilities.
//! 
//! This module provides common Gmail API client creation and authentication
//! logic shared between the fetcher and labeler implementations.

use google_gmail1::{
    hyper_rustls,
    yup_oauth2::{InstalledFlowAuthenticator, InstalledFlowReturnMethod, ApplicationSecret},
    Gmail,
};
use std::fmt;

/// Shared Gmail client with authentication.
/// 
/// This struct provides a configured Gmail API client that can be used
/// by both fetcher and labeler implementations.
#[derive(Clone)]
pub struct GmailClient {
    pub hub: Gmail<hyper_rustls::HttpsConnector<google_gmail1::hyper_util::client::legacy::connect::HttpConnector>>,
}

/// Configuration for Gmail authentication.
#[derive(Debug, Clone)]
pub struct GmailAuthConfig {
    pub client_secret_path: String,
    pub token_path: String,
}

/// Error type for Gmail client operations.
#[derive(Debug)]
pub enum GmailClientError {
    Config { message: String },
    Auth { message: String },
}

impl fmt::Display for GmailClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GmailClientError::Config { message } => write!(f, "Gmail config error: {}", message),
            GmailClientError::Auth { message } => write!(f, "Gmail auth error: {}", message),
        }
    }
}

impl std::error::Error for GmailClientError {}

impl GmailClientError {
    pub fn config<S: Into<String>>(message: S) -> Self {
        Self::Config { message: message.into() }
    }

    pub fn auth<S: Into<String>>(message: S) -> Self {
        Self::Auth { message: message.into() }
    }
}

impl GmailAuthConfig {
    /// Create GmailAuthConfig from file paths.
    pub fn new(client_secret_path: String, token_path: String) -> Self {
        Self {
            client_secret_path,
            token_path,
        }
    }
    
    /// Create GmailAuthConfig from environment variables.
    pub fn from_env() -> Result<Self, GmailClientError> {
        let client_secret_path = std::env::var("GMAIL_CLIENT_SECRET_JSON")
            .map_err(|_| GmailClientError::config("GMAIL_CLIENT_SECRET_JSON environment variable not set"))?;
        let token_path = std::env::var("GMAIL_TOKEN_JSON")
            .map_err(|_| GmailClientError::config("GMAIL_TOKEN_JSON environment variable not set"))?;
        Ok(Self::new(client_secret_path, token_path))
    }
    
    /// Validate that the required files exist.
    pub fn validate_files(&self) -> Result<(), GmailClientError> {
        use std::path::Path;
        
        let secret_path = Path::new(&self.client_secret_path);
        if !secret_path.exists() {
            return Err(GmailClientError::config(format!(
                "Client secret file not found: {}", 
                self.client_secret_path
            )));
        }
        
        // For token file, we only check if the parent directory exists
        // The token file itself may not exist on first run
        let token_path = Path::new(&self.token_path);
        if let Some(parent) = token_path.parent() {
            if !parent.exists() {
                return Err(GmailClientError::config(format!(
                    "Token file directory not found: {}", 
                    parent.display()
                )));
            }
        }
        
        Ok(())
    }
}

impl GmailClient {
    /// Create a new GmailClient from environment variables.
    pub async fn from_env() -> Result<Self, GmailClientError> {
        let config = GmailAuthConfig::from_env()?;
        Self::new(config).await
    }

    /// Create a new GmailClient with explicit configuration.
    pub async fn new(config: GmailAuthConfig) -> Result<Self, GmailClientError> {
        let hub = Self::create_gmail_hub(config).await?;
        Ok(Self { hub })
    }

    /// Create and configure the Gmail API hub.
    async fn create_gmail_hub(
        config: GmailAuthConfig,
    ) -> Result<Gmail<hyper_rustls::HttpsConnector<google_gmail1::hyper_util::client::legacy::connect::HttpConnector>>, GmailClientError> {
        // Validate file paths
        config.validate_files()?;

        // Read and parse client secret
        let secret = std::fs::read_to_string(&config.client_secret_path)
            .map_err(|e| GmailClientError::config(format!(
                "Failed to read client secret file: {}", e
            )))?;

        let secret: ApplicationSecret = {
            // Parse the JSON first
            let google_secret: serde_json::Value = serde_json::from_str(&secret)
                .map_err(|e| GmailClientError::config(format!(
                    "Failed to parse client secret JSON: {}", e
                )))?;
            
            // Check if it's in the Google "installed" format
            if let Some(installed) = google_secret.get("installed") {
                serde_json::from_value(installed.clone())
                    .map_err(|e| GmailClientError::config(format!(
                        "Failed to parse installed client secret: {}", e
                    )))?
            } else {
                serde_json::from_str(&secret)
                    .map_err(|e| GmailClientError::config(format!(
                        "Failed to parse ApplicationSecret: {}", e
                    )))?
            }
        };

        // Set up OAuth2 authentication
        let connector = hyper_rustls::HttpsConnectorBuilder::new()
            .with_native_roots()
            .map_err(|e| GmailClientError::config(format!("Could not load native certs: {}", e)))?
            .https_only()
            .enable_http2()
            .build();

        let executor = google_gmail1::hyper_util::rt::TokioExecutor::new();
        let auth = InstalledFlowAuthenticator::with_client(
            secret,
            InstalledFlowReturnMethod::HTTPRedirect,
            google_gmail1::yup_oauth2::client::CustomHyperClientBuilder::from(
                google_gmail1::hyper_util::client::legacy::Client::builder(executor).build(connector.clone()),
            ),
        )
        .persist_tokens_to_disk(&config.token_path)
        .build()
        .await
        .map_err(|e| GmailClientError::auth(format!(
            "Failed to build authenticator: {}", e
        )))?;

        // Create Gmail hub
        let gmail_hub = Gmail::new(
            google_gmail1::hyper_util::client::legacy::Client::builder(
                google_gmail1::hyper_util::rt::TokioExecutor::new()
            ).build(connector),
            auth,
        );

        Ok(gmail_hub)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gmail_auth_config_new() {
        let config = GmailAuthConfig::new(
            "/path/to/secret.json".to_string(),
            "/path/to/token.json".to_string(),
        );
        assert_eq!(config.client_secret_path, "/path/to/secret.json");
        assert_eq!(config.token_path, "/path/to/token.json");
    }

    #[test]
    fn test_gmail_auth_config_from_env_missing_vars() {
        // Temporarily unset environment variables
        std::env::remove_var("GMAIL_CLIENT_SECRET_JSON");
        std::env::remove_var("GMAIL_TOKEN_JSON");
        
        let result = GmailAuthConfig::from_env();
        assert!(result.is_err());
    }

    #[test]
    fn test_gmail_auth_config_validate_files_nonexistent() {
        let config = GmailAuthConfig::new(
            "/nonexistent/secret.json".to_string(),
            "/nonexistent/token.json".to_string(),
        );
        let result = config.validate_files();
        assert!(result.is_err());
        if let Err(GmailClientError::Config { message }) = result {
            assert!(message.contains("Client secret file not found"));
        } else {
            panic!("Expected Config error for nonexistent file");
        }
    }

    #[test]
    fn test_gmail_client_errors() {
        let config_error = GmailClientError::config("Test config error");
        let auth_error = GmailClientError::auth("Test auth error");
        
        match config_error {
            GmailClientError::Config { .. } => (),
            _ => panic!("Expected Config error"),
        }
        
        match auth_error {
            GmailClientError::Auth { .. } => (),
            _ => panic!("Expected Auth error"),
        }
    }
}
