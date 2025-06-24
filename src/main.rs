use tokio; // for async runtime

mod fetcher;
mod types;

use fetcher::{EmailFetcher, StubFetcher};

fn main() {
    println!("Hello, Agentic Gmail Agent!");
    let fetcher = StubFetcher;
    match fetcher.fetch_unread_emails() {
        Ok(emails) => println!("Fetched {} unread emails.", emails.len()),
        Err(e) => eprintln!("Failed to fetch emails: {e}"),
    }
}

#[cfg(test)]
mod tests {
    // ...empty test module for now...
}
