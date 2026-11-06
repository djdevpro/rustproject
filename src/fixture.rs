//! La fixture anglaise : 30 projets fictifs, une taxonomie et une ontologie en anglais, pour
//! tester Atelier et faire les captures du README.
//!
//! Le fichier `fixtures/demo-en.json` est écrit par `UPDATE_FIXTURES=1 cargo test fixture`, et
//! l'application l'ouvre sans rien enregistrer avec `atelier --fixture fixtures/demo-en.json`.

use egui::Color32;

use crate::icons::Icon;
use crate::model::{Activity, ActivityKind, Attention, Data, Domain, Project, ScanRoot, Source, Status, Step};
use crate::ontology::Tag;

/// Le 6 novembre 2026 à 9 h UTC : les dates de la fixture ne bougent pas d'une écriture à l'autre.
const BASE: u64 = 1_793_955_600;
const DAY: u64 = 86_400;
const ROOT: &str = r"D:\Projects";
const CLIENTS: &str = r"D:\Clients";

pub const PATH: &str = "fixtures/demo-en.json";

fn domains() -> Vec<Domain> {
    let d = |name: &str, icon, rgb: u32| Domain { name: name.into(), icon, color: Color32::from_rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8) };
    vec![
        d("Developer tools", Icon::Wrench, 0x6c9cf0),
        d("AI & agents", Icon::Sparkles, 0xb98cf0),
        d("Learning", Icon::GraduationCap, 0xf0a443),
        d("Web & SaaS", Icon::Globe, 0x3fc1b0),
        d("Games & 3D", Icon::Box, 0xef7f5f),
        d("Audio & voice", Icon::AudioWaveform, 0x7fcf6f),
        d("Writing & personal", Icon::Newspaper, 0xd9b45a),
    ]
}

/// (nom, icône, domaine, type, technos, état, attention, jours sans activité, dossier, une phrase,
/// résultat visé, prochaine action)
type Row = (&'static str, Icon, usize, &'static str, &'static [&'static str], Status, Attention, u32, &'static str, &'static str, &'static str, &'static str);

fn rows() -> Vec<Row> {
    use Attention::*;
    use Status::*;
    vec![
        // Focus
        ("Atlas", Icon::Layers, 0, "Desktop app", &["Rust", "egui"], Active, Focus, 0, r"Atlas", "Desktop dashboard that maps every side project on disk.", "Graph view v2", "Profile the layout pass."),
        ("Scribe Agent", Icon::Sparkles, 1, "AI agent", &["Python", "Claude"], Active, Focus, 0, r"Scribe Agent", "Agent that turns meeting notes into tasks.", "Weekly digest", "Add calendar import."),
        ("Pocket Tutor", Icon::GraduationCap, 2, "Course platform", &["Next.js", "TypeScript"], Active, Focus, 1, r"Pocket Tutor", "Course builder for language teachers.", "First paid course", "Record lesson three."),
        ("Ledgerly", Icon::ReceiptText, 3, "SaaS", &["SvelteKit", "Postgres"], Active, Focus, 2, r"CLIENTS\Ledgerly", "Invoicing app for freelancers with automatic reminders.", "Public beta", "Wire the Stripe webhooks."),
        // Au fil de l'eau
        ("Shipyard", Icon::Ship, 0, "Command line tool", &["Go"], Active, Flow, 3, r"Shipyard", "Deploy tool for a small fleet of servers.", "Zero downtime deploys", "Write the rollback command."),
        ("Promptbook", Icon::Network, 1, "MCP server", &["TypeScript", "MCP"], Active, Flow, 4, r"Promptbook", "MCP server that shares prompt templates across tools.", "Template registry", "Document the tools."),
        ("Lumen API", Icon::Code, 3, "API", &["Bun", "TypeScript"], Active, Flow, 5, r"Lumen\api", "REST API for a photo marketplace.", "Payments live", "Test the upload limits."),
        ("Lumen Web", Icon::Laptop, 3, "Web app", &["React", "TypeScript"], Active, Flow, 5, r"Lumen\web", "Storefront for the Lumen photo marketplace.", "Payments live", "Polish the checkout page."),
        ("Inbox Bot", Icon::Mail, 1, "AI agent", &["Python", "Claude"], Active, Flow, 6, r"Inbox Bot", "Sorts a shared inbox and drafts replies.", "Half the replies drafted", "Tune the triage rules."),
        ("Podsmith", Icon::Mic, 5, "Transcription", &["Python", "Whisper"], Active, Flow, 7, r"Podsmith", "Turns podcast episodes into searchable transcripts.", "Search across episodes", "Cut the latency in half."),
        ("Cartograph", Icon::GitBranch, 0, "Command line tool", &["Rust"], Active, Flow, 8, r"Cartograph", "Charts git activity across many repositories.", "Yearly report", "Add a weekly view."),
        ("Voxel Garden", Icon::Box, 4, "Game", &["Godot", "GDScript"], Active, Flow, 9, r"Games\Voxel Garden", "Cozy farming game made of voxels.", "Playable demo", "Finish the watering loop."),
        ("Field Notes", Icon::NotebookPen, 6, "Note app", &["Tauri", "React"], Active, Flow, 10, r"Field Notes", "Offline notebook with Markdown and sketches.", "Sync between laptops", "Handle edit conflicts."),
        ("Grammar Lab", Icon::BookOpen, 2, "Learning site", &["Astro"], Active, Flow, 12, r"Grammar Lab", "Short grammar drills with spaced repetition.", "Fifty drills", "Write the irregular verbs set."),
        ("Tiny Orbit", Icon::Orbit, 4, "Web toy", &["Three.js", "TypeScript"], Active, Flow, 14, r"Games\Tiny Orbit", "Browser toy that simulates small planetary systems.", "Share links", "Save a system to the URL."),
        ("Waveform Kit", Icon::AudioWaveform, 5, "Library", &["Rust", "WebAssembly"], Active, Flow, 16, r"Waveform Kit", "Audio visualizer components for the web.", "Version 1.0", "Add a spectrum view."),
        ("Quill", Icon::PenLine, 6, "Blog", &["Hugo"], Active, Flow, 18, r"Quill", "Personal blog about building tools.", "One post a month", "Draft the post about Atlas."),
        ("Mentor Match", Icon::Users, 2, "Web app", &["Laravel", "Vue"], Paused, Flow, 20, r"CLIENTS\Mentor Match", "Pairs junior developers with volunteer mentors.", "First ten pairs", "Call the meetup organizers."),
        // En réserve
        ("Trailmix", Icon::Mountain, 6, "Web app", &["SvelteKit"], Paused, Reserve, 80, r"Trailmix", "Hiking log with elevation charts.", "Season recap", "Import GPX files."),
        ("Pixel Board", Icon::Grid3x3, 3, "Web app", &["Elixir", "Phoenix"], Paused, Reserve, 95, r"Pixel Board", "Shared pixel canvas for live events.", "Event mode", "Rate limit the brushes."),
        ("Moodboard AI", Icon::Palette, 1, "Web app", &["Next.js", "TypeScript"], Paused, Reserve, 110, r"Moodboard AI", "Builds moodboards from a short brief.", "Export to Figma", "Pick an image model."),
        ("Habit Loop", Icon::Repeat, 2, "Mobile app", &["Kotlin"], Paused, Reserve, 120, r"Habit Loop", "Habit tracker that explains why streaks break.", "Store release", "Design the weekly review."),
        ("Retro Arcade", Icon::Gamepad2, 4, "Game", &["Rust", "Bevy"], Paused, Reserve, 140, r"Games\Retro Arcade", "Arcade remakes written to learn Rust.", "Three games", "Port the breakout clone."),
        ("Recipe Vault", Icon::CookingPot, 6, "Mobile app", &["Flutter"], Paused, Reserve, 160, r"Recipe Vault", "Family recipe book with shopping lists.", "Shared lists", "Scan the old notebooks."),
        ("Sound Sketch", Icon::Music, 5, "Mobile app", &["Swift"], Idea, Reserve, 200, r"Sound Sketch", "Pocket synth for quick melody ideas.", "First prototype", "Try the audio engine."),
        ("Chess Coach", Icon::Crown, 2, "AI tutor", &["Python"], Idea, Reserve, 210, r"Chess Coach", "Explains chess mistakes in plain words.", "Game review", "Read the engine docs."),
        ("Crawler", Icon::Bug, 0, "Command line tool", &["Python"], Done, Reserve, 300, r"Crawler", "Site crawler that flags broken links.", "Weekly report", "Nothing planned."),
        ("Echo Room", Icon::Radio, 5, "Simulator", &["Rust"], Idea, Reserve, 400, r"Echo Room", "Room reverb simulator.", "First impulse response", "Read about ray tracing for sound."),
        // Archivés
        ("Old Portfolio", Icon::Archive, 6, "Website", &["Jekyll"], Archived, Reserve, 600, r"Old Portfolio", "Previous portfolio site, replaced by Quill.", "Archived", "Nothing planned."),
        ("Weather Widget", Icon::CloudSun, 3, "Widget", &["JavaScript"], Archived, Reserve, 900, r"Weather Widget", "Weather widget for a client site.", "Archived", "Nothing planned."),
    ]
}

/// Étapes, journal et liens des projets en focus (les autres gardent une fiche simple).
fn details(p: &mut Project) {
    let steps: &[(&str, bool)] = match p.name.as_str() {
        "Atlas" => &[("Profile the layout pass", false), ("Ship the graph view", true), ("Write the release notes", false)],
        "Scribe Agent" => &[("Add calendar import", false), ("Summarize action items", true), ("Send the weekly digest", false)],
        "Pocket Tutor" => &[("Record lesson three", false), ("Price the first course", true), ("Invite ten testers", false)],
        "Ledgerly" => &[("Wire the Stripe webhooks", false), ("Send payment reminders", true), ("Open the public beta", false)],
        _ => &[],
    };
    p.steps = steps.iter().map(|(text, done)| Step { text: (*text).into(), done: *done }).collect();
    if p.steps.is_empty() {
        p.steps = vec![Step { text: p.next_action.trim_end_matches('.').into(), done: false }];
    }
    let at = |days: u32| BASE - u64::from(days) * DAY;
    let idle = p.idle_days;
    p.activity = vec![
        Activity { at: at(idle), kind: ActivityKind::Commit, text: format!("Last commit, {}", p.next_action.trim_end_matches('.').to_lowercase()) },
        Activity { at: at(idle + 2), kind: ActivityKind::Edit, text: format!("Next action set to \"{}\"", p.next_action.trim_end_matches('.')) },
        Activity { at: at(idle + 5), kind: ActivityKind::Ai, text: "Classified and summarized in one sentence".into() },
        Activity { at: at(idle + 30), kind: ActivityKind::Scan, text: format!("Found in {ROOT}") },
    ];
    p.notes = vec!["Ideas to sort later.".into()];
    p.key_files = vec!["README.md".into()];
}

/// Une ontologie en anglais : le domaine, et le type de projet.
fn ontology(data: &mut Data) {
    let o = &mut data.ontology;
    let domain = o.add_facet("Domain", "What the project is about and who it serves.");
    let domain_ids: Vec<u64> = data.domains.iter().map(|d| o.add_concept(domain, &d.name, &format!("Projects about {}.", d.name.to_lowercase()))).collect();
    let kind = o.add_facet("Kind", "What gets built, whatever the subject.");
    let kinds = [
        ("Tool", "Command line tools, libraries and developer utilities."),
        ("App", "Desktop, mobile and web applications people use daily."),
        ("Service", "APIs, servers and agents that run without a screen."),
        ("Content", "Courses, blogs, games and other things made to be read or played."),
    ];
    let kind_ids: Vec<u64> = kinds.iter().map(|(n, d)| o.add_concept(kind, n, d)).collect();
    for p in &mut data.projects {
        let k = match p.kind.as_str() {
            "Command line tool" | "Library" | "Simulator" => 0,
            "API" | "MCP server" | "AI agent" | "Transcription" => 2,
            "Game" | "Web toy" | "Blog" | "Learning site" | "Course platform" | "AI tutor" | "Website" => 3,
            _ => 1,
        };
        p.tags = vec![Tag { concept: domain_ids[p.domain], weight: 1.0, manual: false }, Tag { concept: kind_ids[k], weight: 0.86, manual: false }];
    }
}

pub fn english() -> Data {
    let mut data = Data::fresh();
    data.domains = domains();
    data.ontology = Default::default();
    for (order, (name, icon, domain, kind, stack, status, attention, idle, folder, description, outcome, next)) in rows().into_iter().enumerate() {
        let folder = match folder.strip_prefix(r"CLIENTS\") {
            Some(rest) => format!(r"{CLIENTS}\{rest}"),
            None => format!(r"{ROOT}\{folder}"),
        };
        let mut p = Project {
            name: name.into(),
            icon,
            description: description.into(),
            outcome: outcome.into(),
            objective: format!("Take {name} to \"{outcome}\"."),
            last_point: if idle > 60 { "Parked until the idea gets clearer.".into() } else { "The base works, it needs finishing and polish.".into() },
            next_action: next.into(),
            status,
            attention,
            domain,
            kind: kind.into(),
            stack: stack.iter().map(|s| (*s).to_owned()).collect(),
            remote: Some(format!("https://github.com/example/{}", name.to_lowercase().replace(' ', "-"))),
            folders: vec![folder],
            idle_days: idle,
            review: if attention == Attention::Reserve { "When it comes back".into() } else { "Every week".into() },
            confidence: 0.9,
            source: Source::Local,
            order: if attention == Attention::Focus { order as i32 } else { 0 },
            ..Default::default()
        };
        details(&mut p);
        data.add_project(p);
    }
    ontology(&mut data);
    data.config.roots = vec![
        ScanRoot { path: ROOT.into(), enabled: true, depth: 3, found: 34, last_scan: BASE - 600 },
        ScanRoot { path: CLIENTS.into(), enabled: true, depth: 2, found: 3, last_scan: BASE - 600 },
    ];
    data.config.demo_data = true;
    data
}

#[cfg(test)]
mod tests {
    use super::*;

    /// La fixture du dépôt se lit, et elle est bien en anglais. `UPDATE_FIXTURES=1` la réécrit.
    #[test]
    fn fixture_anglaise() {
        if std::env::var_os("UPDATE_FIXTURES").is_some() {
            std::fs::create_dir_all("fixtures").unwrap();
            std::fs::write(PATH, serde_json::to_string_pretty(&english()).unwrap()).unwrap();
        }
        let data = crate::store::Store::open(PATH).load().expect("fixture lisible").expect("fixture présente");
        assert_eq!(data.projects.len(), 30);
        assert_eq!(data.domains.len(), 7);
        assert!(data.projects.iter().filter(|p| p.attention == Attention::Focus && p.status != Status::Archived).count() == 4);
        assert_eq!(data.projects.iter().filter(|p| p.status == Status::Archived).count(), 2);
        // Rien en français : ni accents dans les textes, ni facette « Domaine ».
        let text: String = data.projects.iter().flat_map(|p| [p.name.clone(), p.description.clone(), p.next_action.clone(), p.outcome.clone()]).chain(data.domains.iter().map(|d| d.name.clone())).collect::<Vec<_>>().join(" ");
        assert!(!text.chars().any(|c| "éèêàâçùûôîï".contains(c)), "texte français dans la fixture");
        assert!(data.ontology.facets.iter().all(|f| f.name == "Domain" || f.name == "Kind"));
        // Chaque description tient en une phrase.
        assert!(data.projects.iter().all(|p| p.description.matches(". ").count() == 0 && p.description.ends_with('.')));
    }
}

/// Les captures du README, depuis la fixture : `cargo test captures_readme -- --ignored`.
#[cfg(test)]
mod captures {
    use egui_kittest::Harness;

    use crate::app::{AtelierApp, LibView, Screen};

    fn shot(name: &str, setup: impl Fn(&mut AtelierApp)) {
        let data = crate::store::Store::open(super::PATH).load().unwrap().unwrap();
        let mut h = Harness::builder().with_size(egui::vec2(1480.0, 920.0)).with_max_steps(80).build_eframe(move |cc| AtelierApp::with_data(cc, data.clone(), None));
        h.state_mut().dry_run = true;
        h.state_mut().st.graph.instant = true;
        setup(h.state_mut());
        h.run_steps(6);
        std::fs::create_dir_all("docs").unwrap();
        h.render().unwrap().save(format!("docs/{name}.png")).unwrap();
    }

    #[test]
    #[ignore]
    fn captures_readme() {
        let atlas = |app: &AtelierApp| app.data.projects.iter().position(|p| p.name == "Atlas");
        shot("bureau", |app| app.st.selected = atlas(app));
        shot("carte", |app| {
            app.st.screen = Screen::Library;
            app.st.lib_view = LibView::Map;
            app.st.selected = None;
        });
        shot("grille", |app| {
            app.st.screen = Screen::Library;
            app.st.lib_view = LibView::Grid;
            app.st.selected = atlas(app);
        });
    }
}
