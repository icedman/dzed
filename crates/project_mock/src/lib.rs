pub struct InlayHint {
    pub label: InlayHintLabel,
    pub resolve_state: ResolveState,
    pub padding_left: bool,
    pub padding_right: bool,
    pub tooltip: Option<String>,
}

impl InlayHint {
    pub fn text(&self) -> String {
        match &self.label {
            InlayHintLabel::String(s) => s.clone(),
        }
    }
}

pub enum InlayHintLabel {
    String(String),
}

pub enum ResolveState {
    Resolved,
    Unresolved,
}
