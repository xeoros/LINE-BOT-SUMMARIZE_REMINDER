# C4 Architecture Diagrams

## C4 Model Overview

The C4 model provides a simple way to model software architecture, focusing on the context, containers, components, and code.

---

## Context Diagram

### Level 1: System Context

```mermaid
graph TB
    subgraph "LINE Platform"
        LINE_User[LINE User]
        LINE_Group[LINE Group Chat]
    end

    subgraph "LINE Bot Summarizer"
        BotSystem[Bot System]
    end

    subgraph "External Services"
        Claude_API[Claude API]
        OpenAI_API[OpenAI API]
        Gemini_API[Gemini API]
        PostgreSQL[(PostgreSQL Database)]
    end

    LINE_User -->|Send Message| BotSystem
    LINE_User -->|Request Summary| BotSystem
    LINE_Group -->|Receive Webhook| BotSystem
    LINE_Group -->|Receive Summary| BotSystem

    BotSystem -->|Store Messages| PostgreSQL
    BotSystem -->|Retrieve Messages| PostgreSQL
    BotSystem -->|Generate Summary| Claude_API
    BotSystem -->|Generate Summary| OpenAI_API
    BotSystem -->|Generate Summary| Gemini_API
```

### Description

The LINE Bot Summarizer sits between LINE users/groups and AI services. It:
1. Receives messages from LINE via webhooks
2. Stores messages in PostgreSQL
3. Generates summaries using Claude, OpenAI, or Gemini
4. Sends summaries back to LINE users/groups

---

## Container Diagram

### Level 2: Container View

```mermaid
graph TB
    subgraph "LINE Platform"
        LINE_P[LINE Platform]
    end

    subgraph "LINE Bot Summarizer"
        WebServer[Web Server]
        Scheduler[Scheduled Summaries]
    end

    subgraph "Infrastructure"
        DB[(PostgreSQL)]
        Env[Configuration]
    end

    subgraph "AI Services"
        Claude[Claude API]
        OpenAI[OpenAI API]
        Gemini[Gemini API]
    end

    LINE_P -->|Webhook| WebServer
    LINE_P -->|Push Summary| WebServer

    WebServer -->|Read/Write| DB
    WebServer -->|Generate Summary| Claude
    WebServer -->|Generate Summary| OpenAI
    WebServer -->|Generate Summary| Gemini

    WebServer -->|Load| Env
    Scheduler -->|Load| Env

    Scheduler -->|Read| DB
    Scheduler -->|Generate Summary| Claude
    Scheduler -->|Generate Summary| OpenAI
    Scheduler -->|Generate Summary| Gemini
    Scheduler -->|Push Summary| LINE_P
```

### Container Descriptions

| Container | Technology | Responsibility |
|-----------|------------|----------------|
| Web Server | Axum + Rust | Handle webhooks, commands, health checks |
| Scheduler | tokio-cron-scheduler | Manage scheduled summary jobs |
| PostgreSQL | SQLx | Store and retrieve messages |
| Configuration | dotenv + Environment | Manage settings and API keys |

---

## Component Diagram

### Level 3: Component View

```mermaid
graph TB
    subgraph "Web Server Container"
        subgraph "Web Layer"
            Router[Router]
            WebhookHandler[Webhook Handler]
            HealthCheck[Health Check]
        end

        subgraph "Application Layer"
            CommandParser[Command Parser]
            MessageProcessor[Message Processor]
            ProfileService[Profile Service]
        end

        subgraph "Data Layer"
            DBPool[Connection Pool]
            MessageRepo[Message Repository]
        end

        subgraph "Integration Layer"
            LineClient[LINE API Client]
            AIService[AI Service Factory]
        end

        subgraph "AI Implementations"
            Claude[Claude Service]
            OpenAI[OpenAI Service]
            Gemini[Gemini Service]
        end
    end

    subgraph "Scheduler Container"
        ScheduleLoader[Schedule Loader]
        CronRunner[Cron Runner]
        JobExecutor[Job Executor]
    end

    subgraph "External"
        LINE_API[LINE Messaging API]
        Claude_API[Claude API]
        OpenAI_API[OpenAI API]
        Gemini_API[Gemini API]
        DB[(PostgreSQL)]
    end

    %% Web Layer
    Router -->|Route| WebhookHandler
    Router -->|Route| HealthCheck

    %% Application Layer
    WebhookHandler -->|Parse| CommandParser
    WebhookHandler -->|Process| MessageProcessor
    MessageProcessor -->|Lookup| ProfileService

    %% Data Layer
    MessageProcessor -->|Store| MessageRepo
    MessageProcessor -->|Retrieve| MessageRepo
    CommandParser -->|Retrieve| MessageRepo
    MessageRepo -->|Query| DBPool
    DBPool -->|Connection| DB

    %% Integration Layer
    MessageProcessor -->|Reply| LineClient
    ProfileService -->|Fetch| LineClient
    AIService -->|Create| Claude
    AIService -->|Create| OpenAI
    AIService -->|Create| Gemini

    %% AI Implementations
    Claude -->|Call API| Claude_API
    OpenAI -->|Call API| OpenAI_API
    Gemini -->|Call API| Gemini_API

    %% Scheduler
    ScheduleLoader -->|Load| CronRunner
    CronRunner -->|Trigger| JobExecutor
    JobExecutor -->|Retrieve| MessageRepo
    JobExecutor -->|Generate| AIService
    JobExecutor -->|Push| LineClient

    %% External
    WebhookHandler -->|Verify| LINE_API
    LineClient -->|Reply| LINE_API
    LineClient -->|Push| LINE_API
```

### Component Descriptions

#### Web Layer

| Component | File | Responsibility |
|-----------|-------|----------------|
| Router | `main.rs` | HTTP routing, state management |
| Webhook Handler | `line/webhook.rs` | Event processing, signature verification |
| Health Check | `main.rs` | System health endpoint |

#### Application Layer

| Component | File | Responsibility |
|-----------|-------|----------------|
| Command Parser | `handlers/summary.rs` | Parse summary commands and parameters |
| Message Processor | `main.rs` | Orchestrate message storage and commands |
| Profile Service | `line/client.rs` | Fetch user/group member profiles |

#### Data Layer

| Component | File | Responsibility |
|-----------|-------|----------------|
| Connection Pool | `db/pool.rs` | PostgreSQL connection management |
| Message Repository | `db/models.rs` | CRUD operations for messages |

#### Integration Layer

| Component | File | Responsibility |
|-----------|-------|----------------|
| LINE API Client | `line/client.rs` | LINE Messaging API communication |
| AI Service Factory | `ai/mod.rs` | Create appropriate AI provider instance |

#### AI Implementations

| Component | File | Responsibility |
|-----------|-------|----------------|
| Claude Service | `ai/claude.rs` | Claude API integration |
| OpenAI Service | `ai/openai.rs` | OpenAI API integration |
| Gemini Service | `ai/gemini.rs` | Gemini API integration |

---

## Code Diagram

### Level 4: Code View - Webhook Handler

```mermaid
classDiagram
    class WebhookHandler {
        +verify_signature()
        +parse_event()
        +handle_message()
        +handle_follow()
        +handle_leave()
    }

    class SignatureVerifier {
        +hmac_sha256()
        +base64_encode()
    }

    class EventParser {
        +parse_webhook_body()
        +extract_source()
        +extract_message()
    }

    class ProfileFetcher {
        +get_user_profile()
        +get_group_member_profile()
        +cache_profile()
    }

    class MessageStore {
        +save_message()
        +get_recent_messages()
        +get_by_time_range()
    }

    WebhookHandler --> SignatureVerifier
    WebhookHandler --> EventParser
    WebhookHandler --> ProfileFetcher
    WebhookHandler --> MessageStore
```

### Code View - Summary Command

```mermaid
classDiagram
    class SummaryCommand {
        +parse(text)
        +execute(pool, line, ai, source, id, token)
    }

    class ParameterParser {
        +parse_count()
        +parse_time_range()
        +convert_to_minutes()
    }

    class ConversationFormatter {
        +format_messages()
        +add_timestamp()
        +filter_text_only()
    }

    class AIService {
        <<interface>>
        +generate_summary(messages)
    }

    class SummaryGenerator {
        +fetch_messages()
        +generate_prompt()
        +call_ai_api()
        +format_response()
    }

    SummaryCommand --> ParameterParser
    SummaryCommand --> ConversationFormatter
    SummaryCommand --> MessageStore
    SummaryCommand --> AIService
    AIService <|.. ClaudeService
    AIService <|.. OpenAIService
    AIService <|.. GeminiService
```

### Code View - AI Service

```mermaid
classDiagram
    class AIService {
        <<interface>>
        +generate_summary(messages)
    }

    class ClaudeService {
        -client: reqwest::Client
        -api_key: String
        -model: String
        +new(api_key, model)
        +send_request(messages)
    }

    class OpenAIService {
        -client: reqwest::Client
        -api_key: String
        -model: String
        +new(api_key, model)
        +send_request(messages)
    }

    class GeminiService {
        -client: reqwest::Client
        -api_key: String
        -model: String
        +new(api_key, model)
        +send_request(prompt)
    }

    class PromptBuilder {
        +build_summary_prompt(conversation)
        +get_thai_template()
        +get_english_template()
    }

    AIService <|.. ClaudeService
    AIService <|.. OpenAIService
    AIService <|.. GeminiService
    ClaudeService --> PromptBuilder
    OpenAIService --> PromptBuilder
    GeminiService --> PromptBuilder
```

### Code View - Scheduler

```mermaid
classDiagram
    class ScheduledSummaries {
        -scheduler: JobScheduler
        -pool: Arc<PgPool>
        -line_client: Arc<LineClient>
        -ai_service: Arc<AIService>
        +new(pool, line, ai)
        +load_schedules(path)
        +start()
        +shutdown()
    }

    class ScheduleConfig {
        -schedules: Vec<Schedule>
        +from_file(path)
    }

    class JobRunner {
        -schedule: Schedule
        +execute_job(uuid, lock)
        +fetch_messages()
        +generate_summary()
        +push_summary()
    }

    class CronParser {
        +parse_expression(cron)
        +calculate_next_run()
    }

    ScheduledSummaries --> ScheduleConfig
    ScheduledSummaries --> JobRunner
    JobRunner --> CronParser
```

---

## Data Flow - C4 Level 2

### Deep Data Flow

```mermaid
graph LR
    subgraph "Incoming"
        A[LINE Webhook] --> B[Webhook Handler]
    end

    subgraph "Processing"
        B --> C{Command?}
        C -->|Yes| D[Command Parser]
        C -->|No| E[Message Store]
        D --> F[Message Repo]
        E --> F
    end

    subgraph "Storage"
        F --> G[(PostgreSQL)]
    end

    subgraph "AI Processing"
        F --> H[Conversation Formatter]
        H --> I{AI Provider}
        I -->|Claude| J[Claude API]
        I -->|OpenAI| K[OpenAI API]
        I -->|Gemini| L[Gemini API]
    end

    subgraph "Outgoing"
        J --> M[Summary Text]
        K --> M
        L --> M
        M --> N[LINE Reply/Push]
    end
```

---

## Deployment View

### Production Deployment

```mermaid
graph TB
    subgraph "User Access"
        User[LINE Users]
    end

    subgraph "Cloud Infrastructure"
        subgraph "Application Server"
            LB[Load Balancer]
            App1[Bot Instance 1]
            App2[Bot Instance 2]
            AppN[Bot Instance N]
        end

        subgraph "Database"
            Primary[(Primary PostgreSQL)]
            Replica[(Read Replica)]
        end

        subgraph "Monitoring"
            Logs[Log Aggregation]
            Metrics[Metrics Collection]
            Alerts[Alert System]
        end
    end

    subgraph "External APIs"
        LINE[LINE Platform API]
        Claude[Claude API]
        OpenAI[OpenAI API]
        Gemini[Gemini API]
    end

    User --> LINE
    LINE -->|Webhook| LB
    LB --> App1
    LB --> App2
    LB --> AppN

    App1 --> Primary
    App2 --> Primary
    AppN --> Primary
    App1 --> Replica
    App2 --> Replica
    AppN --> Replica

    App1 --> Logs
    App2 --> Logs
    AppN --> Logs
    App1 --> Metrics
    App2 --> Metrics
    AppN --> Metrics

    App1 -->|Summary| Claude
    App1 -->|Summary| OpenAI
    App1 -->|Summary| Gemini
    App2 -->|Summary| Claude
    App2 -->|Summary| OpenAI
    App2 -->|Summary| Gemini

    App1 -->|Reply/Push| LINE
    App2 -->|Reply/Push| LINE
```

### Deployment Components

| Component | Technology | Count | Responsibility |
|-----------|------------|--------|----------------|
| Load Balancer | Nginx/Cloud LB | 1+ | Distribute traffic |
| App Instances | Docker/Rust | 2+ | Handle requests |
| Database | PostgreSQL | 2+ | Data storage |
| Monitoring | Prometheus/Grafana | 1+ | Observability |

---

## C4 Model Key

| Element | Symbol | Description |
|---------|---------|-------------|
| Person | 👤 | User/actor interacting with system |
| System | 🌐 | Software system at highest level |
| Container | 📦 | Standalone unit (application/service) |
| Component | ⚙️ | Part of a container |
| Database | 💾 | Database or data store |
| External | 🌐 | External system/service |

---

## Diagram Legend

### Notation

- **Boxes**: Represent systems, containers, components
- **Lines**: Represent relationships/data flow
- **Arrow Direction**: Data flow direction
- **Subgraphs**: Logical grouping of elements

### Line Types

| Line Style | Meaning |
|------------|---------|
| `-->` | Synchronous call |
| `-->|` | Asynchronous call |
| `..>` | Implementation/inheritance |
| `--` | Dependency |

---

## Notes

- All diagrams follow C4 Model principles
- Simplicity over detail at higher levels
- Progressive disclosure: more detail at lower levels
- Focus on value delivery and user interactions
- Render using Mermaid-compatible tools
