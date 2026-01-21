use egui::{Direction, Id, InnerResponse, Ui};

enum BoxMunch {
    Split(bool, bool),
    Either,
}

#[must_use]
pub struct BoxItemBuilder {
    stretch: f32,
    munch: BoxMunch,
    min_size: f32,
    max_size: f32,
}

impl BoxItemBuilder {
    fn new() -> Self {
        Self {
            stretch: 0.0,
            munch: BoxMunch::Split(false, false),
            min_size: 0.0,
            max_size: f32::INFINITY,
        }
    }
    pub fn stretch(mut self, stretch: f32) -> Self {
        assert!(stretch > 0.0, "stretch ({stretch}) must be positive");
        self.stretch = stretch;
        self
    }

    pub fn munch_before(mut self) -> Self {
        self.munch = match self.munch {
            BoxMunch::Split(_, after) => BoxMunch::Split(true, after),
            BoxMunch::Either => BoxMunch::Split(true, false),
        };
        self
    }

    pub fn munch_after(mut self) -> Self {
        self.munch = match self.munch {
            BoxMunch::Split(before, _) => BoxMunch::Split(before, true),
            BoxMunch::Either => BoxMunch::Split(false, true),
        };
        self
    }

    pub fn munch_either(mut self) -> Self {
        self.munch = BoxMunch::Either;
        self
    }

    pub fn min_size(mut self, size: f32) -> Self {
        assert!(
            size <= self.max_size,
            "min size ({size}) must be <= max size ({})",
            self.max_size
        );
        self.min_size = size;
        self
    }

    pub fn max_size(mut self, size: f32) -> Self {
        assert!(
            size >= self.min_size,
            "max size ({size}) must be >= max size ({})",
            self.min_size
        );
        self.max_size = size;
        self
    }

    pub fn size(mut self, size: f32) -> Self {
        self.min_size = size;
        self.max_size = size;
        self
    }
}

pub struct BoxLayout<'ui> {
    ui: &'ui Ui,
}

impl<'ui> BoxLayout<'ui> {
    pub fn builder(direction: Direction) -> BoxLayoutBuilder {
        BoxLayoutBuilder {
            direction,
            id_salt: None,

            width_percent: None,
            height_percent: None,
        }
    }

    pub fn horizontal() -> BoxLayoutBuilder {
        Self::builder(Direction::LeftToRight)
    }
    pub fn vertical() -> BoxLayoutBuilder {
        Self::builder(Direction::TopDown)
    }
}

impl<'ui> BoxLayout<'ui> {
    pub fn ui(&self) -> &Ui {
        self.ui
    }
}

impl<'ui> BoxLayout<'ui> {
    pub fn stretch(&mut self, stretch: f32) {
        self.add_empty(0.0, Self::item().stretch(stretch).munch_either())
    }

    pub fn spacing(&mut self, length: f32) {
        self.add_empty(length, Self::item().munch_before().munch_after())
    }

    pub fn item() -> BoxItemBuilder {
        BoxItemBuilder::new()
    }

    pub fn add_ui<R>(
        &mut self,
        item: BoxItemBuilder,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        self.add_ui_dyn(item, &add_contents)
    }

    fn add_ui_dyn<'c, R>(
        &mut self,
        item: BoxItemBuilder,
        add_contents: &(impl FnOnce(&mut Ui) -> R + 'c),
    ) -> InnerResponse<R> {
        unimplemented!()
    }

    pub fn add_empty(&mut self, length: f32, item: BoxItemBuilder) {
        unimplemented!()
    }
}

struct LayoutInfo {
    
}

#[must_use]
pub struct BoxLayoutBuilder {
    direction: Direction,
    id_salt: Option<Id>,

    width_percent: Option<f32>,
    height_percent: Option<f32>,
}

impl BoxLayoutBuilder {
    pub fn direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
    }

    pub fn id_salt(mut self, id_salt: impl Into<Id>) -> Self {
        self.id_salt = Some(id_salt.into());
        self
    }

    pub fn w_full(mut self) -> Self {
        self.width_percent = Some(1.0);
        self
    }

    pub fn h_full(mut self) -> Self {
        self.height_percent = Some(1.0);
        self
    }

    pub fn show<R>(
        self,
        ui: &mut Ui,
        add_contents: impl FnOnce(&mut BoxLayout) -> R,
    ) -> InnerResponse<R> {
        self.show_dyn(ui, &add_contents)
    }

    fn show_dyn<'c, R>(
        self,
        ui: &mut Ui,
        add_contents: &(dyn FnOnce(&mut BoxLayout) -> R + 'c),
    ) -> InnerResponse<R> {
        unimplemented!()
    }
}
