#![windows_subsystem = "windows"]

use std::collections::HashMap;
use std::{fs, io};
use std::fs::File;
use std::io::Write;
use std::time::Duration;
use eframe::egui;
use eframe::egui::{Align, Label, Layout, RichText, ScrollArea, Sense};
use rand::seq::IteratorRandom;
use egui_notify::{Toasts};
use tts::*;

#[derive(PartialEq, Debug)]
enum LevelEnum {
    Low,
    Media,
    High,
}

impl Default for LevelEnum {
    fn default() -> Self {
        LevelEnum::Low
    }
}


#[macro_use]
extern crate serde;

#[cfg(target_os = "windows")]
static DEFAULT_FONT_PATH: &str = "C:\\Windows\\Fonts\\SIMSUN.TTC";
#[cfg(target_os = "macos")]
static DEFAULT_FONT_PATH: &str = "/System/Library/Fonts/STHeiti Light.ttc";

type Category = HashMap<String, HashMap<String, String>>;

trait Tool {
    fn random_sample(&self) -> (String, String);
}

impl Tool for HashMap<String, String> {
    fn random_sample(&self) -> (String, String) {
        if let Some(key) = self.keys().choose(&mut rand::thread_rng()) {
            if let Some(value) = self.get(key) {
                return (key.clone(), value.clone());
            };
        };
        ("".to_string(), "".to_string())
    }
}

#[derive(Default, Deserialize, Serialize)]
struct Words {
    learn: Category,
    complete: Category,
}

impl Words {
    fn complete_word(&mut self, category: &str, word: &str, answer: String) -> Option<()> {
        self.learn.get_mut(category)?.remove(word)?;
        self.complete.entry(category.to_string())
            .or_insert(HashMap::new())
            .insert(word.to_string(), answer);
        Some(())
    }

    fn remaining_words(&self, category: &str) -> usize {
        if let Some(words) = self.learn.get(category) {
            return words.len();
        };
        0
    }

    fn completed_words(&self, category: &str) -> usize {
        if let Some(data) = self.complete.get(category) {
            return data.len();
        }
        0
    }

    fn save_progress(&self) -> Result<(), String> {
        let data = serde_json::to_string(self).expect("Serialize Error");
        save_words_json(&data).expect("Save File Error");
        Ok(())
    }

    fn review(&mut self) {
        for (k, v) in self.complete.iter_mut() {
            for (question, answer) in v.clone() {
                self.learn.entry(k.clone())
                    .or_insert(HashMap::new())
                    .insert(question.clone(), answer.clone());
                v.remove(&question);
            }
        }
    }
}


#[derive(Default)]
struct EnglishApp {
    toasts: Toasts,
    words: Words,
    text: String,
    category: String,
    question: String,
    answer: String,
    correct_rate: i32,
    error_rate: i32,
    tts: Option<Tts>,
    level: LevelEnum,
    hint: bool,
}

fn read_words_json() -> Words {
    let words = Words { learn: HashMap::new(), complete: HashMap::new() };
    let file = fs::read_to_string("words.json");
    match file {
        Ok(fp) => {
            let words = serde_json::from_str::<Words>(&fp);
            match words {
                Ok(w) => return w,
                Err(e) => println!("{}", e)
            }
        }
        Err(e) => println!("{}", e)
    }
    words
}

fn save_words_json(data: &str) -> Result<(), io::Error> {
    let mut fp = File::create("words.json")?;
    fp.write_all(data.as_bytes())?;
    Ok(())
}

impl EnglishApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_custom_fonts(&cc.egui_ctx);
        let words = read_words_json();
        let mut s = Self::default();
        s.words = words;
        s.tts = Tts::default().ok();
        s
    }

    fn play_audio(&mut self) -> Option<()> {
        if self.level == LevelEnum::Low && self.question != "" {
            let x = self.tts.as_mut()?;
            (*x).speak(self.answer.clone(), false);
        }
        Some(())
    }

    fn choice_word(&mut self) {
        if let Some(wd) = self.words.learn.get(&self.category) {
            let sample = wd.random_sample();
            self.question = sample.0;
            self.answer = sample.1;
            self.play_audio();
        }
    }

    fn submit_word(&mut self) {
        if self.text == self.answer {
            self.toasts.success("Success Word").set_duration(Some(Duration::from_secs(3)));
            self.correct_rate += 1;
            self.words.complete_word(&self.category, &self.question, self.answer.clone());
        } else {
            self.error_rate += 1;
            self.toasts.error("Error Word").set_duration(Some(Duration::from_secs(3)));
        }
        self.text = "".to_string();
        self.choice_word();
    }

    fn reload_question(&mut self) {
        self.choice_word();
        self.correct_rate = 0;
        self.error_rate = 0;
    }
}

fn main() {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native("ladder - Language Learning Platform", native_options, Box::new(|cc| Box::new(EnglishApp::new(cc))));
}

fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    if let Ok(font_data) = fs::read(DEFAULT_FONT_PATH) {
        fonts.font_data.insert(
            "宋体".to_owned(),
            egui::FontData::from_owned(font_data),
        );
    };

    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "宋体".to_owned());

    ctx.set_fonts(fonts);
}

impl eframe::App for EnglishApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("left").resizable(false)
            .show(ctx, |ui| {
                ui.set_width(160.0);
                ui.heading(format!("word category: {}", self.words.learn.len()));
                ui.separator();
                ScrollArea::vertical()
                    .show_viewport(ui, |ui, _viewport| {
                        ui.vertical_centered_justified(|ui| {
                            for (category, _) in &self.words.learn.clone() {
                                let category_button = egui::Button::new(format!("{}", category));
                                if self.words.remaining_words(category) == 0 {
                                    ui.add_enabled(false, category_button);
                                } else {
                                    if ui.add(category_button).clicked()
                                    {
                                        self.category = category.clone();
                                        self.reload_question();
                                    };
                                };
                            }
                        });
                    });
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.with_layout(Layout::left_to_right(egui::Align::TOP), |ui| {
                if ui.button("reload").clicked() {
                    self.words = read_words_json();
                    self.reload_question();
                }
                if ui.button("save progress").clicked() {
                    match self.words.save_progress() {
                        Ok(_) => self.toasts.success("Save Success;").set_duration(Some(Duration::from_secs(3))),
                        Err(e) => self.toasts.error(e).set_duration(Some(Duration::from_secs(3))),
                    };
                };
                if ui.button("review").clicked() {
                    self.words.review();
                    self.toasts.success("Review success;").set_duration(Some(Duration::from_secs(3)));
                };
                egui::ComboBox::from_label("Level")
                    .selected_text(format!("{:?}", self.level))
                    .width(50.0)
                    .show_ui(ui, |ui| {
                        ui.set_min_width(50.0);
                        ui.selectable_value(&mut self.level, LevelEnum::Low, "Low");
                        // ui.selectable_value(&mut self.level, LevelEnum::Media, "Median");
                        ui.selectable_value(&mut self.level, LevelEnum::High, "High");
                    })
            });
            ui.horizontal(|ui| {
                if self.category != "" {
                    ui.label("category is");
                    ui.code(format!("{}", self.category));
                    ui.label(format!("remaining word {}", self.words.remaining_words(&self.category)));
                    ui.label(format!("completed word {}", self.words.completed_words(&self.category)));
                };
            });
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.hint, "Hint");
            });
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                if ui.add(Label::new(RichText::new(&self.question).heading()).sense(Sense::click())).clicked() {
                    self.play_audio();
                }
                if self.hint {
                    ui.heading(format!("{}", self.answer));
                } else {
                    ui.heading("");
                }
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.label("Please input english word");
                let word_input = egui::TextEdit::singleline(&mut self.text)
                    .desired_rows(4)
                    .hint_text("Input and click submit")
                    .show(ui);

                if ui.button("submit").clicked() || ui.ctx().input().key_released(egui::Key::Enter) {
                    word_input.response.request_focus();
                    self.submit_word();
                }
            });
        });
        egui::TopBottomPanel::bottom("bottom").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("Correct rate: {}", self.correct_rate));
                ui.put(egui::Rect::from_min_size(ui.min_rect().min + egui::Vec2::new(230.0, 0.0), ui.min_size()), egui::Label::new(format!("Error rate: {}", self.error_rate)));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.hyperlink_to("About", "https://github.com/Magicskys/ladder");
                })
            });
        });
        self.toasts.show(ctx);
    }
}