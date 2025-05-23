use git2::{Repository, Oid}; // Commit is not directly used here anymore for the return type

#[derive(Debug, Clone)] // Added Clone for easier use later if needed
pub struct CommitBriefInfo {
    pub id: String, // Commit hash (short or full)
    pub summary: String, // Commit summary
}

pub fn get_commit_messages(start_ref: &str, end_ref: &str) -> Result<Vec<CommitBriefInfo>, String> {
    let repo = Repository::open(".").map_err(|e| format!("Failed to open repository: {}", e))?;

    let start_oid = repo.revparse_single(start_ref)
        .map_err(|e| format!("Failed to resolve start_ref '{}': {}", start_ref, e))?
        .id();

    let end_commit_oid = repo.revparse_single(end_ref)
        .map_err(|e| format!("Failed to resolve end_ref '{}': {}", end_ref, e))?
        .id();

    let mut revwalk = repo.revwalk().map_err(|e| format!("Failed to create revwalk: {}", e))?;
    revwalk.push(end_commit_oid).map_err(|e| format!("Failed to push end_commit_oid: {}", e))?;
    
    // Attempt to hide the start_ref. If it's not an ancestor, this might not behave as expected
    // or could be an error. For typical linear history A..B, this sets A as the boundary.
    // If start_ref is meant to be inclusive, this logic needs to change.
    // For now, assuming start_ref is exclusive.
    if let Ok(start_commit_obj) = repo.revparse_single(start_ref) {
        if let Ok(start_commit) = start_commit_obj.as_commit() {
             // We want to hide the start_ref itself and all its parents.
            revwalk.hide(start_commit.id()).map_err(|e| format!("Failed to hide start_ref commit: {}", e))?;
        } else {
            // If start_ref is not a commit (e.g. a tag pointing to a non-commit), handle appropriately.
            // For simplicity, we'll error, but this could be refined.
            return Err(format!("start_ref '{}' does not point to a commit", start_ref));
        }
    } else {
        return Err(format!("Failed to resolve start_ref '{}' for hiding", start_ref));
    }


    let mut commits_info = Vec::new();
    for oid_result in revwalk {
        let oid = oid_result.map_err(|e| format!("Error during revwalk: {}", e))?;
        let commit = repo.find_commit(oid).map_err(|e| format!("Failed to find commit {}: {}", oid, e))?;
        
        let summary = commit.summary().unwrap_or("<No commit summary>").to_string();
        // Using short ID for brevity, can be changed to oid.to_string() for full hash
        let id = commit.as_object().short_id()
            .map(|buf| buf.as_str().unwrap_or("").to_string())
            .unwrap_or_else(|_| oid.to_string()); // Fallback to full Oid if short_id fails

        commits_info.push(CommitBriefInfo { id, summary });
    }
    
    // The revwalk typically goes from newest (end_ref) to oldest.
    // If the desired order is oldest to newest, reverse the commits_info.
    commits_info.reverse();

    Ok(commits_info)
}
