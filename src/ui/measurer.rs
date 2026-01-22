use bitflags::bitflags;
use egui::{Id, InnerResponse, Rect, Ui, Vec2};

pub struct Measurer {
    id: Id,
}

bitflags! {
    #[derive(Copy, Clone, Default)]
    struct UseFlags: u8 {
        const WIDTH = 1 << 0;
        const HEIGHT = 1 << 1;
        const POSITION = 1 << 2;

        const SIZE = UseFlags::WIDTH.bits() | UseFlags::HEIGHT.bits();
        const RECT = UseFlags::POSITION.bits() | UseFlags::SIZE.bits();
    }
}

#[derive(Copy, Clone, Default)]
struct MeasurerState {
    previous_rect: Option<Rect>,
    used: UseFlags,
}

fn rect_changed(a: Rect, b: Rect, consider: UseFlags) -> bool {
    (consider.contains(UseFlags::WIDTH) && a.width() != b.width())
        || (consider.contains(UseFlags::HEIGHT) && a.height() != b.height())
        || (consider.contains(UseFlags::POSITION) && a.min != b.min)
}

impl Measurer {
    pub fn new(ui: &mut Ui) -> Self {
        let id = ui.next_auto_id();
        ui.skip_ahead_auto_ids(1);

        Measurer { id }
    }

    fn query(&self, ui: &mut Ui, used_bits: UseFlags) -> Option<Rect> {
        if let Some(r) = ui.data_mut(|data| {
            let s = data.get_temp_mut_or_default::<MeasurerState>(self.id);
            s.used |= used_bits;
            s.previous_rect
        }) {
            Some(r)
        } else {
            ui.ctx().request_discard("Empty Measurer used");
            None
        }
    }

    #[expect(unused)]
    pub fn query_width(&self, ui: &mut Ui) -> Option<f32> {
        self.query(ui, UseFlags::WIDTH).map(|r| r.width())
    }

    pub fn query_height(&self, ui: &mut Ui) -> Option<f32> {
        self.query(ui, UseFlags::HEIGHT).map(|r| r.height())
    }

    #[expect(unused)]
    pub fn query_size(&self, ui: &mut Ui) -> Option<Vec2> {
        self.query(ui, UseFlags::SIZE).map(|r| r.size())
    }

    #[expect(unused)]
    pub fn query_rect(&self, ui: &mut Ui) -> Option<Rect> {
        self.query(ui, UseFlags::RECT)
    }

    pub fn measure<R>(
        &self,
        ui: &mut Ui,
        add_content: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        let inner_r = ui.scope(add_content);
        let new_rect = inner_r.response.rect;

        let invalidated = ui.data_mut(|data| {
            let s = data.get_temp_mut_or_default::<MeasurerState>(self.id);
            let previous = s.previous_rect.replace(new_rect);

            !s.used.is_empty() && previous.is_none_or(|prev| rect_changed(new_rect, prev, s.used))
        });
        if invalidated {
            ui.ctx().request_discard("Used Measurer changed");
        }

        inner_r
    }
}
