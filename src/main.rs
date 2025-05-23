use clap::Parser;

mod git_utils;
mod jira_utils;

// Struct to hold information about each commit's validation status
#[derive(Debug)]
struct CommitValidationInfo {
    commit_hash_short: String,
    commit_summary: String,
    is_valid: bool,
    error_message: Option<String>,
    jira_keys_found: Vec<String>,
}

/// A simple program to parse command line arguments
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Jira URL
    #[clap(long)]
    jira_url: String,

    /// Username
    #[clap(long)]
    username: String,

    /// API token
    #[clap(long)]
    api_token: String,

    /// Start ref
    #[clap(long)]
    start_ref: String,

    /// End ref
    #[clap(long)]
    end_ref: String,
}

#[tokio::main]
async fn main() { // No longer returns Result, will exit directly
    let args = Args::parse();

    println!("Jira URL: {}", args.jira_url);
    println!("Username: {}", args.username);
    // API Token is sensitive, avoid printing it.
    // println!("API Token: {}", args.api_token); 
    println!("Start Ref: {}", args.start_ref);
    println!("End Ref: {}", args.end_ref);

    println!("\n>>> Step 1: Fetching commit information from Git repository...");
    let commits_info_result = git_utils::get_commit_messages(&args.start_ref, &args.end_ref);

    if let Err(e) = commits_info_result {
        eprintln!("Error fetching commit information from Git: {}", e); // Refined message
        std::process::exit(1);
    }

    let commit_briefs = commits_info_result.unwrap();
    if commit_briefs.is_empty() {
        println!("No commits found in the specified range ({}..{}).", args.start_ref, args.end_ref);
        println!("\n>>> Final Result: Validation SUCCESSFUL (No commits to validate).");
        std::process::exit(0);
    }

    let total_commits = commit_briefs.len();
    println!("Found {} commits to validate.", total_commits);
    
    let mut validation_infos: Vec<CommitValidationInfo> = Vec::new();
    let mut valid_commits_count = 0;

    println!("\n>>> Step 2: Validating individual commits...");
    for (index, commit_brief) in commit_briefs.iter().enumerate() {
        print!("  ({}/{}) Validating commit {} ('{}')... ", 
                 index + 1, total_commits, commit_brief.id, commit_brief.summary.lines().next().unwrap_or(""));

        let jira_keys = jira_utils::extract_jira_keys(&commit_brief.summary);
        let mut current_commit_is_valid = false;
        let mut error_msg: Option<String> = None;
        let mut first_key_details = "".to_string();

        if jira_keys.is_empty() {
            error_msg = Some("No Jira keys found in commit summary.".to_string());
        } else {
            let first_key = &jira_keys[0];
            first_key_details = format!(" (Jira Key: {})", first_key);
            match jira_utils::check_jira_issue_exists(
                &args.jira_url,
                &args.username,
                &args.api_token, // Note: API token is used here, but not printed earlier.
                first_key,
            )
            .await
            {
                Ok(true) => {
                    current_commit_is_valid = true;
                }
                Ok(false) => {
                    error_msg = Some(format!("Jira key '{}' not found in Jira project (HTTP 404).", first_key));
                }
                Err(e) => {
                    error_msg = Some(format!("Error validating Jira key '{}': {}", first_key, e));
                    // Optionally, could still print the specific error to stderr here if very verbose logging is desired
                    // eprintln!("\n    Error during Jira check for key {}: {}", first_key, e);
                }
            }
        }

        if current_commit_is_valid {
            valid_commits_count += 1;
            println!("VALID{}",first_key_details);
        } else {
             println!("INVALID - Error: {}", error_msg.as_deref().unwrap_or("Unknown validation error."));
        }

        validation_infos.push(CommitValidationInfo {
            commit_hash_short: commit_brief.id.clone(),
            commit_summary: commit_brief.summary.clone(),
            is_valid: current_commit_is_valid,
            error_message: error_msg, // Store the detailed error message
            jira_keys_found: jira_keys,
        });
    }

    println!("\n-------------------------------------");
    println!(">>> Step 3: Final Validation Summary");
    println!("-------------------------------------");
    println!("Total commits scanned: {}", total_commits);
    println!("Valid commits: {}", valid_commits_count);
    let invalid_commits_count = total_commits - valid_commits_count;
    println!("Invalid commits: {}", invalid_commits_count);

    if invalid_commits_count > 0 {
        println!("\nDetails of INVALID commits:");
        for info in &validation_infos {
            if !info.is_valid {
                println!("  - Commit: {} ('{}')", 
                         info.commit_hash_short, 
                         info.commit_summary.lines().next().unwrap_or(""));
                println!("    Error: {}", info.error_message.as_deref().unwrap_or("Unknown validation error."));
                if !info.jira_keys_found.is_empty() {
                    println!("    Jira Keys Found: {}", info.jira_keys_found.join(", "));
                } else {
                    println!("    Jira Keys Found: None");
                }
            }
        }
        println!("\n>>> Final Result: Validation FAILED.");
        std::process::exit(1);
    } else {
        println!("\n>>> Final Result: Validation SUCCESSFUL.");
        std::process::exit(0);
    }
}
