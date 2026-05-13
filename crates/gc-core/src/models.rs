use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRegistry {
    pub pid: u32,
    #[serde(rename = "sessionId")]
    pub session_id: Uuid,
    pub cwd: String,
    #[serde(rename = "startedAt")]
    pub started_at: i64,
    #[serde(rename = "procStart")]
    pub proc_start: Option<String>,
    pub version: String,
    #[serde(rename = "peerProtocol")]
    pub peer_protocol: Option<u32>,
    pub kind: SessionKind,
    pub entrypoint: String,
    pub status: SessionStatus,
    #[serde(rename = "updatedAt")]
    pub updated_at: i64,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionKind {
    Interactive,
    Background,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    Idle,
    Busy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum SessionEntry {
    User(UserEntry),
    Assistant(AssistantEntry),
    Attachment(AttachmentEntry),
    System(SystemEntry),
    AgentName(AgentNameEntry),
    AiTitle(AiTitleEntry),
    CustomTitle(CustomTitleEntry),
    LastPrompt(LastPromptEntry),
    PermissionMode(PermissionModeEntry),
    PrLink(PrLinkEntry),
    QueueOperation(QueueOperationEntry),
    FileHistorySnapshot(FileHistorySnapshotEntry),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonFields {
    pub uuid: Option<Uuid>,
    #[serde(rename = "parentUuid")]
    pub parent_uuid: Option<Uuid>,
    #[serde(rename = "sessionId")]
    pub session_id: Option<Uuid>,
    pub timestamp: Option<String>,
    pub cwd: Option<String>,
    pub version: Option<String>,
    #[serde(rename = "gitBranch")]
    pub git_branch: Option<String>,
    pub entrypoint: Option<String>,
    #[serde(rename = "userType")]
    pub user_type: Option<String>,
    #[serde(rename = "isSidechain")]
    pub is_sidechain: Option<bool>,
    pub slug: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserEntry {
    #[serde(flatten)]
    pub common: CommonFields,
    pub message: Option<ApiMessage>,
    #[serde(rename = "isCompactSummary")]
    pub is_compact_summary: Option<bool>,
    #[serde(rename = "isMeta")]
    pub is_meta: Option<bool>,
    pub origin: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantEntry {
    #[serde(flatten)]
    pub common: CommonFields,
    pub message: ApiMessage,
    #[serde(rename = "requestId")]
    pub request_id: Option<String>,
    #[serde(rename = "isApiErrorMessage")]
    pub is_api_error_message: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachmentEntry {
    #[serde(flatten)]
    pub common: CommonFields,
    pub attachment: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemEntry {
    #[serde(flatten)]
    pub common: CommonFields,
    pub content: Option<String>,
    pub subtype: Option<String>,
    #[serde(rename = "stopReason")]
    pub stop_reason: Option<String>,
    #[serde(rename = "durationMs")]
    pub duration_ms: Option<f64>,
    #[serde(rename = "hookCount")]
    pub hook_count: Option<u32>,
    #[serde(rename = "hookErrors")]
    pub hook_errors: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentNameEntry {
    #[serde(rename = "sessionId")]
    pub session_id: Uuid,
    #[serde(rename = "agentName")]
    pub agent_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiTitleEntry {
    #[serde(rename = "sessionId")]
    pub session_id: Uuid,
    #[serde(rename = "aiTitle")]
    pub ai_title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomTitleEntry {
    #[serde(rename = "sessionId")]
    pub session_id: Uuid,
    #[serde(rename = "customTitle")]
    pub custom_title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LastPromptEntry {
    #[serde(rename = "sessionId")]
    pub session_id: Uuid,
    #[serde(rename = "lastPrompt")]
    pub last_prompt: Option<String>,
    #[serde(rename = "leafUuid")]
    pub leaf_uuid: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionModeEntry {
    #[serde(rename = "sessionId")]
    pub session_id: Uuid,
    #[serde(rename = "permissionMode")]
    pub permission_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrLinkEntry {
    #[serde(rename = "sessionId")]
    pub session_id: Uuid,
    pub timestamp: Option<String>,
    #[serde(rename = "prUrl")]
    pub pr_url: String,
    #[serde(rename = "prNumber")]
    pub pr_number: Option<u64>,
    #[serde(rename = "prRepository")]
    pub pr_repository: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueOperationEntry {
    #[serde(rename = "sessionId")]
    pub session_id: Uuid,
    pub timestamp: Option<String>,
    pub operation: String,
    pub content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileHistorySnapshotEntry {
    #[serde(rename = "messageId")]
    pub message_id: Option<String>,
    #[serde(rename = "isSnapshotUpdate")]
    pub is_snapshot_update: Option<bool>,
    pub snapshot: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiMessage {
    pub role: String,
    pub content: serde_json::Value,
    pub model: Option<String>,
    pub usage: Option<TokenUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub cache_creation_input_tokens: Option<u64>,
    pub cache_read_input_tokens: Option<u64>,
    pub service_tier: Option<String>,
    pub speed: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SessionSummary {
    pub session_id: Uuid,
    pub project_path: String,
    pub display_name: String,
    pub custom_title: Option<String>,
    pub ai_title: Option<String>,
    pub agent_name: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub version: Option<String>,
    pub git_branch: Option<String>,
    pub kind: Option<SessionKind>,
    pub status: Option<SessionStatus>,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cache_read_tokens: u64,
    pub total_cache_creation_tokens: u64,
    pub message_count: u64,
}

impl SessionSummary {
    pub fn title(&self) -> &str {
        self.custom_title
            .as_deref()
            .or(self.ai_title.as_deref())
            .or(self.agent_name.as_deref())
            .unwrap_or("untitled")
    }

    pub fn total_tokens(&self) -> u64 {
        self.total_input_tokens + self.total_output_tokens
    }
}
