use notify::{Event, EventKind};

pub fn was_modification(event: Event) -> bool {
    matches!(event.kind, EventKind::Remove(_) | EventKind::Create(_) | EventKind::Modify(_))
}
