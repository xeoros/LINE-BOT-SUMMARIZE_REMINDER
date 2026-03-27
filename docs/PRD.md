# Product Requirements Document (PRD)

## Product Name
LINE Chat Summarizer Bot

## Version
1.0.0

## Executive Summary

A LINE Messaging API bot that automatically collects chat messages and generates summaries using AI. The bot operates in both group chats and 1-on-1 conversations, providing on-demand and scheduled summaries optimized for Thai language.

## Problem Statement

Group chats in LINE often contain hundreds of messages making it difficult to:
- Track important decisions made
- Identify action items and assigned responsibilities
- Recall key discussions without scrolling through extensive history
- Maintain context when joining or re-entering a conversation

## Solution

An automated bot that:
1. Joins LINE conversations as a participant
2. Stores all messages in a database
3. Generates AI-powered summaries on demand
4. Supports scheduled automatic summaries
5. Works seamlessly in Thai language

## User Personas

### Primary Users

#### 1. Team Managers
- **Goals**: Track team decisions, action items, and progress
- **Pain Points**: Missed important discussions, unclear action assignments
- **Use Cases**: Daily meeting summaries, project tracking

#### 2. Project Coordinators
- **Goals**: Maintain project history, identify blockers
- **Pain Points**: Lost context in long conversations
- **Use Cases**: Periodic reviews, milestone tracking

#### 3. Social Group Members
- **Goals**: Recall important announcements, plan coordination
- **Pain Points**: Information overload in active groups
- **Use Cases**: Event planning, key discussion recall

## Functional Requirements

### FR-1: Message Collection
- **Priority**: P0 (Must Have)
- **Description**: Automatically store all messages from joined chats
- **Acceptance Criteria**:
  - Captures text messages with sender and timestamp
  - Stores user/group identification
  - Handles duplicate message IDs gracefully
  - Supports user, group, and room source types
  - Stores non-text messages (metadata only)

### FR-2: On-Demand Summaries
- **Priority**: P0 (Must Have)
- **Description**: Generate summaries via chat commands
- **Acceptance Criteria**:
  - `!summarize` command works
  - `/สรุป` (Thai) command works
  - Summary includes key decisions
  - Summary includes action items
  - Summary identifies who said what
  - Response time < 10 seconds

### FR-3: Flexible Summary Commands
- **Priority**: P1 (Should Have)
- **Description**: Support various summary parameters
- **Acceptance Criteria**:
  - `!summarize N` - Last N messages
  - `!summarize Xm` - Last X minutes
  - `!summarize Xh` - Last X hours
  - `!summarize Xd` - Last X days
  - Default to 50 messages if no parameter

### FR-4: Scheduled Summaries
- **Priority**: P1 (Should Have)
- **Description**: Automatic summaries at configurable times
- **Acceptance Criteria**:
  - TOML-based schedule configuration
  - Cron syntax support
  - Multiple schedules per source
  - Push notifications (not reply)
  - Graceful failure handling

### FR-5: AI Provider Support
- **Priority**: P1 (Should Have)
- **Description**: Multiple AI service options
- **Acceptance Criteria**:
  - Claude API support
  - OpenAI API support
  - Gemini API support
  - Configurable via environment
  - Easy switching between providers

### FR-6: Webhook Security
- **Priority**: P0 (Must Have)
- **Description**: Verify LINE webhook authenticity
- **Acceptance Criteria**:
  - HMAC-SHA256 signature verification
  - Reject unauthorized requests
  - Log security events
  - Use channel secret from config

### FR-7: Thai Language Optimization
- **Priority**: P0 (Must Have)
- **Description**: Optimized for Thai language summaries
- **Acceptance Criteria**:
  - Thai prompt templates
  - Proper Thai output formatting
  - English documentation available

### FR-8: Health Monitoring
- **Priority**: P2 (Nice to Have)
- **Description**: System health endpoint
- **Acceptance Criteria**:
  - `/health` endpoint returns status
  - Includes service name
  - Fast response (< 100ms)

## Non-Functional Requirements

### NFR-1: Performance
- **Response Time**: Webhook < 500ms (message storage)
- **Summary Generation**: < 10 seconds for 100 messages
- **Database Queries**: < 100ms
- **Concurrent Users**: Support 100+ simultaneous webhooks

### NFR-2: Reliability
- **Uptime**: 99.5%+ (excluding AI provider outages)
- **Data Loss**: Zero message loss on webhook receipt
- **Graceful Degradation**: AI failures don't crash service

### NFR-3: Security
- **Webhook Verification**: Required for all requests
- **API Keys**: Never logged or exposed
- **SQL Injection**: Prevented via parameterized queries
- **Rate Limiting**: Respect LINE API limits

### NFR-4: Scalability
- **Horizontal**: Stateless web server design
- **Vertical**: Configurable connection pool
- **Database**: Indexed queries for large datasets
- **Storage**: Efficient message storage

### NFR-5: Maintainability
- **Code Quality**: Clean architecture, documented
- **Configuration**: Environment variables only
- **Logging**: Structured, filterable
- **Updates**: Zero-downtime deployment capable

### NFR-6: Observability
- **Logs**: All errors logged
- **Metrics**: Future enhancement
- **Tracing**: Future enhancement
- **Health Checks**: Simple endpoint available

## Technical Constraints

### Platform Constraints
- **Language**: Rust
- **Database**: PostgreSQL
- **Web Framework**: Axum
- **Runtime**: Tokio async

### External Dependencies
- **LINE Messaging API**: Required
- **AI API**: At least one required
- **PostgreSQL**: Required
- **HTTPS**: Required for production webhook

### LINE API Limitations
- **Historical Messages**: NOT available
- **Bot Permissions**: "Read all messages" required for groups
- **Push Limit**: 500/month (free tier)
- **Reply Messages**: Unlimited

## Future Enhancements

### FE-1: AI Selection by Command
- Command: `!summarize --ai=gemini`
- Per-request AI provider switching

### FE-2: Custom Prompts
- User-defined summary templates
- Industry-specific prompts

### FE-3: Multi-Language Support
- Detect chat language automatically
- Multi-lingual summaries

### FE-4: Rich Media Support
- Image OCR and summarization
- Audio transcription
- File content analysis

### FE-5: Export Features
- Export summaries to PDF/Markdown
- Email scheduled summaries
- Integration with note-taking apps

### FE-6: Analytics
- Summary usage statistics
- Most active users identification
- Trend analysis

## Success Metrics

### User Engagement
- Active groups: 10+ within 3 months
- Summaries generated: 50+ per week
- Command usage: 70% on-demand, 30% scheduled

### Technical Metrics
- Uptime: 99.5%+
- Response time: P95 < 5 seconds
- Error rate: < 1%

### Business Metrics
- User satisfaction: 4.5/5.0
- Feature requests: < 5 per month
- Support tickets: Minimal

## Risks & Mitigations

### Risk 1: LINE API Changes
- **Impact**: Breaking changes may require updates
- **Mitigation**: Version pinning, monitoring updates

### Risk 2: AI API Limits
- **Impact**: Rate limiting, service disruption
- **Mitigation**: Multiple provider support, caching

### Risk 3: Data Privacy
- **Impact**: User concerns about message storage
- **Mitigation**: Clear privacy policy, data retention options

### Risk 4: Spam/Automation Abuse
- **Impact**: Excessive API usage, service degradation
- **Mitigation**: Rate limiting, user blocking

## Dependencies

### External Services
| Service | Purpose | SLA |
|----------|---------|------|
| LINE Messaging API | Webhook & messaging | 99.9% |
| Claude API | AI generation | 99.5% |
| OpenAI API | AI generation | 99.5% |
| Gemini API | AI generation | 99.5% |

### Internal Dependencies
- PostgreSQL database
- Environment configuration
- Cron scheduler (for scheduled summaries)

## Open Questions

1. **Data Retention**: How long should messages be stored?
2. **User Consent**: Opt-in requirement for group chats?
3. **Multi-Tenant**: Should this support multiple bot instances?
4. **Admin Features**: Add admin dashboard for configuration?

## Revision History

| Version | Date | Author | Changes |
|---------|--------|---------|----------|
| 1.0.0 | 2026-03-10 | Initial PRD creation |
