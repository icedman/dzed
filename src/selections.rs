use text::{Anchor, AnchorRangeExt, Buffer, Selection, SelectionGoal};

pub struct SelectionCollection {
    pub id: usize,
    pub selections: Vec<Selection<Anchor>>,
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

    // pub fn render_line(&self, line: usize) -> Option<&StyleCache> {
    //     self.style_cache.get(&line)
    // }
}
