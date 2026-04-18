use crate::{state::RelayState, terminal::TerminalConfig};
use axum::{
   Json,
   extract::{Path, State},
   http::StatusCode,
   response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
   collections::HashMap,
   path::{Path as FsPath, PathBuf},
   process::Stdio,
};
use tokio::{fs, io::AsyncWriteExt, process::Command};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RpcSuccess<T> {
   ok: bool,
   value: T,
}

#[derive(Serialize)]
struct RpcFailure {
   ok: bool,
   error: RpcError,
}

#[derive(Serialize)]
struct RpcError {
   message: String,
   code: String,
}

pub async fn health() -> impl IntoResponse {
   Json(serde_json::json!({ "ok": true, "name": "relay" }))
}

pub async fn version() -> impl IntoResponse {
   Json(serde_json::json!({ "version": env!("CARGO_PKG_VERSION") }))
}

pub async fn platform() -> impl IntoResponse {
   Json(serde_json::json!({
      "platform": std::env::consts::OS,
      "arch": std::env::consts::ARCH
   }))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DirectoryEntry {
   name: String,
   is_directory: bool,
   #[serde(rename = "isDirectory")]
   is_directory_alias: bool,
}

#[derive(Serialize)]
struct SymlinkInfo {
   is_symlink: bool,
   target: Option<String>,
   is_dir: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PathPayload {
   path: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WriteFilePayload {
   path: String,
   content: Option<String>,
   contents: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TwoPathPayload {
   source_path: String,
   target_path: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ClipboardEntry {
   path: String,
   is_dir: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FileClipboardState {
   entries: Vec<ClipboardEntry>,
   operation: String,
}

#[derive(Serialize)]
struct PastedEntry {
   source_path: String,
   destination_path: String,
   is_dir: bool,
}

#[derive(Deserialize)]
struct CreateTerminalPayload {
   config: TerminalConfig,
}

#[derive(Deserialize)]
struct TerminalWritePayload {
   id: String,
   data: String,
}

#[derive(Deserialize)]
struct TerminalResizePayload {
   id: String,
   rows: u16,
   cols: u16,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DialogFilterPayload {
   name: String,
   extensions: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OpenDialogPayload {
   directory: Option<bool>,
   multiple: Option<bool>,
   filters: Option<Vec<DialogFilterPayload>>,
   title: Option<String>,
   default_path: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveDialogPayload {
   filters: Option<Vec<DialogFilterPayload>>,
   title: Option<String>,
   default_path: Option<String>,
}

#[derive(Deserialize)]
struct TerminalIdPayload {
   id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClipboardPastePayload {
   target_directory: String,
}

#[derive(Deserialize)]
struct SearchFilesPayload {
   request: SearchFilesRequest,
}

#[derive(Deserialize)]
struct SearchFilesRequest {
   root_path: String,
   query: String,
   case_sensitive: Option<bool>,
   max_results: Option<usize>,
}

#[derive(Serialize)]
struct SearchMatch {
   line_number: usize,
   line_content: String,
   column_start: usize,
   column_end: usize,
}

#[derive(Serialize)]
struct FileSearchResult {
   file_path: String,
   matches: Vec<SearchMatch>,
   total_matches: usize,
}

pub async fn handle_rpc(
   State(state): State<RelayState>,
   Path(command): Path<String>,
   Json(payload): Json<Value>,
) -> impl IntoResponse {
   let result = match command.as_str() {
      "get_home_dir" => {
         let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
         return Json(RpcSuccess {
            ok: true,
            value: home,
         })
         .into_response();
      }
      "list_roots" => {
         let roots = std::env::var("RELAY_WORKSPACE_ROOTS")
            .map(|roots| {
               roots
                  .split(',')
                  .map(str::trim)
                  .filter(|root| !root.is_empty())
                  .map(ToString::to_string)
                  .collect::<Vec<_>>()
            })
            .unwrap_or_else(|_| vec![std::env::var("HOME").unwrap_or_else(|_| ".".to_string())]);
         return Json(RpcSuccess {
            ok: true,
            value: roots,
         })
         .into_response();
      }
      "read_file" => read_file(payload)
         .await
         .map(|value| serde_json::json!(value)),
      "write_file" => write_file(payload).await.map(|_| serde_json::json!(null)),
      "read_directory" => read_directory(payload)
         .await
         .map(|value| serde_json::json!(value)),
      "create_directory" => create_directory(payload)
         .await
         .map(|_| serde_json::json!(null)),
      "delete_path" => delete_path(payload).await.map(|_| serde_json::json!(null)),
      "move_file" | "rename_file" => move_file(payload).await.map(|_| serde_json::json!(null)),
      "copy_file" => copy_file(payload).await.map(|_| serde_json::json!(null)),
      "get_symlink_info" => get_symlink_info(payload)
         .await
         .map(|value| serde_json::json!(value)),
      "read_file_custom" => read_file(payload)
         .await
         .map(|value| serde_json::json!(value)),
      "reveal_item_in_dir" => reveal_item_in_dir(payload)
         .await
         .map(|_| serde_json::json!(null)),
      "open_file_external" => open_file_external(payload)
         .await
         .map(|_| serde_json::json!(null)),
      "dialog_open" => open_dialog(payload).await,
      "dialog_save" => save_dialog(payload).await,
      "search_files_content" => search_files_content(payload)
         .await
         .map(|value| serde_json::json!(value)),
      "clipboard_set" => clipboard_set(state, payload)
         .await
         .map(|_| serde_json::json!(null)),
      "clipboard_get" => clipboard_get(state)
         .await
         .map(|value| serde_json::json!(value)),
      "clipboard_clear" => clipboard_clear(state)
         .await
         .map(|_| serde_json::json!(null)),
      "clipboard_paste" => clipboard_paste(state, payload)
         .await
         .map(|value| serde_json::json!(value)),
      "list_shells" => Ok(serde_json::json!(crate::terminal::list_shells())),
      "create_terminal" => create_terminal(state, payload)
         .await
         .map(|value| serde_json::json!(value)),
      "terminal_write" => terminal_write(state, payload)
         .await
         .map(|_| serde_json::json!(null)),
      "terminal_resize" => terminal_resize(state, payload)
         .await
         .map(|_| serde_json::json!(null)),
      "close_terminal" => close_terminal(state, payload)
         .await
         .map(|_| serde_json::json!(null)),
      "get_system_theme" => Ok(serde_json::json!(get_system_theme().await)),
      "get_system_fonts" | "get_monospace_fonts" => Ok(serde_json::json!(Vec::<Value>::new())),
      "validate_font" => Ok(serde_json::json!(true)),
      "get_editorconfig_properties" => Ok(serde_json::json!({})),
      "format_code" => format_code(payload).await,
      "lint_code" => Ok(serde_json::json!({ "success": true, "diagnostics": [] })),
      "set_project_root" => set_project_root(state, payload)
         .await
         .map(|_| serde_json::json!(null)),
      "start_watching" => start_watching(state, payload)
         .await
         .map(|_| serde_json::json!(null)),
      "stop_watching" => stop_watching(state, payload)
         .await
         .map(|_| serde_json::json!(null)),
      command if command.starts_with("github_") => handle_github(command, payload)
         .await
         .map(|value| serde_json::json!(value)),
      command if command.starts_with("lsp_") => handle_lsp(state, command, payload).await,
      command if command.starts_with("ssh_") || command.contains("remote") => {
         handle_remote(state, command, payload).await
      }
      command
         if command.contains("database")
            || command.contains("sqlite")
            || command.contains("postgres")
            || command.contains("mongo")
            || command.contains("redis")
            || command.ends_with("_credential")
            || command == "list_saved_connections"
            || command == "save_connection"
            || command == "delete_saved_connection"
            || command == "test_connection" =>
      {
         handle_database(state, command, payload).await
      }
      command
         if command.contains("acp")
            || command.contains("chat")
            || command.contains("ai_provider")
            || command == "get_available_agents"
            || command == "install_acp_agent" =>
      {
         handle_ai(state, command, payload).await
      }
      command if command.starts_with("git_") => handle_git(command, payload)
         .await
         .map(|value| serde_json::json!(value)),
      "echo" => Ok(payload),
      _ => return unsupported_rpc(command),
   };

   match result {
      Ok(value) => Json(RpcSuccess { ok: true, value }).into_response(),
      Err(error) => (
         StatusCode::BAD_REQUEST,
         Json(RpcFailure {
            ok: false,
            error: RpcError {
               message: error,
               code: "rpc_error".to_string(),
            },
         }),
      )
         .into_response(),
   }
}

async fn handle_git(command: &str, payload: Value) -> Result<Value, String> {
   if command == "git_discover_repo" {
      let path = payload_string(&payload, "path")?;
      let path = safe_existing_path(&path).await?.display().to_string();
      return relay_version_control::git_discover_repo(path)
         .and_then(|value| serde_json::to_value(value).map_err(|error| error.to_string()));
   }

   let repo_path = payload_string(&payload, "repoPath")?;
   let repo_path = safe_existing_path(&repo_path).await?.display().to_string();

   macro_rules! json_result {
      ($expr:expr) => {
         $expr.and_then(|value| serde_json::to_value(value).map_err(|error| error.to_string()))
      };
   }
   macro_rules! unit_result {
      ($expr:expr) => {
         $expr.map(|_| serde_json::json!(null))
      };
   }

   match command {
      "git_status" => json_result!(relay_version_control::git_status(repo_path)),
      "git_init" => unit_result!(relay_version_control::git_init(repo_path)),
      "git_branches" => json_result!(relay_version_control::git_branches(repo_path)),
      "git_checkout" => json_result!(relay_version_control::git_checkout(
         repo_path,
         payload_string(&payload, "branchName")?,
      )),
      "git_create_branch" => unit_result!(relay_version_control::git_create_branch(
         repo_path,
         payload_string(&payload, "branchName")?,
         payload_optional_string(&payload, "fromBranch"),
      )),
      "git_delete_branch" => unit_result!(relay_version_control::git_delete_branch(
         repo_path,
         payload_string(&payload, "branchName")?,
      )),
      "git_add" => unit_result!(relay_version_control::git_add(
         repo_path,
         payload_string(&payload, "filePath")?,
      )),
      "git_reset" => unit_result!(relay_version_control::git_reset(
         repo_path,
         payload_string(&payload, "filePath")?,
      )),
      "git_add_all" => unit_result!(relay_version_control::git_add_all(repo_path)),
      "git_reset_all" => unit_result!(relay_version_control::git_reset_all(repo_path)),
      "git_discard_file_changes" => unit_result!(relay_version_control::git_discard_file_changes(
         repo_path,
         payload_string(&payload, "filePath")?,
      )),
      "git_discard_all_changes" => {
         unit_result!(relay_version_control::git_discard_all_changes(repo_path))
      }
      "git_diff_file" => json_result!(relay_version_control::git_diff_file(
         repo_path,
         payload_string(&payload, "filePath")?,
         payload_bool(&payload, "staged"),
      )),
      "git_diff_file_with_content" => {
         json_result!(relay_version_control::git_diff_file_with_content(
            repo_path,
            payload_string(&payload, "filePath")?,
            payload_string(&payload, "content")?,
            payload_string(&payload, "base")?,
         ))
      }
      "git_commit_diff" => json_result!(relay_version_control::git_commit_diff(
         repo_path,
         payload_string(&payload, "commitHash")?,
         payload_optional_string(&payload, "filePath"),
      )),
      "git_log" => json_result!(relay_version_control::git_log(
         repo_path,
         payload_optional_u32(&payload, "limit"),
         payload_optional_u32(&payload, "skip"),
      )),
      "git_commit" => unit_result!(relay_version_control::git_commit(
         repo_path,
         payload_string(&payload, "message")?,
      )),
      "git_get_remotes" => json_result!(relay_version_control::git_get_remotes(repo_path)),
      "git_add_remote" => unit_result!(relay_version_control::git_add_remote(
         repo_path,
         payload_string(&payload, "name")?,
         payload_string(&payload, "url")?,
      )),
      "git_remove_remote" => unit_result!(relay_version_control::git_remove_remote(
         repo_path,
         payload_string(&payload, "name")?,
      )),
      "git_fetch" => unit_result!(relay_version_control::git_fetch(
         repo_path,
         payload_optional_string(&payload, "remote"),
      )),
      "git_push" => unit_result!(relay_version_control::git_push(
         repo_path,
         payload_optional_string(&payload, "branch"),
         payload_optional_string(&payload, "remote").unwrap_or_else(|| "origin".to_string()),
      )),
      "git_pull" => unit_result!(relay_version_control::git_pull(
         repo_path,
         payload_optional_string(&payload, "branch"),
         payload_optional_string(&payload, "remote").unwrap_or_else(|| "origin".to_string()),
      )),
      "git_get_tags" => json_result!(relay_version_control::git_get_tags(repo_path)),
      "git_get_stashes" => json_result!(relay_version_control::git_get_stashes(repo_path)),
      "git_create_stash" => unit_result!(relay_version_control::git_create_stash(
         repo_path,
         payload_optional_string(&payload, "message"),
         payload_bool(&payload, "includeUntracked"),
         payload_optional_string_array(&payload, "files"),
      )),
      "git_apply_stash" => unit_result!(relay_version_control::git_apply_stash(
         repo_path,
         payload_u64(&payload, "stashIndex")? as usize,
      )),
      "git_pop_stash" => unit_result!(relay_version_control::git_pop_stash(
         repo_path,
         payload_optional_u64(&payload, "stashIndex").map(|value| value as usize),
      )),
      "git_drop_stash" => unit_result!(relay_version_control::git_drop_stash(
         repo_path,
         payload_u64(&payload, "stashIndex")? as usize,
      )),
      "git_stash_diff" => json_result!(relay_version_control::git_stash_diff(
         repo_path,
         payload_u64(&payload, "stashIndex")? as usize,
      )),
      "git_blame_file" => json_result!(relay_version_control::git_blame_file(
         &repo_path,
         &payload_string(&payload, "filePath")?,
      )),
      _ => Err(format!(
         "RPC command '{}' is unavailable in relay-server",
         command
      )),
   }
}

async fn handle_github(command: &str, payload: Value) -> Result<Value, String> {
   match command {
      "github_check_cli_auth" => {
         if Command::new(resolve_gh_binary())
            .arg("--version")
            .output()
            .await
            .is_err()
         {
            return Ok(serde_json::json!("notInstalled"));
         }
         let output = gh_command(None)
            .args(["auth", "status"])
            .output()
            .await
            .map_err(|error| format!("failed to execute gh: {}", error))?;
         if output.status.success() {
            Ok(serde_json::json!("authenticated"))
         } else {
            Ok(serde_json::json!("notAuthenticated"))
         }
      }
      "github_get_current_user" => Ok(serde_json::json!(github_current_user().await?)),
      "github_list_prs" => {
         let repo_path = github_repo_path(&payload).await?;
         let filter = payload_optional_string(&payload, "filter").unwrap_or_else(|| "all".into());
         let fields = "number,title,state,author,createdAt,updatedAt,isDraft,reviewDecision,url,\
                       headRefName,baseRefName,additions,deletions";
         let mut args = vec!["pr", "list", "--json", fields];
         let mut owned_args = Vec::<String>::new();
         match filter.as_str() {
            "my-prs" => {
               let username = github_current_user().await?;
               owned_args.push(username);
               args.push("--author");
               args.push(owned_args.last().expect("username exists"));
            }
            "review-requests" => {
               args.push("--search");
               args.push("review-requested:@me");
            }
            _ => {}
         }
         gh_json(Some(&repo_path), &args).await
      }
      "github_list_issues" => {
         let repo_path = github_repo_path(&payload).await?;
         gh_json(
            Some(&repo_path),
            &[
               "issue",
               "list",
               "--state",
               "open",
               "--limit",
               "50",
               "--json",
               "number,title,state,author,updatedAt,url,labels",
            ],
         )
         .await
      }
      "github_list_workflow_runs" => {
         let repo_path = github_repo_path(&payload).await?;
         gh_json(
            Some(&repo_path),
            &[
               "run",
               "list",
               "--limit",
               "50",
               "--json",
               "databaseId,displayTitle,name,workflowName,event,status,conclusion,updatedAt,url,\
                headBranch,headSha",
            ],
         )
         .await
      }
      "github_open_pr_in_browser" => {
         let repo_path = github_repo_path(&payload).await?;
         let pr_number = payload_i64(&payload, "prNumber")?;
         gh_unit(
            Some(&repo_path),
            &["pr", "view", &pr_number.to_string(), "--web"],
         )
         .await
      }
      "github_checkout_pr" => {
         let repo_path = github_repo_path(&payload).await?;
         let pr_number = payload_i64(&payload, "prNumber")?;
         gh_unit(
            Some(&repo_path),
            &["pr", "checkout", &pr_number.to_string()],
         )
         .await
      }
      "github_get_pr_details" => {
         let repo_path = github_repo_path(&payload).await?;
         let pr_number = payload_i64(&payload, "prNumber")?;
         gh_json(
            Some(&repo_path),
            &[
               "pr",
               "view",
               &pr_number.to_string(),
               "--json",
               "number,title,body,state,author,createdAt,updatedAt,isDraft,reviewDecision,url,\
                headRefName,baseRefName,additions,deletions,changedFiles,commits,\
                statusCheckRollup,reviewRequests,mergeStateStatus,mergeable,labels,assignees",
            ],
         )
         .await
      }
      "github_get_pr_diff" => {
         let repo_path = github_repo_path(&payload).await?;
         let pr_number = payload_i64(&payload, "prNumber")?;
         let output = gh_output(Some(&repo_path), &["pr", "diff", &pr_number.to_string()]).await?;
         Ok(serde_json::json!(
            String::from_utf8_lossy(&output.stdout).to_string()
         ))
      }
      "github_get_pr_files" => {
         let repo_path = github_repo_path(&payload).await?;
         let pr_number = payload_i64(&payload, "prNumber")?;
         let value = gh_json(
            Some(&repo_path),
            &["pr", "view", &pr_number.to_string(), "--json", "files"],
         )
         .await?;
         Ok(value
            .get("files")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([])))
      }
      "github_get_pr_comments" => {
         let repo_path = github_repo_path(&payload).await?;
         let pr_number = payload_i64(&payload, "prNumber")?;
         let value = gh_json(
            Some(&repo_path),
            &["pr", "view", &pr_number.to_string(), "--json", "comments"],
         )
         .await?;
         Ok(value
            .get("comments")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([])))
      }
      "github_get_issue_details" => {
         let repo_path = github_repo_path(&payload).await?;
         let issue_number = payload_i64(&payload, "issueNumber")?;
         gh_json(
            Some(&repo_path),
            &[
               "issue",
               "view",
               &issue_number.to_string(),
               "--json",
               "number,title,body,state,author,createdAt,updatedAt,url,labels,assignees,comments",
            ],
         )
         .await
      }
      "github_get_workflow_run_details" => {
         let repo_path = github_repo_path(&payload).await?;
         let run_id = payload_i64(&payload, "runId")?;
         gh_json(
            Some(&repo_path),
            &[
               "run",
               "view",
               &run_id.to_string(),
               "--json",
               "databaseId,name,displayTitle,workflowName,event,status,conclusion,createdAt,\
                updatedAt,url,headBranch,headSha,jobs",
            ],
         )
         .await
      }
      _ => Err(format!(
         "RPC command '{}' is unavailable in relay-server",
         command
      )),
   }
}

async fn github_repo_path(payload: &Value) -> Result<PathBuf, String> {
   let repo_path = payload_string(payload, "repoPath")?;
   safe_existing_path(&repo_path).await
}

async fn github_current_user() -> Result<String, String> {
   let output = gh_output(None, &["api", "user", "--jq", ".login"]).await?;
   Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

async fn gh_json(repo_path: Option<&FsPath>, args: &[&str]) -> Result<Value, String> {
   let output = gh_output(repo_path, args).await?;
   serde_json::from_slice(&output.stdout).map_err(|error| error.to_string())
}

async fn gh_unit(repo_path: Option<&FsPath>, args: &[&str]) -> Result<Value, String> {
   gh_output(repo_path, args).await?;
   Ok(serde_json::json!(null))
}

async fn gh_output(
   repo_path: Option<&FsPath>,
   args: &[&str],
) -> Result<std::process::Output, String> {
   let mut command = gh_command(repo_path);
   command.args(args);
   let output = command
      .output()
      .await
      .map_err(|error| format!("failed to execute gh: {}", error))?;
   if output.status.success() {
      Ok(output)
   } else {
      Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
   }
}

fn gh_command(repo_path: Option<&FsPath>) -> Command {
   let mut command = Command::new(resolve_gh_binary());
   if let Some(repo_path) = repo_path {
      command.current_dir(repo_path);
   }
   command
}

fn resolve_gh_binary() -> String {
   let exe = if cfg!(target_os = "windows") {
      "gh.exe"
   } else {
      "gh"
   };
   let Some(paths) = std::env::var_os("PATH") else {
      return exe.to_string();
   };
   for dir in std::env::split_paths(&paths) {
      let path = dir.join(exe);
      if path.exists() {
         return path.display().to_string();
      }
   }
   exe.to_string()
}

async fn handle_lsp(state: RelayState, command: &str, payload: Value) -> Result<Value, String> {
   let manager = state.lsp_manager;
   match command {
      "lsp_start" => {
         manager
            .start_lsp_for_workspace(
               PathBuf::from(payload_string(&payload, "workspacePath")?),
               payload_optional_string(&payload, "serverPath"),
               payload_optional_string_array(&payload, "serverArgs"),
               payload.get("initializationOptions").cloned(),
            )
            .await
            .map_err(|error| error.to_string())?;
         Ok(serde_json::json!(null))
      }
      "lsp_start_for_file" => {
         manager
            .start_lsp_for_file(
               PathBuf::from(payload_string(&payload, "filePath")?),
               PathBuf::from(payload_string(&payload, "workspacePath")?),
               payload_optional_string(&payload, "serverPath"),
               payload_optional_string_array(&payload, "serverArgs"),
               payload.get("initializationOptions").cloned(),
            )
            .await
            .map_err(|error| error.to_string())?;
         Ok(serde_json::json!(null))
      }
      "lsp_stop" => {
         manager
            .shutdown_workspace(&PathBuf::from(payload_string(&payload, "workspacePath")?))
            .map_err(|error| error.to_string())?;
         Ok(serde_json::json!(null))
      }
      "lsp_stop_for_file" => {
         manager
            .stop_lsp_for_file(&PathBuf::from(payload_string(&payload, "filePath")?))
            .map_err(|error| error.to_string())?;
         Ok(serde_json::json!(null))
      }
      "lsp_get_completions" => to_json(
         manager
            .get_completions(
               &payload_string(&payload, "filePath")?,
               payload_u64(&payload, "line")? as u32,
               payload_u64(&payload, "character")? as u32,
            )
            .await
            .map_err(|error| error.to_string()),
      ),
      "lsp_get_hover" => to_json(
         manager
            .get_hover(
               &payload_string(&payload, "filePath")?,
               payload_u64(&payload, "line")? as u32,
               payload_u64(&payload, "character")? as u32,
            )
            .await
            .map_err(|error| error.to_string()),
      ),
      "lsp_get_definition" => {
         let response = manager
            .get_definition(
               &payload_string(&payload, "filePath")?,
               payload_u64(&payload, "line")? as u32,
               payload_u64(&payload, "character")? as u32,
            )
            .await
            .map_err(|error| error.to_string())?;
         to_json(flatten_definition_response(response))
      }
      "lsp_get_semantic_tokens" => {
         let response = manager
            .get_semantic_tokens(&payload_string(&payload, "filePath")?)
            .await
            .map_err(|error| error.to_string())?;
         to_json(flatten_semantic_tokens(response))
      }
      "lsp_get_code_lens" => {
         let response = manager
            .get_code_lens(&payload_string(&payload, "filePath")?)
            .await
            .map_err(|error| error.to_string())?;
         to_json(flatten_code_lens(response))
      }
      "lsp_get_inlay_hints" => {
         let response = manager
            .get_inlay_hints(
               &payload_string(&payload, "filePath")?,
               payload_u64(&payload, "startLine")? as u32,
               payload_u64(&payload, "endLine")? as u32,
            )
            .await
            .map_err(|error| error.to_string())?;
         to_json(flatten_inlay_hints(response))
      }
      "lsp_get_document_symbols" => {
         let response = manager
            .get_document_symbols(&payload_string(&payload, "filePath")?)
            .await
            .map_err(|error| error.to_string())?;
         to_json(flatten_document_symbol_response(response))
      }
      "lsp_get_signature_help" => to_json(
         manager
            .get_signature_help(
               &payload_string(&payload, "filePath")?,
               payload_u64(&payload, "line")? as u32,
               payload_u64(&payload, "character")? as u32,
            )
            .await
            .map_err(|error| error.to_string()),
      ),
      "lsp_get_references" => to_json(
         manager
            .get_references(
               &payload_string(&payload, "filePath")?,
               payload_u64(&payload, "line")? as u32,
               payload_u64(&payload, "character")? as u32,
            )
            .await
            .map_err(|error| error.to_string()),
      ),
      "lsp_rename" => to_json(
         manager
            .rename(
               &payload_string(&payload, "filePath")?,
               payload_u64(&payload, "line")? as u32,
               payload_u64(&payload, "character")? as u32,
               payload_string(&payload, "newName")?,
            )
            .await
            .map_err(|error| error.to_string()),
      ),
      "lsp_get_code_actions" => {
         let diagnostic: LspDiagnosticContext =
            serde_json::from_value(payload.get("diagnostic").cloned().unwrap_or(Value::Null))
               .map_err(|error| error.to_string())?;
         let actions = manager
            .get_code_actions(
               &payload_string(&payload, "filePath")?,
               diagnostic.into_diagnostic(),
            )
            .await
            .map_err(|error| error.to_string())?;
         to_json(flatten_code_actions(actions))
      }
      "lsp_apply_code_action" => {
         let action_payload = payload
            .get("actionPayload")
            .cloned()
            .ok_or_else(|| "missing field 'actionPayload'".to_string())?;
         let action = serde_json::from_value(action_payload).map_err(|error| error.to_string())?;
         let (applied, reason) = manager
            .apply_code_action(&payload_string(&payload, "filePath")?, action)
            .await
            .map_err(|error| error.to_string())?;
         Ok(serde_json::json!({ "applied": applied, "reason": reason }))
      }
      "lsp_document_open" => {
         manager
            .notify_document_open(
               &payload_string(&payload, "filePath")?,
               payload_string(&payload, "content")?,
               payload_optional_string(&payload, "languageId"),
            )
            .map_err(|error| error.to_string())?;
         Ok(serde_json::json!(null))
      }
      "lsp_document_change" => {
         manager
            .notify_document_change(
               &payload_string(&payload, "filePath")?,
               payload_string(&payload, "content")?,
               payload_i64(&payload, "version")? as i32,
            )
            .map_err(|error| error.to_string())?;
         Ok(serde_json::json!(null))
      }
      "lsp_document_close" => {
         manager
            .notify_document_close(&payload_string(&payload, "filePath")?)
            .map_err(|error| error.to_string())?;
         Ok(serde_json::json!(null))
      }
      _ => Err(format!(
         "RPC command '{}' is unavailable in relay-server",
         command
      )),
   }
}

async fn handle_remote(state: RelayState, command: &str, payload: Value) -> Result<Value, String> {
   match command {
      "ssh_connect" => {
         let connection = relay_remote::ssh_connect(
            payload_string(&payload, "connectionId")?,
            payload_string(&payload, "host")?,
            payload_u64(&payload, "port")? as u16,
            payload_string(&payload, "username")?,
            payload_optional_string(&payload, "password"),
            payload_optional_string(&payload, "keyPath"),
            payload_bool(&payload, "useSftp"),
         )
         .await?;
         state.events.emit(
            "ssh_connection_status",
            serde_json::json!({ "connectionId": connection.id, "connected": true }),
         );
         to_json(Ok(connection))
      }
      "ssh_disconnect" | "ssh_disconnect_only" => {
         let connection_id = payload_string(&payload, "connectionId")?;
         relay_remote::ssh_disconnect(connection_id.clone()).await?;
         state.events.emit(
            "ssh_connection_status",
            serde_json::json!({ "connectionId": connection_id, "connected": false }),
         );
         Ok(serde_json::json!(null))
      }
      "ssh_get_connected_ids" => to_json(relay_remote::ssh_get_connected_ids().await),
      "ssh_write_file" => {
         relay_remote::ssh_write_file(
            payload_string(&payload, "connectionId")?,
            payload_string(&payload, "filePath")?,
            payload_string(&payload, "content")?,
         )
         .await?;
         Ok(serde_json::json!(null))
      }
      "ssh_read_directory" => to_json(
         relay_remote::ssh_read_directory(
            payload_string(&payload, "connectionId")?,
            payload_string(&payload, "path")?,
         )
         .await,
      ),
      "ssh_read_file" => to_json(
         relay_remote::ssh_read_file(
            payload_string(&payload, "connectionId")?,
            payload_string(&payload, "filePath")?,
         )
         .await,
      ),
      "ssh_create_file" => {
         relay_remote::ssh_create_file(
            payload_string(&payload, "connectionId")?,
            payload_string(&payload, "filePath")?,
         )
         .await?;
         Ok(serde_json::json!(null))
      }
      "ssh_create_directory" => {
         relay_remote::ssh_create_directory(
            payload_string(&payload, "connectionId")?,
            payload_string(&payload, "directoryPath")?,
         )
         .await?;
         Ok(serde_json::json!(null))
      }
      "ssh_delete_path" => {
         relay_remote::ssh_delete_path(
            payload_string(&payload, "connectionId")?,
            payload_string(&payload, "targetPath")?,
            payload_bool(&payload, "isDirectory"),
         )
         .await?;
         Ok(serde_json::json!(null))
      }
      "ssh_rename_path" => {
         relay_remote::ssh_rename_path(
            payload_string(&payload, "connectionId")?,
            payload_string(&payload, "sourcePath")?,
            payload_string(&payload, "targetPath")?,
         )
         .await?;
         Ok(serde_json::json!(null))
      }
      "ssh_copy_path" => {
         relay_remote::ssh_copy_path(
            payload_string(&payload, "connectionId")?,
            payload_string(&payload, "sourcePath")?,
            payload_string(&payload, "targetPath")?,
            payload_bool(&payload, "isDirectory"),
         )
         .await?;
         Ok(serde_json::json!(null))
      }
      "create_remote_terminal" => to_json(
         relay_remote::create_remote_terminal(
            state.event_sink,
            payload_string(&payload, "host")?,
            payload_u64(&payload, "port")? as u16,
            payload_string(&payload, "username")?,
            payload_optional_string(&payload, "password"),
            payload_optional_string(&payload, "keyPath"),
            payload_optional_string(&payload, "workingDirectory"),
            payload_u64(&payload, "rows")? as u16,
            payload_u64(&payload, "cols")? as u16,
         )
         .await,
      ),
      "remote_terminal_write" => {
         relay_remote::remote_terminal_write(
            payload_string(&payload, "id")?,
            payload_string(&payload, "data")?,
         )
         .await?;
         Ok(serde_json::json!(null))
      }
      "remote_terminal_resize" => {
         relay_remote::remote_terminal_resize(
            payload_string(&payload, "id")?,
            payload_u64(&payload, "rows")? as u16,
            payload_u64(&payload, "cols")? as u16,
         )
         .await?;
         Ok(serde_json::json!(null))
      }
      "close_remote_terminal" => {
         relay_remote::close_remote_terminal(payload_string(&payload, "id")?).await?;
         Ok(serde_json::json!(null))
      }
      "store_remote_credential" => {
         state
            .secrets
            .store(
               secret_key("remote_cred_", &payload_string(&payload, "connectionId")?),
               payload_string(&payload, "password")?,
            )
            .await
            .map_err(|error| error.to_string())?;
         Ok(serde_json::json!(null))
      }
      "get_remote_credential" => to_json(Ok(state
         .secrets
         .get(&secret_key(
            "remote_cred_",
            &payload_string(&payload, "connectionId")?,
         ))
         .await)),
      "remove_remote_credential" => {
         state
            .secrets
            .remove(&secret_key(
               "remote_cred_",
               &payload_string(&payload, "connectionId")?,
            ))
            .await
            .map_err(|error| error.to_string())?;
         Ok(serde_json::json!(null))
      }
      _ => Err(format!(
         "RPC command '{}' is unavailable in relay-server",
         command
      )),
   }
}

async fn handle_database(
   state: RelayState,
   command: &str,
   payload: Value,
) -> Result<Value, String> {
   let manager = state.database.as_ref();
   match command {
      "connect_database" => {
         let config = database_config(&payload)?;
         let password = payload_optional_string(&payload, "password");
         to_json(relay_database::connect_database(config, password, manager).await)
      }
      "disconnect_database" => to_json(
         relay_database::disconnect_database(payload_string(&payload, "connectionId")?, manager)
            .await,
      ),
      "test_connection" => {
         let config = database_config(&payload)?;
         let password = payload_optional_string(&payload, "password");
         to_json(relay_database::test_connection(config, password).await)
      }
      "store_db_credential" => {
         state
            .secrets
            .store(
               secret_key("db_cred_", &payload_string(&payload, "connectionId")?),
               payload_string(&payload, "password")?,
            )
            .await
            .map_err(|error| error.to_string())?;
         Ok(serde_json::json!(null))
      }
      "get_db_credential" => to_json(Ok(state
         .secrets
         .get(&secret_key(
            "db_cred_",
            &payload_string(&payload, "connectionId")?,
         ))
         .await)),
      "remove_db_credential" => {
         state
            .secrets
            .remove(&secret_key(
               "db_cred_",
               &payload_string(&payload, "connectionId")?,
            ))
            .await
            .map_err(|error| error.to_string())?;
         Ok(serde_json::json!(null))
      }
      "save_connection" => {
         save_json_list_item(&state, "db_saved_connections", "connection", payload)
            .await
            .map(|_| serde_json::json!(null))
      }
      "list_saved_connections" => {
         let json = state.secrets.get("db_saved_connections").await;
         Ok(match json {
            Some(json) => serde_json::from_str(&json).unwrap_or_else(|_| serde_json::json!([])),
            None => serde_json::json!([]),
         })
      }
      "delete_saved_connection" => {
         delete_saved_connection(&state, &payload_string(&payload, "connectionId")?).await?;
         Ok(serde_json::json!(null))
      }
      "get_sqlite_tables" => to_json(
         relay_database::providers::sqlite::get_sqlite_tables(db_path(&payload).await?).await,
      ),
      "query_sqlite" => to_json(
         relay_database::providers::sqlite::query_sqlite(
            db_path(&payload).await?,
            payload_string(&payload, "query")?,
         )
         .await,
      ),
      "query_sqlite_filtered" => {
         let params = serde_json::from_value(
            payload
               .get("params")
               .cloned()
               .ok_or_else(|| "missing field 'params'".to_string())?,
         )
         .map_err(|error| error.to_string())?;
         to_json(
            relay_database::providers::sqlite::query_sqlite_filtered(
               db_path(&payload).await?,
               params,
            )
            .await,
         )
      }
      "execute_sqlite" => to_json(
         relay_database::providers::sqlite::execute_sqlite(
            db_path(&payload).await?,
            payload_string(&payload, "statement")?,
         )
         .await,
      ),
      "insert_sqlite_row" => to_json(
         relay_database::providers::sqlite::insert_sqlite_row(
            db_path(&payload).await?,
            payload_string(&payload, "table")?,
            payload_string_array(&payload, "columns")?,
            payload_value_array(&payload, "values")?,
         )
         .await,
      ),
      "update_sqlite_row" => to_json(
         relay_database::providers::sqlite::update_sqlite_row(
            db_path(&payload).await?,
            payload_string(&payload, "table")?,
            payload_string_array(&payload, "setColumns")?,
            payload_value_array(&payload, "setValues")?,
            payload_string(&payload, "whereColumn")?,
            payload.get("whereValue").cloned().unwrap_or(Value::Null),
         )
         .await,
      ),
      "delete_sqlite_row" => to_json(
         relay_database::providers::sqlite::delete_sqlite_row(
            db_path(&payload).await?,
            payload_string(&payload, "table")?,
            payload_string(&payload, "whereColumn")?,
            payload.get("whereValue").cloned().unwrap_or(Value::Null),
         )
         .await,
      ),
      "get_sqlite_foreign_keys" => to_json(
         relay_database::providers::sqlite::get_sqlite_foreign_keys(
            db_path(&payload).await?,
            payload_string(&payload, "table")?,
         )
         .await,
      ),
      command
         if command.starts_with("get_duckdb_")
            || command.starts_with("query_duckdb")
            || command.starts_with("execute_duckdb")
            || command.contains("_duckdb_") =>
      {
         handle_duckdb(command, payload).await
      }
      command if command.contains("postgres") => handle_postgres(command, payload, manager).await,
      command if command.contains("mysql") => handle_mysql(command, payload, manager).await,
      command if command.contains("mongo") => handle_mongo(command, payload, manager).await,
      command if command.starts_with("redis_") => handle_redis(command, payload, manager).await,
      _ => Err(format!(
         "RPC command '{}' is unavailable in relay-server",
         command
      )),
   }
}

async fn handle_ai(state: RelayState, command: &str, payload: Value) -> Result<Value, String> {
   match command {
      "init_chat_database" => {
         state.chat_history.initialize()?;
         Ok(serde_json::json!(null))
      }
      "save_chat" => {
         state.chat_history.save_chat(
            serde_json::from_value(
               payload
                  .get("chat")
                  .cloned()
                  .ok_or_else(|| "missing field 'chat'".to_string())?,
            )
            .map_err(|error| error.to_string())?,
            serde_json::from_value(
               payload
                  .get("messages")
                  .cloned()
                  .unwrap_or_else(|| serde_json::json!([])),
            )
            .map_err(|error| error.to_string())?,
            serde_json::from_value(
               payload
                  .get("toolCalls")
                  .or_else(|| payload.get("tool_calls"))
                  .cloned()
                  .unwrap_or_else(|| serde_json::json!([])),
            )
            .map_err(|error| error.to_string())?,
         )?;
         Ok(serde_json::json!(null))
      }
      "load_all_chats" => to_json(Ok(state.chat_history.load_all_chats()?)),
      "load_chat" => to_json(Ok(state
         .chat_history
         .load_chat(&payload_string(&payload, "chatId")?)?)),
      "delete_chat" => {
         state
            .chat_history
            .delete_chat(&payload_string(&payload, "chatId")?)?;
         Ok(serde_json::json!(null))
      }
      "search_chats" => to_json(Ok(state
         .chat_history
         .search_chats(&payload_string(&payload, "query")?)?)),
      "get_chat_stats" => {
         let stats = state.chat_history.get_stats()?;
         Ok(serde_json::json!({
            "total_chats": stats.total_chats,
            "total_messages": stats.total_messages,
            "total_tool_calls": stats.total_tool_calls,
         }))
      }
      "store_ai_provider_token" => {
         state
            .secrets
            .store(
               secret_key("ai_token_", &payload_string(&payload, "providerId")?),
               payload_string(&payload, "token")?,
            )
            .await
            .map_err(|error| error.to_string())?;
         Ok(serde_json::json!(null))
      }
      "get_ai_provider_token" => to_json(Ok(state
         .secrets
         .get(&secret_key(
            "ai_token_",
            &payload_string(&payload, "providerId")?,
         ))
         .await)),
      "remove_ai_provider_token" => {
         state
            .secrets
            .remove(&secret_key(
               "ai_token_",
               &payload_string(&payload, "providerId")?,
            ))
            .await
            .map_err(|error| error.to_string())?;
         Ok(serde_json::json!(null))
      }
      "get_acp_status" => {
         let bridge = { state.acp_bridge.lock().await.clone() };
         serde_json::to_value(bridge.get_status().await).map_err(|error| error.to_string())
      }
      "get_available_agents" => {
         let mut bridge = state.acp_bridge.lock().await;
         serde_json::to_value(bridge.detect_agents()).map_err(|error| error.to_string())
      }
      "start_acp_agent" => {
         let bridge = { state.acp_bridge.lock().await.clone() };
         let status = bridge
            .start_agent(
               &payload_string(&payload, "agentId")?,
               payload_optional_string(&payload, "workspacePath"),
               payload_optional_string(&payload, "sessionId"),
            )
            .await
            .map_err(|error| error.to_string())?;
         serde_json::to_value(status).map_err(|error| error.to_string())
      }
      "stop_acp_agent" => {
         let bridge = { state.acp_bridge.lock().await.clone() };
         bridge
            .stop_agent()
            .await
            .map_err(|error| error.to_string())?;
         serde_json::to_value(bridge.get_status().await).map_err(|error| error.to_string())
      }
      "send_acp_prompt" => {
         let bridge = { state.acp_bridge.lock().await.clone() };
         bridge
            .send_prompt(&payload_string(&payload, "prompt")?)
            .await
            .map_err(|error| error.to_string())?;
         Ok(serde_json::json!(null))
      }
      "cancel_acp_prompt" => {
         let bridge = { state.acp_bridge.lock().await.clone() };
         bridge
            .cancel_prompt()
            .await
            .map_err(|error| error.to_string())?;
         Ok(serde_json::json!(null))
      }
      "respond_acp_permission" => {
         let args = payload.get("args").unwrap_or(&payload);
         let bridge = { state.acp_bridge.lock().await.clone() };
         bridge
            .respond_to_permission(
               payload_string(args, "requestId")?,
               payload_bool(args, "approved"),
               payload_bool(args, "cancelled"),
            )
            .await
            .map_err(|error| error.to_string())?;
         Ok(serde_json::json!(null))
      }
      "set_acp_session_config_option" => {
         let args = payload.get("args").unwrap_or(&payload);
         let bridge = { state.acp_bridge.lock().await.clone() };
         bridge
            .set_session_config_option(
               &payload_string(args, "configId")?,
               &payload_string(args, "value")?,
            )
            .await
            .map_err(|error| error.to_string())?;
         Ok(serde_json::json!(null))
      }
      "set_acp_session_mode" => {
         let bridge = { state.acp_bridge.lock().await.clone() };
         bridge
            .set_session_mode(&payload_string(&payload, "modeId")?)
            .await
            .map_err(|error| error.to_string())?;
         Ok(serde_json::json!(null))
      }
      "install_acp_agent" => install_acp_agent(state, payload).await,
      _ => Err(format!(
         "RPC command '{}' is unavailable in relay-server",
         command
      )),
   }
}

async fn install_acp_agent(state: RelayState, payload: Value) -> Result<Value, String> {
   let agent_id = payload_string(&payload, "agentId")?;
   let agent = {
      let mut bridge = state.acp_bridge.lock().await;
      bridge
         .detect_agents()
         .into_iter()
         .find(|agent| agent.id == agent_id)
         .ok_or_else(|| format!("Unknown ACP agent: {}", agent_id))?
   };

   let tool_config = tool_config_from_agent(&agent)?;
   let installed_binary = relay_tooling::ToolInstaller::install(&state.data_dir, &tool_config)
      .await
      .map_err(|error| error.to_string())?;
   write_acp_wrapper(&state.data_dir, &agent, &tool_config, &installed_binary).await?;

   let installed = {
      let mut bridge = state.acp_bridge.lock().await;
      bridge
         .refresh_agents()
         .into_iter()
         .find(|candidate| candidate.id == agent_id)
         .ok_or_else(|| format!("Installed ACP agent disappeared: {}", agent_id))?
   };

   serde_json::to_value(installed).map_err(|error| error.to_string())
}

fn tool_config_from_agent(
   agent: &relay_ai::AgentConfig,
) -> Result<relay_tooling::ToolConfig, String> {
   let runtime = match agent.install_runtime.clone() {
      Some(relay_ai::AgentRuntime::Node) => relay_tooling::ToolRuntime::Node,
      Some(relay_ai::AgentRuntime::Python) => relay_tooling::ToolRuntime::Python,
      Some(relay_ai::AgentRuntime::Go) => relay_tooling::ToolRuntime::Go,
      Some(relay_ai::AgentRuntime::Rust) => relay_tooling::ToolRuntime::Rust,
      Some(relay_ai::AgentRuntime::Binary) => relay_tooling::ToolRuntime::Binary,
      None => {
         return Err(format!(
            "{} does not support managed installation",
            agent.name
         ));
      }
   };

   let package = agent
      .install_package
      .clone()
      .ok_or_else(|| format!("{} is missing installation metadata", agent.name))?;

   Ok(relay_tooling::ToolConfig {
      name: agent.binary_name.clone(),
      command: agent.install_command.clone(),
      runtime,
      package: Some(package),
      download_url: None,
      args: vec![],
      env: HashMap::new(),
   })
}

async fn write_acp_wrapper(
   data_dir: &FsPath,
   agent: &relay_ai::AgentConfig,
   tool_config: &relay_tooling::ToolConfig,
   installed_binary: &FsPath,
) -> Result<(), String> {
   let wrapper_path = acp_wrapper_path(data_dir, &agent.id);
   if let Some(parent) = wrapper_path.parent() {
      fs::create_dir_all(parent)
         .await
         .map_err(|error| error.to_string())?;
   }

   let wrapper_contents = match agent.install_runtime {
      Some(relay_ai::AgentRuntime::Node) => {
         let managed_root = data_dir.join("runtimes");
         let node_path = relay_runtime::RuntimeManager::get_runtime(
            Some(&managed_root),
            relay_runtime::RuntimeType::Node,
         )
         .await
         .map_err(|error| error.to_string())?;
         let entrypoint = relay_tooling::ToolInstaller::get_lsp_launch_path(data_dir, tool_config)
            .map_err(|error| error.to_string())?;
         build_node_wrapper(&node_path, &entrypoint)
      }
      _ => build_binary_wrapper(installed_binary),
   };

   fs::write(&wrapper_path, wrapper_contents)
      .await
      .map_err(|error| error.to_string())?;
   make_wrapper_executable(&wrapper_path)?;
   Ok(())
}

fn acp_wrapper_path(data_dir: &FsPath, agent_id: &str) -> PathBuf {
   let file_name = if cfg!(windows) {
      format!("{agent_id}.cmd")
   } else {
      agent_id.to_string()
   };
   data_dir.join("tools").join("acp").join(file_name)
}

fn build_binary_wrapper(binary: &FsPath) -> String {
   #[cfg(target_os = "windows")]
   {
      format!("@echo off\r\n\"{}\" %*\r\n", binary.display())
   }

   #[cfg(not(target_os = "windows"))]
   {
      format!("#!/bin/sh\nexec \"{}\" \"$@\"\n", binary.display())
   }
}

fn build_node_wrapper(node_path: &FsPath, entrypoint: &FsPath) -> String {
   #[cfg(target_os = "windows")]
   {
      format!(
         "@echo off\r\n\"{}\" \"{}\" %*\r\n",
         node_path.display(),
         entrypoint.display()
      )
   }

   #[cfg(not(target_os = "windows"))]
   {
      format!(
         "#!/bin/sh\nexec \"{}\" \"{}\" \"$@\"\n",
         node_path.display(),
         entrypoint.display()
      )
   }
}

fn make_wrapper_executable(path: &FsPath) -> Result<(), String> {
   #[cfg(unix)]
   {
      use std::os::unix::fs::PermissionsExt;
      let metadata = std::fs::metadata(path).map_err(|error| error.to_string())?;
      let mut permissions = metadata.permissions();
      permissions.set_mode(0o755);
      std::fs::set_permissions(path, permissions).map_err(|error| error.to_string())?;
   }

   #[cfg(not(unix))]
   {
      let _ = path;
   }

   Ok(())
}

fn to_json<T: Serialize>(result: Result<T, String>) -> Result<Value, String> {
   result.and_then(|value| serde_json::to_value(value).map_err(|error| error.to_string()))
}

fn secret_key(prefix: &str, id: &str) -> String {
   format!("{}{}", prefix, id)
}

fn database_config(payload: &Value) -> Result<relay_database::ConnectionConfig, String> {
   serde_json::from_value(
      payload
         .get("config")
         .cloned()
         .ok_or_else(|| "missing field 'config'".to_string())?,
   )
   .map_err(|error| error.to_string())
}

async fn db_path(payload: &Value) -> Result<String, String> {
   let path = payload_string(payload, "path")?;
   Ok(safe_existing_path(&path).await?.display().to_string())
}

async fn save_json_list_item(
   state: &RelayState,
   storage_key: &str,
   item_key: &str,
   payload: Value,
) -> Result<(), String> {
   let item = payload
      .get(item_key)
      .cloned()
      .ok_or_else(|| format!("missing field '{}'", item_key))?;
   let id = item
      .get("id")
      .and_then(Value::as_str)
      .ok_or_else(|| format!("missing string field '{}.id'", item_key))?;
   let mut items = match state.secrets.get(storage_key).await {
      Some(json) => serde_json::from_str::<Vec<Value>>(&json).unwrap_or_default(),
      None => Vec::new(),
   };
   if let Some(existing) = items
      .iter_mut()
      .find(|candidate| candidate.get("id").and_then(Value::as_str) == Some(id))
   {
      *existing = item;
   } else {
      items.push(item);
   }
   state
      .secrets
      .store(
         storage_key.to_string(),
         serde_json::to_string(&items).map_err(|error| error.to_string())?,
      )
      .await
      .map_err(|error| error.to_string())
}

async fn delete_saved_connection(state: &RelayState, connection_id: &str) -> Result<(), String> {
   let mut items = match state.secrets.get("db_saved_connections").await {
      Some(json) => serde_json::from_str::<Vec<Value>>(&json).unwrap_or_default(),
      None => Vec::new(),
   };
   items.retain(|candidate| candidate.get("id").and_then(Value::as_str) != Some(connection_id));
   state
      .secrets
      .store(
         "db_saved_connections".to_string(),
         serde_json::to_string(&items).map_err(|error| error.to_string())?,
      )
      .await
      .map_err(|error| error.to_string())?;
   state
      .secrets
      .remove(&secret_key("db_cred_", connection_id))
      .await
      .map_err(|error| error.to_string())
}

async fn handle_duckdb(command: &str, payload: Value) -> Result<Value, String> {
   match command {
      "get_duckdb_tables" => to_json(
         relay_database::providers::duckdb::get_duckdb_tables(db_path(&payload).await?).await,
      ),
      "query_duckdb" => to_json(
         relay_database::providers::duckdb::query_duckdb(
            db_path(&payload).await?,
            payload_string(&payload, "query")?,
         )
         .await,
      ),
      "query_duckdb_filtered" => {
         let params = serde_json::from_value(
            payload
               .get("params")
               .cloned()
               .ok_or_else(|| "missing field 'params'".to_string())?,
         )
         .map_err(|error| error.to_string())?;
         to_json(
            relay_database::providers::duckdb::query_duckdb_filtered(
               db_path(&payload).await?,
               params,
            )
            .await,
         )
      }
      "execute_duckdb" => to_json(
         relay_database::providers::duckdb::execute_duckdb(
            db_path(&payload).await?,
            payload_string(&payload, "statement")?,
         )
         .await,
      ),
      "insert_duckdb_row" => to_json(
         relay_database::providers::duckdb::insert_duckdb_row(
            db_path(&payload).await?,
            payload_string(&payload, "table")?,
            payload_string_array(&payload, "columns")?,
            payload_value_array(&payload, "values")?,
         )
         .await,
      ),
      "update_duckdb_row" => to_json(
         relay_database::providers::duckdb::update_duckdb_row(
            db_path(&payload).await?,
            payload_string(&payload, "table")?,
            payload_string_array(&payload, "setColumns")?,
            payload_value_array(&payload, "setValues")?,
            payload_string(&payload, "whereColumn")?,
            payload.get("whereValue").cloned().unwrap_or(Value::Null),
         )
         .await,
      ),
      "delete_duckdb_row" => to_json(
         relay_database::providers::duckdb::delete_duckdb_row(
            db_path(&payload).await?,
            payload_string(&payload, "table")?,
            payload_string(&payload, "whereColumn")?,
            payload.get("whereValue").cloned().unwrap_or(Value::Null),
         )
         .await,
      ),
      "get_duckdb_foreign_keys" => to_json(
         relay_database::providers::duckdb::get_duckdb_foreign_keys(
            db_path(&payload).await?,
            payload_string(&payload, "table")?,
         )
         .await,
      ),
      _ => Err(format!(
         "RPC command '{}' is unavailable in relay-server",
         command
      )),
   }
}

async fn handle_postgres(
   command: &str,
   payload: Value,
   manager: &relay_database::ConnectionManager,
) -> Result<Value, String> {
   let connection_id = || payload_string(&payload, "connectionId");
   match command {
      "get_postgres_tables" => to_json(
         relay_database::providers::postgres::get_postgres_tables(connection_id()?, manager).await,
      ),
      "query_postgres" => to_json(
         relay_database::providers::postgres::query_postgres(
            connection_id()?,
            payload_string(&payload, "query")?,
            manager,
         )
         .await,
      ),
      "query_postgres_filtered" => {
         let params = serde_json::from_value(
            payload
               .get("params")
               .cloned()
               .ok_or_else(|| "missing field 'params'".to_string())?,
         )
         .map_err(|error| error.to_string())?;
         to_json(
            relay_database::providers::postgres::query_postgres_filtered(
               connection_id()?,
               params,
               manager,
            )
            .await,
         )
      }
      "execute_postgres" => to_json(
         relay_database::providers::postgres::execute_postgres(
            connection_id()?,
            payload_string(&payload, "statement")?,
            manager,
         )
         .await,
      ),
      "get_postgres_foreign_keys" => to_json(
         relay_database::providers::postgres::get_postgres_foreign_keys(
            connection_id()?,
            payload_string(&payload, "table")?,
            manager,
         )
         .await,
      ),
      "get_postgres_table_schema" => to_json(
         relay_database::providers::postgres::get_postgres_table_schema(
            connection_id()?,
            payload_string(&payload, "table")?,
            manager,
         )
         .await,
      ),
      "insert_postgres_row" => to_json(
         relay_database::providers::postgres::insert_postgres_row(
            connection_id()?,
            payload_string(&payload, "table")?,
            payload_string_array(&payload, "columns")?,
            payload_value_array(&payload, "values")?,
            manager,
         )
         .await,
      ),
      "update_postgres_row" => to_json(
         relay_database::providers::postgres::update_postgres_row(
            connection_id()?,
            payload_string(&payload, "table")?,
            payload_string_array(&payload, "setColumns")?,
            payload_value_array(&payload, "setValues")?,
            payload_string(&payload, "whereColumn")?,
            payload.get("whereValue").cloned().unwrap_or(Value::Null),
            manager,
         )
         .await,
      ),
      "delete_postgres_row" => to_json(
         relay_database::providers::postgres::delete_postgres_row(
            connection_id()?,
            payload_string(&payload, "table")?,
            payload_string(&payload, "whereColumn")?,
            payload.get("whereValue").cloned().unwrap_or(Value::Null),
            manager,
         )
         .await,
      ),
      "get_postgres_subscription_info" => to_json(
         relay_database::providers::postgres::get_postgres_subscription_info(
            connection_id()?,
            payload_string(&payload, "subscription")?,
            manager,
         )
         .await,
      ),
      "get_postgres_subscription_status" => to_json(
         relay_database::providers::postgres::get_postgres_subscription_status(
            connection_id()?,
            payload_string(&payload, "subscription")?,
            manager,
         )
         .await,
      ),
      "create_postgres_subscription" => {
         let params = serde_json::from_value(
            payload
               .get("params")
               .cloned()
               .ok_or_else(|| "missing field 'params'".to_string())?,
         )
         .map_err(|error| error.to_string())?;
         to_json(
            relay_database::providers::postgres::create_postgres_subscription(
               connection_id()?,
               params,
               manager,
            )
            .await,
         )
      }
      "drop_postgres_subscription" => to_json(
         relay_database::providers::postgres::drop_postgres_subscription(
            connection_id()?,
            payload_string(&payload, "subscription")?,
            payload_bool(&payload, "withDropSlot"),
            manager,
         )
         .await,
      ),
      "set_postgres_subscription_enabled" => to_json(
         relay_database::providers::postgres::set_postgres_subscription_enabled(
            connection_id()?,
            payload_string(&payload, "subscription")?,
            payload_bool(&payload, "enabled"),
            manager,
         )
         .await,
      ),
      "refresh_postgres_subscription" => to_json(
         relay_database::providers::postgres::refresh_postgres_subscription(
            connection_id()?,
            payload_string(&payload, "subscription")?,
            payload_bool(&payload, "copyData"),
            manager,
         )
         .await,
      ),
      _ => Err(format!(
         "RPC command '{}' is unavailable in relay-server",
         command
      )),
   }
}

async fn handle_mysql(
   command: &str,
   payload: Value,
   manager: &relay_database::ConnectionManager,
) -> Result<Value, String> {
   let connection_id = || payload_string(&payload, "connectionId");
   match command {
      "get_mysql_tables" => to_json(
         relay_database::providers::mysql::get_mysql_tables(connection_id()?, manager).await,
      ),
      "query_mysql" => to_json(
         relay_database::providers::mysql::query_mysql(
            connection_id()?,
            payload_string(&payload, "query")?,
            manager,
         )
         .await,
      ),
      "query_mysql_filtered" => {
         let params = serde_json::from_value(
            payload
               .get("params")
               .cloned()
               .ok_or_else(|| "missing field 'params'".to_string())?,
         )
         .map_err(|error| error.to_string())?;
         to_json(
            relay_database::providers::mysql::query_mysql_filtered(
               connection_id()?,
               params,
               manager,
            )
            .await,
         )
      }
      "execute_mysql" => to_json(
         relay_database::providers::mysql::execute_mysql(
            connection_id()?,
            payload_string(&payload, "statement")?,
            manager,
         )
         .await,
      ),
      "get_mysql_foreign_keys" => to_json(
         relay_database::providers::mysql::get_mysql_foreign_keys(
            connection_id()?,
            payload_string(&payload, "table")?,
            manager,
         )
         .await,
      ),
      "get_mysql_table_schema" => to_json(
         relay_database::providers::mysql::get_mysql_table_schema(
            connection_id()?,
            payload_string(&payload, "table")?,
            manager,
         )
         .await,
      ),
      "insert_mysql_row" => to_json(
         relay_database::providers::mysql::insert_mysql_row(
            connection_id()?,
            payload_string(&payload, "table")?,
            payload_string_array(&payload, "columns")?,
            payload_value_array(&payload, "values")?,
            manager,
         )
         .await,
      ),
      "update_mysql_row" => to_json(
         relay_database::providers::mysql::update_mysql_row(
            connection_id()?,
            payload_string(&payload, "table")?,
            payload_string_array(&payload, "setColumns")?,
            payload_value_array(&payload, "setValues")?,
            payload_string(&payload, "whereColumn")?,
            payload.get("whereValue").cloned().unwrap_or(Value::Null),
            manager,
         )
         .await,
      ),
      "delete_mysql_row" => to_json(
         relay_database::providers::mysql::delete_mysql_row(
            connection_id()?,
            payload_string(&payload, "table")?,
            payload_string(&payload, "whereColumn")?,
            payload.get("whereValue").cloned().unwrap_or(Value::Null),
            manager,
         )
         .await,
      ),
      _ => Err(format!(
         "RPC command '{}' is unavailable in relay-server",
         command
      )),
   }
}

async fn handle_mongo(
   command: &str,
   payload: Value,
   manager: &relay_database::ConnectionManager,
) -> Result<Value, String> {
   match command {
      "get_mongo_databases" => to_json(
         relay_database::providers::mongodb::get_mongo_databases(
            payload_string(&payload, "connectionId")?,
            manager,
         )
         .await,
      ),
      "get_mongo_collections" => to_json(
         relay_database::providers::mongodb::get_mongo_collections(
            payload_string(&payload, "connectionId")?,
            payload_string(&payload, "database")?,
            manager,
         )
         .await,
      ),
      "query_mongo_documents" => to_json(
         relay_database::providers::mongodb::query_mongo_documents(
            payload_string(&payload, "connectionId")?,
            payload_string(&payload, "database")?,
            payload_string(&payload, "collection")?,
            payload_optional_string(&payload, "filterJson"),
            payload_optional_string(&payload, "sortJson"),
            payload_optional_u64(&payload, "limit").map(|value| value as i64),
            payload_optional_u64(&payload, "skip"),
            manager,
         )
         .await,
      ),
      "insert_mongo_document" => to_json(
         relay_database::providers::mongodb::insert_mongo_document(
            payload_string(&payload, "connectionId")?,
            payload_string(&payload, "database")?,
            payload_string(&payload, "collection")?,
            payload_string(&payload, "documentJson")?,
            manager,
         )
         .await,
      ),
      "update_mongo_document" => to_json(
         relay_database::providers::mongodb::update_mongo_document(
            payload_string(&payload, "connectionId")?,
            payload_string(&payload, "database")?,
            payload_string(&payload, "collection")?,
            payload_string(&payload, "filterJson")?,
            payload_string(&payload, "updateJson")?,
            manager,
         )
         .await,
      ),
      "delete_mongo_document" => to_json(
         relay_database::providers::mongodb::delete_mongo_document(
            payload_string(&payload, "connectionId")?,
            payload_string(&payload, "database")?,
            payload_string(&payload, "collection")?,
            payload_string(&payload, "filterJson")?,
            manager,
         )
         .await,
      ),
      _ => Err(format!(
         "RPC command '{}' is unavailable in relay-server",
         command
      )),
   }
}

async fn handle_redis(
   command: &str,
   payload: Value,
   manager: &relay_database::ConnectionManager,
) -> Result<Value, String> {
   match command {
      "redis_scan_keys" => {
         let keys = relay_database::providers::redis_db::redis_scan_keys(
            payload_string(&payload, "connectionId")?,
            payload_optional_string(&payload, "pattern"),
            payload_optional_u64(&payload, "count").map(|value| value as usize),
            manager,
         )
         .await?;
         Ok(serde_json::json!({ "keys": keys, "cursor": "0" }))
      }
      "redis_get_value" => to_json(
         relay_database::providers::redis_db::redis_get_value(
            payload_string(&payload, "connectionId")?,
            payload_string(&payload, "key")?,
            manager,
         )
         .await,
      ),
      "redis_set_value" => {
         relay_database::providers::redis_db::redis_set_value(
            payload_string(&payload, "connectionId")?,
            payload_string(&payload, "key")?,
            payload_string(&payload, "value")?,
            payload_optional_u64(&payload, "ttl").map(|value| value as i64),
            manager,
         )
         .await?;
         Ok(serde_json::json!(null))
      }
      "redis_delete_key" => to_json(
         relay_database::providers::redis_db::redis_delete_key(
            payload_string(&payload, "connectionId")?,
            payload_string(&payload, "key")?,
            manager,
         )
         .await,
      ),
      "redis_get_info" => to_json(
         relay_database::providers::redis_db::redis_get_info(
            payload_string(&payload, "connectionId")?,
            manager,
         )
         .await,
      ),
      _ => Err(format!(
         "RPC command '{}' is unavailable in relay-server",
         command
      )),
   }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LspDiagnosticContext {
   line: u32,
   column: u32,
   end_line: u32,
   end_column: u32,
   message: String,
   source: Option<String>,
   code: Option<String>,
   severity: Option<String>,
}

impl LspDiagnosticContext {
   fn into_diagnostic(self) -> lsp_types::Diagnostic {
      use lsp_types::{DiagnosticSeverity, NumberOrString, Position, Range};
      let severity = match self.severity.as_deref() {
         Some("error") => Some(DiagnosticSeverity::ERROR),
         Some("warning") => Some(DiagnosticSeverity::WARNING),
         Some("info") => Some(DiagnosticSeverity::INFORMATION),
         _ => None,
      };
      let code = self.code.map(|value| {
         value
            .parse::<i32>()
            .map(NumberOrString::Number)
            .unwrap_or(NumberOrString::String(value))
      });

      lsp_types::Diagnostic {
         range: Range {
            start: Position {
               line: self.line,
               character: self.column,
            },
            end: Position {
               line: self.end_line,
               character: self.end_column,
            },
         },
         severity,
         code,
         code_description: None,
         source: self.source,
         message: self.message,
         related_information: None,
         tags: None,
         data: None,
      }
   }
}

fn flatten_definition_response(
   response: Option<lsp_types::GotoDefinitionResponse>,
) -> Result<Option<Vec<lsp_types::Location>>, String> {
   Ok(match response {
      Some(lsp_types::GotoDefinitionResponse::Scalar(location)) => Some(vec![location]),
      Some(lsp_types::GotoDefinitionResponse::Array(locations)) => Some(locations),
      Some(lsp_types::GotoDefinitionResponse::Link(links)) => Some(
         links
            .into_iter()
            .map(|link| lsp_types::Location {
               uri: link.target_uri,
               range: link.target_selection_range,
            })
            .collect(),
      ),
      None => None,
   })
}

fn flatten_semantic_tokens(
   response: Option<lsp_types::SemanticTokensResult>,
) -> Result<Vec<Value>, String> {
   let data = match response {
      Some(lsp_types::SemanticTokensResult::Tokens(tokens)) => tokens.data,
      Some(lsp_types::SemanticTokensResult::Partial(partial)) => partial.data,
      None => return Ok(vec![]),
   };

   let mut current_line = 0;
   let mut current_char = 0;
   Ok(data
      .iter()
      .map(|token| {
         if token.delta_line > 0 {
            current_line += token.delta_line;
            current_char = token.delta_start;
         } else {
            current_char += token.delta_start;
         }
         serde_json::json!({
            "line": current_line,
            "startChar": current_char,
            "length": token.length,
            "tokenType": token.token_type,
            "tokenModifiers": token.token_modifiers_bitset,
         })
      })
      .collect())
}

fn flatten_code_lens(response: Option<Vec<lsp_types::CodeLens>>) -> Result<Vec<Value>, String> {
   Ok(response
      .unwrap_or_default()
      .into_iter()
      .filter_map(|lens| {
         let command = lens.command?;
         Some(serde_json::json!({
            "line": lens.range.start.line,
            "title": command.title,
            "command": command.command,
         }))
      })
      .collect())
}

fn flatten_inlay_hints(response: Option<Vec<lsp_types::InlayHint>>) -> Result<Vec<Value>, String> {
   Ok(response
      .unwrap_or_default()
      .into_iter()
      .map(|hint| {
         let label = match hint.label {
            lsp_types::InlayHintLabel::String(value) => value,
            lsp_types::InlayHintLabel::LabelParts(parts) => {
               parts.into_iter().map(|part| part.value).collect()
            }
         };
         let kind = hint.kind.map(|kind| match kind {
            lsp_types::InlayHintKind::TYPE => "type",
            lsp_types::InlayHintKind::PARAMETER => "parameter",
            _ => "other",
         });
         serde_json::json!({
            "line": hint.position.line,
            "character": hint.position.character,
            "label": label,
            "kind": kind,
            "paddingLeft": hint.padding_left.unwrap_or(false),
            "paddingRight": hint.padding_right.unwrap_or(false),
         })
      })
      .collect())
}

fn flatten_document_symbol_response(
   response: Option<lsp_types::DocumentSymbolResponse>,
) -> Result<Vec<Value>, String> {
   Ok(match response {
      Some(lsp_types::DocumentSymbolResponse::Flat(infos)) => infos
         .into_iter()
         .map(|info| {
            serde_json::json!({
               "name": info.name,
               "kind": symbol_kind_label(info.kind),
               "detail": Value::Null,
               "line": info.location.range.start.line,
               "character": info.location.range.start.character,
               "endLine": info.location.range.end.line,
               "endCharacter": info.location.range.end.character,
               "containerName": info.container_name,
            })
         })
         .collect(),
      Some(lsp_types::DocumentSymbolResponse::Nested(symbols)) => {
         flatten_document_symbols(&symbols, None)
      }
      None => vec![],
   })
}

fn flatten_document_symbols(
   symbols: &[lsp_types::DocumentSymbol],
   container: Option<&str>,
) -> Vec<Value> {
   let mut result = Vec::new();
   for symbol in symbols {
      result.push(serde_json::json!({
         "name": symbol.name,
         "kind": symbol_kind_label(symbol.kind),
         "detail": symbol.detail,
         "line": symbol.selection_range.start.line,
         "character": symbol.selection_range.start.character,
         "endLine": symbol.selection_range.end.line,
         "endCharacter": symbol.selection_range.end.character,
         "containerName": container,
      }));
      if let Some(children) = &symbol.children {
         result.extend(flatten_document_symbols(children, Some(&symbol.name)));
      }
   }
   result
}

fn flatten_code_actions(
   actions: Vec<lsp_types::CodeActionOrCommand>,
) -> Result<Vec<Value>, String> {
   Ok(actions
      .into_iter()
      .enumerate()
      .map(|(index, action)| {
         let payload = serde_json::to_value(&action).unwrap_or(Value::Null);
         match action {
            lsp_types::CodeActionOrCommand::Command(command) => serde_json::json!({
               "id": format!("command-{}", index),
               "title": command.title,
               "kind": Value::Null,
               "isPreferred": false,
               "disabledReason": Value::Null,
               "hasCommand": true,
               "hasEdit": false,
               "payload": payload,
            }),
            lsp_types::CodeActionOrCommand::CodeAction(code_action) => serde_json::json!({
               "id": format!("code-action-{}", index),
               "title": code_action.title,
               "kind": code_action.kind.as_ref().map(|kind| kind.as_str().to_string()),
               "isPreferred": code_action.is_preferred.unwrap_or(false),
               "disabledReason": code_action.disabled.as_ref().map(|disabled| disabled.reason.clone()),
               "hasCommand": code_action.command.is_some(),
               "hasEdit": code_action.edit.is_some(),
               "payload": payload,
            }),
         }
      })
      .collect())
}

fn symbol_kind_label(kind: lsp_types::SymbolKind) -> &'static str {
   match kind {
      lsp_types::SymbolKind::FILE => "file",
      lsp_types::SymbolKind::MODULE => "module",
      lsp_types::SymbolKind::NAMESPACE => "namespace",
      lsp_types::SymbolKind::PACKAGE => "package",
      lsp_types::SymbolKind::CLASS => "class",
      lsp_types::SymbolKind::METHOD => "method",
      lsp_types::SymbolKind::PROPERTY => "property",
      lsp_types::SymbolKind::FIELD => "field",
      lsp_types::SymbolKind::CONSTRUCTOR => "constructor",
      lsp_types::SymbolKind::ENUM => "enum",
      lsp_types::SymbolKind::INTERFACE => "interface",
      lsp_types::SymbolKind::FUNCTION => "function",
      lsp_types::SymbolKind::VARIABLE => "variable",
      lsp_types::SymbolKind::CONSTANT => "constant",
      lsp_types::SymbolKind::STRING => "string",
      lsp_types::SymbolKind::NUMBER => "number",
      lsp_types::SymbolKind::BOOLEAN => "boolean",
      lsp_types::SymbolKind::ARRAY => "array",
      lsp_types::SymbolKind::OBJECT => "object",
      lsp_types::SymbolKind::KEY => "key",
      lsp_types::SymbolKind::NULL => "null",
      lsp_types::SymbolKind::ENUM_MEMBER => "enum-member",
      lsp_types::SymbolKind::STRUCT => "struct",
      lsp_types::SymbolKind::EVENT => "event",
      lsp_types::SymbolKind::OPERATOR => "operator",
      lsp_types::SymbolKind::TYPE_PARAMETER => "type-parameter",
      _ => "unknown",
   }
}

fn payload_string(payload: &Value, key: &str) -> Result<String, String> {
   payload
      .get(key)
      .and_then(Value::as_str)
      .map(ToString::to_string)
      .ok_or_else(|| format!("missing string field '{}'", key))
}

fn payload_optional_string(payload: &Value, key: &str) -> Option<String> {
   payload
      .get(key)
      .and_then(Value::as_str)
      .filter(|value| !value.is_empty())
      .map(ToString::to_string)
}

fn payload_bool(payload: &Value, key: &str) -> bool {
   payload.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn payload_u64(payload: &Value, key: &str) -> Result<u64, String> {
   payload
      .get(key)
      .and_then(Value::as_u64)
      .ok_or_else(|| format!("missing integer field '{}'", key))
}

fn payload_i64(payload: &Value, key: &str) -> Result<i64, String> {
   payload
      .get(key)
      .and_then(Value::as_i64)
      .ok_or_else(|| format!("missing integer field '{}'", key))
}

fn payload_optional_u64(payload: &Value, key: &str) -> Option<u64> {
   payload.get(key).and_then(Value::as_u64)
}

fn payload_optional_u32(payload: &Value, key: &str) -> Option<u32> {
   payload
      .get(key)
      .and_then(Value::as_u64)
      .and_then(|value| u32::try_from(value).ok())
}

fn payload_optional_string_array(payload: &Value, key: &str) -> Option<Vec<String>> {
   payload.get(key).and_then(Value::as_array).map(|values| {
      values
         .iter()
         .filter_map(Value::as_str)
         .map(ToString::to_string)
         .collect()
   })
}

fn payload_string_array(payload: &Value, key: &str) -> Result<Vec<String>, String> {
   payload_optional_string_array(payload, key)
      .ok_or_else(|| format!("missing string array field '{}'", key))
}

fn payload_value_array(payload: &Value, key: &str) -> Result<Vec<Value>, String> {
   payload
      .get(key)
      .and_then(Value::as_array)
      .cloned()
      .ok_or_else(|| format!("missing array field '{}'", key))
}

fn unsupported_rpc(command: String) -> axum::response::Response {
   (
      StatusCode::NOT_IMPLEMENTED,
      Json(RpcFailure {
         ok: false,
         error: RpcError {
            message: format!("RPC command '{}' is unavailable in relay-server", command),
            code: "unsupported_rpc".to_string(),
         },
      }),
   )
      .into_response()
}

async fn read_file(payload: Value) -> Result<String, String> {
   let payload: PathPayload = serde_json::from_value(payload).map_err(|error| error.to_string())?;
   let path = safe_existing_path(&payload.path).await?;
   fs::read_to_string(path)
      .await
      .map_err(|error| error.to_string())
}

async fn write_file(payload: Value) -> Result<(), String> {
   let payload: WriteFilePayload =
      serde_json::from_value(payload).map_err(|error| error.to_string())?;
   let path = safe_writable_path(&payload.path).await?;
   if let Some(parent) = path.parent() {
      fs::create_dir_all(parent)
         .await
         .map_err(|error| error.to_string())?;
   }
   let content = payload.content.or(payload.contents).unwrap_or_default();
   fs::write(path, content)
      .await
      .map_err(|error| error.to_string())
}

async fn read_directory(payload: Value) -> Result<Vec<DirectoryEntry>, String> {
   let payload: PathPayload = serde_json::from_value(payload).map_err(|error| error.to_string())?;
   let path = safe_existing_path(&payload.path).await?;
   let mut entries = fs::read_dir(path)
      .await
      .map_err(|error| error.to_string())?;
   let mut result = Vec::new();

   while let Some(entry) = entries
      .next_entry()
      .await
      .map_err(|error| error.to_string())?
   {
      let metadata = entry.metadata().await.map_err(|error| error.to_string())?;
      let is_directory = metadata.is_dir();
      result.push(DirectoryEntry {
         name: entry.file_name().to_string_lossy().to_string(),
         is_directory,
         is_directory_alias: is_directory,
      });
   }

   result.sort_by(|left, right| left.name.cmp(&right.name));
   Ok(result)
}

async fn create_directory(payload: Value) -> Result<(), String> {
   let payload: PathPayload = serde_json::from_value(payload).map_err(|error| error.to_string())?;
   let path = safe_writable_path(&payload.path).await?;
   fs::create_dir_all(path)
      .await
      .map_err(|error| error.to_string())
}

async fn delete_path(payload: Value) -> Result<(), String> {
   let payload: PathPayload = serde_json::from_value(payload).map_err(|error| error.to_string())?;
   let path = safe_existing_path(&payload.path).await?;
   let metadata = fs::metadata(&path)
      .await
      .map_err(|error| error.to_string())?;
   if metadata.is_dir() {
      fs::remove_dir_all(path)
         .await
         .map_err(|error| error.to_string())
   } else {
      fs::remove_file(path)
         .await
         .map_err(|error| error.to_string())
   }
}

async fn move_file(payload: Value) -> Result<(), String> {
   let payload: TwoPathPayload =
      serde_json::from_value(payload).map_err(|error| error.to_string())?;
   let source = safe_existing_path(&payload.source_path).await?;
   let target = safe_writable_path(&payload.target_path).await?;
   fs::rename(source, target)
      .await
      .map_err(|error| error.to_string())
}

async fn copy_file(payload: Value) -> Result<(), String> {
   let payload: TwoPathPayload =
      serde_json::from_value(payload).map_err(|error| error.to_string())?;
   let source = safe_existing_path(&payload.source_path).await?;
   let target = safe_writable_path(&payload.target_path).await?;
   fs::copy(source, target)
      .await
      .map(|_| ())
      .map_err(|error| error.to_string())
}

async fn get_symlink_info(payload: Value) -> Result<SymlinkInfo, String> {
   let payload: PathPayload = serde_json::from_value(payload).map_err(|error| error.to_string())?;
   let path = safe_existing_path(&payload.path).await?;
   let symlink_metadata = fs::symlink_metadata(&path)
      .await
      .map_err(|error| error.to_string())?;
   let target = if symlink_metadata.file_type().is_symlink() {
      fs::read_link(&path)
         .await
         .ok()
         .map(|target| target.display().to_string())
   } else {
      None
   };

   Ok(SymlinkInfo {
      is_symlink: symlink_metadata.file_type().is_symlink(),
      target,
      is_dir: fs::metadata(path)
         .await
         .map(|metadata| metadata.is_dir())
         .unwrap_or(false),
   })
}

async fn reveal_item_in_dir(payload: Value) -> Result<(), String> {
   let payload: PathPayload = serde_json::from_value(payload).map_err(|error| error.to_string())?;
   let path = safe_existing_path(&payload.path).await?;

   #[cfg(target_os = "macos")]
   let mut command = {
      let mut command = Command::new("open");
      command.arg("-R").arg(path);
      command
   };

   #[cfg(target_os = "linux")]
   let mut command = {
      let mut command = Command::new("xdg-open");
      command.arg(path.parent().unwrap_or(&path));
      command
   };

   #[cfg(target_os = "windows")]
   let mut command = {
      let mut command = Command::new("explorer");
      command.arg(format!("/select,{}", path.display()));
      command
   };

   command
      .stdout(Stdio::null())
      .stderr(Stdio::null())
      .spawn()
      .map(|_| ())
      .map_err(|error| error.to_string())
}

async fn open_file_external(payload: Value) -> Result<(), String> {
   let payload: PathPayload = serde_json::from_value(payload).map_err(|error| error.to_string())?;
   let path = safe_existing_path(&payload.path).await?;

   #[cfg(target_os = "macos")]
   let mut command = {
      let mut command = Command::new("open");
      command.arg(path);
      command
   };

   #[cfg(target_os = "linux")]
   let mut command = {
      let mut command = Command::new("xdg-open");
      command.arg(path);
      command
   };

   #[cfg(target_os = "windows")]
   let mut command = {
      let mut command = Command::new("cmd");
      command.arg("/C").arg("start").arg("").arg(path);
      command
   };

   command
      .stdout(Stdio::null())
      .stderr(Stdio::null())
      .spawn()
      .map(|_| ())
      .map_err(|error| error.to_string())
}

async fn open_dialog(payload: Value) -> Result<Value, String> {
   let options: OpenDialogPayload =
      serde_json::from_value(payload).map_err(|error| error.to_string())?;
   let result = tokio::task::spawn_blocking(move || {
      let dialog = configure_file_dialog(
         rfd::FileDialog::new(),
         options.title.as_deref(),
         options.default_path.as_deref(),
         options.filters.as_deref(),
      );

      if options.directory.unwrap_or(false) {
         if options.multiple.unwrap_or(false) {
            let paths = dialog
               .pick_folders()
               .unwrap_or_default()
               .into_iter()
               .map(|path| path.display().to_string())
               .collect::<Vec<_>>();
            return serde_json::json!(paths);
         }

         return match dialog.pick_folder() {
            Some(path) => serde_json::json!(path.display().to_string()),
            None => Value::Null,
         };
      }

      if options.multiple.unwrap_or(false) {
         let paths = dialog
            .pick_files()
            .unwrap_or_default()
            .into_iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>();
         return serde_json::json!(paths);
      }

      match dialog.pick_file() {
         Some(path) => serde_json::json!(path.display().to_string()),
         None => Value::Null,
      }
   })
   .await
   .map_err(|error| error.to_string())?;

   Ok(result)
}

async fn save_dialog(payload: Value) -> Result<Value, String> {
   let options: SaveDialogPayload =
      serde_json::from_value(payload).map_err(|error| error.to_string())?;
   let result = tokio::task::spawn_blocking(move || {
      let dialog = configure_file_dialog(
         rfd::FileDialog::new(),
         options.title.as_deref(),
         options.default_path.as_deref(),
         options.filters.as_deref(),
      );

      match dialog.save_file() {
         Some(path) => serde_json::json!(path.display().to_string()),
         None => Value::Null,
      }
   })
   .await
   .map_err(|error| error.to_string())?;

   Ok(result)
}

fn configure_file_dialog(
   mut dialog: rfd::FileDialog,
   title: Option<&str>,
   default_path: Option<&str>,
   filters: Option<&[DialogFilterPayload]>,
) -> rfd::FileDialog {
   if let Some(title) = title {
      dialog = dialog.set_title(title);
   }

   if let Some(default_path) = default_path.filter(|path| !path.is_empty()) {
      let path = FsPath::new(default_path);
      if path.is_dir() {
         dialog = dialog.set_directory(path);
      } else {
         if let Some(parent) = path.parent() {
            dialog = dialog.set_directory(parent);
         }
         if let Some(file_name) = path.file_name().and_then(|name| name.to_str()) {
            dialog = dialog.set_file_name(file_name);
         }
      }
   }

   for filter in filters.unwrap_or(&[]) {
      let extensions = filter
         .extensions
         .iter()
         .map(String::as_str)
         .collect::<Vec<_>>();
      if !extensions.is_empty() {
         dialog = dialog.add_filter(&filter.name, &extensions);
      }
   }

   dialog
}

async fn create_terminal(state: RelayState, payload: Value) -> Result<String, String> {
   let payload: CreateTerminalPayload =
      serde_json::from_value(payload).map_err(|error| error.to_string())?;
   if let Some(working_directory) = &payload.config.working_directory {
      safe_existing_path(working_directory).await?;
   }
   state
      .terminal_manager
      .create_terminal(payload.config)
      .map_err(|error| error.to_string())
}

async fn terminal_write(state: RelayState, payload: Value) -> Result<(), String> {
   let payload: TerminalWritePayload =
      serde_json::from_value(payload).map_err(|error| error.to_string())?;
   state
      .terminal_manager
      .write_to_terminal(&payload.id, &payload.data)
      .map_err(|error| error.to_string())
}

async fn terminal_resize(state: RelayState, payload: Value) -> Result<(), String> {
   let payload: TerminalResizePayload =
      serde_json::from_value(payload).map_err(|error| error.to_string())?;
   state
      .terminal_manager
      .resize_terminal(&payload.id, payload.rows, payload.cols)
      .map_err(|error| error.to_string())
}

async fn close_terminal(state: RelayState, payload: Value) -> Result<(), String> {
   let payload: TerminalIdPayload =
      serde_json::from_value(payload).map_err(|error| error.to_string())?;
   state
      .terminal_manager
      .close_terminal(&payload.id)
      .map_err(|error| error.to_string())
}

async fn clipboard_set(state: RelayState, payload: Value) -> Result<(), String> {
   let payload: FileClipboardState =
      serde_json::from_value(payload).map_err(|error| error.to_string())?;
   if payload.operation != "copy" && payload.operation != "cut" {
      return Err("clipboard operation must be copy or cut".to_string());
   }
   for entry in &payload.entries {
      safe_existing_path(&entry.path).await?;
   }
   *state.file_clipboard.lock().await = Some(payload);
   state
      .events
      .emit("file-clipboard-changed", serde_json::json!({}));
   Ok(())
}

async fn clipboard_get(state: RelayState) -> Result<Option<FileClipboardState>, String> {
   Ok(state.file_clipboard.lock().await.clone())
}

async fn clipboard_clear(state: RelayState) -> Result<(), String> {
   *state.file_clipboard.lock().await = None;
   state
      .events
      .emit("file-clipboard-changed", serde_json::json!({}));
   Ok(())
}

async fn clipboard_paste(state: RelayState, payload: Value) -> Result<Vec<PastedEntry>, String> {
   let payload: ClipboardPastePayload =
      serde_json::from_value(payload).map_err(|error| error.to_string())?;
   let target_directory = safe_existing_path(&payload.target_directory).await?;
   let clipboard = state
      .file_clipboard
      .lock()
      .await
      .clone()
      .ok_or_else(|| "clipboard is empty".to_string())?;

   let mut pasted = Vec::new();
   for entry in &clipboard.entries {
      let source = safe_existing_path(&entry.path).await?;
      let file_name = source
         .file_name()
         .ok_or_else(|| "clipboard source must have a file name".to_string())?;
      let destination = unique_destination(target_directory.join(file_name)).await;
      if entry.is_dir {
         copy_directory_recursive(&source, &destination).await?;
      } else {
         fs::copy(&source, &destination)
            .await
            .map_err(|error| error.to_string())?;
      }
      if clipboard.operation == "cut" {
         if entry.is_dir {
            fs::remove_dir_all(&source)
               .await
               .map_err(|error| error.to_string())?;
         } else {
            fs::remove_file(&source)
               .await
               .map_err(|error| error.to_string())?;
         }
      }
      pasted.push(PastedEntry {
         source_path: source.display().to_string(),
         destination_path: destination.display().to_string(),
         is_dir: entry.is_dir,
      });
   }

   if clipboard.operation == "cut" {
      *state.file_clipboard.lock().await = None;
   }
   state
      .events
      .emit("file-clipboard-changed", serde_json::json!({}));
   Ok(pasted)
}

async fn search_files_content(payload: Value) -> Result<Vec<FileSearchResult>, String> {
   let payload: SearchFilesPayload =
      serde_json::from_value(payload).map_err(|error| error.to_string())?;
   let root = safe_existing_path(&payload.request.root_path).await?;
   let query = payload.request.query;
   if query.is_empty() {
      return Ok(Vec::new());
   }
   let case_sensitive = payload.request.case_sensitive.unwrap_or(false);
   let query_cmp = if case_sensitive {
      query.clone()
   } else {
      query.to_lowercase()
   };
   let max_results = payload.request.max_results.unwrap_or(100).max(1);
   let mut results = Vec::new();
   let mut stack = vec![root];

   while let Some(path) = stack.pop() {
      if results.len() >= max_results {
         break;
      }
      let metadata = match fs::metadata(&path).await {
         Ok(metadata) => metadata,
         Err(_) => continue,
      };
      if metadata.is_dir() {
         if should_skip_search_dir(&path) {
            continue;
         }
         let mut entries = match fs::read_dir(&path).await {
            Ok(entries) => entries,
            Err(_) => continue,
         };
         while let Ok(Some(entry)) = entries.next_entry().await {
            stack.push(entry.path());
         }
         continue;
      }
      if metadata.len() > 2 * 1024 * 1024 {
         continue;
      }
      let content = match fs::read_to_string(&path).await {
         Ok(content) => content,
         Err(_) => continue,
      };
      let mut matches = Vec::new();
      for (line_index, line) in content.lines().enumerate() {
         let line_cmp = if case_sensitive {
            line.to_string()
         } else {
            line.to_lowercase()
         };
         let mut offset = 0;
         while let Some(index) = line_cmp[offset..].find(&query_cmp) {
            let start = offset + index;
            let end = start + query_cmp.len();
            matches.push(SearchMatch {
               line_number: line_index + 1,
               line_content: line.to_string(),
               column_start: start,
               column_end: end,
            });
            offset = end;
         }
      }
      if !matches.is_empty() {
         let total_matches = matches.len();
         results.push(FileSearchResult {
            file_path: path.display().to_string(),
            matches,
            total_matches,
         });
      }
   }

   Ok(results)
}

#[derive(Debug, Deserialize)]
struct FormatPayload {
   request: FormatRequest,
}

#[derive(Debug, Deserialize)]
struct FormatRequest {
   content: String,
   language: String,
   formatter: String,
   formatter_config: Option<FormatterConfig>,
   file_path: Option<String>,
   workspace_folder: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct FormatterConfig {
   command: String,
   args: Option<Vec<String>>,
   env: Option<HashMap<String, String>>,
   input_method: Option<String>,
   output_method: Option<String>,
}

async fn format_code(payload: Value) -> Result<Value, String> {
   let payload: FormatPayload =
      serde_json::from_value(payload).map_err(|error| error.to_string())?;
   let response = if let Some(config) = &payload.request.formatter_config {
      format_with_generic(
         &payload.request.content,
         config,
         payload.request.file_path.as_deref(),
         payload.request.workspace_folder.as_deref(),
      )
      .await
   } else {
      match payload.request.formatter.as_str() {
         "prettier" => {
            format_with_prettier(&payload.request.content, &payload.request.language).await
         }
         "rustfmt" => {
            run_stdin_formatter("rustfmt", &["--emit", "stdout"], &payload.request.content).await
         }
         "gofmt" => run_stdin_formatter("gofmt", &[], &payload.request.content).await,
         "eslint" => {
            run_stdin_formatter(
               "npx",
               &[
                  "eslint",
                  "--fix-dry-run",
                  "--stdin",
                  "--stdin-filename",
                  "temp.js",
               ],
               &payload.request.content,
            )
            .await
         }
         formatter => formatter_response(
            &payload.request.content,
            false,
            Some(format!("Unsupported formatter: {}", formatter)),
         ),
      }
   };

   Ok(response)
}

async fn format_with_generic(
   content: &str,
   config: &FormatterConfig,
   file_path: Option<&str>,
   workspace_folder: Option<&str>,
) -> Value {
   let command = substitute_formatter_variables(&config.command, file_path, workspace_folder);
   let args = config
      .args
      .as_ref()
      .map(|args| {
         args
            .iter()
            .map(|arg| substitute_formatter_variables(arg, file_path, workspace_folder))
            .collect::<Vec<_>>()
      })
      .unwrap_or_default();
   let input_method = config.input_method.as_deref().unwrap_or("stdin");
   let output_method = config.output_method.as_deref().unwrap_or("stdout");

   let mut cmd = Command::new(&command);
   cmd.args(&args).stderr(Stdio::piped());
   if input_method == "stdin" {
      cmd.stdin(Stdio::piped());
   }
   if output_method == "stdout" {
      cmd.stdout(Stdio::piped());
   }
   if let Some(env) = &config.env {
      for (key, value) in env {
         cmd.env(
            key,
            substitute_formatter_variables(value, file_path, workspace_folder),
         );
      }
   }

   run_formatter_command(
      cmd,
      content,
      input_method == "stdin",
      output_method == "stdout",
   )
   .await
}

async fn format_with_prettier(content: &str, language: &str) -> Value {
   let parser = match language {
      "javascript" | "js" => "babel",
      "typescript" | "ts" => "typescript",
      "json" => "json",
      "html" => "html",
      "css" => "css",
      "markdown" | "md" => "markdown",
      _ => "babel",
   };
   let extension = match language {
      "javascript" | "js" => "js",
      "typescript" | "ts" => "ts",
      "json" => "json",
      "html" => "html",
      "css" => "css",
      "markdown" | "md" => "md",
      _ => "js",
   };
   run_stdin_formatter(
      "npx",
      &[
         "prettier",
         "--parser",
         parser,
         "--stdin-filepath",
         &format!("temp.{}", extension),
      ],
      content,
   )
   .await
}

async fn run_stdin_formatter(command: &str, args: &[&str], content: &str) -> Value {
   let mut cmd = Command::new(command);
   cmd.args(args)
      .stdin(Stdio::piped())
      .stdout(Stdio::piped())
      .stderr(Stdio::piped());
   run_formatter_command(cmd, content, true, true).await
}

async fn run_formatter_command(
   mut cmd: Command,
   content: &str,
   write_stdin: bool,
   read_stdout: bool,
) -> Value {
   let mut child = match cmd.spawn() {
      Ok(child) => child,
      Err(error) => {
         return formatter_response(
            content,
            false,
            Some(format!("Formatter not available: {}", error)),
         );
      }
   };

   if write_stdin
      && let Some(mut stdin) = child.stdin.take()
      && let Err(error) = stdin.write_all(content.as_bytes()).await
   {
      return formatter_response(
         content,
         false,
         Some(format!("Failed to write to formatter stdin: {}", error)),
      );
   }

   match child.wait_with_output().await {
      Ok(output) if output.status.success() => {
         let formatted = if read_stdout {
            String::from_utf8_lossy(&output.stdout).to_string()
         } else {
            content.to_string()
         };
         serde_json::json!({
            "formatted_content": formatted,
            "success": true,
            "error": Value::Null,
         })
      }
      Ok(output) => formatter_response(
         content,
         false,
         Some(format!(
            "Formatter error: {}",
            String::from_utf8_lossy(&output.stderr)
         )),
      ),
      Err(error) => formatter_response(
         content,
         false,
         Some(format!("Failed to run formatter: {}", error)),
      ),
   }
}

fn formatter_response(content: &str, success: bool, error: Option<String>) -> Value {
   serde_json::json!({
      "formatted_content": content,
      "success": success,
      "error": error,
   })
}

fn substitute_formatter_variables(
   template: &str,
   file_path: Option<&str>,
   workspace_folder: Option<&str>,
) -> String {
   let mut result = template.to_string();

   if let Some(path) = file_path {
      let path_ref = FsPath::new(path);
      result = result.replace("${file}", path);
      result = result.replace(
         "${fileBasename}",
         path_ref
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(path),
      );
      result = result.replace(
         "${fileBasenameNoExtension}",
         path_ref
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or(path),
      );
      result = result.replace(
         "${fileDirname}",
         path_ref
            .parent()
            .and_then(|path| path.to_str())
            .unwrap_or(""),
      );
      result = result.replace(
         "${fileExtname}",
         &path_ref
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| format!(".{}", extension))
            .unwrap_or_default(),
      );
   }

   if let Some(workspace) = workspace_folder {
      result = result.replace("${workspaceFolder}", workspace);
   }

   result
}

async fn set_project_root(state: RelayState, payload: Value) -> Result<(), String> {
   let payload: PathPayload = serde_json::from_value(payload).map_err(|error| error.to_string())?;
   let path = safe_existing_path(&payload.path).await?;
   state
      .file_watcher
      .watch_project_root(path.display().to_string())
      .await
      .map_err(|error| error.to_string())
}

async fn start_watching(state: RelayState, payload: Value) -> Result<(), String> {
   let payload: PathPayload = serde_json::from_value(payload).map_err(|error| error.to_string())?;
   let path = safe_existing_path(&payload.path).await?;
   state
      .file_watcher
      .watch_path(path.display().to_string())
      .await
      .map_err(|error| error.to_string())
}

async fn stop_watching(state: RelayState, payload: Value) -> Result<(), String> {
   let payload: PathPayload = serde_json::from_value(payload).map_err(|error| error.to_string())?;
   let path = safe_existing_path(&payload.path).await?;
   state
      .file_watcher
      .stop_watching(path.display().to_string())
      .map_err(|error| error.to_string())
}

async fn unique_destination(path: PathBuf) -> PathBuf {
   if fs::metadata(&path).await.is_err() {
      return path;
   }

   let parent = path.parent().map(PathBuf::from).unwrap_or_default();
   let stem = path
      .file_stem()
      .map(|value| value.to_string_lossy().to_string())
      .unwrap_or_else(|| "copy".to_string());
   let extension = path
      .extension()
      .map(|value| value.to_string_lossy().to_string());

   for index in 1..1000 {
      let file_name = match &extension {
         Some(extension) => format!("{} copy {}.{}", stem, index, extension),
         None => format!("{} copy {}", stem, index),
      };
      let candidate = parent.join(file_name);
      if fs::metadata(&candidate).await.is_err() {
         return candidate;
      }
   }

   path
}

async fn copy_directory_recursive(source: &FsPath, destination: &FsPath) -> Result<(), String> {
   fs::create_dir_all(destination)
      .await
      .map_err(|error| error.to_string())?;
   let mut stack = vec![(source.to_path_buf(), destination.to_path_buf())];
   while let Some((current_source, current_destination)) = stack.pop() {
      let mut entries = fs::read_dir(&current_source)
         .await
         .map_err(|error| error.to_string())?;
      while let Some(entry) = entries
         .next_entry()
         .await
         .map_err(|error| error.to_string())?
      {
         let source_path = entry.path();
         let destination_path = current_destination.join(entry.file_name());
         let metadata = entry.metadata().await.map_err(|error| error.to_string())?;
         if metadata.is_dir() {
            fs::create_dir_all(&destination_path)
               .await
               .map_err(|error| error.to_string())?;
            stack.push((source_path, destination_path));
         } else {
            fs::copy(&source_path, &destination_path)
               .await
               .map_err(|error| error.to_string())?;
         }
      }
   }
   Ok(())
}

fn should_skip_search_dir(path: &FsPath) -> bool {
   matches!(
      path.file_name().and_then(|name| name.to_str()),
      Some(".git" | "node_modules" | "target" | "dist" | ".next" | ".turbo")
   )
}

async fn get_system_theme() -> &'static str {
   #[cfg(target_os = "macos")]
   {
      let output = Command::new("defaults")
         .args(["read", "-g", "AppleInterfaceStyle"])
         .stdout(Stdio::piped())
         .stderr(Stdio::null())
         .output()
         .await;
      if let Ok(output) = output
         && output.status.success()
         && String::from_utf8_lossy(&output.stdout)
            .trim()
            .eq_ignore_ascii_case("dark")
      {
         return "dark";
      }
      return "light";
   }

   #[cfg(not(target_os = "macos"))]
   {
      "light"
   }
}

async fn safe_existing_path(path: &str) -> Result<PathBuf, String> {
   let canonical = fs::canonicalize(path)
      .await
      .map_err(|error| error.to_string())?;
   ensure_allowed_path(&canonical).await?;
   Ok(canonical)
}

async fn safe_writable_path(path: &str) -> Result<PathBuf, String> {
   let path = PathBuf::from(path);
   let parent = path
      .parent()
      .filter(|parent| !parent.as_os_str().is_empty())
      .unwrap_or_else(|| FsPath::new("."));
   let canonical_parent = fs::canonicalize(parent)
      .await
      .map_err(|error| error.to_string())?;
   ensure_allowed_path(&canonical_parent).await?;
   let file_name = path
      .file_name()
      .ok_or_else(|| "path must include a file or directory name".to_string())?;
   Ok(canonical_parent.join(file_name))
}

async fn ensure_allowed_path(path: &FsPath) -> Result<(), String> {
   let roots = allowed_roots().await?;
   if roots.is_empty() {
      return Ok(());
   }

   let allow_symlink_escape = std::env::var("RELAY_ALLOW_SYMLINK_ESCAPE")
      .map(|value| value == "true")
      .unwrap_or(false);
   if allow_symlink_escape {
      return Ok(());
   }

   if roots.iter().any(|root| path.starts_with(root)) {
      Ok(())
   } else {
      Err("path is outside RELAY_WORKSPACE_ROOTS".to_string())
   }
}

async fn allowed_roots() -> Result<Vec<PathBuf>, String> {
   let Ok(raw_roots) = std::env::var("RELAY_WORKSPACE_ROOTS") else {
      return Ok(Vec::new());
   };

   let mut roots = Vec::new();
   for root in raw_roots
      .split(',')
      .map(str::trim)
      .filter(|root| !root.is_empty())
   {
      roots.push(
         fs::canonicalize(root).await.map_err(|error| {
            format!("invalid RELAY_WORKSPACE_ROOTS entry '{}': {}", root, error)
         })?,
      );
   }
   Ok(roots)
}
