use text::AnchorRangeExt;
use text::{Anchor, Buffer, Selection, SelectionGoal};

pub struct SelectionCollection {
    pub selections: Vec<Selection<Anchor>>,
    id: usize,
}

impl SelectionCollection {
    pub fn new() -> Self {
        return SelectionCollection {
            selections: Vec::<Selection<Anchor>>::new(),
            id: 0,
        };
    }

    pub fn first(&self) -> Option<&Selection<Anchor>> {
        self.selections.first()
    }

    pub fn last(&self) -> Option<&Selection<Anchor>> {
        self.selections.last()
    }

    pub fn add(&mut self, buffer: &Buffer, offset: usize) -> Selection<Anchor> {
        let anchor = Selection {
            id: self.id,
            start: Anchor::MIN,
            end: Anchor::MIN,
            reversed: false,
            goal: SelectionGoal::None,
        };
        self.selections.push(anchor.clone());
        self.id += 1;
        anchor
    }

    pub fn update(&mut self, selection: &Selection<Anchor>) {
        if let Some(selected) = self.selections.iter_mut().find(|s| s.id == selection.id) {
            *selected = selection.clone();
        }
    }

    pub fn clear(&mut self) {
        self.selections.clear();
    }
}
