use agentic_mail_agent::action::impls::labeler::{ConcreteGmailLabeler, EmailLabeler};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧹 Cleaning up test labels...");
    
    let _ = rustls::crypto::ring::default_provider().install_default();
    let labeler = ConcreteGmailLabeler::from_env().await?;
    
    // Get all labels
    let all_labels = labeler.list_all_labels().await?;
    
    // Find test labels (any with TEST in the name)
    let test_labels: Vec<_> = all_labels.iter()
        .filter(|label| label.name.contains("TEST"))
        .collect();
    
    if test_labels.is_empty() {
        println!("  ✅ No test labels found");
        return Ok(());
    }
    
    println!("  🗑️ Found {} test labels to remove:", test_labels.len());
    for label in &test_labels {
        println!("    - {}", label.name);
    }
    
    // Delete each test label
    for label in test_labels {
        match labeler.delete_label(&label.id).await {
            Ok(_) => println!("    ✅ Deleted: {}", label.name),
            Err(e) => println!("    ❌ Failed to delete {}: {}", label.name, e),
        }
        
        // Small delay between deletions
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
    
    println!("🧹 Cleanup completed!");
    Ok(())
}