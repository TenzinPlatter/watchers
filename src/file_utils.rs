use notify::{Event, EventKind};

use std::path::Path;

pub fn was_modification(event: &Event) -> bool {
    matches!(
        event.kind,
        EventKind::Remove(_) | EventKind::Create(_) | EventKind::Modify(_)
    )
}
