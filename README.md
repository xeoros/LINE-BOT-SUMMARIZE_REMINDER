# LINE Chat Summarizer Bot

A LINE bot written in Rust that automatically summarizes conversations using AI (Claude, OpenAI, or Gemini). The bot sits in group chats or 1-on-1 conversations, stores messages in PostgreSQL, and provides on-demand or scheduled summaries.

## Features

- **Message Collection**: Automatically stores LINE messages in PostgreSQL
- **On-Demand Summaries**: Request summaries with commands like `!summarize` or `/สรุป`
- **Flexible Commands**: Support for message count (`!summarize 100`) and time ranges (`!summarize 2h`)
- **Scheduled Summaries**: Auto-summarize at configurable times using cron syntax
- **Configurable AI**: Choose between Claude, OpenAI, or Gemini as your AI provider
- **Thai Language Support**: Optimized for Thai language summarization

## Architecture

```
LINE App → Webhook (ngrok) → Rust Server → PostgreSQL
                                   ↓
                              AI API (Claude/OpenAI/Gemini)
                                   ↓
                              LINE Reply API → User
```

## Tech Stack

- **Language**: Rust
- **Web Framework**: Axum
- **Database**: PostgreSQL (via SQLx)
- **AI**: Claude API / OpenAI API / Gemini API
- **Tunneling**: ngrok (for local development)

## Getting Started

### Prerequisites

- Rust 1.70+
- PostgreSQL
- ngrok (for local development)
- LINE Developers account

### Step 1: Setup LINE Platform

1. Go to [LINE Developers Console](https://developers.line.biz/console/)
2. Create a new Messaging API channel
3. Get your:
   - Channel Access Token (Long-lived)
   - Channel Secret
4. Enable webhook and set the URL later (after running ngrok)

### Step 2: Setup Database

Start PostgreSQL using Docker:

```bash
docker run -d --name line-bot-db \
  -e POSTGRES_PASSWORD=password \
  -e POSTGRES_DB=line_bot_db \
  -p 5432:5432 postgres
```

Apply the database schema:

```bash
psql -h localhost -U postgres -f sql/schema.sql
```

### Step 3: Configure Environment

Copy the example environment file:

```bash
cp .env.example .env
```

Edit `.env` with your credentials:

```env
LINE_CHANNEL_ACCESS_TOKEN=your_line_channel_access_token_here
LINE_CHANNEL_SECRET=your_line_channel_secret_here
DATABASE_URL=postgresql://postgres:password@localhost:5432/line_bot_db
AI_PROVIDER=claude
CLAUDE_API_KEY=your_claude_api_key_here
```

### Step 4: Run the Bot

```bash
# Install dependencies
cargo build

# Run the bot
cargo run
```

### Step 5: Setup ngrok

In another terminal:

```bash
ngrok http 3000
```

Copy the HTTPS URL (e.g., `https://abc123.ngrok-free.app`) and set it as your webhook URL in the LINE Developers Console:

```
https://your-ngrok-url.ngrok-free.app/webhook
```

### Step 6: Test the Bot

1. Add the bot to a group or send it a message
2. Send some messages in the chat
3. Send `!summarize` or `/สรุป` to get a summary

## Commands

| Command          | Description                             |
| ---------------- | --------------------------------------- |
| `!summarize`     | Summarize last 50 messages (default)    |
| `/สรุป`          | Thai version of summarize command       |
| `!summarize 100` | Summarize last 100 messages             |
| `!summarize 2h`  | Summarize messages from last 2 hours    |
| `/สรุป 30m`      | Summarize messages from last 30 minutes |

Time formats: `10m` (minutes), `1h` (hours), `1d` (days)

## Reminder / Checklist Commands

Create and manage task reminders with scheduled notifications:

| Command | Description |
|---------|-------------|
| `!task in 30m` + task list | Create checklist with reminder |
| `!task` + task list | Create checklist without reminder |
| `done <number>` | Mark task as done (e.g., `done 1`) |
| `!task` | Show all checklists |
| `delete <checklist_id>` | Delete a checklist |

### Examples

**Create a checklist with 30-minute reminder:**
```
!task in 30m
1. ซื้อของ
2. ทำการบ้าน
3. โทรหาหมอ
```

**Create a checklist with 2-hour reminder:**
```
!task in 2h
1. ประชุม team
2. ส่งรายงาน
```

**Create a checklist with 1-day reminder:**
```
!task in 1d
1. จ่ายบิล
2. ซื้อของในบ้าน
```

**Mark task as done:**
```
done 1
```

**Show all checklists:**
```
!task
```

The bot will send a reminder notification when the time is up, and you can mark tasks as complete using `done <number>`.

## Scheduled Summaries

Configure automatic summaries in `config/schedules.toml`:

```toml
[[schedules]]
source_type = "group"
source_id = "C456def"  # Replace with actual group ID
cron = "0 18 * * *"    # Daily at 6 PM
message_count = 100    # Summarize last 100 messages
```

Cron format: `minute hour day month day_of_week`

Examples:

- `0 18 * * *` - Every day at 6 PM
- `0 9 * * 1-5` - Weekdays at 9 AM
- `*/30 * * * *` - Every 30 minutes

## AI Provider Configuration

### Claude (Default)

```env
AI_PROVIDER=claude
CLAUDE_API_KEY=your_claude_api_key_here
CLAUDE_MODEL=claude-sonnet-4-6
```

### OpenAI

```env
AI_PROVIDER=openai
OPENAI_API_KEY=your_openai_api_key_here
OPENAI_MODEL=gpt-4o
```

### Gemini

```env
AI_PROVIDER=gemini
GEMINI_API_KEY=your_gemini_api_key_here
GEMINI_MODEL=gemini-2.0-flash
```

## Finding Group/User IDs

To find your group or user ID for scheduled summaries:

1. Add the bot to the group
2. Send a message in the group
3. Check the database:

```bash
psql -h localhost -U postgres -d line_bot_db -c "SELECT DISTINCT source_id, source_type FROM messages;"
```

Or check the logs when the bot receives messages.

## Project Structure

```
LINE_Bot_Summarize/
├── Cargo.toml
├── .env
├── sql/
│   └── schema.sql
├── config/
│   └── schedules.toml
└── src/
    ├── main.rs
    ├── config.rs
    ├── db/
    │   ├── mod.rs
    │   ├── pool.rs
    │   └── models.rs
    ├── line/
    │   ├── mod.rs
    │   ├── webhook.rs
    │   └── client.rs
    ├── ai/
    │   ├── mod.rs
    │   ├── claude.rs
    │   ├── openai.rs
    │   └── prompt.rs
    ├── scheduler/
    │   └── mod.rs
    └── handlers/
        └── summary.rs
```

## Limitations

1. **Historical Messages**: LINE does NOT provide historical messages. Only new messages after the bot joins will be captured.

2. **Bot Permissions**: In group chats, the bot must have "Read all messages" permission (requires group admin).

3. **Free Tier Limits**: LINE Messaging API free tier: 500 push messages/month (reply messages are unlimited).

4. **Webhook Security**: Webhook signature verification is implemented for security.

5. **Message Types**: Currently focuses on text messages. Images, stickers, and other types are stored but not included in summaries.

## Deployment

For production deployment:

1. Set up a PostgreSQL database (DigitalOcean, AWS RDS, etc.)
2. Deploy to a server (DigitalOcean Droplet, Railway, Render, etc.)
3. Use a domain with SSL certificate
4. Update LINE webhook URL to your production domain
5. Set up proper environment variables on the server

## Contributing

Contributions are welcome! Please feel free to submit issues or pull requests.

## License

MIT License

### How to Start

# Use Claude (default)

cargo run -- --provider claude

# Use OpenAI

cargo run -- --provider openai

# Use Gemini

cargo run -- --provider gemini

## Admin Dashboard

A built-in web dashboard for monitoring and managing reminder cronjobs.

### Access

```
http://localhost:8080/admin
```

### Features

- **Stats Overview**: View pending reminders, completed today, total checklists
- **Pending Reminders**: List all pending reminders with task counts and next notification time
- **Reschedule**: Adjust reminder times (+30m, +1h, +1d)
- **Test Alert**: Send a test notification to any LINE user ID
- **Auto-refresh**: Dashboard refreshes every 30 seconds

### API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/admin/` | GET | Dashboard UI |
| `/admin/api/stats` | GET | Get stats (pending, completed, total) |
| `/admin/api/reminders` | GET | List pending reminders |
| `/admin/api/reminders/:id/reschedule` | POST | Reschedule reminder `{"minutes": 30}` |
| `/admin/api/test-alert` | POST | Send test alert `{"target_id": "U123..."}` |
| `/admin/api/schedule` | GET/POST | Get/update schedule config |
