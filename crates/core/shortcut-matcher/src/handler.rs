use crate::types::ShortcutAction;

pub trait ShortcutHandler: Send + Sync {
    fn handle_action(&self, action: ShortcutAction);
    fn handle_error(&self, error: String);
}
