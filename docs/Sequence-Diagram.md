# Sequence Diagrams

## Table of Contents
1. [Message Flow: User Sends Message](#message-flow-user-sends-message)
2. [Message Flow: Summary Request](#message-flow-summary-request)
3. [Message Flow: Scheduled Summary](#message-flow-scheduled-summary)
4. [Authentication Flow: Webhook Verification](#authentication-flow-webhook-verification)
5. [Profile Lookup Flow](#profile-lookup-flow)

---

## Message Flow: User Sends Message

### Diagram

```mermaid
sequenceDiagram
    participant User as 👤
    participant LINE as 📱
    participant Webhook as 🌐
    participant DB as 💾
    participant LINE_API as 🔌
    participant AI as 🤖

    User->>LINE: Send "Hello, everyone!"
    LINE->>Webhook: POST /webhook (with signature)
    Webhook->>Webhook: Verify signature
    Webhook->>DB: Save message
    DB-->>Webhook: Message ID
    Webhook->>LINE_API: Get user profile
    LINE_API-->>Webhook: Display name
    Webhook->>DB: Update display name
    Webhook-->>LINE: 200 OK
    LINE-->>User: Message delivered
```

### Description

1. User sends a message in LINE chat
2. LINE Platform sends webhook event to bot
3. Bot verifies webhook signature
4. Bot stores message in PostgreSQL
5. Bot fetches sender's display name from LINE API
6. Bot updates message with display name
7. Bot responds with 200 OK

### Notes

- Message is stored even if profile fetch fails
- Display name lookup is asynchronous
- Webhook signature must match channel secret

---

## Message Flow: Summary Request

### Diagram

```mermaid
sequenceDiagram
    participant User as 👤
    participant LINE as 📱
    participant Webhook as 🌐
    participant DB as 💾
    participant AI as 🤖
    participant LINE_API as 🔌

    User->>LINE: Send "!summarize"
    LINE->>Webhook: POST /webhook (summary event)
    Webhook->>Webhook: Parse command
    Webhook->>Webhook: Parse parameters (none)
    Webhook->>DB: Get recent messages (default: 50)
    DB-->>Webhook: 50 messages
    Webhook->>Webhook: Format conversation
    Webhook->>AI: Generate summary
    AI-->>Webhook: Summary text
    Webhook->>LINE_API: Reply message
    LINE_API-->>Webhook: Success
    Webhook-->>LINE: 200 OK
    LINE-->>User: Display summary
```

### Description

1. User sends `!summarize` command
2. LINE sends webhook with message event
3. Bot detects summary command
4. Bot fetches 50 recent messages from database
5. Bot formats messages as conversation string
6. Bot sends conversation to AI API
7. AI returns summary in Thai
8. Bot replies to user with summary

### Variations

**With Message Count** (`!summarize 100`)
```
Webhook->>DB: Get 100 messages
```

**With Time Range** (`!summarize 2h`)
```
Webhook->>DB: Get messages from last 2 hours
```

**Thai Command** (`/สรุป`)
```
User->>LINE: Send "/สรุป"
// Rest of flow identical
```

---

## Message Flow: Scheduled Summary

### Diagram

```mermaid
sequenceDiagram
    participant Scheduler as ⏰
    participant Cron as 🔄
    participant DB as 💾
    participant AI as 🤖
    participant LINE_API as 🔌
    participant User as 👤

    Cron->>Scheduler: Trigger schedule
    Scheduler->>DB: Get messages (scheduled count/range)
    DB-->>Scheduler: N messages
    Scheduler->>Scheduler: Format conversation
    Scheduler->>AI: Generate summary
    AI-->>Scheduler: Summary text
    Scheduler->>LINE_API: Push message to group
    LINE_API-->>Scheduler: Success
    LINE-->>User: Summary notification
```

### Description

1. Cron scheduler triggers at scheduled time
2. Scheduler reads schedule configuration
3. Fetches messages based on configuration
4. Formats as conversation string
5. Generates summary via AI
6. Pushes summary to group/user via LINE API
7. User receives summary notification

### Notes

- Push messages are not replies to a webhook
- Push messages count toward LINE API quota
- Failed schedules should be logged but not crash

---

## Authentication Flow: Webhook Verification

### Diagram

```mermaid
sequenceDiagram
    participant LINE as 📱
    participant Webhook as 🌐
    participant HMAC as 🔐
    participant Secret as 🔑

    LINE->>Webhook: POST /webhook
    Note over LINE,Webhook: Headers: X-Line-Signature
    Webhook->>Secret: Get channel secret
    Secret-->>Webhook: Secret value
    Webhook->>HMAC: Compute SHA256(body + secret)
    HMAC-->>Webhook: Hash result
    Webhook->>Webhook: Base64 encode hash
    Webhook->>Webhook: Compare with header signature
    alt Signature matches
        Webhook->>Webhook: Process event
        Webhook-->>LINE: 200 OK
    else Signature invalid
        Webhook-->>LINE: 401 Unauthorized
    end
```

### Algorithm

```
signature = base64(hmac_sha256(channel_secret, request_body))
```

### Security Benefits

- **Authentication**: Only LINE can send valid requests
- **Integrity**: Body cannot be modified in transit
- **Non-repudiation**: Proves request came from LINE

---

## Profile Lookup Flow

### Diagram

```mermaid
sequenceDiagram
    participant Webhook as 🌐
    participant LINE_API as 🔌
    participant DB as 💾

    Webhook->>LINE_API: GET /v2/bot/profile/{userId}
    Note over Webhook,LINE_API: Headers: Authorization: Bearer {token}
    LINE_API-->>Webhook: {userId, displayName, pictureUrl}
    Webhook->>DB: Update message display_name
    DB-->>Webhook: Update success
```

### Group Member Profile

```mermaid
sequenceDiagram
    participant Webhook as 🌐
    participant LINE_API as 🔌

    Webhook->>LINE_API: GET /v2/bot/group/{groupId}/member/{userId}
    Note over Webhook,LINE_API: Headers: Authorization: Bearer {token}
    LINE_API-->>Webhook: {userId, displayName}
```

### Error Handling

| Scenario | Action |
|-----------|---------|
| Profile fetch succeeds | Update message with display name |
| Profile fetch fails | Log warning, continue without display name |
| Bot not in group | Profile unavailable, skip update |
| Rate limit exceeded | Retry after delay, log warning |

---

## AI Provider Selection Flow

### Diagram

```mermaid
sequenceDiagram
    participant Config as ⚙️
    participant Factory as 🏭
    participant Claude as 🧠
    participant OpenAI as 🤖
    participant Gemini as ✨
    participant App as 🌐

    Config->>Factory: create_ai_service(provider, api_keys, models)
    alt provider == "claude"
        Factory->>Claude: ClaudeService::new(api_key, model)
        Claude-->>Factory: ClaudeService instance
    else provider == "openai"
        Factory->>OpenAI: OpenAIService::new(api_key, model)
        OpenAI-->>Factory: OpenAIService instance
    else provider == "gemini"
        Factory->>Gemini: GeminiService::new(api_key, model)
        Gemini-->>Factory: GeminiService instance
    end
    Factory-->>App: Box<dyn AIService>
```

---

## Startup Sequence

### Diagram

```mermaid
sequenceDiagram
    participant User as 👤
    participant Main as 🚀
    participant Config as ⚙️
    participant DB as 💾
    participant AI as 🤖
    participant Scheduler as ⏰
    participant Server as 🌐

    User->>Main: Start application
    Main->>Config: Load from environment
    Config-->>Main: Config struct
    Main->>DB: Create connection pool
    DB-->>Main: PgPool
    Main->>AI: Create service (provider, keys, models)
    AI-->>Main: AIService instance
    Main->>Scheduler: Initialize with DB, LINE client, AI
    Scheduler->>Scheduler: Load schedules from config file
    Scheduler->>Scheduler: Register cron jobs
    Main->>Server: Start on port 3000
    Server-->>User: Ready for requests
    Note over Server: Listening for webhooks...
```

---

## Shutdown Sequence

### Diagram

```mermaid
sequenceDiagram
    participant User as 👤
    participant Server as 🌐
    participant Scheduler as ⏰
    participant DB as 💾

    User->>Server: SIGTERM/SIGINT
    Server->>Server: Stop accepting new connections
    Server->>Scheduler: Shutdown scheduler
    Scheduler->>Scheduler: Cancel all jobs
    Scheduler-->>Server: All jobs stopped
    Server->>DB: Close connections
    DB-->>Server: Connections closed
    Server-->>User: Exit successfully
```

---

## Error Handling Sequence

### Diagram (Webhook Error)

```mermaid
sequenceDiagram
    participant LINE as 📱
    participant Webhook as 🌐
    participant Logger as 📝

    LINE->>Webhook: POST /webhook
    Webhook->>Webhook: Verify signature
    alt Invalid signature
        Webhook->>Logger: Log error
        Webhook-->>LINE: 401 Unauthorized
    else Valid signature
        Webhook->>Webhook: Parse body
        alt Parse error
            Webhook->>Logger: Log error
            Webhook-->>LINE: 400 Bad Request
        else Parse success
            Webhook->>Webhook: Process event
            alt Process error
                Webhook->>Logger: Log error
                Webhook-->>LINE: 200 OK (event failed silently)
            else Process success
                Webhook-->>LINE: 200 OK
            end
        end
    end
```

### Diagram (AI Error)

```mermaid
sequenceDiagram
    participant Command as 📋
    participant AI as 🤖
    participant Logger as 📝
    participant LINE as 📱

    Command->>AI: generate_summary(messages)
    alt AI API error
        AI->>Logger: Log error
        AI-->>Command: Err(API error)
        Command->>Logger: Log generation failure
        Command->>LINE: Reply with error message
    else AI timeout
        AI->>Logger: Log timeout
        AI-->>Command: Err(Timeout)
        Command->>LINE: Reply with timeout message
    else Success
        AI-->>Command: Ok(summary text)
        Command->>LINE: Reply with summary
    end
```

---

## Notes

- All sequence diagrams use Mermaid syntax
- Can be rendered in GitHub, GitLab, VSCode with Mermaid extension
- Timestamps and async operations shown where relevant
- Error paths included for robustness understanding
