# Architecture Documentation

## Overview

The LINE Chat Summarizer Bot is a Rust-based application that collects messages from LINE chats, stores them in PostgreSQL, and generates summaries using AI services (Claude, OpenAI, or Gemini).

## System Architecture

```
┌─────────────┐
│  LINE App  │
└──────┬──────┘
       │ Webhook (HTTPS)
       ▼
┌─────────────────────────────────────────────────────────────┐
│                   Rust Web Server                        │
│  ┌───────────────┐  ┌───────────────────┐          │
│  │  Web Handler  │  │  AI Service      │          │
│  │  (Axum)      │──│  (Trait-based)   │          │
│  └───────┬───────┘  └────────┬──────────┘          │
│          │                   │                          │
│          ▼                   ▼                          │
│  ┌───────────────┐  ┌─────────────┐            │
│  │ LINE Client   │  │ Scheduler  │            │
│  └───────────────┘  └─────────────┘            │
└────────────────────────┬────────────────────────────────┘
                     │
         ┌───────────┼───────────┐
         │           │           │
         ▼           ▼           ▼
    ┌─────────┐ ┌──────┐ ┌──────────┐
    │PostgreSQL│ │Claude│ │ OpenAI   │
    │ Database │ │ API  │ │ API      │
    └─────────┘ └──────┘ └──────────┘
                           │
                           ▼
                      ┌──────────┐
                      │ Gemini   │
                      │ API      │
                      └──────────┘
```

## Component Breakdown

### 1. Web Server Layer (Axum)
- **File**: `src/main.rs`
- **Responsibility**: HTTP request handling, routing, webhook processing
- **Key Components**:
  - `/webhook` endpoint - Receives LINE webhook events
  - `/health` endpoint - Health check
  - State management - Shared database pool, LINE client, AI service

### 2. Configuration Layer
- **File**: `src/config.rs`
- **Responsibility**: Environment variable management
- **Key Settings**:
  - Database connection string
  - LINE API credentials
  - AI provider selection and API keys
  - Server port
  - Schedules config path

### 3. Database Layer (SQLx)
- **Files**: `src/db/mod.rs`, `src/db/pool.rs`, `src/db/models.rs`
- **Responsibility**: PostgreSQL operations and data models
- **Key Operations**:
  - `save_message()` - Store new messages
  - `get_recent_messages()` - Fetch by count
  - `get_messages_by_time_range()` - Fetch by time range

### 4. LINE API Integration
- **Files**: `src/line/mod.rs`, `src/line/webhook.rs`, `src/line/client.rs`
- **Responsibility**: LINE Messaging API communication
- **Key Functions**:
  - `verify_webhook_signature()` - Security validation
  - `reply_message()` - Reply to webhook event
  - `push_message()` - Send proactive messages
  - `get_user_profile()` - Fetch user information
  - `get_group_member_profile()` - Fetch group member info

### 5. AI Service Layer (Trait-based)
- **Files**: `src/ai/mod.rs`, `src/ai/claude.rs`, `src/ai/openai.rs`, `src/ai/gemini.rs`
- **Responsibility**: AI API abstraction and integration
- **Architecture**: Strategy pattern with `AIService` trait
- **Providers**:
  - Claude API (Anthropic)
  - OpenAI API (GPT models)
  - Gemini API (Google)

### 6. Scheduler Layer
- **File**: `src/scheduler/mod.rs`
- **Responsibility**: Cron-based scheduled summaries
- **Key Features**:
  - TOML-based schedule configuration
  - Automatic summary generation and push
  - Support for multiple schedules

### 7. Command Handler Layer
- **File**: `src/handlers/summary.rs`
- **Responsibility**: Parse and execute summary commands
- **Supported Commands**:
  - `!summarize` - Default summary
  - `/สรุป` - Thai summary
  - `!summarize N` - N messages
  - `!summarize Xm` - X minutes
  - `!summarize Xh` - X hours
  - `!summarize Xd` - X days

## Database Schema

### Messages Table

| Column | Type | Description |
|---------|--------|-------------|
| id | SERIAL | Primary key |
| message_id | VARCHAR(100) | LINE message ID (unique) |
| source_type | VARCHAR(20) | 'user', 'group', or 'room' |
| source_id | VARCHAR(100) | User/group/room ID |
| sender_id | VARCHAR(100) | Sender's LINE user ID |
| display_name | TEXT | Sender's display name |
| message_type | VARCHAR(20) | 'text', 'image', 'sticker', etc. |
| message_text | TEXT | Message content (text only) |
| created_at | TIMESTAMP | Message timestamp |

### Indexes
- `idx_messages_source` - For querying by source (composite)
- `idx_messages_created_at` - For time-based queries
- `idx_messages_message_id` - For duplicate prevention

## Security Considerations

1. **Webhook Signature Verification**
   - HMAC-SHA256 validation
   - Prevents unauthorized requests
   - Uses LINE channel secret

2. **API Key Protection**
   - Environment variables only
   - Never logged or exposed
   - Per-provider validation

3. **SQL Injection Prevention**
   - Parameterized queries via SQLx
   - Compile-time query validation
   - Type-safe database operations

## Scalability Considerations

### Horizontal Scaling
- Stateless web server design
- Database connection pooling
- AI service abstraction for easy provider switching

### Vertical Scaling
- Configurable connection pool size
- Tunable max_tokens for AI requests
- Async processing for concurrent requests

### Performance Optimization
- Indexed database queries
- Batch message retrieval
- Connection reuse (HTTP/2)
- Async I/O throughout

## Error Handling Strategy

### Recovery Model
- Webhook errors logged but don't block
- API retries not implemented (future enhancement)
- Graceful degradation for optional features

### Logging Levels
- `ERROR` - Critical failures
- `WARN` - Non-critical issues
- `INFO` - Normal operations
- `DEBUG` - Detailed execution flow

## Deployment Architecture

### Development (ngrok)
```
LINE Platform ←→ ngrok ←→ Local Server ←→ PostgreSQL (Docker)
```

### Production
```
LINE Platform ←→ HTTPS Server ←→ PostgreSQL (Managed/Cloud)
                     ↓
                 AI Services (External APIs)
```

## Technology Stack

| Layer | Technology |
|--------|-----------|
| Language | Rust 1.70+ |
| Web Framework | Axum 0.7 |
| Async Runtime | Tokio |
| Database | PostgreSQL via SQLx |
| AI APIs | Claude, OpenAI, Gemini |
| Serialization | serde, serde_json |
| HTTP Client | reqwest |
| Config | dotenv |
| Logging | tracing |
| Scheduling | tokio-cron-scheduler |
| TOML | toml (schedules) |
