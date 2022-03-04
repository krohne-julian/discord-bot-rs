use lazy_static::lazy_static;
use regex::Regex;
use serenity::model::channel::{Message, MessageType};

lazy_static! {
    static ref RE: Regex = Regex::new(r#"(http(s)?://)[-a-zA-Z0-9@:%._+~#=]+\.[a-z]+\b"#).unwrap();
}

pub fn is_feedback_request(msg: &Message) -> bool {
    let link_result = RE.is_match(&msg.content);
    let file_result = msg
        .attachments
        .iter()
        .any(|attachment| attachment.url.ends_with(".mp3") || attachment.url.ends_with(".wav"));

    link_result || file_result
}

pub fn is_feedback_reply(msg: &Message, min_len: usize) -> bool {
    if msg.kind == MessageType::InlineReply && msg.content.len() > min_len {
        if let Some(ref_msg) = msg.referenced_message.as_ref() {
            return is_feedback_request(&ref_msg) && ref_msg.author != msg.author;
        }
    }
    false
}

#[cfg(test)]
mod test {
    #[test]
    fn regex() {
        use super::RE;

        assert!(RE.is_match("https://www.youtube.com/watch?v=dQw4w9WgXcQ"));
    }
}
