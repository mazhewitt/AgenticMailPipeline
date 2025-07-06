//! Gmail tests - require Gmail API credentials
//!
//! These tests require valid Gmail OAuth2 credentials and internet access.
//! They test Gmail API integration for fetching and labeling emails.
//!
//! Prerequisites:
//! - Set up OAuth2 credentials:
//!   - GMAIL_CLIENT_SECRET_JSON: Path to client secret JSON file
//!   - GMAIL_TOKEN_JSON: Path to OAuth2 token JSON file
//! - Ensure Gmail API is enabled in Google Cloud Console
//! - Grant appropriate Gmail scopes (modify, readonly)
//!
//! Run with: cargo test --test gmail

mod integration_gmail_fetcher;
mod integration_gmail_labeling;
mod integration_test_data_creation;
