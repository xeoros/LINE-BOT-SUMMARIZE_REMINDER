pub fn get_summary_prompt(conversation: &str) -> String {
    format!(
        r#"คุณคือผู้ช่วยสรุปการสนทนา โปรดสรุปการสนทนาบน LINE ดังต่อไปนี้เป็นภาษาไทย

เน้นไปที่: การตัดสินใจสำคัญ รายการที่ต้องดำเนินการ (action items) และใครพูดอะไร

การสนทนา:
{}

โปรดสรุปเป็น Markdown โดยใช้หัวข้อและ bullet list ตามรูปแบบด้านบนี้:

# สรุปย่อ
- ...

## การตัดสินใจสำคัญ
- ...

## รายการที่ต้องดำเนินการ
- [ ] ...

## ใครพูดอะไร
- ชื่อ/ผู้ใช้: ...

ถ้าไม่มีข้อมูลในหัวข้อใด ให้เขียน - ไม่มี
และตอบกลับเฉพาะ Markdown ตามรูปแบบด้านบน"#,
        conversation
    )
}

pub fn get_summary_prompt_english(conversation: &str) -> String {
    format!(
        r#"You are a conversation summary assistant. Please summarize the following LINE conversation.
Highlight: key decisions, action items, and who said what.

Conversation:
{}

Return summary in Markdown with headings and bullet lists in this format:

# Summary
- ...

## Key Decisions
- ...

## Action Items
- [ ] ...

## Who Said What
- Name/User: ...

If a section has no content, write - None.
Return only the Markdown in the format above."#,
        conversation
    )
}

pub fn get_thread_summary_prompt(thread_conversation: &str) -> String {
    format!(
        r#"คุณคือผู้ช่วยสรุปการสนทนา โปรดสรุปการสนทนาบน Slack ดังต่อไปนี้เป็นภาษาไทย

เน้นไปที่: การตัดสินใจสำคัญ รายการที่ต้องดำเนินการ (action items) และใครพูดอะไร
และใช้ข้อมูลเกี่ยว้เกี่ยว้ในการสนทนาแบบ thread/reply ตามลำดับ

การสนทนาแบบ Thread (มีการอ้างอิกถังถังถัง):
{}

โปรดสรุปเป็น Markdown โดยใช้หัวข้อและ bullet list ตามรูปแบบด้านบนี้:

# สรุปย่อการสนทนาแบบ Thread
- หัวข้อประเดียนของการสนทนานี้

## การตัดสินใจสำคัญหลักหลัก
- การตัดสินใจสำคัญหลักหลักที่ทำขึ้นใน thread

## รายการที่ต้องดำเนินการแต่ละหลัก
- [ ] รายการที่แต่ละหลักต้องดำเนินการ

## การอ้างอิกถังถังถัง
- บริบาณการที่มีการอ้างอิกถังถังถัง

## ใครพูดอะไรใน thread
- ชื่อ/ผู้ใช้: ข้อความ (หลักหลักต้องทราบ)
  - ชื่อ/ผู้ใช้: ตอบกลับ #1 (อ้างอิกถังถังถัง)
    - ชื่อ/ผู้ใช้: ความตอบ #2 (อ้างอิกถังถังถังถัง)

ถ้าไม่มีข้อมูลในหัวข้อใด ให้เขียน - ไม่มี
และตอบกลับเฉพาะ Markdown ตามรูปแบบด้านบน"#,
        thread_conversation
    )
}

pub fn get_thread_summary_prompt_english(thread_conversation: &str) -> String {
    format!(
        r#"You are a conversation summary assistant. Please summarize the following Slack thread conversation.
Highlight: key decisions, action items, reply relationships, and who said what.
Include thread structure showing reply hierarchy with proper indentation.

Thread Conversation:
{}

Return summary in Markdown with headings and bullet lists in this format:

# Thread Summary
- Main topic or decision made

## Key Decisions
- ...

## Action Items
- [ ] ...

## Reply Structure
- Main message by [User] at [Time]
  - Reply by [User] at [Time] (indented)
    - Reply by [User] at [Time] (further indented)

## Who Said What
- Name/User: ... (with reply relationships marked)

If a section has no content, write - None.
Return only the Markdown in format above."#,
        thread_conversation
    )
}

pub fn get_title_prompt(tasks: &[String]) -> String {
    let tasks_text = tasks
        .iter()
        .enumerate()
        .map(|(i, task)| format!("{}. {}", i + 1, task))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"คุณคือผู้ช่วยสร้างชื่อเรื่อง โปรดสร้างชื่อเรื่องที่สั้นกระชับ (ไม่เกิน 20 คำ) สำหรับรายการงานต่อไปนี้เป็นภาษาไทย

รายการงาน:
{}

ตอบกลับเฉพาะชื่อเรื่องเดียว โดยไม่ต้องมีข้อความเพิ่มเติมอื่นๆ"#,
        tasks_text
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_prompt_includes_conversation() {
        let prompt = get_summary_prompt("hello world");
        assert!(prompt.contains("hello world"));
        assert!(prompt.contains("# สรุปย่อ"));
        assert!(prompt.contains("## รายการที่ต้องดำเนินการ"));
    }

    #[test]
    fn summary_prompt_english_includes_conversation() {
        let prompt = get_summary_prompt_english("hello world");
        assert!(prompt.contains("hello world"));
        assert!(prompt.contains("# Summary"));
        assert!(prompt.contains("## Action Items"));
    }

    #[test]
    fn thread_prompt_includes_thread_text() {
        let prompt = get_thread_summary_prompt("thread content");
        assert!(prompt.contains("thread content"));
        assert!(prompt.contains("# สรุปย่อการสนทนาแบบ Thread"));
    }

    #[test]
    fn thread_prompt_english_includes_thread_text() {
        let prompt = get_thread_summary_prompt_english("thread content");
        assert!(prompt.contains("thread content"));
        assert!(prompt.contains("# Thread Summary"));
    }

    #[test]
    fn title_prompt_numbers_tasks() {
        let prompt = get_title_prompt(&vec!["Task A".to_string(), "Task B".to_string()]);
        assert!(prompt.contains("1. Task A"));
        assert!(prompt.contains("2. Task B"));
    }
}
