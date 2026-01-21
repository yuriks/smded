use std::convert::TryInto;

impl eframe::App for Application {
    fn update(&mut self, ctx: &Context, frame: &mut Frame) {
        ctx.options_mut(|opt| opt.max_passes = 2.try_into().unwrap());

        // egui::CentralPanel::default().show(ctx, |ui| {
        egui::Window::new("Initial Setup")
            .default_size([900.0, 300.0])
            .show(ctx, |ui| {
                //ctx.debug_text(format!("Size: {:?}", ui.available_size()));
                ui.heading("Hello World!");
                ui.button("Above");
                BoxLayout::horizontal().w_full().show(ui, |layout| {
                    for i in 0..=8 {
                        if i == 0 {
                            layout.stretch(2.0);
                        } else if i == 4 {
                            layout.stretch(1.0);
                        } else if i == 6 {
                            layout.spacing(2.0 * layout.ui().spacing().item_spacing.x)
                        }
                        layout.add_ui(
                            BoxLayout::item().chain_if(i == 4, |l| l.stretch(1.0)),
                            |ui| {
                                let _ = ui.button(format!("Button {i}"));
                            },
                        );
                    }
                });
                ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
                    ui.button("Below");
                });

                ui.allocate_space(ui.available_size());

                /*if let Some(res) = self.picked_path_new.take_response()
                    && let Some(p) = res
                {
                    self.picked_path = Some(p.path().to_owned());
                }

                if let Some(path) = &self.picked_path {
                    let s = format!("Picked file: {}", path.display());
                    ui.label(s);
                } else {
                    ui.label("No ROM loaded.");
                }
                if ui
                    .add_enabled(
                        !self.picked_path_new.is_pending(),
                        Button::new("Load Super Metroid ROM..."),
                    )
                    .clicked()
                {
                    self.picked_path_new
                        .launch(Box::pin(rfd::AsyncFileDialog::new().set_parent(frame).pick_folder()));
                }

                ui.label("Recent projects:");*/

                // let show_table = |ui: &mut egui::Ui| {
                //     TableBuilder::new(ui)
                //         .auto_shrink(true)
                //         .min_scrolled_height(40.0)
                //         .max_scroll_height(f32::INFINITY)
                //         .striped(true)
                //         .column(Column::remainder())
                //         .body(|body| {
                //             body.rows(18.0, 15, |mut row| {
                //                 let row_index = row.index();
                //                 row.col(|ui| {
                //                     ui.label(format!("Room {}", row_index + 1));
                //                 });
                //             });
                //         });
                // };

                //show_table(ui);

                // ui.separator();
                // ui.button("Above");
                // // stretch_contents(ui, 1.0, |ui| {
                // //     ui.with_layout(Layout::left_to_right(Align::Center).with_cross_justify(true), |ui| {
                // //         ui.button("Biggg stretch!");
                // //         ui.label("Damn this is great");
                // //     });
                // // });
                // // ui.horizontal_wrapped(|ui| {
                // Flex::horizontal().w_full().show(ui, |flex| {
                //     for i in 0..=8 {
                //         if i == 0 {
                //             flex.add_ui(item().grow(2.0), |_| {});
                //             // stretch_with_weight(ui, 2.0);
                //         }
                //         if i == 4 {
                //             flex.grow();
                //             // stretch(ui);
                //             // stretch_contents(ui, 1.0, |ui| {
                //             //     let _ = ui.button("XYZ Button XYZ");
                //             // });
                //         }
                //         if i == 6 {
                //             flex.grow();
                //             // stretch(ui);
                //             // stretch_contents(ui, 1.0, |ui| {
                //             //     let _ = ui.button("X Button");
                //             // });
                //         }
                //         //ui.add_sized([90.0, 30.0], Button::new(format!("Button {} ", i)));
                //         flex.add(item(), Button::new(format!("Button {i}")));
                //     }
                //     flex.add_ui(item().grow(2.0), |_| {});
                // });
                // ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
                //     ui.button("Below");
                // });
                // ui.with_layout(Layout::left_to_right(Align::TOP), |ui| {
                //     if ui.button("OK").clicked() {}
                //     if ui.button("Exit").clicked() {}
                // });
                // });

                //flex.grow();
                //flex.add_widget(item(), Separator::default());
                // flex.add_ui(item(), |ui| {
                //     ui.separator();
                //     ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                //         if ui.button("OK").clicked() {}
                //         if ui.button("Exit").clicked() {}
                //     });
                // });

                // egui::TopBottomPanel::bottom("button_strip")
                //     .resizable(false)
                //     .show_separator_line(false)
                //     .min_height(0.0)
                //     .show_inside(ui, |ui| {
                //         ui.separator();
                //         ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                //             if ui.button("OK").clicked() {}
                //             if ui.button("Exit").clicked() {}
                //         });
                //     });
                //
                // egui::CentralPanel::default().show_inside(ui, |ui| {
                //
                //     // ui.button("What is happening");
                //
                // });

                //ui.allocate_space(ui.available_size())
            });
    }
}