//! Atelier — gestionnaire de projets personnels, en egui.
//!
//! Interface :
//! - `theme.rs`, `icons.rs`, `widgets.rs` : apparence et boîte à widgets.
//! - `app.rs`      : le shell (navigation, actions, tâches de fond, enregistrement).
//! - `panel.rs`, `launchers.rs`, `update.rs`, `screens/` : panneau projet, lanceurs, écrans.
//!
//! Moteur :
//! - `model.rs`    : projets, taxonomie, lanceurs, configuration (sérialisables).
//! - `store.rs`    : le fichier `atelier.json` et l'index sémantique.
//! - `scan.rs`     : les dossiers des dossiers sources, sur 1 à 3 niveaux.
//! - `classify.rs` : projet ou pas, et la fiche (règles locales ou Ollama).
//! - `ollama.rs`, `github.rs` : les services externes.
//! - `search.rs`   : recherche floue et sémantique.
//! - `jobs.rs`     : les tâches de fond.
//! - `mcp.rs`      : le serveur MCP (`atelier --mcp`).
//! - `launch.rs`   : lancement réel des lanceurs.

mod app;
mod classify;
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
mod panel;
mod release;
mod scan;
mod screens;
mod search;
mod store;
mod theme;
mod update;
mod widgets;

#[cfg(test)]
mod tests;

fn main() -> eframe::Result {
    // `atelier --mcp` : serveur MCP sur l'entrée et la sortie standard, sans interface.
    if std::env::args().any(|a| a == "--mcp") {
        std::process::exit(mcp::serve());
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Atelier")
            .with_inner_size([1480.0, 920.0])
            .with_min_inner_size([1180.0, 720.0])
            .with_decorations(false),
        ..Default::default()
    };
    // `atelier --fixture fixtures/demo-en.json` : des projets fictifs, rien n'est enregistré.
    let args: Vec<String> = std::env::args().collect();
    let fixture = args.iter().position(|a| a == "--fixture").and_then(|i| args.get(i + 1)).cloned();
    eframe::run_native(
        "atelier",
        options,
        Box::new(move |cc| {
            Ok(Box::new(match fixture {
                Some(path) => app::AtelierApp::from_fixture(cc, &path),
                None => app::AtelierApp::new(cc),
            }))
        }),
    )
}
