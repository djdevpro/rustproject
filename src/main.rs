//! Rust Project — gestionnaire de projets personnels, en egui.
//!
//! Interface :
//! - `theme.rs`, `icons.rs`, `widgets.rs` : apparence et boîte à widgets.
//! - `app.rs`      : le shell (navigation, actions, tâches de fond, enregistrement).
//! - `panel.rs`, `launchers.rs`, `update.rs`, `screens/` : panneau projet, lanceurs, écrans.
//!
//! Moteur :
//! - `model.rs`    : projets, taxonomie, lanceurs, configuration (sérialisables).
//! - `store.rs`    : le fichier `rust-project.json` et l'index sémantique.
//! - `scan.rs`     : les dossiers des dossiers sources, sur 1 à 3 niveaux.
//! - `classify.rs` : projet ou pas, et la fiche (règles locales ou Ollama).
//! - `ollama.rs`, `github.rs` : les services externes.
//! - `search.rs`   : recherche floue et sémantique.
//! - `jobs.rs`     : les tâches de fond.
//! - `mcp.rs`      : le serveur MCP (`rust-project --mcp`).
//! - `launch.rs`   : lancement réel des lanceurs.

// Sous Windows, l'application s'ouvre sans console (publiée ou en développement) ; `--mcp` garde
// l'entrée et la sortie standard que lui passe le client. Lancée depuis un terminal, elle y écrit
// ses messages (voir `attach_parent_console`).
#![cfg_attr(not(test), windows_subsystem = "windows")]

mod app;
mod classify;
mod context;
#[cfg(any(target_os = "linux", test))]
mod desktop;
mod devtools;
#[cfg(test)]
mod fixture;
mod github;
mod graph;
mod i18n;
mod icons;
mod jobs;
mod launch;
mod launchers;
mod mcp;
mod model;
mod ollama;
mod ollaya;
mod ontology;
mod packages;
mod panel;
mod release;
mod scan;
mod screens;
mod search;
mod select;
mod store;
mod tags;
mod theme;
mod update;
mod widgets;

#[cfg(test)]
mod tests;

fn main() -> eframe::Result {
    // `rust-project --mcp` : serveur MCP sur l'entrée et la sortie standard, sans interface.
    if std::env::args().any(|a| a == "--mcp") {
        std::process::exit(mcp::serve());
    }
    #[cfg(windows)]
    attach_parent_console();
    #[cfg(target_os = "linux")]
    desktop::install();
    // L'icône de la fenêtre (barre des tâches, Alt+Tab) ; sans elle, eframe met la sienne.
    let icon = eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png")).expect("icône de la fenêtre");
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Rust Project")
            .with_icon(icon)
            .with_inner_size([1480.0, 920.0])
            .with_min_inner_size([1180.0, 720.0])
            .with_decorations(false),
        ..Default::default()
    };
    // `rust-project --fixture fixtures/demo-en.json` : des projets fictifs, rien n'est enregistré.
    let args: Vec<String> = std::env::args().collect();
    let fixture = args.iter().position(|a| a == "--fixture").and_then(|i| args.get(i + 1)).cloned();
    eframe::run_native(
        "rust-project",
        options,
        Box::new(move |cc| {
            Ok(Box::new(match fixture {
                Some(path) => app::AtelierApp::from_fixture(cc, &path),
                None => app::AtelierApp::new(cc),
            }))
        }),
    )
}

/// Lancée depuis un terminal (`cargo run`…), l'application y écrit ses messages ; d'un double-clic,
/// il n'y a pas de terminal et rien ne s'ouvre.
#[cfg(windows)]
fn attach_parent_console() {
    unsafe extern "system" {
        fn AttachConsole(process_id: u32) -> i32;
    }
    const ATTACH_PARENT_PROCESS: u32 = u32::MAX;
    // SAFETY : simple appel système, sans pointeur ; un échec (pas de terminal parent) est normal.
    unsafe {
        AttachConsole(ATTACH_PARENT_PROCESS);
    }
}
