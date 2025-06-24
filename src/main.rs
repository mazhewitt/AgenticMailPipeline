use agentic_mail_agent::fetcher::{EmailFetcher, GmailFetcher, StubFetcher};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Install default crypto provider for rustls
    let _ = rustls::crypto::ring::default_provider().install_default();
    
    println!("Hello, Agentic Gmail Agent!");
    
    // Try to use real Gmail fetcher if environment variables are set,
    // otherwise use stub fetcher for development
    let fetcher: Box<dyn EmailFetcher> = match GmailFetcher::from_env() {
        Ok(gmail_fetcher) => {
            println!("Using Gmail API fetcher");
            Box::new(gmail_fetcher)
        }
        Err(_) => {
            println!("Gmail credentials not found, using stub fetcher with demo data");
            // Create some demo emails to show the new functionality
            use agentic_mail_agent::email::Email;
            let demo_emails = vec![
                Email::new(
                    "demo-1".to_string(),
                    Some("Welcome to Agentic Mail Agent".to_string()),
                    Some("This is a demo email showing the new subject and snippet functionality. The agent can now extract both the email subject line and a preview of the email content.".to_string())
                ),
                Email::new(
                    "demo-2".to_string(), 
                    Some("Meeting Reminder".to_string()),
                    Some("Don't forget about the team meeting tomorrow at 2 PM. We'll be discussing the new email processing features.".to_string())
                ),
                Email::with_id("demo-3".to_string()), // Email with no subject/snippet
            ];
            Box::new(StubFetcher::with_emails(demo_emails))
        }
    };
    
    match fetcher.fetch_unread_emails().await {
        Ok(emails) => {
            println!("Fetched {} unread emails.", emails.len());
            for email in &emails {
                println!("  ID: {}", email.id);
                if let Some(subject) = &email.subject {
                    println!("  Subject: {}", subject);
                } else {
                    println!("  Subject: (No subject)");
                }
                if let Some(snippet) = &email.snippet {
                    println!("  Preview: {}", snippet);
                } else {
                    println!("  Preview: (No preview)");
                }
                println!("  ---");
            }
        },
        Err(e) => eprintln!("Failed to fetch emails: {}", e),
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    // ...empty test module for now...
}
