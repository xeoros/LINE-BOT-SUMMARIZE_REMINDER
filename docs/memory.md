# Memory & Knowledge Base

## Project Context

**Project Name**: LINE Chat Summarizer Bot
**Technology**: Rust, Axum, PostgreSQL
**Status**: Development Complete
**Last Updated**: 2026-03-10

---

## Architecture Decisions

### Why Rust?
- **Performance**: Zero-cost abstractions, memory safety
- **Concurrency**: Tokio async runtime excels at I/O operations
- **Reliability**: Compiler catches many runtime errors
- **Web**: Axum provides excellent async HTTP performance

### Why PostgreSQL?
- **Type Safety**: SQLx compile-time query verification
- **Reliability**: ACID transactions for message storage
- **Indexing**: Efficient queries on large datasets
- **JSON Support**: Flexible data storage if needed

### Why Axum?
- **Modern**: Tower-based middleware ecosystem
- **Async-first**: Built for Tokio
- **Type-safe**: Extractors and responses
- **Simple**: Minimal boilerplate

### Why Trait-Based AI?
- **Flexibility**: Easy to add new AI providers
- **Testing**: Mock implementations for unit tests
- **Runtime**: Select provider via configuration
- **Maintainability**: Each provider isolated

---

## Key Constraints & Limitations

### LINE API Limitations

| Limitation | Impact | Workaround |
|------------|--------|------------|
| No historical messages | Can't summarize old chats | Bot must join before conversation starts |
| Webhook replay protection | Duplicate events | Store message_id, ignore duplicates |
| Group permissions | "Read all messages" required | Ask group admin for permission |
| Push quota (free tier) | 500/month | Use reply when possible, upgrade plan |

### AI API Limitations

| Provider | Limitation | Mitigation |
|----------|-------------|-------------|
| Claude | Rate limits | Exponential backoff |
| OpenAI | Rate limits | Queue requests, retry logic |
| Gemini | Rate limits | Implement caching |

### Technical Constraints

- **Single-threaded AI calls**: No parallel summarization (future enhancement)
- **No message caching**: Every summary fetches from DB (optimization opportunity)
- **No persistent profiles**: Fetched on every message (optimization opportunity)

---

## Important Implementation Details

### Webhook Signature Verification
- **Algorithm**: HMAC-SHA256
- **Secret**: LINE Channel Secret
- **Format**: Base64 encoded hash
- **Location**: `src/line/webhook.rs::verify_webhook_signature()`

### Message Storage Idempotency
- **Mechanism**: `ON CONFLICT (message_id) DO NOTHING`
- **Benefit**: Safe webhook replay
- **Location**: `sql/schema.sql` message_id unique constraint

### AI Model Defaults
| Provider | Default Model | Reason |
|----------|--------------|--------|
| Claude | claude-sonnet-4-6 | Best balance of speed/quality |
| OpenAI | gpt-4o | Multi-modal, fast |
| Gemini | gemini-2.0-flash | Fast, cost-effective |

### Thai Language Support
- **Prompt Template**: `src/ai/prompt.rs::get_summary_prompt()`
- **Key Phrases**: "การตัดสินใจสำคัญ", "รายการที่ต้องดำเนินการ"
- **Formatting**: Thai bullet points, proper numbering

### Time Range Parsing
- **Formats**: `10m`, `1h`, `1d`
- **Conversion**: All to minutes for database query
- **Location**: `src/handlers/summary.rs::parse_parameter()`

---

## Common Patterns

### Error Handling Pattern

```rust
// Function result
pub async fn do_something() -> Result<Output> {
    // ...
}

// Contextual errors
anyhow::bail!("Failed to do X: {}", reason)
anyhow::Context::context("Failed to do X")?
```

### Database Query Pattern

```rust
// Use sqlx macros for compile-time validation
sqlx::query!(
    "SELECT * FROM messages WHERE source_type = $1 AND source_id = $2"
)
.bind(source_type)
.bind(source_id)
.fetch_all(pool)
.await?
```

### Async/Await Pattern

```rust
// Use Arc for shared state across async tasks
let pool = Arc::new(create_pool()?);

// Clone Arc for tasks
let pool_clone = Arc::clone(&pool);

// Spawn async tasks
tokio::spawn(async move {
    // Use pool_clone here
});
```

### Configuration Pattern

```rust
// Load with dotenv
dotenv::dotenv().ok();

// Use unwrap_or_else for defaults
let port = std::env::var("PORT")
    .unwrap_or_else(|_| "3000".to_string())
    .parse()?;
```

---

## Database Schema Notes

### Table: Messages
**Purpose**: Store all LINE messages received by bot

**Indexes**:
1. `idx_messages_source` - Composite on (source_type, source_id, created_at DESC)
   - Used for: Get recent messages per source
2. `idx_messages_created_at` - On created_at DESC
   - Used for: Time-based queries across sources
3. `idx_messages_message_id` - Unique on message_id
   - Used for: Duplicate prevention

**Query Patterns**:
```sql
-- Recent messages (use source index)
SELECT * FROM messages
WHERE source_type = $1 AND source_id = $2
ORDER BY created_at DESC
LIMIT $3;

-- Time range (use created_at index)
SELECT * FROM messages
WHERE source_type = $1
  AND source_id = $2
  AND created_at >= NOW() - INTERVAL '1 minute' * $3
ORDER BY created_at ASC;
```

---

## External API Endpoints

### LINE Messaging API

| Endpoint | Method | Purpose |
|----------|---------|---------|
| `/v2/bot/message/reply` | POST | Reply to webhook event |
| `/v2/bot/message/push` | POST | Send proactive message |
| `/v2/bot/profile/{userId}` | GET | Get user profile |
| `/v2/bot/group/{groupId}/member/{userId}` | GET | Get group member profile |

### Claude API

| Endpoint | Method | Purpose |
|----------|---------|---------|
| `/v1/messages` | POST | Generate summary |
| Model IDs | - | claude-sonnet-4-6, claude-opus-4-6, claude-haiku-4-5 |

### OpenAI API

| Endpoint | Method | Purpose |
|----------|---------|---------|
| `/v1/chat/completions` | POST | Generate summary |
| Model IDs | - | gpt-4o, gpt-4o-mini, gpt-4-turbo |

### Gemini API

| Endpoint | Method | Purpose |
|----------|---------|---------|
| `/v1beta/models/{model}:generateContent` | POST | Generate summary |
| Model IDs | - | gemini-2.0-flash, gemini-2.5-pro-preview |

---

## Environment Variables Reference

### Required

| Variable | Format | Description |
|-----------|---------|-------------|
| `DATABASE_URL` | `postgresql://...` | PostgreSQL connection string |
| `LINE_CHANNEL_ACCESS_TOKEN` | `{token}` | LINE API access token |
| `LINE_CHANNEL_SECRET` | `{secret}` | LINE webhook signature secret |
| `AI_PROVIDER` | `claude/openai/gemini` | Selected AI service |

### Provider-Specific Required

| Variable | When Required | Format |
|-----------|----------------|---------|
| `CLAUDE_API_KEY` | AI_PROVIDER=claude | `sk-ant-...` |
| `OPENAI_API_KEY` | AI_PROVIDER=openai | `sk-...` |
| `GEMINI_API_KEY` | AI_PROVIDER=gemini | `{key}` |

### Optional (with defaults)

| Variable | Default | Description |
|-----------|---------|-------------|
| `CLAUDE_MODEL` | `claude-sonnet-4-6` | Claude model ID |
| `OPENAI_MODEL` | `gpt-4o` | OpenAI model ID |
| `GEMINI_MODEL` | `gemini-2.0-flash` | Gemini model ID |
| `PORT` | `3000` | Server port |
| `SCHEDULES_CONFIG_PATH` | `config/schedules.toml` | Schedule config file |

---

## Debugging Tips

### Common Issues

**1. Webhook not receiving events**
```bash
# Check ngrok is running
curl https://your-ngrok-url.ngrok-free.app/health

# Verify webhook URL in LINE Console matches ngrok

# Check webhook signature verification
# Temporarily disable to test
```

**2. Database connection errors**
```bash
# Check PostgreSQL is running
docker ps | grep postgres

# Test connection manually
psql $DATABASE_URL

# Verify schema applied
psql $DATABASE_URL -f sql/schema.sql
```

**3. AI API errors**
```bash
# Test API key manually
curl -H "x-api-key: $CLAUDE_API_KEY" \
  https://api.anthropic.com/v1/messages

# Check quota and billing
# Visit provider console
```

**4. Scheduled summaries not firing**
```bash
# Verify config file exists
cat config/schedules.toml

# Check bot logs for schedule loading
cargo run | grep "schedule"

# Verify cron syntax
# Use online cron validator
```

### Logging Levels

```rust
// INFO - Normal operations
info!("Bot started");
info!("Message saved: {}", message_id);

// WARN - Non-critical issues
warn!("Failed to fetch profile for user {}", user_id);
warn!("AI API retry {} of {}", attempt, max_attempts);

// ERROR - Critical failures
error!("Webhook signature verification failed: {}", err);
error!("Database connection lost: {}", err);
```

---

## Performance Notes

### Query Performance

| Query | Expected Time | Optimization |
|--------|---------------|-------------|
| Save message | < 10ms | Index on message_id |
| Get recent (50) | < 50ms | Source index |
| Get by time range | < 100ms | Created_at index |

### Bottlenecks

1. **AI API calls**: Main latency source
   - Mitigation: Caching, parallel requests

2. **Profile fetching**: Serial LINE API calls
   - Mitigation: In-memory cache with TTL

3. **Database connection**: Pool exhaustion under load
   - Mitigation: Increase pool size, connection timeout

### Optimization Opportunities

- [ ] Batch message inserts
- [ ] Profile caching (LRU cache)
- [ ] Summary result caching
- [ ] Connection pooling for LINE API
- [ ] Async profile fetching

---

## Security Considerations

### Secrets Management
- **Never commit**: `.env` file, API keys
- **Use**: Environment variables only
- **Rotate**: API keys regularly
- **Limit**: Database user permissions

### Webhook Security
- **Always verify**: X-Line-Signature header
- **Use HTTPS**: Required for production
- **Reject invalid**: 401 Unauthorized
- **Log attempts**: Security audit trail

### SQL Injection Prevention
- **Use SQLx**: Compile-time query validation
- **Never concatenate**: SQL strings with user input
- **Parameterized**: All user data in bind parameters

---

## Deployment Checklist

### Pre-Deployment

- [ ] All tests passing
- [ ] Environment variables configured
- [ ] Database schema applied
- [ ] SSL certificate valid
- [ ] AI API quota sufficient
- [ ] Webhook URL configured in LINE Console
- [ ] Schedules configured (if using)
- [ ] Health check endpoint accessible
- [ ] Error monitoring configured
- [ ] Log aggregation configured

### Post-Deployment

- [ ] Webhook receiving events
- [ ] Messages storing in database
- [ ] Summary commands working
- [ ] Scheduled summaries firing
- [ ] Health check responding
- [ ] No errors in logs
- [ ] AI API calls successful

---

## Known Issues

| ID | Description | Status | Impact |
|----|-------------|--------|--------|
| None | N/A | N/A | N/A |

---

## Change Log

### 2026-03-10

#### Added
- Gemini AI provider support
- Configurable Claude model selection
- Configurable OpenAI model selection
- Configurable Gemini model selection

#### Changed
- Updated AI service factory to support all providers
- Updated environment variable handling
- Updated README with all provider options

---

## Useful Commands

### Development
```bash
# Run with debug logging
RUST_LOG=debug cargo run

# Run tests
cargo test

# Run with cargo watch (auto-reload)
cargo install cargo-watch
cargo watch -x run

# Format code
cargo fmt

# Lint code
cargo clippy

# Build for release
cargo build --release
```

### Database
```bash
# Connect to database
psql $DATABASE_URL

# Check schema
\d messages

# Query messages
SELECT * FROM messages ORDER BY created_at DESC LIMIT 10;

# Count messages per source
SELECT source_type, source_id, COUNT(*) FROM messages GROUP BY 1, 2;

# Find group IDs
SELECT DISTINCT source_id, source_type FROM messages WHERE source_type = 'group';
```

### Docker
```bash
# Start PostgreSQL
docker run -d --name line-bot-db \
  -e POSTGRES_PASSWORD=password \
  -e POSTGRES_DB=line_bot_db \
  -p 5432:5432 postgres

# View logs
docker logs line-bot-db

# Stop and remove
docker stop line-bot-db && docker rm line-bot-db
```

### ngrok
```bash
# Start ngrok
ngrok http 3000

# Get webhook URL
# Copy HTTPS URL from output

# Inspect requests
# Visit http://localhost:4040
```

---

## Resources

### Documentation
- [README.md](../README.md) - Project overview and setup
- [Architect.md](Architect.md) - System architecture
- [PRD.md](PRD.md) - Product requirements
- [Feature.md](Feature.md) - Feature documentation
- [Task.md](Task.md) - Development tasks
- [Test.md](Test.md) - Testing procedures
- [Sequence-Diagram.md](Sequence-Diagram.md) - Sequence diagrams
- [Data-Flow-Diagram.md](Data-Flow-Diagram.md) - Data flow diagrams
- [C4-Diagram.md](C4-Diagram.md) - C4 architecture diagrams
- [FSD.md](FSD.md) - Feature-Sliced Design

### External Documentation
- [LINE Messaging API](https://developers.line.biz/en/reference/messaging-api)
- [Claude API](https://docs.anthropic.com)
- [OpenAI API](https://platform.openai.com/docs)
- [Gemini API](https://ai.google.dev/gemini-api/docs)
- [Axum Framework](https://docs.rs/axum)
- [SQLx](https://docs.rs/sqlx)

---

## Notes

- This memory file serves as project knowledge base
- Update when significant decisions are made
- Track issues and their resolutions
- Document workarounds and lessons learned
- Keep this file in sync with code changes
