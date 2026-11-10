use crate::audio::Audio;
use crate::screens::chords::{self, Chords};
use crate::screens::common::{Ctx, Nav};
use crate::screens::fretboard_trainer::{self, FretboardTrainer};
use crate::screens::progress::{self, Drill, Session};
use crate::screens::scales::{self, Scales};
use crate::screens::settings as settings_screen;
use crate::settings::Settings;
use crate::srs::Progress;
use crate::storage;
use crate::theme;
use crate::util::Rng;
use crate::widgets::notation::MUSIC_FONT;
use egui::{FontData, FontDefinitions, FontFamily, RichText};
use std::sync::Arc;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Screen {
    Fretboard,
    Chords,
    Scales,
    Progress,
    Settings,
}

pub struct App {
    settings: Settings,
    saved_settings: String,
    progress: Progress,
    audio: Audio,
    rng: Rng,
    screen: Screen,
    fb: FretboardTrainer,
    chords: Chords,
    scales: Scales,
    nav: Option<Nav>,
    session: Option<Session>,
    last_save: f64,
    applied: Option<f32>,
}

const SETTINGS_FILE: &str = "settings.json";
const PROGRESS_FILE: &str = "progress.json";

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> App {
        let mut fonts = FontDefinitions::default();
        fonts.font_data.insert(
            MUSIC_FONT.to_owned(),
            Arc::new(FontData::from_static(include_bytes!("../assets/Bravura.otf"))),
        );
        fonts
            .families
            .insert(FontFamily::Name(MUSIC_FONT.into()), vec![MUSIC_FONT.to_owned()]);
        // Hack (bundled monospace font) has arrows and symbols the UI font lacks
        if let Some(prop) = fonts.families.get_mut(&FontFamily::Proportional) {
            prop.push("Hack".to_owned());
        }
        cc.egui_ctx.set_fonts(fonts);
        theme::apply(&cc.egui_ctx);

        let settings: Settings = storage::load(SETTINGS_FILE);
        let progress: Progress = storage::load(PROGRESS_FILE);
        let audio = Audio::new(settings.volume);
        let saved_settings = serde_json::to_string(&settings).unwrap_or_default();
        let mut app = App {
            settings,
            saved_settings,
            progress,
            audio,
            rng: Rng::seeded(),
            screen: Screen::Fretboard,
            fb: FretboardTrainer::default(),
            chords: Chords::default(),
            scales: Scales::default(),
            nav: None,
            session: None,
            last_save: 0.0,
            applied: None,
        };
        // Developer option: start on a given screen, e.g. GT_START=chords:read
        if let Ok(start) = std::env::var("GT_START") {
            app.start_at(&start);
        }
        app
    }

    fn start_at(&mut self, s: &str) {
        match s {
            "fb" | "fb:explore" => self.screen = Screen::Fretboard,
            "fb:name" => self.go_to_drill(Drill::FbName),
            "fb:find" => self.go_to_drill(Drill::FbFind),
            "fb:staff" => self.go_to_drill(Drill::FbStaff),
            "chords" | "chords:library" => self.screen = Screen::Chords,
            "chords:build" => {
                self.screen = Screen::Chords;
                self.chords.set_tab(chords::Tab::BuildIt);
            }
            "chords:read" => self.go_to_drill(Drill::ChRead),
            "chords:name" => self.go_to_drill(Drill::ChName),
            "chords:identify" => {
                self.screen = Screen::Chords;
                self.chords.set_tab(chords::Tab::Identify);
            }
            "scales" | "scales:library" => self.screen = Screen::Scales,
            "scales:fill" => self.go_to_drill(Drill::ScFill),
            "scales:degree" => self.go_to_drill(Drill::ScDegree),
            "scales:play" => {
                self.screen = Screen::Scales;
                self.scales.set_tab(scales::Tab::PlayPosition);
            }
            "progress" => self.screen = Screen::Progress,
            "settings" => self.screen = Screen::Settings,
            _ => eprintln!("GT_START: unknown screen {s:?}"),
        }
    }

    fn save(&mut self) {
        if self.progress.dirty {
            storage::save(PROGRESS_FILE, &self.progress);
            self.progress.dirty = false;
        }
        let s = serde_json::to_string(&self.settings).unwrap_or_default();
        if s != self.saved_settings {
            storage::save(SETTINGS_FILE, &self.settings);
            self.saved_settings = s;
        }
    }

    fn go_to_drill(&mut self, d: Drill) {
        match d {
            Drill::FbName => {
                self.screen = Screen::Fretboard;
                self.fb.set_mode(fretboard_trainer::Mode::NameNote);
            }
            Drill::FbFind => {
                self.screen = Screen::Fretboard;
                self.fb.set_mode(fretboard_trainer::Mode::FindNote);
            }
            Drill::FbStaff => {
                self.screen = Screen::Fretboard;
                self.fb.set_mode(fretboard_trainer::Mode::StaffToFret);
            }
            Drill::ChRead => {
                self.screen = Screen::Chords;
                self.chords.set_tab(chords::Tab::SightRead);
            }
            Drill::ChName => {
                self.screen = Screen::Chords;
                self.chords.set_tab(chords::Tab::NameIt);
            }
            Drill::ScDegree => {
                self.screen = Screen::Scales;
                self.scales.set_tab(scales::Tab::WhichDegree);
            }
            Drill::ScFill => {
                self.screen = Screen::Scales;
                self.scales.set_tab(scales::Tab::FillBox);
            }
        }
    }

    /// Is the drill the session is on currently between questions?
    fn drill_idle(&self) -> bool {
        match self.screen {
            Screen::Fretboard => self.fb.is_idle(),
            Screen::Chords => self.chords.is_idle(),
            Screen::Scales => self.scales.is_idle(),
            _ => true,
        }
    }

    fn session_bar(&mut self, ui: &mut egui::Ui) {
        if self.session.is_none() {
            return;
        }
        // switch to the next drill only between questions, never mid-answer
        if self.drill_idle()
            && let Some(d) = self.session.as_mut().and_then(|s| s.advance(&self.progress))
        {
            self.go_to_drill(d);
        }
        let Some(s) = &self.session else { return };
        let answered = s.answered(&self.progress);
        let mut stop = false;
        egui::Frame::group(ui.style()).fill(theme::WINDOW).show(ui, |ui| {
            ui.horizontal(|ui| {
                if s.finished(&self.progress) {
                    ui.label(RichText::new(format!("🎉 Today's practice done: {} questions!", s.total)).size(18.0).strong());
                    if ui.button("close").clicked() {
                        stop = true;
                    }
                } else {
                    ui.label(
                        RichText::new(format!(
                            "Today's practice: {}/{}  ·  now: {}",
                            answered,
                            s.total,
                            s.current().label()
                        ))
                        .size(17.0)
                        .strong(),
                    );
                    ui.add(
                        egui::ProgressBar::new(answered as f32 / s.total as f32)
                            .desired_width(200.0)
                            .fill(theme::ACCENT),
                    );
                    if ui.button("stop session").clicked() {
                        stop = true;
                    }
                }
            });
        });
        ui.add_space(4.0);
        if stop {
            self.session = None;
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ectx = ui.ctx().clone();
        if self.applied != Some(self.settings.ui_scale) {
            ectx.set_zoom_factor(self.settings.ui_scale);
            self.applied = Some(self.settings.ui_scale);
        }
        let time = ui.input(|i| i.time);

        let side = egui::Frame::new().fill(theme::SIDEBAR).inner_margin(egui::Margin::same(10));
        egui::Panel::left("nav").resizable(false).exact_size(180.0).frame(side).show(ui, |ui| {
            ui.add_space(4.0);
            ui.label(RichText::new("🎸 Guitar Trainer").size(20.0).strong().color(theme::ACCENT_HOVER));
            ui.add_space(14.0);
            for (s, label) in [
                (Screen::Fretboard, "Fretboard"),
                (Screen::Chords, "Chords"),
                (Screen::Scales, "Scales"),
                (Screen::Progress, "Progress"),
                (Screen::Settings, "Settings"),
            ] {
                let b = egui::Button::selectable(self.screen == s, RichText::new(label).size(19.0))
                    .min_size(egui::vec2(150.0, 38.0));
                if ui.add(b).clicked() {
                    self.screen = s;
                }
            }
            ui.add_space(16.0);
            ui.add_space(8.0);
            ui.label(RichText::new("Tuning").weak());
            ui.label(RichText::new(self.settings.tuning().name).size(13.0));
            ui.label(RichText::new(self.settings.fretboard_style.label()).size(13.0).weak());
            ui.add_space(6.0);
            let d = self.progress.today();
            ui.label(RichText::new(format!("Today: {} answers", d.attempts)).weak());
            if !self.audio.available() {
                ui.label(RichText::new("🔇 no audio device").weak());
            }
        });

        egui::CentralPanel::default().show(ui, |ui| {
            self.session_bar(ui);
            let mut ctx = Ctx {
                settings: &mut self.settings,
                progress: &mut self.progress,
                audio: &mut self.audio,
                rng: &mut self.rng,
                nav: &mut self.nav,
                time,
            };
            match self.screen {
                Screen::Fretboard => self.fb.ui(ui, &mut ctx),
                Screen::Chords => self.chords.ui(ui, &mut ctx),
                Screen::Scales => self.scales.ui(ui, &mut ctx),
                Screen::Progress => {
                    if progress::ui(ui, &mut ctx) {
                        let s = Session::new(&self.progress);
                        let d = s.current();
                        self.session = Some(s);
                        self.go_to_drill(d);
                    }
                }
                Screen::Settings => settings_screen::ui(ui, &mut ctx),
            }
        });

        match self.nav.take() {
            Some(Nav::Chord { root, ty }) => {
                self.chords.open(root, ty);
                self.screen = Screen::Chords;
            }
            Some(Nav::Scale { root, id }) => {
                self.scales.open(root, id);
                self.screen = Screen::Scales;
            }
            None => {}
        }

        // Save answers within ~2 s (and settings every 10 s) so a crash or kill loses nothing.
        let due = if self.progress.dirty { 2.0 } else { 10.0 };
        if time - self.last_save > due {
            self.last_save = time;
            self.save();
        }
        if self.progress.dirty {
            ectx.request_repaint_after(std::time::Duration::from_millis(2100));
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.save();
    }
}
