use crate::gfx::GridModel;
use crate::project::ProjectData;
use crate::room::{
    BLOCK_SIZE_PX, BlockScreenLayer, LevelBlock, Room, RoomRef, RoomState, SCREEN_SIZE_BLOCKS,
    SCREEN_SIZE_PX, ScrollType,
};
use crate::tileset;
use crate::tileset::{LoadedTilesetLayout, Tileset};
use crate::ui::tile_view;
use crate::ui::views::EditorWindow;
use egui::{
    Align2, Color32, FontId, Id, Rect, Sense, Stroke, StrokeKind, Ui, UiBuilder, Vec2, vec2,
};
use std::any::Any;

const ID_SALT: &str = concat!(module_path!(), "::RoomEditor");

pub struct RoomEditor {
    room: RoomRef,
    selected_state: usize,
    zoom_level: i32,

    layers: Vec<LayerInstance>,
}

trait EditorLayer: Any {
    fn show_ui(
        &mut self,
        room: &Room,
        state: &RoomState,
        tileset_layout: Option<&LoadedTilesetLayout<&Tileset>>,
        ui: &mut Ui,
        zoom: f32,
    );
}

struct BackdropLayer;
impl EditorLayer for BackdropLayer {
    fn show_ui(
        &mut self,
        room: &Room,
        _state: &RoomState,
        tileset_layout: Option<&LoadedTilesetLayout<&Tileset>>,
        ui: &mut Ui,
        zoom: f32,
    ) {
        if let Some(tileset_layout) = tileset_layout {
            let palette = &tileset_layout.palette_source.palette;
            if let Some(color) = palette.0.first() {
                let color = Color32::from(*color);

                let room_size =
                    vec2(room.width as f32, room.height as f32) * (SCREEN_SIZE_PX as f32 * zoom);
                let (rect, _response) = ui.allocate_exact_size(room_size, Sense::empty());
                ui.painter().rect_filled(rect, 0.0, color);
            }
        }
    }
}

struct LevelLayerModel<'a> {
    grid: &'a BlockScreenLayer,
}

impl GridModel for LevelLayerModel<'_> {
    type Item = LevelBlock;

    fn dimensions(&self) -> [usize; 2] {
        self.grid
            .blocks
            .dimensions()
            .map(|d| d * SCREEN_SIZE_BLOCKS)
    }

    fn get(&self, x: usize, y: usize) -> Option<Self::Item> {
        let screen_x = x / SCREEN_SIZE_BLOCKS;
        let screen_y = y / SCREEN_SIZE_BLOCKS;
        let tile_x = x % SCREEN_SIZE_BLOCKS;
        let tile_y = y % SCREEN_SIZE_BLOCKS;

        let screen = self.grid.blocks.get_screen(screen_x, screen_y)?;
        let tile = screen.get(tile_y * SCREEN_SIZE_BLOCKS + tile_x)?;
        Some(*tile)
    }
}

struct Layer2Layer;
impl EditorLayer for Layer2Layer {
    fn show_ui(
        &mut self,
        _room: &Room,
        state: &RoomState,
        tileset_layout: Option<&LoadedTilesetLayout<&Tileset>>,
        ui: &mut Ui,
        zoom: f32,
    ) {
        if let Some(tileset_layout) = tileset_layout
            && let Some(layer2) = &state.layer2
        {
            let model = LevelLayerModel { grid: layer2 };
            tile_view::draw_tiletable_grid(ui, tileset_layout, model, zoom, true);
        }
    }
}

struct Layer1Layer;
impl EditorLayer for Layer1Layer {
    fn show_ui(
        &mut self,
        _room: &Room,
        state: &RoomState,
        tileset_layout: Option<&LoadedTilesetLayout<&Tileset>>,
        ui: &mut Ui,
        zoom: f32,
    ) {
        if let Some(tileset_layout) = tileset_layout {
            let model = LevelLayerModel {
                grid: &state.layer1,
            };
            tile_view::draw_tiletable_grid(ui, tileset_layout, model, zoom, true);
        }
    }
}

struct CollisionLayer;
impl EditorLayer for CollisionLayer {
    #[expect(unused)]
    fn show_ui(
        &mut self,
        room: &Room,
        state: &RoomState,
        tileset_layout: Option<&LoadedTilesetLayout<&Tileset>>,
        ui: &mut Ui,
        zoom: f32,
    ) {
        todo!()
    }
}

struct ScrollsLayer;
impl EditorLayer for ScrollsLayer {
    fn show_ui(
        &mut self,
        room: &Room,
        state: &RoomState,
        _tileset_layout: Option<&LoadedTilesetLayout<&Tileset>>,
        ui: &mut Ui,
        zoom: f32,
    ) {
        let scrolls = &state.scrolls.scrolls;

        tile_view::draw_each_screen(
            [room.width, room.height].map(usize::from),
            SCREEN_SIZE_PX as f32 * zoom,
            ui,
            |ui, [screen_x, screen_y], rect| {
                let Some(&scroll_value) = scrolls.get_screen(screen_x, screen_y) else {
                    return;
                };
                let scroll_type = ScrollType::try_from(scroll_value);
                let stroke_color = match scroll_type {
                    Ok(ScrollType::Red) => Color32::RED,
                    Ok(ScrollType::Green) => Color32::GREEN,
                    Ok(ScrollType::Blue) => Color32::from_rgb(0, 128, 255),
                    Err(()) => Color32::YELLOW,
                };
                let stroke_rect = rect.expand(-4.0);
                ui.painter()
                    .rect_stroke(stroke_rect, 0.0, (1.5, stroke_color), StrokeKind::Middle);
                if scroll_type.is_err() {
                    ui.painter().text(
                        stroke_rect.expand(-2.0).min,
                        Align2::LEFT_TOP,
                        format!("{scroll_value:#02X}"),
                        FontId::monospace(12.0),
                        stroke_color,
                    );
                }
            },
        );
    }
}

struct GridLayer;
impl EditorLayer for GridLayer {
    fn show_ui(
        &mut self,
        room: &Room,
        _state: &RoomState,
        _tileset_layout: Option<&LoadedTilesetLayout<&Tileset>>,
        ui: &mut Ui,
        zoom: f32,
    ) {
        const MINOR_GRID_STROKE: Stroke = Stroke {
            width: 1.0,
            color: Color32::GRAY,
        };
        const MAJOR_GRID_STROKE: Stroke = Stroke {
            width: 1.5,
            color: Color32::GRAY,
        };

        let block_size = BLOCK_SIZE_PX as f32 * zoom;
        tile_view::draw_each_screen(
            [room.width, room.height].map(usize::from),
            SCREEN_SIZE_PX as f32 * zoom,
            ui,
            |ui, [screen_x, screen_y], rect| {
                let p = ui.painter();
                for x in 1..SCREEN_SIZE_BLOCKS {
                    let x = rect.left() + (x as f32 * block_size);
                    p.vline(x, rect.y_range(), MINOR_GRID_STROKE);
                }
                for y in 0..SCREEN_SIZE_BLOCKS {
                    let y = rect.top() + (y as f32 * block_size);
                    p.hline(rect.x_range(), y, MINOR_GRID_STROKE);
                }

                if screen_x > 0 {
                    p.vline(rect.left(), rect.y_range(), MAJOR_GRID_STROKE);
                }
                if screen_y > 0 {
                    p.hline(rect.x_range(), rect.top(), MAJOR_GRID_STROKE);
                }
            },
        );
    }
}

struct LayerInstance {
    visible: bool,
    layer: Box<dyn EditorLayer>,
}

impl RoomEditor {
    pub fn new(room: RoomRef) -> Self {
        Self {
            room,
            selected_state: 0,
            zoom_level: 0,

            layers: vec![
                LayerInstance {
                    visible: true,
                    layer: Box::new(BackdropLayer),
                },
                LayerInstance {
                    visible: true,
                    layer: Box::new(Layer2Layer),
                },
                LayerInstance {
                    visible: true,
                    layer: Box::new(Layer1Layer),
                },
                // TODO: CollisionLayer
                LayerInstance {
                    visible: true,
                    layer: Box::new(ScrollsLayer),
                },
                LayerInstance {
                    visible: true,
                    layer: Box::new(GridLayer),
                },
            ],
        }
    }
}

impl EditorWindow for RoomEditor {
    fn title(&self, project_data: &ProjectData) -> String {
        if let Some(room) = project_data.rooms.get(self.room) {
            format!("Room: {}", &room.name)
        } else {
            format!("Room: <{:?}>", self.room)
        }
    }

    fn stable_id(&self) -> Id {
        Id::new(ID_SALT).with(self.room)
    }

    fn show_contents(&mut self, project_data: &mut ProjectData, ui: &mut Ui) {
        let Some(room) = project_data.rooms.get(self.room) else {
            ui.close();
            return;
        };

        ui.horizontal(|ui| {
            ui.label("Current RoomState:");
            egui::ComboBox::from_id_salt("current-roomstate").show_index(
                ui,
                &mut self.selected_state,
                room.states.len(),
                |i| format!("State {i}"),
            );

            ui.separator();
            ui.label("Zoom: ");
            ui.add(egui::Slider::new(&mut self.zoom_level, -2..=4).show_value(false));
        });
        let state = &room.states[self.selected_state];

        let tileset = project_data
            .tileset_ids
            .get(&state.tileset_index)
            .and_then(|r| project_data.tilesets.get(*r));
        let tileset_cre_index = 0;
        let tileset_cre = project_data
            .cre_tileset_ids
            .get(&tileset_cre_index)
            .and_then(|r| project_data.tilesets.get(*r));
        let tileset_layout = tileset.map(|t| tileset::detect_sources_layout(t, tileset_cre));

        egui::ScrollArea::both().show(ui, |ui| {
            let pixel_size = ui.pixels_per_point().recip();
            let zoom = (self.zoom_level as f32).exp2() * pixel_size;

            let desired_size = vec2(f32::from(room.width), f32::from(room.height))
                * (SCREEN_SIZE_PX as f32 * zoom);

            let padding = Vec2::splat(SCREEN_SIZE_PX as f32);
            let (_id, rect) = ui.allocate_space(desired_size + padding);

            let room_rect = Rect::from_center_size(rect.center(), desired_size);
            for layer in &mut self.layers {
                if !layer.visible {
                    continue;
                }
                ui.scope_builder(UiBuilder::new().max_rect(room_rect), |ui| {
                    layer
                        .layer
                        .show_ui(room, state, tileset_layout.as_ref(), ui, zoom)
                });
            }
        });
    }
}
