use std::{
    collections::{BTreeMap, HashMap, HashSet},
    env,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use chrono::{TimeZone, Utc};
use serde::Serialize;
use serde_json::{json, Value};

use crate::redaction;

const RPC_TIMEOUT: Duration = Duration::from_secs(15);
const MAX_RPC_FRAME_BYTES: usize = 16 * 1024 * 1024;
const MAX_RUNS: u64 = 50;
const MAX_PREVIEW_CHARS: usize = 240;
const MAX_RUN_STEPS: usize = 1_000;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RuntimeConnectionState {
    Connected,
    Unavailable,
    Error,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexRuntimeSkill {
    pub name: String,
    pub source_name: String,
    pub scope: String,
    pub enabled: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexRuntimeHook {
    pub event_name: String,
    pub source_name: String,
    pub enabled: bool,
    pub trust_status: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexRunSummary {
    pub id: String,
    pub title: String,
    pub preview: String,
    pub status: String,
    pub source: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub parent_thread_id: Option<String>,
    pub git_branch: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexRuntimeSnapshot {
    pub state: RuntimeConnectionState,
    pub codex_version: Option<String>,
    pub observed_at: String,
    pub message: Option<String>,
    pub skills: Vec<CodexRuntimeSkill>,
    pub hooks: Vec<CodexRuntimeHook>,
    pub runs: Vec<CodexRunSummary>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexRunStep {
    pub id: String,
    pub turn_id: String,
    pub kind: String,
    pub label: String,
    pub status: Option<String>,
    pub detail: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexTurnSummary {
    pub id: String,
    pub status: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub duration_ms: Option<u64>,
    pub step_count: usize,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexRunDetail {
    pub id: String,
    pub title: String,
    pub status: String,
    pub turns: Vec<CodexTurnSummary>,
    pub steps: Vec<CodexRunStep>,
    pub item_type_counts: BTreeMap<String, usize>,
    pub completed_turns: usize,
    pub failed_turns: usize,
    pub truncated: bool,
}

struct RpcRequest {
    id: u64,
    method: &'static str,
    params: Value,
}

pub fn inspect_workspace(workspace: &Path) -> CodexRuntimeSnapshot {
    let observed_at = Utc::now().to_rfc3339();
    let Some(binary) = codex_binary() else {
        return CodexRuntimeSnapshot {
            state: RuntimeConnectionState::Unavailable,
            codex_version: None,
            observed_at,
            message: Some(
                "Codex CLI was not found. Set HARNESS_LENS_CODEX_BIN to connect runtime evidence."
                    .to_string(),
            ),
            skills: Vec::new(),
            hooks: Vec::new(),
            runs: Vec::new(),
        };
    };

    let workspace_path = workspace.to_string_lossy().into_owned();
    let requests = [
        RpcRequest {
            id: 1,
            method: "skills/list",
            params: json!({ "cwds": [workspace_path], "forceReload": true }),
        },
        RpcRequest {
            id: 2,
            method: "hooks/list",
            params: json!({ "cwds": [workspace_path] }),
        },
        RpcRequest {
            id: 3,
            method: "thread/list",
            params: json!({
                "limit": MAX_RUNS,
                "cwd": workspace_path,
                "sortKey": "updated_at",
                "sortDirection": "desc",
                "sourceKinds": [
                    "cli", "vscode", "exec", "appServer", "subAgent",
                    "subAgentReview", "subAgentCompact", "subAgentThreadSpawn",
                    "subAgentOther", "unknown"
                ]
            }),
        },
    ];

    match call_app_server(&binary, &requests) {
        Ok(responses) => CodexRuntimeSnapshot {
            state: RuntimeConnectionState::Connected,
            codex_version: codex_version(&binary),
            observed_at,
            message: None,
            skills: parse_skills(responses.get(&1)),
            hooks: parse_hooks(responses.get(&2)),
            runs: parse_runs(responses.get(&3)),
        },
        Err(error) => CodexRuntimeSnapshot {
            state: RuntimeConnectionState::Error,
            codex_version: codex_version(&binary),
            observed_at,
            message: Some(error),
            skills: Vec::new(),
            hooks: Vec::new(),
            runs: Vec::new(),
        },
    }
}

pub fn load_run(thread_id: &str) -> Result<CodexRunDetail, String> {
    let binary = codex_binary().ok_or_else(|| {
        "Codex CLI was not found. Set HARNESS_LENS_CODEX_BIN to connect runtime evidence."
            .to_string()
    })?;
    let responses = call_app_server(
        &binary,
        &[RpcRequest {
            id: 1,
            method: "thread/read",
            params: json!({ "threadId": thread_id, "includeTurns": true }),
        }],
    )?;
    parse_run_detail(
        responses
            .get(&1)
            .ok_or_else(|| "Codex App Server returned no run detail.".to_string())?,
    )
}

fn call_app_server(binary: &Path, requests: &[RpcRequest]) -> Result<HashMap<u64, Value>, String> {
    let mut child = Command::new(binary)
        .args(["app-server", "--listen", "stdio://"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("Unable to start Codex App Server: {error}"))?;

    let result = exchange_messages(&mut child, requests);
    let _ = child.kill();
    let _ = child.wait();
    result
}

fn exchange_messages(
    child: &mut Child,
    requests: &[RpcRequest],
) -> Result<HashMap<u64, Value>, String> {
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Codex App Server stdout was unavailable.".to_string())?;
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || read_bounded_json_lines(stdout, sender));

    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| "Codex App Server stdin was unavailable.".to_string())?;
    write_rpc(
        &mut stdin,
        &json!({
            "method": "initialize",
            "id": 0,
            "params": {
                "clientInfo": {
                    "name": "harness_lens",
                    "title": "Harness Lens",
                    "version": env!("CARGO_PKG_VERSION")
                },
                "capabilities": { "experimentalApi": true }
            }
        }),
    )?;
    stdin
        .flush()
        .map_err(|error| format!("Unable to send Codex App Server request: {error}"))?;

    let deadline = Instant::now() + RPC_TIMEOUT;
    let (_, initialize_result) = receive_response(&receiver, &HashSet::from([0]), deadline)?;
    if initialize_result.is_null() {
        return Err("Codex App Server returned an invalid initialize response.".to_string());
    }

    write_rpc(
        &mut stdin,
        &json!({ "method": "initialized", "params": {} }),
    )?;
    for request in requests {
        write_rpc(
            &mut stdin,
            &json!({ "method": request.method, "id": request.id, "params": request.params }),
        )?;
    }
    stdin
        .flush()
        .map_err(|error| format!("Unable to send Codex App Server request: {error}"))?;

    let mut responses = HashMap::new();
    let expected_ids = requests
        .iter()
        .map(|request| request.id)
        .collect::<HashSet<_>>();
    while responses.len() < requests.len() {
        let (id, result) = receive_response(&receiver, &expected_ids, deadline)?;
        responses.insert(id, result);
    }

    drop(stdin);
    drop(receiver);
    if responses.len() != requests.len() {
        return Err(format!(
            "Codex App Server timed out after {} seconds.",
            RPC_TIMEOUT.as_secs()
        ));
    }
    Ok(responses)
}

fn receive_response(
    receiver: &mpsc::Receiver<Result<Value, String>>,
    expected_ids: &HashSet<u64>,
    deadline: Instant,
) -> Result<(u64, Value), String> {
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(format!(
                "Codex App Server timed out after {} seconds.",
                RPC_TIMEOUT.as_secs()
            ));
        }
        match receiver.recv_timeout(remaining) {
            Ok(Ok(value)) => {
                let Some(id) = value.get("id").and_then(Value::as_u64) else {
                    continue;
                };
                if !expected_ids.contains(&id) {
                    continue;
                }
                if let Some(error) = value.get("error") {
                    let message = error
                        .get("message")
                        .and_then(Value::as_str)
                        .map(safe_preview)
                        .unwrap_or_else(|| "Unknown App Server error".to_string());
                    return Err(format!("Codex App Server rejected a request: {message}"));
                }
                let result = value.get("result").cloned().ok_or_else(|| {
                    "Codex App Server returned a response without a result.".to_string()
                })?;
                return Ok((id, result));
            }
            Ok(Err(error)) => return Err(error),
            Err(mpsc::RecvTimeoutError::Timeout) => {
                return Err(format!(
                    "Codex App Server timed out after {} seconds.",
                    RPC_TIMEOUT.as_secs()
                ));
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err("Codex App Server closed before completing the request.".to_string());
            }
        }
    }
}

fn read_bounded_json_lines(
    stdout: impl std::io::Read,
    sender: mpsc::Sender<Result<Value, String>>,
) {
    let mut reader = BufReader::new(stdout);
    let mut frame = Vec::new();
    loop {
        let buffer = match reader.fill_buf() {
            Ok(buffer) => buffer,
            Err(error) => {
                let _ = sender.send(Err(format!(
                    "Unable to read Codex App Server output: {error}"
                )));
                return;
            }
        };
        if buffer.is_empty() {
            return;
        }

        let newline = buffer.iter().position(|byte| *byte == b'\n');
        let consumed = newline.map_or(buffer.len(), |position| position + 1);
        let payload_bytes = newline.map_or(consumed, |position| position);
        if frame.len() + payload_bytes > MAX_RPC_FRAME_BYTES {
            let _ = sender.send(Err(format!(
                "Codex App Server returned a frame larger than {} MiB. The run was not loaded.",
                MAX_RPC_FRAME_BYTES / 1024 / 1024
            )));
            return;
        }
        frame.extend_from_slice(&buffer[..payload_bytes]);
        reader.consume(consumed);

        if newline.is_none() {
            continue;
        }
        let value = match serde_json::from_slice::<Value>(&frame) {
            Ok(value) => value,
            Err(_) => {
                let _ = sender.send(Err(
                    "Codex App Server returned an invalid JSONL frame.".to_string()
                ));
                return;
            }
        };
        if sender.send(Ok(value)).is_err() {
            return;
        }
        frame.clear();
    }
}

fn write_rpc(stdin: &mut impl Write, value: &Value) -> Result<(), String> {
    serde_json::to_writer(&mut *stdin, value)
        .map_err(|error| format!("Unable to encode Codex App Server request: {error}"))?;
    stdin
        .write_all(b"\n")
        .map_err(|error| format!("Unable to send Codex App Server request: {error}"))
}

fn codex_binary() -> Option<PathBuf> {
    if let Some(path) = env::var_os("HARNESS_LENS_CODEX_BIN").map(PathBuf::from) {
        if path.is_file() {
            return Some(path);
        }
    }

    if let Some(path) = env::var_os("PATH").and_then(|paths| {
        env::split_paths(&paths)
            .map(|directory| directory.join("codex"))
            .find(|candidate| candidate.is_file())
    }) {
        return Some(path);
    }

    let mut candidates = vec![
        PathBuf::from("/Applications/ChatGPT.app/Contents/Resources/codex"),
        PathBuf::from("/Applications/Codex.app/Contents/Resources/codex"),
        PathBuf::from("/opt/homebrew/bin/codex"),
        PathBuf::from("/usr/local/bin/codex"),
    ];
    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join(".local/bin/codex"));
        candidates.push(home.join(".cargo/bin/codex"));
    }
    candidates.into_iter().find(|candidate| candidate.is_file())
}

fn codex_version(binary: &Path) -> Option<String> {
    Command::new(binary)
        .arg("--version")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|version| version.trim().to_string())
}

fn parse_skills(result: Option<&Value>) -> Vec<CodexRuntimeSkill> {
    let mut skills = result
        .and_then(|value| value.get("data"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .flat_map(|entry| {
            entry
                .get("skills")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
        })
        .filter_map(|skill| {
            Some(CodexRuntimeSkill {
                name: skill.get("name")?.as_str()?.to_string(),
                source_name: safe_file_name(skill.get("path")?.as_str()?),
                scope: skill
                    .get("scope")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string(),
                enabled: skill
                    .get("enabled")
                    .and_then(Value::as_bool)
                    .unwrap_or(true),
            })
        })
        .collect::<Vec<_>>();
    skills.sort_by(|left, right| left.name.cmp(&right.name));
    skills
}

fn parse_hooks(result: Option<&Value>) -> Vec<CodexRuntimeHook> {
    let mut hooks = result
        .and_then(|value| value.get("data"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .flat_map(|entry| {
            entry
                .get("hooks")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
        })
        .filter_map(|hook| {
            Some(CodexRuntimeHook {
                event_name: hook.get("eventName")?.as_str()?.to_string(),
                source_name: safe_file_name(hook.get("sourcePath")?.as_str()?),
                enabled: hook.get("enabled").and_then(Value::as_bool).unwrap_or(true),
                trust_status: hook
                    .get("trustStatus")
                    .and_then(Value::as_str)
                    .map(str::to_string),
            })
        })
        .collect::<Vec<_>>();
    hooks.sort_by(|left, right| left.event_name.cmp(&right.event_name));
    hooks
}

fn parse_runs(result: Option<&Value>) -> Vec<CodexRunSummary> {
    result
        .and_then(|value| value.get("data"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|thread| {
            let id = thread.get("id")?.as_str()?.to_string();
            let preview = safe_preview(
                thread
                    .get("preview")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
            );
            let title = thread
                .get("name")
                .and_then(Value::as_str)
                .filter(|name| !name.trim().is_empty())
                .map(safe_preview)
                .unwrap_or_else(|| {
                    preview
                        .lines()
                        .find(|line| !line.trim().is_empty())
                        .map(safe_preview)
                        .filter(|line| !line.is_empty())
                        .unwrap_or_else(|| "Untitled Codex run".to_string())
                });
            Some(CodexRunSummary {
                id,
                title,
                preview,
                status: thread_status(thread.get("status")),
                source: thread_source(thread.get("source")),
                created_at: timestamp(thread.get("createdAt")),
                updated_at: timestamp(thread.get("updatedAt")),
                parent_thread_id: thread
                    .get("parentThreadId")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                git_branch: thread
                    .pointer("/gitInfo/branch")
                    .and_then(Value::as_str)
                    .map(safe_preview),
            })
        })
        .collect()
}

fn parse_run_detail(result: &Value) -> Result<CodexRunDetail, String> {
    let thread = result
        .get("thread")
        .ok_or_else(|| "Codex App Server returned an invalid run detail.".to_string())?;
    let id = thread
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| "Codex run detail did not include an id.".to_string())?
        .to_string();
    let preview = safe_preview(
        thread
            .get("preview")
            .and_then(Value::as_str)
            .unwrap_or_default(),
    );
    let title = thread
        .get("name")
        .and_then(Value::as_str)
        .map(safe_preview)
        .filter(|title| !title.is_empty())
        .unwrap_or_else(|| {
            preview
                .lines()
                .find(|line| !line.trim().is_empty())
                .map(safe_preview)
                .filter(|line| !line.is_empty())
                .unwrap_or_else(|| "Untitled Codex run".to_string())
        });

    let mut turns = Vec::new();
    let mut steps = Vec::new();
    let mut item_type_counts = BTreeMap::new();
    let mut completed_turns = 0;
    let mut failed_turns = 0;
    let mut truncated = false;

    for turn in thread
        .get("turns")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let turn_id = turn
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("unknown-turn")
            .to_string();
        let status = turn
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        match status.as_str() {
            "completed" => completed_turns += 1,
            "failed" | "interrupted" => failed_turns += 1,
            _ => {}
        }

        let turn_items = turn
            .get("items")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or_default();
        for (index, item) in turn_items.iter().enumerate() {
            let kind = item
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string();
            *item_type_counts.entry(kind.clone()).or_insert(0) += 1;
            if steps.len() < MAX_RUN_STEPS {
                steps.push(normalize_step(item, &turn_id, index, &kind));
            } else {
                truncated = true;
            }
        }

        turns.push(CodexTurnSummary {
            id: turn_id,
            status,
            started_at: timestamp(turn.get("startedAt")),
            completed_at: timestamp(turn.get("completedAt")),
            duration_ms: turn.get("durationMs").and_then(Value::as_u64),
            step_count: turn_items.len(),
        });
    }

    Ok(CodexRunDetail {
        id,
        title,
        status: thread_status(thread.get("status")),
        turns,
        steps,
        item_type_counts,
        completed_turns,
        failed_turns,
        truncated,
    })
}

fn normalize_step(item: &Value, turn_id: &str, index: usize, kind: &str) -> CodexRunStep {
    let label = match kind {
        "userMessage" => "User request",
        "agentMessage" => "Agent message",
        "reasoning" => "Reasoning",
        "commandExecution" => "Command execution",
        "fileChange" => "File changes",
        "mcpToolCall" => "MCP tool call",
        "dynamicToolCall" => "Dynamic tool call",
        "webSearch" => "Web search",
        "subAgentActivity" => "Subagent activity",
        "imageGeneration" => "Image generation",
        "enteredReviewMode" => "Review started",
        "exitedReviewMode" => "Review completed",
        _ => kind,
    }
    .to_string();

    let status = item
        .get("status")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| item.get("kind").and_then(Value::as_str).map(str::to_string));
    let detail = match kind {
        "fileChange" => item
            .get("changes")
            .and_then(Value::as_array)
            .map(|changes| format!("{} changed file(s)", changes.len())),
        "subAgentActivity" => item
            .get("agentPath")
            .and_then(Value::as_str)
            .map(Path::new)
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            .map(|name| format!("Agent: {name}")),
        "mcpToolCall" | "dynamicToolCall" => item
            .get("tool")
            .or_else(|| item.get("name"))
            .and_then(Value::as_str)
            .map(|tool| format!("Tool: {}", safe_preview(tool))),
        "commandExecution" => item
            .get("exitCode")
            .and_then(Value::as_i64)
            .map(|code| format!("Exit code: {code}")),
        _ => None,
    };

    CodexRunStep {
        id: item
            .get("id")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| format!("{turn_id}-{index}")),
        turn_id: turn_id.to_string(),
        kind: kind.to_string(),
        label,
        status,
        detail,
    }
}

fn safe_preview(value: &str) -> String {
    let redacted = redaction::redact(value);
    let collapsed = redacted.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut output = collapsed
        .chars()
        .take(MAX_PREVIEW_CHARS)
        .collect::<String>();
    if collapsed.chars().count() > MAX_PREVIEW_CHARS {
        output.push('…');
    }
    output
}

fn safe_file_name(value: &str) -> String {
    Path::new(value)
        .file_name()
        .and_then(|name| name.to_str())
        .map(safe_preview)
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "<unknown>".to_string())
}

fn thread_status(status: Option<&Value>) -> String {
    status
        .and_then(|value| value.get("type").or(Some(value)))
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string()
}

fn thread_source(source: Option<&Value>) -> String {
    match source {
        Some(Value::String(source)) => source.clone(),
        Some(Value::Object(source)) if source.contains_key("subAgent") => "subAgent".to_string(),
        Some(Value::Object(source)) => source
            .keys()
            .next()
            .cloned()
            .unwrap_or_else(|| "unknown".to_string()),
        _ => "unknown".to_string(),
    }
}

fn timestamp(value: Option<&Value>) -> Option<String> {
    let seconds = value.and_then(Value::as_i64)?;
    Utc.timestamp_opt(seconds, 0)
        .single()
        .map(|timestamp| timestamp.to_rfc3339())
}

#[cfg(test)]
mod tests {
    use std::{io::Cursor, sync::mpsc};

    use super::{
        parse_run_detail, parse_runs, read_bounded_json_lines, safe_file_name, safe_preview,
    };
    use serde_json::json;

    #[test]
    fn normalizes_runs_without_exposing_unbounded_preview_content() {
        let result = json!({
            "data": [{
                "id": "thread-1",
                "name": "Fix token=super-secret",
                "preview": "Authorization: Bearer abc.def.ghi\nContinue the task",
                "status": { "type": "notLoaded" },
                "source": { "subAgent": { "thread_spawn": {} } },
                "createdAt": 1_786_508_348_i64,
                "updatedAt": 1_786_508_420_i64
            }]
        });

        let runs = parse_runs(Some(&result));

        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].source, "subAgent");
        assert!(!runs[0].preview.contains("abc.def.ghi"));
        assert!(!runs[0].title.contains("super-secret"));
    }

    #[test]
    fn extracts_metadata_only_flight_recorder_steps() {
        let result = json!({
            "thread": {
                "id": "thread-1",
                "name": "A safe run",
                "status": { "type": "notLoaded" },
                "turns": [{
                    "id": "turn-1",
                    "status": "completed",
                    "startedAt": 1_786_508_348_i64,
                    "completedAt": 1_786_508_350_i64,
                    "durationMs": 2_000,
                    "items": [
                        { "id": "one", "type": "userMessage", "content": "do not return me" },
                        { "id": "two", "type": "mcpToolCall", "tool": "github.search", "arguments": { "token": "secret" }, "status": "completed" },
                        { "id": "three", "type": "fileChange", "changes": [{ "path": "/private/file" }] }
                    ]
                }]
            }
        });

        let detail = parse_run_detail(&result).expect("valid run detail");

        assert_eq!(detail.completed_turns, 1);
        assert_eq!(detail.steps.len(), 3);
        assert_eq!(
            detail.steps[1].detail.as_deref(),
            Some("Tool: github.search")
        );
        assert_eq!(detail.steps[2].detail.as_deref(), Some("1 changed file(s)"));
        let serialized = serde_json::to_string(&detail).expect("serializable detail");
        assert!(!serialized.contains("do not return me"));
        assert!(!serialized.contains("/private/file"));
        assert!(!serialized.contains("secret"));
    }

    #[test]
    fn collapses_and_limits_preview_text() {
        let preview = safe_preview(&format!("{} {}", "word ".repeat(300), "token=hidden"));
        assert!(preview.chars().count() <= super::MAX_PREVIEW_CHARS + 1);
        assert!(!preview.contains("hidden"));
    }

    #[test]
    fn marks_runs_truncated_after_the_bounded_step_limit() {
        let items = (0..1_001)
            .map(|index| json!({ "id": format!("item-{index}"), "type": "reasoning" }))
            .collect::<Vec<_>>();
        let result = json!({
            "thread": {
                "id": "thread-large",
                "status": { "type": "notLoaded" },
                "turns": [{ "id": "turn-1", "status": "completed", "items": items }]
            }
        });

        let detail = parse_run_detail(&result).expect("valid bounded run");

        assert_eq!(detail.steps.len(), 1_000);
        assert!(detail.truncated);
        assert_eq!(detail.item_type_counts.get("reasoning"), Some(&1_001));
    }

    #[test]
    fn rejects_invalid_jsonl_without_echoing_the_frame() {
        let (sender, receiver) = mpsc::channel();

        read_bounded_json_lines(Cursor::new(b"not-json\n"), sender);

        let error = receiver
            .recv()
            .expect("reader result")
            .expect_err("invalid frame must fail closed");
        assert_eq!(error, "Codex App Server returned an invalid JSONL frame.");
        assert!(!error.contains("not-json"));
    }

    #[test]
    fn runtime_declarations_only_expose_a_safe_file_name() {
        assert_eq!(
            safe_file_name("/Users/developer/.agents/skills/review/SKILL.md"),
            "SKILL.md"
        );
        assert_eq!(safe_file_name("/"), "<unknown>");
    }
}
