use super::errors::GitHubError;
use octocrab::Octocrab;

/// Handler for GitHub comment operations
#[derive(Debug, Clone)]
#[allow(dead_code)] // Architectural - fields will be used when comment features are implemented
pub struct CommentHandler {
    octocrab: Octocrab,
    owner: String,
    repo: String,
}

#[allow(dead_code)] // Comment functionality for future GitHub integration
impl CommentHandler {
    pub fn new(octocrab: Octocrab, owner: String, repo: String) -> Self {
        Self {
            octocrab,
            owner,
            repo,
        }
    }

    /// Create a comment on an issue
    pub async fn create_issue_comment(
        &self,
        issue_number: u64,
        body: &str,
    ) -> Result<octocrab::models::issues::Comment, GitHubError> {
        let comment = self
            .octocrab
            .issues(&self.owner, &self.repo)
            .create_comment(issue_number, body)
            .await?;

        println!("💬 Created comment on issue #{issue_number}");
        Ok(comment)
    }

    /// Get comments for an issue
    pub async fn get_issue_comments(
        &self,
        issue_number: u64,
    ) -> Result<Vec<octocrab::models::issues::Comment>, GitHubError> {
        let comments = self
            .octocrab
            .issues(&self.owner, &self.repo)
            .list_comments(issue_number)
            .send()
            .await?;

        Ok(comments.items)
    }

    /// Update an existing comment
    pub async fn update_comment(
        &self,
        comment_id: u64,
        body: &str,
    ) -> Result<octocrab::models::issues::Comment, GitHubError> {
        let comment = self
            .octocrab
            .issues(&self.owner, &self.repo)
            .update_comment(octocrab::models::CommentId(comment_id), body)
            .await?;

        println!("✏️  Updated comment #{comment_id}");
        Ok(comment)
    }

    /// Delete a comment
    pub async fn delete_comment(&self, comment_id: u64) -> Result<(), GitHubError> {
        self.octocrab
            .issues(&self.owner, &self.repo)
            .delete_comment(octocrab::models::CommentId(comment_id))
            .await?;

        println!("🗑️  Deleted comment #{comment_id}");
        Ok(())
    }

    /// Create a comment on a pull request
    pub async fn create_pr_comment(
        &self,
        pr_number: u64,
        body: &str,
    ) -> Result<octocrab::models::issues::Comment, GitHubError> {
        // PR comments use the same API as issue comments
        self.create_issue_comment(pr_number, body).await
    }

    /// Get comments for a pull request
    pub async fn get_pr_comments(
        &self,
        pr_number: u64,
    ) -> Result<Vec<octocrab::models::issues::Comment>, GitHubError> {
        // PR comments use the same API as issue comments
        self.get_issue_comments(pr_number).await
    }

    // Review comments methods are commented out as they require different octocrab API patterns
    // These can be implemented when needed with the correct octocrab API calls

    /// Create a review comment on a pull request (inline code comment)
    pub async fn create_pr_review_comment(
        &self,
        pr_number: u64,
        body: &str,
        commit_id: &str,
        path: &str,
        line: u32,
    ) -> Result<octocrab::models::pulls::Comment, GitHubError> {
        use serde_json::json;
        
        let route = format!(
            "/repos/{}/{}/pulls/{}/comments",
            &self.owner, &self.repo, pr_number
        );
        
        let data = json!({
            "body": body,
            "commit_id": commit_id,
            "path": path,
            "line": line
        });
        
        let review_comment = self
            .octocrab
            .post(route, Some(&data))
            .await?;

        println!("💬 Created PR review comment on #{pr_number} at {path}:{line}");
        Ok(review_comment)
    }

    /// Get review comments for a pull request
    pub async fn get_pr_review_comments(
        &self,
        pr_number: u64,
    ) -> Result<Vec<octocrab::models::pulls::Comment>, GitHubError> {
        let comments = self
            .octocrab
            .pulls(&self.owner, &self.repo)
            .list_comments(Some(pr_number))
            .send()
            .await?;

        Ok(comments.items)
    }

    /// Update a PR review comment
    pub async fn update_pr_review_comment(
        &self,
        comment_id: u64,
        body: &str,
    ) -> Result<octocrab::models::pulls::Comment, GitHubError> {
        let comment = self
            .octocrab
            .pulls(&self.owner, &self.repo)
            .comment(octocrab::models::CommentId(comment_id))
            .update(body)
            .await?;

        println!("✏️  Updated PR review comment #{comment_id}");
        Ok(comment)
    }

    /// Delete a PR review comment  
    pub async fn delete_pr_review_comment(&self, comment_id: u64) -> Result<(), GitHubError> {
        self.octocrab
            .pulls(&self.owner, &self.repo)
            .comment(octocrab::models::CommentId(comment_id))
            .delete()
            .await?;

        println!("🗑️  Deleted PR review comment #{comment_id}");
        Ok(())
    }

    /// Search for comments containing specific text
    pub async fn search_comments(
        &self,
        issue_number: u64,
        search_text: &str,
    ) -> Result<Vec<octocrab::models::issues::Comment>, GitHubError> {
        let comments = self.get_issue_comments(issue_number).await?;

        let matching_comments = comments
            .into_iter()
            .filter(|comment| {
                comment
                    .body
                    .as_ref()
                    .map(|body| body.contains(search_text))
                    .unwrap_or(false)
            })
            .collect();

        Ok(matching_comments)
    }

    /// Get the latest comment on an issue
    pub async fn get_latest_comment(
        &self,
        issue_number: u64,
    ) -> Result<Option<octocrab::models::issues::Comment>, GitHubError> {
        let comments = self.get_issue_comments(issue_number).await?;

        // Comments are typically returned in chronological order, so get the last one
        Ok(comments.into_iter().last())
    }

    /// Count comments on an issue
    pub async fn count_comments(&self, issue_number: u64) -> Result<usize, GitHubError> {
        let comments = self.get_issue_comments(issue_number).await?;
        Ok(comments.len())
    }
}
