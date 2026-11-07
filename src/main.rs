
use eframe::egui;
use serde::{Deserialize, Serialize};
use std::{env, fs, path::PathBuf};

const MANIFEST_URL: &str =
    "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

#[derive(Serialize, Deserialize)]
struct Profiles {
    usernames: Vec<String>,
}

#[derive(Deserialize, Clone)]
struct Version {
    id: String,
    #[serde(rename = "type")]
    kind: String,
    url: String,
}

#[derive(Deserialize)]
#[derive(Deserialize)]
struct Manifest {
    versions: Vec<Version>,
}

struct Launcher {
    profiles: Vec<String>,
    username: String,
    selected: usize,
    versions: Vec<Version>,
    version: String,
    status: String,
}

fn folder() -> PathBuf {
    env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| env::current_dir().unwrap())
        .join("ParanthaLauncher")
}

fn load_profiles() -> Vec<String> {
    let path = folder().join("profiles.json");

    if let Ok(data) = fs::read_to_string(path) {
        if let Ok(p) = serde_json::from_str::<Profiles>(&data) {
            if !p.usernames.is_empty() {
                return p.usernames;
            }
        }
    }

    vec!["Player".to_string()]
}

fn save_profiles(names: &[String]) -> Result<(), String> {
    fs::create_dir_all(folder()).map_err(|e| e.to_string())?;

    let data = serde_json::to_string_pretty(
        &Profiles {
            usernames: names.to_vec(),
        }
    ).map_err(|e| e.to_string())?;

    fs::write(folder().join("profiles.json"), data)
        .map_err(|e| e.to_string())
}

impl Default for Launcher {
    fn default() -> Self {
        Self {
            profiles: load_profiles(),
            username: String::new(),
            selected: 0,
            versions: Vec::new(),
            version: String::new(),
            status: "Welcome to Parantha Launcher!".into(),
        }
    }
}

impl eframe::App for Launcher {
    fn update(
        &mut self,
        ctx: &egui::Context,
        _frame: &mut eframe::Frame,
    ) {
        ctx.set_visuals(egui::Visuals::dark());

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("PARANTHA LAUNCHER");
                ui.label("Minecraft Java Edition");
                ui.label("BUILD 2.0");

                ui.add_space(20.0);

                ui.group(|ui| {
                    ui.heading("LOCAL ACCOUNTS");

                    let current = self.profiles
                        .get(self.selected)
                        .cloned()
                        .unwrap_or_default();

                    egui::ComboBox::from_id_salt("accounts")
                        .selected_text(current)
                        .show_ui(ui, |ui| {
                            for (i, name) in self.profiles.iter().enumerate() {
                                ui.selectable_value(
                                    &mut self.selected, i, name
                                );
                            }
                        });

                    ui.label("New username:");
                    ui.text_edit_singleline(&mut self.username);

                    if ui.button("CREATE PROFILE").clicked() {
                        let name = self.username.trim().to_string();

                        let valid = (3..=16).contains(&name.len())
                            && name.chars().all(|c|
                                c.is_ascii_alphanumeric() || c == '_'
                            );

                        if valid && !self.profiles.iter().any(
                            |p| p.eq_ignore_ascii_case(&name)
                        ) {
                            self.profiles.push(name);
                            self.selected = self.profiles.len() - 1;

                            self.status = match save_profiles(&self.profiles) {
                                Ok(()) => "Profile saved!".into(),
                                Err(e) => format!("Save error: {e}"),
                            };

                            self.username.clear();
                        } else {
                            self.status =
                                "Invalid or existing username.".into();
                        }
                    }

                    ui.label("Local profile only; not Microsoft authentication.");
                });

                ui.add_space(15.0);

                ui.group(|ui| {
                    ui.heading("MINECRAFT VERSIONS");

                    if ui.button("REFRESH VERSIONS").clicked() {
                        self.status = "Loading versions...".into();

                        match reqwest::blocking::get(MANIFEST_URL)
                            .and_then(|r| r.error_for_status())
                            .and_then(|r| r.json::<Manifest>())
                        {
                            Ok(data) => {
                                self.versions = data.versions;
                                self.status = format!(
                                    "Loaded {} versions!",
                                    self.versions.len()
                                );
                            }
                            Err(e) => {
                                self.status = format!("Error: {e}");
                            }
                        }
                    }

                    let releases: Vec<Version> = self.versions
                        .iter()
                        .filter(|v| v.kind == "release")
                        .cloned()
                        .collect();

                    egui::ComboBox::from_id_salt("versions")
                        .selected_text(if self.version.is_empty() {
                            "Select Minecraft version"
                        } else {
                            &self.version
                        })
                        .show_ui(ui, |ui| {
                            for v in &releases {
                                ui.selectable_value(
                                    &mut self.version,
                                    v.id.clone(),
                                    &v.id,
                                );
                            }
                        });

                    if ui.button("DOWNLOAD METADATA").clicked() {
                        if let Some(v) = releases.iter().find(
                            |v| v.id == self.version
                        ) {
                            let result = reqwest::blocking::get(&v.url)
                                .and_then(|r| r.error_for_status())
                                .and_then(|r| r.text());

                            match result {
                                Ok(data) => {
                                    let path = folder()
                                        .join("versions")
                                        .join(&v.id);

                                    let result = fs::create_dir_all(&path)
                                        .and_then(|_| {
                                            fs::write(
                                                path.join(format!("{}.json", v.id)),
                                                data
                                            )
                                        });

                                    self.status = match result {
                                        Ok(()) => format!(
                                            "Metadata saved for {}!", v.id
                                        ),
                                        Err(e) => format!("Save error: {e}"),
                                    };
                                }
                                Err(e) => {
                                    self.status = format!("Download error: {e}");
                                }
                            }
                        } else {
                            self.status = "Select a version first.".into();
                        }
                    }

                    ui.label("Metadata only; full installation comes next.");
                });

                ui.add_space(20.0);

                ui.group(|ui| {
                    ui.heading("STATUS");
                    ui.label(&self.status);
                    ui.add_enabled(
                        false,
                        egui::Button::new("PLAY — COMING SOON")
                    );
                });

                ui.add_space(15.0);
                ui.label("PARANTHA LAUNCHER | MADE WITH RUST");
            });
        });
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 650.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Parantha Launcher",
        options,
        Box::new(|_cc| Ok(Box::new(Launcher::default()))),
    )
}
