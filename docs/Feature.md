# Features Documentation

## Table of Contents
1. [Message Collection](#message-collection)
2. [On-Demand Summaries](#on-demand-summaries)
3. [Flexible Summary Commands](#flexible-summary-commands)
4. [Scheduled Summaries](#scheduled-summaries)
5. [Multi-AI Provider Support](#multi-ai-provider-support)
6. [Webhook Security](#webhook-security)
7. [Thai Language Support](#thai-language-support)
8. [Profile Lookup](#profile-lookup)

---

## Message Collection

### Description
Automatically captures and stores all messages from LINE conversations where the bot is added as a participant.

### How It Works
1. LINE sends webhook events to the bot's `/webhook` endpoint
2. Bot verifies the webhook signature using HMAC-SHA256
3. Parses event data (message type, sender, content)
4. Fetches sender's display name from LINE API
5. Stores message in PostgreSQL database

### Supported Message Types
| Type | Stored | Included in Summary |
|-------|---------|---------------------|
| Text | ✅ Full content | ✅ Yes |
| Image | ✅ Metadata only | ❌ No |
| Video | ✅ Metadata only | ❌ No |
| Audio | ✅ Metadata only | ❌ No |
| Sticker | ✅ Metadata only | ❌ No |
| File | ✅ Metadata only | ❌ No |
| Location | ✅ Metadata only | ❌ No |

### Database Fields
- **message_id**: Unique LINE message ID
- **source_type**: `user`, `group`, or `room`
- **source_id**: User ID, group ID, or room ID
- **sender_id**: Sender's LINE user ID
- **display_name**: Sender's display name (fetched via API)
- **message_type**: Type of message
- **message_text**: Text content (if applicable)
- **created_at**: Timestamp of message

### Implementation
- **File**: `src/db/models.rs`
- **Function**: `Message::save()`
- **Error Handling**: Duplicates ignored (ON CONFLICT DO NOTHING)

---

## On-Demand Summaries

### Description
Generate conversation summaries instantly by typing commands in the chat.

### Commands
| Command | Language | Default Behavior |
|----------|------------|------------------|
| `!summarize` | English | Last 50 messages |
| `/สรุป` | Thai | Last 50 messages |

### Summary Content
Each summary includes:
- **Key Decisions**: Important agreements made
- **Action Items**: Tasks assigned to participants
- **Speaker Attribution**: Who said what
- **Context**: Relevant background information

### Output Format (Thai)
```text
สรุปการสนทนา:

การตัดสินใจสำคัญ:
- ตกลงเวลาประชุมวันพรุ่งนี้เวลา 9:00
- Alice จะเตรียมเอกสารงานประชุม

รายการที่ต้องดำเนินการ:
1. Bob - ส่งรายงานก่อนวันพรุ่งนี้
2. Charlie - จัดที่ประชุม
3. Alice - เตรียมเอกสาร

การสนทนาหลัก:
- [Alice 10:02]: ขอประชุมวันพรุ่งนี้ตี 9 นะ
- [Bob 10:05]: โอเค ผมว่างครับ
...
```

### Implementation
- **File**: `src/handlers/summary.rs`
- **Trait**: `AIService::generate_summary()`
- **Flow**:
  1. Parse command text
  2. Fetch messages from database
  3. Format as conversation string
  4. Send to AI API
  5. Reply with summary

---

## Flexible Summary Commands

### Description
Customize summary scope with count or time-based parameters.

### Syntax

#### Message Count
```
!summarize <number>
```

Examples:
- `!summarize 100` - Last 100 messages
- `/สรุป 50` - Last 50 messages

#### Time Range
```
!summarize <number><unit>
```

Units:
- `m` - minutes
- `h` - hours
- `d` - days

Examples:
- `!summarize 30m` - Last 30 minutes
- `!summarize 2h` - Last 2 hours
- `/สรุป 1d` - Last 24 hours
- `!summarize 7d` - Last week

### Parsing Logic
1. Extract parameter after command
2. Check for unit suffix (m, h, d)
3. If numeric only → message count
4. If numeric + unit → time range in minutes
5. Convert to appropriate query:
   - Count: `get_recent_messages(limit)`
   - Time: `get_messages_by_time_range(minutes)`

### Default Values
- No parameter = 50 messages
- Invalid parameter = Error message

### Implementation
- **File**: `src/handlers/summary.rs`
- **Function**: `parse_parameter()`
- **Time Format**: `10m`, `1h`, `1d`

---

## Scheduled Summaries

### Description
Automatically generate and send summaries at scheduled times using cron syntax.

### Configuration File
**Location**: `config/schedules.toml`

**Format**:
```toml
[[schedules]]
source_type = "group"
source_id = "C456def"
cron = "0 18 * * *"      # Daily at 6 PM
message_count = 100

[[schedules]]
source_type = "group"
source_id = "C789xyz"
cron = "0 9 * * 1-5"     # Weekdays at 9 AM
time_range = "24h"          # Last 24 hours
```

### Cron Syntax
```
minute hour day month day_of_week
```

Examples:
| Cron | Description |
|-------|-------------|
| `0 18 * * *` | Daily at 6:00 PM |
| `0 9 * * 1-5` | Weekdays at 9:00 AM |
| `*/30 * * * *` | Every 30 minutes |
| `0 */2 * * *` | Every 2 hours |
| `0 12 * * 1` | Every Monday at noon |

### Configuration Options
| Field | Type | Required | Description |
|-------|--------|----------|-------------|
| `source_type` | string | ✅ | `user`, `group`, or `room` |
| `source_id` | string | ✅ | User ID or group ID |
| `cron` | string | ✅ | Cron expression |
| `message_count` | integer | ❌ | Number of messages |
| `time_range` | string | ❌ | Time range (e.g., `24h`, `2d`) |

### Behavior
1. Reads schedule config at startup
2. Creates cron jobs for each schedule
3. On trigger:
   - Fetches messages from database
   - Generates summary via AI
   - Sends push message to source
4. Continues running until shutdown

### Finding Source IDs
1. Add bot to group/chat
2. Send a message
3. Query database:
```bash
psql -h localhost -U postgres -d line_bot_db \
  -c "SELECT DISTINCT source_id, source_type FROM messages;"
```

### Implementation
- **File**: `src/scheduler/mod.rs`
- **Library**: `tokio-cron-scheduler`
- **Function**: `load_schedules()`, `add_schedule()`

---

## Multi-AI Provider Support

### Description
Switch between AI providers (Claude, OpenAI, Gemini) via environment configuration.

### Supported Providers

#### Claude (Anthropic)
```env
AI_PROVIDER=claude
CLAUDE_API_KEY=sk-ant-...
CLAUDE_MODEL=claude-sonnet-4-6
```

**Models**:
- `claude-sonnet-4-6` (recommended, default)
- `claude-opus-4-6` (most capable)
- `claude-haiku-4-5` (fastest)

#### OpenAI
```env
AI_PROVIDER=openai
OPENAI_API_KEY=sk-...
OPENAI_MODEL=gpt-4o
```

**Models**:
- `gpt-4o` (recommended, default)
- `gpt-4o-mini` (faster, cheaper)
- `gpt-4-turbo` (legacy)
- `gpt-3.5-turbo` (cost-effective)

#### Gemini (Google)
```env
AI_PROVIDER=gemini
GEMINI_API_KEY=...
GEMINI_MODEL=gemini-2.0-flash
```

**Models**:
- `gemini-2.0-flash` (recommended, default)
- `gemini-2.0-flash-exp` (experimental)
- `gemini-2.5-pro-preview` (more capable)

### Architecture
- **Pattern**: Strategy Pattern
- **Trait**: `AIService` with `generate_summary()` method
- **Factory**: `create_ai_service()` instantiates based on provider

### Switching Providers
1. Stop the bot
2. Change `AI_PROVIDER` in `.env`
3. Add/update API key for new provider
4. Restart bot

### Implementation
- **File**: `src/ai/mod.rs`, `src/ai/claude.rs`, `src/ai/openai.rs`, `src/ai/gemini.rs`
- **Trait**: `AIService`
- **Factory**: `create_ai_service()`

---

## Webhook Security

### Description
Verify all incoming LINE webhook requests to prevent unauthorized access.

### How It Works
1. LINE sends request with `X-Line-Signature` header
2. Bot reads raw request body
3. Computes HMAC-SHA256 using channel secret
4. Base64-encodes the hash
5. Compares with signature header
6. Rejects if mismatch

### Algorithm
```
signature = base64(hmac_sha256(channel_secret, request_body))
```

### Security Benefits
- **Authentication**: Only LINE can send valid webhooks
- **Integrity**: Request body cannot be modified
- **Non-repudiation**: Proves request origin

### Failure Handling
| Scenario | Action |
|-----------|---------|
| Missing signature | Return 401 Unauthorized |
| Invalid signature | Return 401 Unauthorized |
| Valid signature | Process event |
| Verification error | Return 500 Internal Server Error |

### Implementation
- **File**: `src/line/webhook.rs`
- **Function**: `verify_webhook_signature()`
- **Library**: `hmac`, `sha2`, `base64`

---

## Thai Language Support

### Description
Optimized for Thai language conversation summaries.

### Thai Commands
| English | Thai |
|---------|--------|
| `!summarize` | `/สรุป` |

### Thai Prompt Template
```text
คุณคือผู้ช่วยสรุปการสนทนา โปรดสรุปการสนทนาบน LINE ดังต่อไปนี้เป็นภาษาไทย
เน้นไปที่: การตัดสินใจสำคัญ รายการที่ต้องดำเนินการ (action items) และใครพูกอะไร

การสนทนา:
{conversation}

โปรดสรุปในรูปแบบที่อ่านง่ายและชัดเจน
```

### Thai Output Format
- Thai language response
- Thai bullet points and numbering
- Proper Thai date/time formatting
- Thai action item indicators

### Implementation
- **File**: `src/ai/prompt.rs`
- **Function**: `get_summary_prompt()`

---

## Profile Lookup

### Description
Fetches sender display names from LINE API for better attribution.

### When It Happens
1. New message received
2. Message stored in database
3. Bot fetches sender profile

### Profile Data
| Field | Source | Description |
|--------|---------|-------------|
| user_id | Sender ID | LINE user identifier |
| display_name | Profile API | Display name |
| picture_url | Profile API | Profile picture (not stored) |

### API Endpoints
- **User Profile**: `GET /v2/bot/profile/{userId}`
- **Group Member**: `GET /v2/bot/group/{groupId}/member/{userId}`

### Caching Strategy
- Currently: No caching (fetches on every message)
- Future: In-memory cache with TTL

### Error Handling
- Profile fetch failures don't block message storage
- Fallback to "Unknown" if profile unavailable
- Logs warnings for failed lookups

### Implementation
- **File**: `src/line/client.rs`
- **Methods**: `get_user_profile()`, `get_group_member_profile()`

---

## Feature Matrix

| Feature | Status | Priority | Complexity |
|----------|----------|------------|
| Message Collection | ✅ Implemented | Low |
| On-Demand Summaries | ✅ Implemented | Medium |
| Flexible Commands | ✅ Implemented | Medium |
| Scheduled Summaries | ✅ Implemented | High |
| Multi-AI Support | ✅ Implemented | Medium |
| Webhook Security | ✅ Implemented | Low |
| Thai Language | ✅ Implemented | Low |
| Profile Lookup | ✅ Implemented | Medium |

---

## Future Features

### Planned
- [ ] AI selection per command
- [ ] Custom summary templates
- [ ] Multi-language detection
- [ ] Rich media analysis
- [ ] Export to PDF/Markdown
- [ ] Admin dashboard
- [ ] Usage analytics

### Considered
- [ ] Reply threading
- [ ] Sentiment analysis
- [ ] Keyword alerts
- [ ] Integration with calendar apps
- [ ] Multi-tenant support
