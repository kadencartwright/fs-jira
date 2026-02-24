use std::sync::Arc;

use regex::Regex;

use crate::cache::InMemoryCache;
use crate::jira::JiraClient;
use crate::logging;
use crate::render::{render_issue_comments_markdown, render_issue_markdown};

pub fn seed_workspace_listings(
    jira: &JiraClient,
    cache: &InMemoryCache,
    workspaces: &[(String, String)],
) -> usize {
    let mut seeded = 0;
    for (workspace, jql) in workspaces {
        match jira.list_issue_refs_for_jql(jql) {
            Ok(items) => {
                let count = items.len();
                cache.upsert_workspace_issues(workspace, items);
                logging::info(format!(
                    "seeded workspace listing for {} with {} issues",
                    workspace, count
                ));
                seeded += 1;
            }
            Err(err) => {
                logging::warn(format!("failed to seed workspace {}: {}", workspace, err));
            }
        }
    }
    seeded
}

pub struct SyncResult {
    pub issues_cached: usize,
    pub issues_skipped: usize,
    pub errors: Vec<String>,
}

pub fn sync_issues(
    jira: &JiraClient,
    cache: &Arc<InMemoryCache>,
    workspaces: &[(String, String)],
    budget: usize,
    force_full: bool,
) -> SyncResult {
    let mut result = SyncResult {
        issues_cached: 0,
        issues_skipped: 0,
        errors: Vec::new(),
    };

    if budget == 0 {
        return result;
    }

    if !cache.has_persistence() {
        result
            .errors
            .push("cache.db_path must be configured for sync".to_string());
        return result;
    }

    for (workspace, base_jql) in workspaces {
        let cursor = if force_full {
            None
        } else {
            cache.get_sync_cursor(workspace)
        };

        let (base_filter, base_order) = split_jql_order_by(base_jql);
        let jql = match &cursor {
            Some(since) => {
                logging::info(format!(
                    "incremental sync for workspace {} since {}",
                    workspace, since
                ));
                let order_clause =
                    base_order.unwrap_or_else(|| "ORDER BY updated DESC".to_string());
                format!(
                    "({}) AND updated > \"{}\" {}",
                    base_filter, since, order_clause
                )
            }
            None => {
                logging::info(format!("initial full sync for workspace {}", workspace));
                base_jql.trim().to_string()
            }
        };

        let page_size = budget.min(100);

        match jira.search_issues_bulk(&jql, page_size) {
            Ok(issues) => {
                let latest_refs: Vec<_> = issues
                    .iter()
                    .map(|issue| crate::jira::IssueRef {
                        key: issue.key.clone(),
                        updated: issue.updated.clone(),
                    })
                    .collect();

                if cursor.is_none() {
                    cache.upsert_workspace_issues(workspace, latest_refs);
                } else {
                    let mut merged = cache
                        .get_workspace_issues_snapshot(workspace)
                        .map(|snapshot| snapshot.issues)
                        .unwrap_or_default();

                    for new_ref in latest_refs {
                        if let Some(existing) =
                            merged.iter_mut().find(|item| item.key == new_ref.key)
                        {
                            existing.updated = new_ref.updated.clone();
                        } else {
                            merged.push(new_ref);
                        }
                    }

                    merged.sort_by(|a, b| a.key.cmp(&b.key));
                    cache.upsert_workspace_issues(workspace, merged);
                }

                if issues.is_empty() {
                    logging::info(format!("sync for workspace {}: no changes", workspace));
                    result.issues_skipped += 1;
                    continue;
                }

                let remaining_budget = budget.saturating_sub(result.issues_cached);
                let count = issues.len().min(remaining_budget);

                let to_cache: Vec<_> = issues
                    .iter()
                    .take(count)
                    .map(|issue| {
                        let markdown = render_issue_markdown(issue).into_bytes();
                        (issue.key.clone(), markdown, issue.updated.clone())
                    })
                    .collect();

                let sidecars: Vec<_> = issues
                    .iter()
                    .take(count)
                    .map(|issue| {
                        (
                            issue.key.clone(),
                            render_issue_comments_markdown(issue).into_bytes(),
                            issue.updated.clone(),
                        )
                    })
                    .collect();

                let cached = cache.upsert_issues_batch(&to_cache);
                let _ = cache.upsert_issue_sidecars_batch(&sidecars);
                result.issues_cached += cached;

                if let Some(latest) = issues.first().and_then(|i| i.updated.as_ref()) {
                    cache.set_sync_cursor(workspace, latest);
                    logging::info(format!(
                        "updated sync cursor for workspace {} to {}",
                        workspace, latest
                    ));
                }

                logging::info(format!(
                    "sync for workspace {}: cached {} issues",
                    workspace, cached
                ));

                if result.issues_cached >= budget {
                    break;
                }
            }
            Err(err) => {
                let msg = format!("sync failed for workspace {}: {}", workspace, err);
                logging::warn(&msg);
                result.errors.push(msg);
            }
        }
    }

    result
}

fn split_jql_order_by(jql: &str) -> (String, Option<String>) {
    let order_re = Regex::new(r"(?i)\border\s+by\b").expect("valid order by regex");
    let trimmed = jql.trim();

    if let Some(matched) = order_re.find(trimmed) {
        let filter = trimmed[..matched.start()].trim().to_string();
        let order = trimmed[matched.start()..].trim().to_string();
        if filter.is_empty() {
            (trimmed.to_string(), None)
        } else {
            (filter, Some(order))
        }
    } else {
        (trimmed.to_string(), None)
    }
}

#[cfg(test)]
mod tests {
    use super::split_jql_order_by;

    #[test]
    fn split_jql_order_by_extracts_order_clause() {
        let (filter, order) = split_jql_order_by("project in (DEVO, DATA) ORDER BY updated DESC");
        assert_eq!(filter, "project in (DEVO, DATA)");
        assert_eq!(order.as_deref(), Some("ORDER BY updated DESC"));
    }

    #[test]
    fn split_jql_order_by_without_order_clause_keeps_query() {
        let (filter, order) = split_jql_order_by("project = DEVO");
        assert_eq!(filter, "project = DEVO");
        assert!(order.is_none());
    }
}
