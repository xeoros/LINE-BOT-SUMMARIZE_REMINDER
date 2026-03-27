# Test Documentation

## Test Strategy

The LINE Chat Summarizer Bot follows a multi-layered testing approach:
1. **Unit Tests** - Individual component testing
2. **Integration Tests** - Component interaction testing
3. **System Tests** - End-to-end workflows
4. **Manual Tests** - User acceptance testing

---

## Prerequisites

### Test Environment Setup

1. **Start PostgreSQL**:
```bash
docker run -d --name line-bot-db-test \
  -e POSTGRES_PASSWORD=test_password \
  -e POSTGRES_DB=line_bot_test \
  -p 5433:5432 postgres
```

2. **Apply Test Schema**:
```bash
psql -h localhost -p 5433 -U postgres -d line_bot_test -f sql/schema.sql
```

3. **Set Test Environment**:
```bash
cp .env .env.test
# Update .env.test with test database URL
export DATABASE_URL="postgresql://postgres:test_password@localhost:5433/line_bot_test"
```

4. **Start ngrok**:
```bash
ngrok http 3000
# Copy HTTPS URL for LINE webhook
```

---

## Unit Tests

### Database Layer Tests

#### Test: Message Save
**File**: `src/db/models.rs`
**Description**: Verify message storage functionality

**Test Cases**:
```rust
#[test]
fn test_save_message() {
    // Should save new message
    // Should return message ID
    // Should handle duplicate IDs
}

#[test]
fn test_save_duplicate_message() {
    // Should ignore duplicate message_id
    // Should not error
}
```

**Run**:
```bash
cargo test db::models
```

#### Test: Message Retrieval

```rust
#[test]
fn test_get_recent_messages() {
    // Should return N most recent messages
    // Should order by created_at DESC
}

#[test]
fn test_get_messages_by_time_range() {
    // Should return messages within time range
    // Should order by created_at ASC
}
```

### AI Service Tests

#### Test: Claude Integration
**File**: `src/ai/claude.rs`

```rust
#[test]
fn test_claude_service_creation() {
    // Should create service with API key
    // Should store model name
}
```

### Command Parser Tests

#### Test: Summary Commands
**File**: `src/handlers/summary.rs`

**Test Cases**:
```rust
#[test]
fn test_parse_summarize_command() {
    let cmd = SummaryCommand::parse("!summarize");
    assert!(cmd.is_some());
    assert_eq!(cmd.unwrap().command_type, SummaryCommandType::Summarize);
}

#[test]
fn test_parse_thai_command() {
    let cmd = SummaryCommand::parse("/สรุป");
    assert!(cmd.is_some());
}

#[test]
fn test_parse_message_count() {
    let cmd = SummaryCommand::parse("!summarize 100");
    assert_eq!(cmd.unwrap().parameter, Some(SummaryParameter::MessageCount(100)));
}

#[test]
fn test_parse_time_range_minutes() {
    let cmd = SummaryCommand::parse("!summarize 30m");
    assert_eq!(cmd.unwrap().parameter, Some(SummaryParameter::TimeRange(30)));
}

#[test]
fn test_parse_time_range_hours() {
    let cmd = SummaryCommand::parse("!summarize 2h");
    assert_eq!(cmd.unwrap().parameter, Some(SummaryParameter::TimeRange(120)));
}

#[test]
fn test_parse_invalid_command() {
    let cmd = SummaryCommand::parse("hello world");
    assert!(cmd.is_none());
}
```

**Run**:
```bash
cargo test handlers::summary
```

### Config Tests

#### Test: Environment Variables
**File**: `src/config.rs`

```rust
#[test]
fn test_config_from_env() {
    // Should load from environment
    // Should validate required variables
}

#[test]
fn test_config_missing_required() {
    // Should error on missing DATABASE_URL
    // Should error on missing LINE keys
}
```

---

## Integration Tests

### Webhook Handler Tests

#### Test: Webhook Verification
**Description**: Verify webhook signature validation

**Steps**:
1. Generate test webhook body
2. Compute signature with test secret
3. Send to `/webhook` with signature
4. Expect 200 OK

**Invalid Signature**:
1. Send webhook with wrong signature
2. Expect 401 Unauthorized

#### Test: Message Event Handling
**Description**: Verify message events are stored

**Steps**:
1. Send test message webhook
2. Verify message in database
3. Check sender profile fetched
4. Verify timestamp correct

### LINE API Tests

#### Test: Reply Message
**Description**: Verify LINE reply functionality

**Steps**:
1. Get reply token from test webhook
2. Call `reply_message()` with text
3. Verify LINE API called
4. Check mock response

#### Test: Push Message
**Description**: Verify LINE push functionality

**Steps**:
1. Call `push_message()` to test user ID
2. Verify LINE API called
3. Check mock response

### Database Integration Tests

#### Test: Concurrent Message Storage
**Description**: Verify database handles concurrent writes

**Steps**:
1. Send 100 messages simultaneously
2. Verify all stored
3. Check no duplicates
4. Verify all timestamps correct

---

## System Tests

### End-to-End Workflow: Summary Request

**Test Case 1: Basic Summary**

| Step | Action | Expected Result |
|-------|---------|----------------|
| 1 | Start bot | Server running on port 3000 |
| 2 | Send "hello" to bot | Message stored in DB |
| 3 | Send 10 messages | All 10 messages stored |
| 4 | Send "!summarize" | Summary generated and replied |
| 5 | Check reply | Thai summary with key points |
| 6 | Check logs | AI API called with conversation |

**Test Case 2: Thai Command**

| Step | Action | Expected Result |
|-------|---------|----------------|
| 1 | Send "/สรุป" | Summary generated and replied |
| 2 | Check reply | Thai language summary |

**Test Case 3: Message Count**

| Step | Action | Expected Result |
|-------|---------|----------------|
| 1 | Send 50 messages | All stored |
| 2 | Send "!summarize 10" | Summary of last 10 messages |
| 3 | Check content | Only 10 messages included |

**Test Case 4: Time Range**

| Step | Action | Expected Result |
|-------|---------|----------------|
| 1 | Send 5 messages now | Stored with timestamps |
| 2 | Wait 5 minutes |
| 3 | Send 5 messages | All stored |
| 4 | Send "!summarize 10m" | Summary of last 10 minutes |
| 5 | Check content | Only recent messages |

**Test Case 5: Empty Chat**

| Step | Action | Expected Result |
|-------|---------|----------------|
| 1 | Add bot to new group | Welcome message sent |
| 2 | Send "!summarize" | "No messages" response |
| 3 | Send 1 message | Stored |
| 4 | Send "!summarize" | Summary of 1 message |

### End-to-End Workflow: Scheduled Summary

**Test Case 1: Daily Schedule**

| Step | Action | Expected Result |
|-------|---------|----------------|
| 1 | Create `config/schedules.toml` | Config file exists |
| 2 | Add test schedule | Cron entry for 1 minute from now |
| 3 | Restart bot | Schedule loaded |
| 4 | Wait for trigger | Summary sent automatically |
| 5 | Check LINE | Push message received |
| 6 | Check logs | Summary generated and sent |

**Test Case 2: Multiple Schedules**

| Step | Action | Expected Result |
|-------|---------|----------------|
| 1 | Add 2 schedules | Both loaded |
| 2 | Wait for both triggers | Both summaries sent |
| 3 | Check timestamps | Correct cron timing |

---

## Manual Tests

### LINE Platform Setup

**Prerequisites**:
- LINE Developers account
- LINE app with Messaging API enabled
- ngrok running

**Test Steps**:

1. **Create LINE Channel**:
   - [ ] Login to LINE Developers Console
   - [ ] Create new channel
   - [ ] Enable Messaging API
   - [ ] Get Channel Access Token
   - [ ] Get Channel Secret

2. **Configure Webhook**:
   - [ ] Get ngrok HTTPS URL
   - [ ] Set webhook URL to `{ngrok_url}/webhook`
   - [ ] Enable webhook use
   - [ ] Enable "Auto-reply messages"
   - [ ] Verify webhook (test endpoint)

3. **Add Bot to Group**:
   - [ ] Get bot's LINE ID
   - [ ] Create group or use existing
   - [ ] Invite bot to group
   - [ ] Bot joins successfully
   - [ ] Welcome message displayed

### On-Demand Summary Tests

**Test Checklist**:

| Command | Bot Added | Messages | Summary Generated | Thai Language | Correct Timeframe |
|----------|-----------|-----------|------------------|------------------|
| `!summarize` | ✅ | 50 | ✅ | ✅ |
| `!summarize` | ✅ | 0 | N/A | N/A |
| `/สรุป` | ✅ | 20 | ✅ | ✅ |
| `!summarize 10` | ✅ | 100 | ✅ | ✅ |
| `!summarize 2h` | ✅ | 50 | ✅ | ✅ |
| `!summarize 1d` | ✅ | 100 | ✅ | ✅ |

### Scheduled Summary Tests

**Test Checklist**:

| Schedule | Trigger | Message Sent | Correct Summary |
|----------|---------|--------------|----------------|
| Daily 6 PM | ✅ | ✅ | ✅ |
| Weekdays 9 AM | ✅ | ✅ | ✅ |
| Every 30 min | ✅ | ✅ | ✅ |

### AI Provider Tests

**Test Checklist**:

| Provider | API Key | Model | Summary Generated | Quality |
|----------|----------|--------|------------------|----------|
| Claude | ✅ | claude-sonnet-4-6 | ✅ | Good |
| OpenAI | ✅ | gpt-4o | ✅ | Good |
| Gemini | ✅ | gemini-2.0-flash | ✅ | Good |

### Security Tests

**Test Checklist**:

| Test | Result |
|-------|--------|
| Valid webhook signature | ✅ Processed |
| Invalid webhook signature | ❌ Rejected (401) |
| Missing signature header | ❌ Rejected (401) |
| Wrong channel secret | ❌ Rejected (401) |
| Replay attack protection | ✅ Timestamp check |

### Edge Cases

**Test Checklist**:

| Scenario | Expected Behavior |
|-----------|------------------|
| Bot leaves group | Leave event logged |
| Bot rejoins group | Join event handled |
| User joins group | No action |
| Message from blocked user | Stored normally |
| Non-text message (sticker) | Stored, not in summary |
| Very long message (>10k chars) | Stored, handled by AI |
| Concurrent summaries | Both processed |
| AI API timeout | Error logged, no crash |
| Database connection lost | Retry, connection pool |
| Large conversation (1000+ messages) | Summarized with limits |

---

## Performance Tests

### Load Testing

**Test: Concurrent Webhooks**

**Setup**:
```bash
# Use tools like hey or wrk
hey -n 1000 -c 100 https://your-bot-url/webhook
```

**Metrics**:
- Requests/sec: > 100
- P95 Latency: < 500ms
- Error rate: < 1%

**Test: Large Conversation Summary**

**Setup**:
```rust
// Generate 1000 test messages
for i in 0..1000 {
    save_message(...);
}

// Time the summary
let start = Instant::now();
let summary = generate_summary().await;
let duration = start.elapsed();
```

**Metrics**:
- Generation time: < 10 seconds
- Memory usage: < 512MB
- API calls: 1

---

## Test Results Template

```markdown
## Test Run - [Date]

### Environment
- OS: [macOS/Linux/Windows]
- Rust version: [1.70+]
- Database: PostgreSQL [version]
- AI Provider: [Claude/OpenAI/Gemini]

### Unit Tests
- Total: [X]
- Passed: [X]
- Failed: [X]
- Coverage: [X]%

### Integration Tests
- Total: [X]
- Passed: [X]
- Failed: [X]

### System Tests
- Basic Summary: [✅/❌]
- Thai Command: [✅/❌]
- Message Count: [✅/❌]
- Time Range: [✅/❌]
- Scheduled: [✅/❌]

### Performance
- Webhook Latency P95: [X]ms
- Summary Generation: [X]s
- Concurrent Requests: [X]/s

### Issues Found
1. [Description]
2. [Description]

### Notes
[Additional observations]
```

---

## Continuous Testing

### CI/CD Pipeline

**GitHub Actions Example**:
```yaml
name: Test

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres
        env:
          POSTGRES_PASSWORD: test
          POSTGRES_DB: line_bot_test
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
      - run: cargo test --all-features
      - run: cargo clippy -- -D warnings
```

### Automated Testing Schedule

| Frequency | Type | Scope |
|-----------|--------|--------|
| On PR | Unit + Integration | Changed files |
| On Merge | System | Core features |
| Nightly | Performance + Load | All features |
| Weekly | Manual | End-to-end |

---

## Troubleshooting Tests

### Common Issues

**Issue: Tests fail with database connection**

**Solution**:
```bash
# Check PostgreSQL is running
docker ps | grep postgres

# Check connection string
echo $DATABASE_URL

# Test connection manually
psql $DATABASE_URL
```

**Issue: Webhook not receiving events**

**Solution**:
```bash
# Check ngrok status
ngrok http 3000

# Verify webhook URL in LINE Console

# Check webhook test endpoint
curl -X POST https://your-url/webhook
```

**Issue: AI API returns errors**

**Solution**:
```bash
# Check API key validity
curl -H "x-api-key: $CLAUDE_API_KEY" \
  https://api.anthropic.com/v1/messages

# Check quota and billing
# Visit provider console
```

---

## Test Coverage Goals

| Component | Target Coverage | Current |
|-----------|----------------|----------|
| Database | 80% | - |
| AI Provider Integration | 90% | - |
| Reminder Checklist | 90% | - |
| Summarize Commands | 90% | - |
| **Overall (in-scope)** | **90%** | - |

### Coverage Command (In-Scope)

Prereqs:
```bash
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov
```

Run:
```bash
./scripts/coverage.sh
```

---

## Notes

- Test before deploying to production
- Keep test data separate from production
- Mock external APIs for unit tests
- Use real APIs for integration tests
- Document all test failures
- Update tests as features change
