mod fetcher;
mod types;

use fetcher::{EmailFetcher, GmailFetcher, StubFetcher};

fn main() {
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
            println!("Gmail credentials not found, using stub fetcher");
            Box::new(StubFetcher)
        }
    };
    
    match fetcher.fetch_unread_emails() {
        Ok(emails) => println!("Fetched {} unread emails.", emails.len()),
        Err(e) => eprintln!("Failed to fetch emails: {e}"),
    }
}

#[cfg(test)]
mod tests {
    // ...empty test module for now...
}
