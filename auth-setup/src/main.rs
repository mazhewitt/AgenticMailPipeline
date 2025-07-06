//! OAuth2 setup utility for Gmail API access.
//!
//! This utility helps you obtain and save OAuth2 tokens for Gmail API access.
//! It performs the OAuth2 flow and saves the token to a file.

use std::env;
use std::fs;
use std::path::Path;
use yup_oauth2::{InstalledFlowAuthenticator, InstalledFlowReturnMethod};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔐 Gmail API OAuth2 Setup Utility");
    println!("==================================");

    // Check for client secret file
    let client_secret_path = env::var("GMAIL_CLIENT_SECRET_JSON").unwrap_or_else(|_| {
        println!(
            "⚠️  GMAIL_CLIENT_SECRET_JSON not set, using default: ./secrets/client-secret.json"
        );
        "./secrets/client-secret.json".to_string()
    });

    let token_path = env::var("GMAIL_TOKEN_JSON").unwrap_or_else(|_| {
        println!("⚠️  GMAIL_TOKEN_JSON not set, using default: ./secrets/token.json");
        "./secrets/token.json".to_string()
    });

    println!("📁 Client secret file: {client_secret_path}");
    println!("📁 Token will be saved to: {token_path}");

    // Check if client secret exists
    if !Path::new(&client_secret_path).exists() {
        eprintln!("❌ Error: Client secret file not found at {client_secret_path}");
        eprintln!("   Please download your OAuth2 client secret JSON from:");
        eprintln!("   https://console.developers.google.com/apis/credentials");
        eprintln!("   And place it at the specified path.");
        std::process::exit(1);
    }

    // Read client secret
    let secret_content = fs::read_to_string(&client_secret_path)
        .map_err(|e| format!("Failed to read client secret file: {e}"))?;
    println!(
        "📄 Read {} bytes from client secret file",
        secret_content.len()
    );

    let secret: yup_oauth2::ApplicationSecret = {
        // First try to parse as standard Google format
        let google_secret: serde_json::Value = serde_json::from_str(&secret_content)
            .map_err(|e| format!("Failed to parse client secret JSON: {e}"))?;

        // Check if it's in the Google "installed" format
        if let Some(installed) = google_secret.get("installed") {
            // Extract the fields from the "installed" object
            serde_json::from_value(installed.clone()).map_err(|e| {
                format!("Failed to parse client secret from 'installed' section: {e}")
            })?
        } else {
            // Try parsing as direct ApplicationSecret format
            serde_json::from_str(&secret_content)
                .map_err(|e| format!("Failed to parse client secret JSON: {e}"))?
        }
    };

    println!("✅ Client secret loaded successfully");

    // Create authenticator
    println!("🌐 Starting OAuth2 flow...");
    println!("   A browser window will open for authentication.");
    println!("   Please sign in to your Gmail account and grant the requested permissions.");
    println!("   ");
    println!("   ⚠️  IMPORTANT: This setup requests comprehensive Gmail permissions");
    println!("   including read access to individual message content.");
    println!("   These permissions are needed to fetch email subjects and bodies.");
    println!("   ");

    let auth = InstalledFlowAuthenticator::builder(secret, InstalledFlowReturnMethod::HTTPRedirect)
        .persist_tokens_to_disk(&token_path)
        .build()
        .await
        .map_err(|e| format!("Failed to build authenticator: {e}"))?;

    // Test the token by requesting it for Gmail scopes needed for full message access
    println!("🔑 Testing token with Gmail scopes...");
    println!("   Requesting scopes:");
    println!("   - gmail.readonly (read messages and labels)");
    println!("   - gmail.compose (needed for some message operations)");
    println!("   - gmail.modify (needed for full message access)");

    let scopes = &[
        "https://www.googleapis.com/auth/gmail.readonly",
        "https://www.googleapis.com/auth/gmail.modify",
        "https://www.googleapis.com/auth/gmail.compose",
    ];
    let _token = auth
        .token(scopes)
        .await
        .map_err(|e| format!("Failed to obtain token: {e}"))?;

    println!("✅ Token obtained successfully with required Gmail scopes");
    println!("   This token can now access individual Gmail message content");

    println!("✅ OAuth2 setup completed successfully!");
    println!("   Token saved to: {token_path}");
    println!("   ");
    println!("   🔍 The token file should now contain:");
    println!("   - access_token (for immediate API calls)");
    println!("   - refresh_token (for automatic token renewal)");
    println!("   - scope information (including Gmail permissions)");
    println!("   ");
    println!("   You can now run the main application:");
    println!("   cargo run");

    // Create or update environment script
    create_env_script(&client_secret_path, &token_path)
        .map_err(|e| format!("Failed to create environment script: {e}"))?;

    Ok(())
}

fn create_env_script(
    client_secret_path: &str,
    token_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let env_script = format!(
        r#"#!/bin/bash
# Gmail API Environment Variables
# Generated by auth_setup utility

export GMAIL_CLIENT_SECRET_JSON="{client_secret_path}"
export GMAIL_TOKEN_JSON="{token_path}"

echo "✅ Gmail API environment variables set:"
echo "   GMAIL_CLIENT_SECRET_JSON=$GMAIL_CLIENT_SECRET_JSON"
echo "   GMAIL_TOKEN_JSON=$GMAIL_TOKEN_JSON"
echo ""
echo "You can now run: cargo run"
"#
    );

    fs::write("./set_gmail_env.sh", env_script)?;

    // Make the script executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata("./set_gmail_env.sh")?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions("./set_gmail_env.sh", perms)?;
    }

    println!("📝 Created environment script: ./set_gmail_env.sh");
    println!("   Run: source ./set_gmail_env.sh");

    Ok(())
}
