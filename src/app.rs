use crate::app_config::AppConfig;
use crate::file::*;
use crate::find::FindTools;
use eframe::egui;
use eframe::egui::Color32;
use eframe::epi;

#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))]
pub struct CodeShare {
    config: AppConfig,

    #[cfg_attr(feature = "persistence", serde(skip))]
    file_status: FileStatus,
    text_buf: String,
    finder: FindTools,
    active_popup: Popup,
    err_msg: Option<String>,
    status_msg: Option<String>,
    switch_to_editor: bool,
}

impl Default for CodeShare {
    fn default() -> Self {
        Self {
            config: AppConfig::default(),
            file_status: FileStatus::default(),
            text_buf: String::new(),
            finder: FindTools::default(),
            active_popup: Popup::None,
            err_msg: None,
            status_msg: Some("code_share loaded".to_string()),
            switch_to_editor: false,
        }
    }
}

impl epi::App for CodeShare {
    fn name(&self) -> &str {
        "code_share"
    }

    /// Called once before the first frame.
    fn setup(
        &mut self,
        ctx: &egui::CtxRef,
        _frame: &mut epi::Frame<'_>,
        _storage: Option<&dyn epi::Storage>,
    ) {
        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        #[cfg(feature = "persistence")]
        if let Some(storage) = _storage {
            *self = epi::get_value(storage, epi::APP_KEY).unwrap_or_default()
        }
        CodeShare::change_app_font_size(ctx, self.config.get_font_size());
        self.text_buf.clear();
        self.status_msg = Some("code_share loaded".to_string());
        self.err_msg = None;
        self.active_popup = Popup::None;
        self.file_status.set_unsaved(false);
        self.finder.full_reset();

        //  Disable text wrapping
        let mut style = (*ctx.style()).clone();
        style.wrap = Some(false);
        ctx.set_style(style);
    }

    /// Called by the frame work to save state before shutdown.
    /// Note that you must enable the `persistence` feature for this to work.
    #[cfg(feature = "persistence")]
    fn save(&mut self, storage: &mut dyn epi::Storage) {
        epi::set_value(storage, epi::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::CtxRef, frame: &mut epi::Frame<'_>) {
        let Self {
            config,
            file_status,
            text_buf,
            finder,
            active_popup,
            err_msg,
            status_msg,
            switch_to_editor,
        } = self;

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                egui::menu::menu_button(ui, "File", |ui| {
                    if ui.button("New").clicked() {
                        match file_status.is_unsaved() {
                            false => {
                                text_buf.clear();
                                file_status.reset();
                                *status_msg = Some("New File Opened".to_string());
                            }
                            true => {
                                *active_popup = Popup::FileNotSavedNew;
                            }
                        }
                    }
                    if ui.button("Open").clicked() {
                        match file_status.is_unsaved() {
                            false => {
                                *active_popup = Popup::OpenFile;
                            }
                            true => {
                                *active_popup = Popup::FileNotSavedOpen;
                            }
                        }
                    }
                    if ui.button("Save").clicked() {
                        *active_popup = Popup::SaveFile;
                    }
                    if ui.button("Save As").clicked() {
                        *active_popup = Popup::SaveAs
                    }
                    if ui.button("Quit").clicked() {
                        frame.quit();
                    }
                });
                egui::menu::menu_button(ui, "Edit", |ui| {
                    ui.label("to be built");
                });
                egui::menu::menu_button(ui, "View", |ui| {
                    ui.checkbox(&mut config.line_nums, "Line Numbers");
                    ui.horizontal(|ui| {
                        if ui.button(" - ").clicked() {
                            config.dec_font_size();
                            CodeShare::change_app_font_size(ctx, config.get_font_size());
                        }
                        let font_size_str = format!("Font Size: {}", config.get_font_size());
                        ui.label(font_size_str);
                        if ui.button(" + ").clicked() {
                            config.inc_font_size();
                            CodeShare::change_app_font_size(ctx, config.get_font_size());
                        }
                    });
                });
                egui::menu::menu_button(ui, "Tools", |ui| {
                    if ui.button("Find").clicked() {
                        *active_popup = Popup::Find;
                    }
                    if ui.button("Find and Replace").clicked() {
                        *active_popup = Popup::FindAndReplace;
                    }
                });
            });
        });

        //  Save File "popup"
        if *active_popup == Popup::SaveFile {
            match file_status.is_new() {
                true => *active_popup = Popup::SaveAs,
                false => {
                    match file_status.save_file(text_buf) {
                        Ok(_) => {
                            *active_popup = Popup::None;
                            *status_msg = Some("Save Successful".to_string());
                        }
                        Err(e) => {
                            let error = format!("Save failed: {}", e);
                            *err_msg = Some(error);
                            *active_popup = Popup::Error;
                            *status_msg = Some("Save Failed".to_string())
                        }
                    };
                }
            };
        }
        //  Open file popup
        if *active_popup == Popup::OpenFile {
            match file_status.open_file() {
                Ok(Some(contents)) => {
                    *text_buf = contents;
                    *active_popup = Popup::None;
                    *status_msg = Some("Open Successful".to_string());
                }
                Ok(None) => {
                    *active_popup = Popup::None;
                    *status_msg = Some("Open Cancelled".to_string());
                }
                Err(e) => {
                    *err_msg = Some(e.to_string());
                    *active_popup = Popup::Error;
                }
            };
        }
        //   Save as popup
        if *active_popup == Popup::SaveAs {
            match file_status.save_file_as(text_buf) {
                Ok(Some(_)) => {
                    *active_popup = Popup::None;
                    *status_msg = Some("Save Successful".to_string());
                }
                Ok(None) => {
                    *active_popup = Popup::None;
                    *status_msg = Some("Save Cancelled".to_string());
                }
                Err(e) => {
                    *err_msg = Some(e.to_string());
                    *active_popup = Popup::Error;
                }
            };
        }
        //  File not saved popup (creating new file)
        if *active_popup == Popup::FileNotSavedNew {
            egui::Window::new("File Not Saved")
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.label("Current file has not been saved");
                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked() {
                            *active_popup = Popup::SaveFile;
                        }
                        if ui.button("Save As").clicked() {
                            *active_popup = Popup::SaveAs;
                        }
                        if ui.button("Continue without saving").clicked() {
                            text_buf.clear();
                            file_status.reset();
                            *active_popup = Popup::None;
                            *status_msg = Some("Cont. w/o saving".to_string());
                        }
                    });
                });
        }
        //  File not saved popup (opening different file)
        if *active_popup == Popup::FileNotSavedOpen {
            egui::Window::new("File Not Saved")
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.label("Current file has not been saved");
                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked() {
                            *active_popup = Popup::SaveFile;
                        }
                        if ui.button("Save As").clicked() {
                            *active_popup = Popup::SaveAs;
                        }
                        if ui.button("Continue without saving").clicked() {
                            *active_popup = Popup::OpenFile;
                            *status_msg = Some("Cont. w/o saving".to_string());
                        }
                    });
                });
        }
        //  Find Popup
        if *active_popup == Popup::Find {
            egui::Window::new("Find")
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        let search_box = ui.add(
                            egui::widgets::TextEdit::singleline(&mut finder.query_buf)
                                .hint_text("Find"),
                        );
                        if search_box.changed() && &finder.get_query() != "" {
                            let query = finder.get_query();
                            finder.reset_matches();
                            for (loc, _str) in text_buf.match_indices(&query) {
                                finder.add_match(loc);
                            }
                            CodeShare::highlight_text_no_switch(ctx, finder);
                        } else if &finder.get_query() == "" {
                            finder.reset_matches();
                        }
                        let info_str = format!("{} maches found", finder.number_of_matches());
                        ui.label(info_str);
                    });
                    ui.horizontal(|ui| {
                        let prev_but = ui.add(egui::widgets::Button::new("Previous"));
                        let next_but = ui.add(egui::widgets::Button::new("Next"));
                        if finder.number_of_matches() == 0 {
                            prev_but.enabled();
                            next_but.enabled();
                        }
                        if prev_but.clicked() && finder.number_of_matches() != 0 {
                            if finder.initial_click_made {
                                finder.selected_loc_dec();
                            }
                            CodeShare::highlight_text(ctx, finder, switch_to_editor);
                        }
                        if next_but.clicked() && finder.number_of_matches() != 0 {
                            if finder.initial_click_made {
                                finder.selected_loc_inc();
                            }
                            CodeShare::highlight_text(ctx, finder, switch_to_editor);
                        }
                        if ui.button("Close").clicked() {
                            finder.full_reset();
                            *active_popup = Popup::None;
                        }
                    });
                });
        }
        //  Error Popup
        if *active_popup == Popup::Error {
            egui::Window::new("Error")
                .collapsible(false)
                .show(ctx, |ui| {
                    let error = match err_msg {
                        Some(msg) => msg.clone(),
                        None => String::from("Unknown Error"),
                    };
                    ui.label(error);
                    if ui.button("Close").clicked() {
                        *active_popup = Popup::None;
                        *err_msg = None;
                    }
                });
        }

        //  Keyboard Shortcuts
        //  Save
        if ctx.input().modifiers.command == true && ctx.input().key_pressed(egui::Key::S) == true {
            *active_popup = Popup::SaveFile;
        }
        //  New
        if ctx.input().modifiers.command == true && ctx.input().key_pressed(egui::Key::N) == true {
            match file_status.is_unsaved() {
                false => {
                    text_buf.clear();
                    file_status.reset();
                    *status_msg = Some("New File Opened".to_string());
                }
                true => {
                    *active_popup = Popup::FileNotSavedNew;
                }
            }
        }
        //  Open
        if ctx.input().modifiers.command == true && ctx.input().key_pressed(egui::Key::O) == true {
            match file_status.is_unsaved() {
                false => {
                    *active_popup = Popup::OpenFile;
                }
                true => {
                    *active_popup = Popup::FileNotSavedOpen;
                }
            }
        }

        egui::TopBottomPanel::bottom("info bar")
            .frame(
                egui::Frame::none()
                    .fill(Color32::from_rgb(232, 188, 68))
                    .corner_radius(0.0),
            )
            .show(ctx, |ui| {
                let indicator = match file_status.is_unsaved() {
                    true => "*",
                    false => "",
                };
                let mut title_line = format!("{}{}", file_status.get_path_string(), indicator);
                ui.horizontal(|ui| {
                    ui.add(
                        egui::widgets::TextEdit::singleline(&mut title_line)
                            .code_editor()
                            .frame(false)
                            .interactive(false)
                            .text_color(egui::Color32::BLACK),
                    );

                    ui.with_layout(egui::Layout::right_to_left(), |ui| {
                        let mut msg_to_display = match status_msg {
                            Some(msg) => msg,
                            None => "",
                        };
                        ui.add(
                            egui::widgets::TextEdit::singleline(&mut msg_to_display)
                                .code_editor()
                                .frame(false)
                                .interactive(false)
                                .text_color(Color32::BLACK)
                                .desired_width(config.get_font_size() * 10.0),
                        );
                    });
                });
            });

        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(Color32::from_rgb(14, 15, 23))
                    .corner_radius(0.0),
            )
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let mut layouter = |ui: &egui::Ui, string: &str, _wrap_width: f32| {
                        ui.fonts().layout_no_wrap(
                            string.to_string(),
                            egui::TextStyle::Monospace,
                            Color32::WHITE,
                        )
                    };
                    ui.horizontal_top(|ui| {
                        let mut lines_str = get_line_num_str(text_buf.lines().count());
                        if config.line_nums {
                            ui.add(
                                egui::TextEdit::multiline(&mut lines_str)
                                    //.desired_width(config.get_font_size() * 2.7)
                                    .desired_width(0.0)
                                    .code_editor()
                                    .frame(false)
                                    .interactive(false)
                                    .layouter(&mut layouter)
                            );
                        }
                        ui.separator();
                        egui::ScrollArea::horizontal().show(ui, |ui| {
                            let editor = ui.add_sized(
                                ui.available_size(),
                                egui::TextEdit::multiline(text_buf)
                                    .code_editor()
                                    .lock_focus(true)
                                    .frame(false)
                                    .id(egui::Id::new("editor"))
                                    .desired_width(f32::INFINITY)
                                    .layouter(&mut layouter),
                            );
                            if editor.changed() {
                                file_status.set_unsaved(true);
                            }
                            if *switch_to_editor == true {
                                editor.request_focus();
                                *switch_to_editor = false;
                            }
                        });
                    });
                });
            });
    }
}

impl CodeShare {
    fn change_app_font_size(ctx: &egui::CtxRef, size: f32) {
        let mut fonts = egui::FontDefinitions::default();
        fonts.family_and_size.insert(
            egui::TextStyle::Monospace,
            (egui::FontFamily::Monospace, size),
        );
        ctx.set_fonts(fonts);
    }

    fn highlight_text(ctx: &egui::CtxRef, finder: &mut FindTools, switch_to_editor: &mut bool) {
        if let Some(mut editor_state) = egui::TextEdit::load_state(ctx, egui::Id::new("editor")) {
            if let Some((start_index, len)) = finder.get_current_match() {
                let min_curs = egui::epaint::text::cursor::CCursor::new(start_index);
                let max_curs = egui::epaint::text::cursor::CCursor::new(start_index + len);
                editor_state.set_ccursor_range(Some(egui::text_edit::CCursorRange::two(
                    min_curs, max_curs,
                )));
                egui::TextEdit::store_state(ctx, egui::Id::new("editor"), editor_state);
                *switch_to_editor = true;
            }
        }
    }

    fn highlight_text_no_switch(ctx: &egui::CtxRef, finder: &mut FindTools) {
        if let Some(mut editor_state) = egui::TextEdit::load_state(ctx, egui::Id::new("editor")) {
            if let Some((start_index, len)) = finder.get_current_match() {
                let min_curs = egui::epaint::text::cursor::CCursor::new(start_index);
                let max_curs = egui::epaint::text::cursor::CCursor::new(start_index + len);
                editor_state.set_ccursor_range(Some(egui::text_edit::CCursorRange::two(
                    min_curs, max_curs,
                )));
                egui::TextEdit::store_state(ctx, egui::Id::new("editor"), editor_state);
                finder.initial_click_made = false; // Don't change this property with .get_current_match()
            }
        }
    }
}

fn get_line_num_str(count: usize) -> String {
    let mut lines_str = String::with_capacity(count);
    let num_digits = get_num_digits(count);
    //TODO
    for i in 1..=count {
        let leading_spaces = num_digits - get_num_digits(i);
        let temp_str = format!("{:width$}{}", "", i, width=leading_spaces);
        lines_str.push_str(&format!("{}\n", temp_str));
    }


    lines_str.push('~');
    lines_str
}
fn get_num_digits(num: usize) -> usize {
    let mut num = num;
    let mut dig_count: usize = 1;
    while num / 10 > 0 {
        num = num / 10;
        dig_count += 1;
    }
    dig_count
}

#[derive(PartialEq)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
enum Popup {
    OpenFile,
    SaveFile,
    SaveAs,
    FileNotSavedNew,
    FileNotSavedOpen,
    Error,
    Find,
    FindAndReplace,
    None,
}
