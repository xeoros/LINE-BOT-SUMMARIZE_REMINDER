# Data Flow Diagrams

## Table of Contents
1. [System Overview](#system-overview)
2. [Webhook Data Flow](#webhook-data-flow)
3. [Summary Generation Flow](#summary-generation-flow)
4. [Scheduled Summary Flow](#scheduled-summary-flow)
5. [Database Data Flow](#database-data-flow)
6. [Configuration Flow](#configuration-flow)
7. [Error Handling Flow](#error-handling-flow)

---

## System Overview

### High-Level Data Flow

```mermaid
graph TB
    LINE[LINE Platform] -->|Webhook Events| Webhook[Rust Web Server]
    Webhook -->|Store| DB[(PostgreSQL Database)]
    Webhook -->|Generate Summary| AI[AI APIs]
    AI -->|Claude/OpenAI/Gemini| External[External AI Services]
    Webhook -->|Send Summary| LINE
    Config[Configuration<br/>.env + TOML] -->|Load| Webhook
    Scheduler[Cron Scheduler] -->|Fetch| DB
    Scheduler -->|Generate| AI
    Scheduler -->|Push| LINE
```

### Component Interactions

| Component | Receives From | Sends To | Data Type |
|-----------|---------------|-----------|------------|
| LINE Platform | User | Webhook | Webhook events |
| Webhook | LINE, DB, AI | LINE, DB, AI | Messages, summaries |
| Database | Webhook, Scheduler | Webhook, Scheduler | Message data |
| AI Services | Webhook, Scheduler | Webhook, Scheduler | AI responses |
| Scheduler | Config | DB, AI, LINE | Schedule triggers |
| Configuration | - | Webhook, Scheduler | Environment vars |

---

## Webhook Data Flow

### Detailed Flow Diagram

```mermaid
graph TD
    A[LINE Event] --> B{Event Type}
    B -->|message| C[Parse Message]
    B -->|follow/join| D[Send Welcome]
    B -->|leave| E[Log Leave]
    B -->|other| F[Log & Ignore]

    C --> G{Is Summary Command?}
    G -->|Yes| H[Parse Parameters]
    G -->|No| I[Store Message]

    H --> J{Parameter Type}
    J -->|Count| K[Fetch N Messages]
    J -->|Time Range| L[Fetch Time Range]
    J -->|None| M[Fetch Default: 50]

    K --> N[Format Conversation]
    L --> N
    M --> N

    N --> O[Call AI API]
    O --> P[Receive Summary]
    P --> Q[Reply to LINE]

    I --> R[Get Sender Profile]
    R --> S[Update Display Name]
    S --> T[Insert into Messages Table]

    Q --> U[200 OK]
    D --> U
    E --> U
    F --> U
    T --> U
```

### Data Transformations

| Step | Input | Transformation | Output |
|-------|--------|----------------|---------|
| Webhook Verification | Body + Secret | HMAC-SHA256 → Base64 |
| Command Parsing | Raw text | Parsed command + parameters |
| Message Storage | Event data | Database record |
| Profile Lookup | User ID | Display name string |
| Conversation Format | Messages | Single string with timestamps |
| AI Call | Conversation string | Summary text |

---

## Summary Generation Flow

### Data Flow Diagram

```mermaid
graph LR
    A[Summary Command] --> B[Parser]
    B --> C[Parameter Extraction]
    C --> D{Parameter Type}

    D -->|Count| E[DB Query: LIMIT N]
    D -->|Time| F[DB Query: created_at >= NOW - range]
    D -->|None| G[DB Query: LIMIT 50]

    E --> H[Message Records]
    F --> H
    G --> H

    H --> I[Filter: Text Only]
    I --> J[Sort: Created ASC]
    J --> K[Format: [Name Time]: Text]

    K --> L[AI Service]
    L --> M{Provider Type}

    M -->|Claude| N[Claude API]
    M -->|OpenAI| O[OpenAI API]
    M -->|Gemini| P[Gemini API]

    N --> Q[Thai Summary]
    O --> Q
    P --> Q

    Q --> R[Reply to LINE]
```

### Data Models

**Message Record**:
```rust
struct Message {
    id: i32,
    message_id: String,
    source_type: SourceType,      // user/group/room
    source_id: String,             // user/group/room ID
    sender_id: Option<String>,
    display_name: Option<String>,
    message_type: MessageType,     // text/image/sticker
    message_text: Option<String>,
    created_at: DateTime<Utc>,
}
```

**Conversation Format**:
```
[Alice 10:02]: Hello everyone!
[Bob 10:05]: Hi Alice, how are you?
[Charlie 10:10]: Good morning!
```

---

## Scheduled Summary Flow

### Data Flow Diagram

```mermaid
graph TD
    A[Config File<br/>schedules.toml] --> B[Load Schedules]
    B --> C[Parse TOML]
    C --> D[Create Cron Jobs]

    D --> E[Cron Scheduler]
    E --> F{Schedule Triggered?}
    F -->|No| E
    F -->|Yes| G[Extract Config]

    G --> H{Message Filter Type}
    H -->|Count| I[DB: SELECT * ORDER BY created_at DESC LIMIT N]
    H -->|Time| J[DB: SELECT * WHERE created_at >= NOW - range]

    I --> K[Message Array]
    J --> K

    K --> L[Format Conversation]
    L --> M[Call AI API]
    M --> N[Receive Summary]

    N --> O[LINE API: Push Message]
    O --> P[Group/User Receives Summary]

    E --> Q{Next Schedule?}
    Q -->|Yes| E
    Q -->|No| E
```

### Schedule Configuration

**TOML Format**:
```toml
[[schedules]]
source_type = "group"
source_id = "C456def"
cron = "0 18 * * *"
message_count = 100
```

**Parsed Data**:
```rust
struct Schedule {
    source_type: String,      // "group", "user", "room"
    source_id: String,       // Target ID
    cron: String,           // Cron expression
    message_count: Option<i32>,
    time_range: Option<String>,
}
```

---

## Database Data Flow

### Entity Relationship Diagram

```mermaid
erDiagram
    MESSAGE {
        int id PK
        string message_id UK
        string source_type
        string source_id
        string sender_id FK
        string display_name
        string message_type
        text message_text
        timestamp created_at
    }

    MESSAGE ||--o{ SOURCE : "belongs to"
    SOURCE {
        string source_type PK
        string source_id PK
        string source_name
    }

    MESSAGE }|--|| SENDER : "sent by"
    SENDER {
        string sender_id PK
        string display_name
        string picture_url
    }
```

### Query Flows

**Store Message**:
```sql
INSERT INTO messages (
    message_id,
    source_type,
    source_id,
    sender_id,
    display_name,
    message_type,
    message_text
) VALUES ($1, $2, $3, $4, $5, $6, $7)
ON CONFLICT (message_id) DO NOTHING
```

**Fetch Recent Messages**:
```sql
SELECT *
FROM messages
WHERE source_type = $1 AND source_id = $2
ORDER BY created_at DESC
LIMIT $3
```

**Fetch by Time Range**:
```sql
SELECT *
FROM messages
WHERE source_type = $1
  AND source_id = $2
  AND created_at >= NOW() - INTERVAL '1 minute' * $3
ORDER BY created_at ASC
```

---

## Configuration Flow

### Environment Variables Flow

```mermaid
graph TB
    A[.env File] --> B[dotenv Library]
    B --> C[Config::from_env]
    C --> D{Load Variables}

    D -->|DATABASE_URL| E[Create PgPool]
    D -->|LINE_*| F[Create LineClient]
    D -->|AI_PROVIDER| G{Select Provider}
    D -->|CLAUDE_*| H[Claude Config]
    D -->|OPENAI_*| I[OpenAI Config]
    D -->|GEMINI_*| J[Gemini Config]
    D -->|PORT| K[Bind Port]
    D -->|SCHEDULES_*| L[Load Schedules]

    G -->|claude| H
    G -->|openai| I
    G -->|gemini| J

    H --> M[Create ClaudeService]
    I --> N[Create OpenAIService]
    J --> O[Create GeminiService]

    M --> P[AIService Trait]
    N --> P
    O --> P

    E --> Q[AppState]
    F --> Q
    P --> Q
    K --> Q
    L --> R[ScheduledSummaries]
```

### Configuration Sources

| Source | Variables | Used By |
|---------|------------|----------|
| Environment | DATABASE_URL, PORT | Web server |
| Environment | LINE_CHANNEL_ACCESS_TOKEN, LINE_CHANNEL_SECRET | LINE client |
| Environment | AI_PROVIDER, CLAUDE_*, OPENAI_*, GEMINI_* | AI services |
| TOML File | Schedules configuration | Scheduler |

---

## Error Handling Flow

### Error Propagation Diagram

```mermaid
graph TD
    A[Incoming Request] --> B{Validate}
    B -->|Invalid| C[Return Error Response]
    B -->|Valid| D[Process Request]

    D --> E{Database Operation}
    E -->|Success| F[Continue]
    E -->|Error| G[Log Error]
    G --> H{Retry?}
    H -->|Yes| I[Retry Operation]
    H -->|No| J[Propagate Error]
    I --> E

    J --> K{Critical?}
    K -->|Yes| L[Return 500]
    K -->|No| M[Graceful Degradation]

    F --> N{AI Operation}
    N -->|Success| O[Return Result]
    N -->|Error| P[Log Error]
    P --> Q{Retry?}
    Q -->|Yes| R[Retry AI Call]
    Q -->|No| S[Use Fallback/Empty]
    S --> T[Return Partial Result]

    L --> U[Send Response]
    M --> U
    O --> U
    T --> U
    C --> U
```

### Error Categories

| Error Type | Example | Handling Strategy |
|-------------|---------|------------------|
| Input Validation | Invalid command | Return error message |
| Database | Connection lost | Retry, log, return error |
| LINE API | Rate limit | Exponential backoff |
| AI API | Timeout | Retry once, return timeout message |
| Configuration | Missing env var | Crash on startup |

---

## AI Service Data Flow

### Provider Abstraction Flow

```mermaid
graph LR
    A[Summary Request] --> B[AIService Trait]
    B --> C{Provider Instance}

    C -->|Claude| D[ClaudeService]
    C -->|OpenAI| E[OpenAIService]
    C -->|Gemini| F[GeminiService]

    D --> G[Generate Request]
    E --> H[Generate Request]
    F --> I[Generate Request]

    G --> J[Serialize JSON]
    H --> J
    I --> J

    J --> K{External API}
    K -->|Claude| L[api.anthropic.com]
    K -->|OpenAI| M[api.openai.com]
    K -->|Gemini| N[generativelanguage.googleapis.com]

    L --> O[Deserialize Response]
    M --> O
    N --> O

    O --> P[Extract Text]
    P --> Q[Return Summary]
```

### Request/Response Data Structures

**Claude Request**:
```json
{
  "model": "claude-sonnet-4-6",
  "max_tokens": 4096,
  "messages": [
    {
      "role": "user",
      "content": "prompt..."
    }
  ]
}
```

**Claude Response**:
```json
{
  "id": "msg_...",
  "content": [
    {
      "text": "summary..."
    }
  ]
}
```

---

## State Management Flow

### Application State

```mermaid
graph TB
    A[Application Start] --> B[Create AppState]
    B --> C[Initialize Components]
    C --> D[Arc<PgPool>]
    C --> E[Arc<LineClient>]
    C --> F[Arc<dyn AIService>]
    C --> G[Config]

    D --> H[Shared Across Threads]
    E --> H
    F --> H
    G --> H

    H --> I[Handle Requests]
    I --> J{Request Type}

    J -->|Webhook| K[Use All Components]
    J -->|Health| L[Return Status]
    J -->|Scheduler| M[Use All Components]

    K --> N[Read/Write DB]
    K --> O[Call AI API]
    K --> P[Send LINE Messages]
    M --> N
    M --> O
    M --> P

    N --> Q[Async Operations]
    O --> Q
    P --> Q
```

### State Sharing

| Component | Type | Access Pattern | Thread Safety |
|-----------|------|----------------|---------------|
| Database Pool | Arc<PgPool> | Shared, read/write | Thread-safe via Arc |
| LINE Client | Arc<LineClient> | Shared, read/write | Thread-safe via Arc |
| AI Service | Arc<dyn AIService> | Shared, read | Thread-safe via Arc |
| Config | Config | Clone per thread | No sharing needed |

---

## Notes

- All diagrams use Mermaid syntax
- Render in GitHub, VSCode with Mermaid extension
- Data flows show transformations at each step
- Error flows highlight failure paths
- Async operations indicated with parallel notation where applicable
