-- LINE Chat Summarizer Bot Database Schema
-- Extended for Slack thread support

CREATE TABLE IF NOT EXISTS messages (
    id SERIAL PRIMARY KEY,
    message_id VARCHAR(100) UNIQUE NOT NULL,
    source_type VARCHAR(20) NOT NULL,  -- 'user', 'group', 'room', 'slack_channel', 'slack_user'
    source_id VARCHAR(100) NOT NULL,   -- userId, groupId, roomId, channelId
    sender_id VARCHAR(100),
    display_name TEXT,
    message_type VARCHAR(20) NOT NULL,  -- 'text', 'image', 'sticker', etc.
    message_text TEXT,
    thread_id VARCHAR(100),              -- Thread timestamp for Slack/line threads
    parent_message_id VARCHAR(100),     -- Parent message ID for reply hierarchy
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_messages_source ON messages(source_type, source_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_messages_created_at ON messages(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_messages_message_id ON messages(message_id);
CREATE INDEX IF NOT EXISTS idx_messages_thread ON messages(thread_id);
CREATE INDEX IF NOT EXISTS idx_messages_parent ON messages(parent_message_id);

-- Comments for documentation
COMMENT ON TABLE messages IS 'Stores all messages received by the bot (LINE and Slack)';
COMMENT ON COLUMN messages.source_type IS 'Type of source: user, group, room, slack_channel, or slack_user';
COMMENT ON COLUMN messages.source_id IS 'Source ID: userId, groupId, roomId, or channelId';
COMMENT ON COLUMN messages.sender_id IS 'User ID of the sender (LINE userId or Slack userId)';
COMMENT ON COLUMN messages.display_name IS 'Display name of the sender';
COMMENT ON COLUMN messages.message_type IS 'Type of message: text, image, sticker, audio, video, etc.';
COMMENT ON COLUMN messages.message_text IS 'Text content of the message (null for non-text messages)';
COMMENT ON COLUMN messages.thread_id IS 'Thread timestamp or ID for grouping thread conversations';
COMMENT ON COLUMN messages.parent_message_id IS 'Parent message ID for establishing reply-to relationships in threads';

-- Reminders and Checklist System
CREATE TABLE IF NOT EXISTS reminders (
    id SERIAL PRIMARY KEY,
    reminder_id VARCHAR(100) UNIQUE NOT NULL,
    source_type VARCHAR(20) NOT NULL,      -- 'user', 'group', 'room'
    source_id VARCHAR(100) NOT NULL,       -- userId, groupId, roomId
    sender_id VARCHAR(100),
    checklist_id VARCHAR(100),             -- Groups related tasks into a checklist
    task_number INTEGER NOT NULL,           -- Order in checklist (1, 2, 3...)
    task_text TEXT NOT NULL,               -- Task description
    is_completed BOOLEAN DEFAULT FALSE,
    notify_at TIMESTAMP WITH TIME ZONE,     -- When to send reminder notification
    completed_at TIMESTAMP WITH TIME ZONE,  -- When task was marked complete
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_reminders_source ON reminders(source_type, source_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_reminders_checklist ON reminders(checklist_id);
CREATE INDEX IF NOT EXISTS idx_reminders_notify_at ON reminders(notify_at) WHERE notify_at IS NOT NULL AND is_completed = FALSE;
CREATE INDEX IF NOT EXISTS idx_reminders_pending ON reminders(is_completed, notify_at) WHERE is_completed = FALSE;

-- Checklists metadata
CREATE TABLE IF NOT EXISTS checklists (
    id SERIAL PRIMARY KEY,
    checklist_id VARCHAR(100) UNIQUE NOT NULL,
    source_type VARCHAR(20) NOT NULL,
    source_id VARCHAR(100) NOT NULL,
    sender_id VARCHAR(100),
    title TEXT,                              -- Optional title for the checklist
    group_name TEXT,                          -- Group/chat name from LINE API
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_checklists_source ON checklists(source_type, source_id, created_at DESC);
