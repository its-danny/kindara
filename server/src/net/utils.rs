#[macro_export]
macro_rules! command_messages {
    ($inbox:expr) => {
        $inbox.iter().filter_map(|m| {
            if let Message::Command(content) = &m.content {
                Some((m, content))
            } else {
                None
            }
        })
    };
}

#[macro_export]
macro_rules! text_messages {
    ($inbox:expr) => {
        $inbox.iter().filter_map(|m| {
            if let Message::Text(content) = &m.content {
                Some((m, content))
            } else {
                None
            }
        })
    };
}
