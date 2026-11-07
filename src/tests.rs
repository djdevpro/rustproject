//! Tests d'interface (egui_kittest) : l'app tourne hors écran, on clique par le nom des widgets,
//! et on vérifie l'état de l'application après chaque action (sans GPU : ils tournent aussi en CI).
//!
//! En test, rien n'est réellement lancé ni redémarré (`dry_run`) : l'app garde seulement la trace
//! des lignes de commande.
//!

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

use crate::app::{AtelierApp, Modal, PanelTab, Screen, UpdatePhase};
use crate::icons::Icon;
use crate::model::{Attention, LauncherKind};

/// Taille par défaut de la fenêtre, et taille minimale (pour traquer les effets de bord).
const NORMAL: (f32, f32) = (1480.0, 920.0);
const MINI: (f32, f32) = (1180.0, 720.0);

fn harness_sized((w, h): (f32, f32)) -> Harness<'static, AtelierApp> {
    let mut harness = Harness::builder().with_size(egui::vec2(w, h)).with_max_steps(40).build_eframe(|cc| AtelierApp::demo(cc));
    harness.state_mut().dry_run = true;
    // La carte se pose d'un coup, sans animation : des captures identiques à chaque fois.
    harness.state_mut().st.graph.instant = true;
    harness.run();
    harness
}

fn harness() -> Harness<'static, AtelierApp> {
    harness_sized(NORMAL)
}

/// Clique le premier widget portant ce nom (certains noms existent deux fois : navigation et titre).
fn click(h: &mut Harness<'static, AtelierApp>, label: &str) {
    // Par l'accessibilité : fiable même dans une zone défilante d'une fenêtre modale, où appui et
    // relâchement simulés dans la même image ne suffisent pas.
    h.query_all_by_label(label).next().unwrap_or_else(|| panic!("aucun widget « {label} »")).click_accesskit();
    h.run();
}

/// Comme `click`, mais sans laisser le temps filer (pour lire une notification de 4 s).
fn click_now(h: &mut Harness<'static, AtelierApp>, label: &str) {
    h.query_all_by_label(label).next().unwrap_or_else(|| panic!("aucun widget « {label} »")).click_accesskit();
    h.run_steps(2);
}

#[test]
fn bureau() {
    let h = harness();
    // Le bureau s'ouvre sur un projet sélectionné, sans notification.
    assert_eq!(h.state().st.screen, Screen::Desk);
    assert!(h.state().st.selected.is_some() && h.state().toasts.is_empty());
}

#[test]
fn galerie_et_lanceur_sur_mesure() {
    let mut h = harness();
    click(&mut h, "Ajouter un lanceur");
    assert!(matches!(h.state().st.modal, Some(Modal::Gallery(_))));

    // Ajouter Cursor à ImmoFormation depuis la galerie.
    click(&mut h, "Ajouter Cursor");
    let p = h.state().st.selected.expect("un projet sélectionné");
    let names: Vec<String> = h.state().data.launchers_of(p).iter().map(|&id| h.state().data.launchers[id].name.clone()).collect();
    assert!(names.contains(&"Cursor".to_owned()), "{names:?}");

    // Onglet « Créer un lanceur », rempli comme le ferait l'utilisateur.
    click(&mut h, "Créer un lanceur");
    if let Some(Modal::Gallery(g)) = &mut h.state_mut().st.modal {
        g.draft.name = "Storybook".into();
        g.draft.command = "npm run storybook -- --port {port}".into();
        g.draft.icon = Icon::BookOpen;
        g.draft.color = crate::theme::rgb(0xff5c8a);
        g.draft.shortcut = "Ctrl+5".into();
    }
    h.run();
    click(&mut h, "Créer le lanceur");
    click(&mut h, "Terminé");
    assert!(h.state().st.modal.is_none());
}

#[test]
fn lancement_direct() {
    let mut h = harness();
    // Un vrai dossier pour le projet ouvert (celui de la démo n'existe que sur la machine de l'auteur).
    let dir = std::env::temp_dir().join(format!("atelier-lancement-{}", std::process::id())).join("ImmoFormation");
    std::fs::create_dir_all(&dir).unwrap();
    let p = h.state().st.selected.unwrap();
    h.state_mut().data.projects[p].folders = vec![dir.display().to_string()];
    h.run_steps(1);
    // Un lanceur part directement : pas de notification, juste la commande (et l'éclat de la tuile).
    click_now(&mut h, "VS Code");
    let line = h.state().launched.last().cloned().expect("une commande lancée");
    assert!(line.starts_with("code \"") && line.ends_with("ImmoFormation\""), "{line}");
    assert!(h.state().toasts.is_empty(), "pas de notification quand tout va bien");
    assert!(h.state().st.flash.is_some());

    // Claude Code : une nouvelle conversation dans Claude Desktop, sur le dossier (encodé).
    click_now(&mut h, "Claude Code");
    let line = h.state().launched.last().cloned().unwrap();
    let url = format!("claude://code/new?folder={}", crate::model::url_encode(&dir.display().to_string()));
    let expected = if cfg!(windows) {
        format!(r#"start "" "{url}""#)
    } else if cfg!(target_os = "macos") {
        format!(r#"open "{url}""#)
    } else {
        format!(r#"xdg-open "{url}""#)
    };
    assert_eq!(line, expected);
    assert!(!url.contains(['\\', ' ']), "{url}");

    // « Dans un terminal » : une fenêtre PowerShell (Windows), l'app Terminal (Mac) ou le terminal du système (Linux),
    // placée dans le dossier du projet.
    let reprise = crate::model::gallery().into_iter().find(|l| l.name == "Claude (reprise)").unwrap();
    let line = crate::launch::plan(&reprise, &h.state().data.projects[h.state().st.selected.unwrap()]).unwrap().line;
    if cfg!(windows) {
        assert!(line.starts_with("start \"Claude (reprise)\" /D ") && line.ends_with("powershell -NoExit -Command claude --continue"), "{line}");
    } else if cfg!(target_os = "macos") {
        assert!(line.starts_with("osascript -e ") && line.contains("&& claude --continue") && line.contains("Terminal"), "{line}");
    } else {
        assert!(line.starts_with("x-terminal-emulator -e sh -c 'claude --continue;"), "{line}");
    }

    // Dossier introuvable : rien n'est lancé, une notification explique pourquoi.
    let p = h.state().st.selected.unwrap();
    h.state_mut().data.projects[p].folders = vec![r"C:\nexiste\pas".into()];
    let before = h.state().launched.len();
    click_now(&mut h, "VS Code");
    assert_eq!(h.state().launched.len(), before);
    assert!(h.state().toasts.iter().any(|t| t.title.contains("n'a pas pu être lancé")));
}

#[test]
fn bibliotheque() {
    let mut h = harness();
    click(&mut h, "Bibliothèque");
    assert_eq!(h.state().st.screen, Screen::Library);
    click(&mut h, "Grille");
    click(&mut h, "Liste");
}

#[test]
fn a_revisiter() {
    let mut h = harness();
    // « À revisiter » n'est plus dans la barre du haut : on y va depuis « À redécouvrir ».
    assert!(h.query_by_label("À revisiter").is_none());
    click(&mut h, "Passer en revue");
    assert_eq!(h.state().st.screen, Screen::Revisit);
    click(&mut h, "Au fil de l'eau");
    assert_eq!(h.state().st.decisions.len(), 1);
}

#[test]
fn configuration() {
    let mut h = harness();
    click(&mut h, "Configuration");
    // Chaque section s'ouvre sans erreur.
    for label in ["Dossiers sources", "Découverte", "IA & modèles", "Ontologie", "États & attention", "Apparence", "Données"] {
        click(&mut h, label);
        assert_eq!(h.state().st.cfg.label(), label);
    }
}

#[test]
fn modifier_et_ressources() {
    let mut h = harness();
    click(&mut h, "Modifier");
    // Atelier a un long chemin : il doit être coupé au milieu, sans déborder du panneau.
    h.state_mut().st.selected = Some(0);
    h.state_mut().st.tab = PanelTab::Resources;
    h.run();
}

#[test]
fn focus_sans_limite_puis_avec() {
    let mut h = harness();
    // Par défaut, pas de limite : un 5e projet en focus, la rangée défile.
    click(&mut h, "API locale");
    click(&mut h, "Changer l'attention");
    click_item(&mut h, "Focus");
    assert_eq!(h.state().data.count(Attention::Focus), 5);
    assert!(h.state().toasts.is_empty(), "la pastille montre déjà le changement : pas de notification");

    // Avec la limite activée (4), un projet de plus est refusé, avec une explication.
    h.state_mut().data.config.limit_focus = true;
    click(&mut h, "Portfolio");
    click(&mut h, "Changer l'attention");
    h.query_all_by_role_and_label(egui::accesskit::Role::Button, "Focus").next().expect("l'entrée « Focus »").click_accesskit();
    h.run_steps(2);
    assert_eq!(h.state().data.count(Attention::Focus), 5);
    assert!(h.state().toasts.iter().any(|t| t.title.starts_with("Focus complet")));
}

/// Clique une entrée de menu, pas un simple texte qui porterait le même nom.
fn click_item(h: &mut Harness<'static, AtelierApp>, label: &str) {
    h.query_all_by_role_and_label(egui::accesskit::Role::Button, label).next().unwrap_or_else(|| panic!("aucune entrée « {label} »")).click_accesskit();
    h.run();
}

#[test]
fn etat_change_depuis_la_pastille() {
    let mut h = harness();
    let p = h.state().st.selected.expect("un projet ouvert");
    click(&mut h, "Changer l'état");
    click_item(&mut h, "En pause");
    assert_eq!(h.state().data.projects[p].status, crate::model::Status::Paused);
    assert!(h.state().toasts.is_empty());
}

#[test]
fn modifier_enregistre_tout_seul() {
    let mut h = harness();
    click(&mut h, "Modifier");
    let p = h.state().st.selected.expect("un projet ouvert");
    assert!(h.query_by_label("Enregistrer").is_none() && h.query_by_label("Annuler").is_none(), "plus de boutons");
    let edits = |h: &Harness<'static, AtelierApp>| h.state().data.projects[p].activity.iter().filter(|a| a.text == "Fiche modifiée").count();
    let before = edits(&h);

    // On tape : c'est enregistré tout de suite, sans notification.
    h.state_mut().st.draft.as_mut().expect("le formulaire").draft.name = "Nouveau nom".into();
    h.run();
    assert_eq!(h.state().data.projects[p].name, "Nouveau nom");
    h.state_mut().st.draft.as_mut().unwrap().draft.description = "Une autre phrase.".into();
    h.run();
    assert_eq!(h.state().data.projects[p].description, "Une autre phrase.");
    assert!(h.state().toasts.is_empty(), "pas de notification à chaque frappe");
    assert_eq!(edits(&h), before + 1, "une seule ligne de journal pour la séance");

    // Un nom effacé (en cours de frappe) ne vide pas la fiche.
    h.state_mut().st.draft.as_mut().unwrap().draft.name.clear();
    h.run();
    assert_eq!(h.state().data.projects[p].name, "Nouveau nom");

    // Changé ailleurs pendant que l'onglet est ouvert : le formulaire suit, sans rien annuler.
    let target = if h.state().data.projects[p].attention == Attention::Reserve { Attention::Flow } else { Attention::Reserve };
    h.state_mut().data.projects[p].attention = target;
    h.run();
    assert_eq!(h.state().data.projects[p].attention, target);
    assert_eq!(h.state().st.draft.as_ref().unwrap().draft.attention, target);
}

#[test]
fn choisir_une_icone() {
    let mut h = harness();
    click(&mut h, "Modifier");
    let p = h.state().st.selected.unwrap();
    h.get_by_label("Icône du projet").scroll_to_me();
    h.run();
    click(&mut h, "Icône du projet");
    click_item(&mut h, "rocket");
    assert!(crate::icons::same(h.state().data.projects[p].icon, Icon::Rocket));
}

#[test]
fn ontologie_editable_et_proposition() {
    let mut h = harness();
    h.state_mut().st.screen = Screen::Config;
    h.state_mut().st.cfg = crate::app::CfgSection::Ontology;
    h.run();
    let before = h.state().data.ontology.concepts.len();
    click(&mut h, "Ajouter un concept");
    assert_eq!(h.state().data.ontology.concepts.len(), before + 1);
    assert!(h.state().data.ontology.concepts.last().unwrap().manual, "créé à la main : gardé par l'IA");

    // Une proposition de l'IA, en attente : on la voit, puis on l'applique.
    let proposal = crate::ontology::from_json(&serde_json::json!({ "facettes": [
        { "nom": "Nature", "description": "Ce que c'est", "concepts": [
            { "nom": "Application desktop", "definition": "Un logiciel installé sur l'ordinateur.", "synonymes": ["bureau"] },
            { "nom": "Outil en ligne de commande", "definition": "Un programme pour le terminal.", "synonymes": ["cli"] }
        ]}
    ]}))
    .unwrap();
    h.state_mut().st.onto_proposal = Some(("qwen2.5:7b".into(), proposal));
    h.run();
    click(&mut h, "Remplacer (garde tes concepts manuels)");
    let o = &h.state().data.ontology;
    assert!(o.facet_named("Nature").is_some());
    assert!(o.concepts.iter().any(|c| c.name == "Nouveau concept"), "le concept manuel est resté");
    assert!(h.state().st.onto_proposal.is_none());
}

#[test]
fn depot_detecte_depuis_git() {
    let root = std::env::temp_dir().join(format!("atelier-depot-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".git")).unwrap();
    std::fs::write(root.join(".git").join("config"), "[remote \"origin\"]\n\turl = git@github.com:moi/projet.git\n").unwrap();
    let mut h = harness();
    click(&mut h, "Modifier");
    let p = h.state().st.selected.unwrap();
    h.state_mut().data.projects[p].remote = None;
    h.run();
    // Un nouveau dossier, dans un dépôt Git : l'adresse vient toute seule.
    h.state_mut().st.draft.as_mut().unwrap().draft.folders = vec![root.display().to_string()];
    h.run();
    assert_eq!(h.state().data.projects[p].remote.as_deref(), Some("https://github.com/moi/projet"));
    assert_eq!(h.state().st.draft.as_ref().unwrap().draft.remote, "https://github.com/moi/projet", "le champ l'affiche");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn mise_a_jour() {
    let mut h = harness();
    // Une version plus récente, telle que GitHub la décrirait (aucun réseau en test).
    let release = crate::release::Release {
        version: "9.9.0".into(),
        name: "Rust Project 9.9.0".into(),
        notes: vec!["Carte en graphe animé".into(), "Glisser un projet vers une attention".into(), "Mise à jour depuis les releases GitHub".into()],
        published: "4 nov. 2026".into(),
        page: "https://github.com/djdevpro/rustproject/releases/tag/v9.9.0".into(),
        prerelease: false,
        asset: Some(crate::release::Asset { name: crate::release::asset_name("9.9.0"), url: String::new(), size: 19_300_000, sha256: Some("00".into()) }),
        sums: None,
    };
    h.state_mut().st.updater.apply(Some(release));
    assert_eq!(h.state().st.updater.phase, UpdatePhase::Available);
    click(&mut h, "Mise à jour");
    assert!(matches!(h.state().st.modal, Some(Modal::Update)));
    // Téléchargée et vérifiée : prête à installer au redémarrage.
    h.state_mut().st.updater.phase = UpdatePhase::Ready;
    h.run();
    click(&mut h, "Redémarrer maintenant");
    assert!(h.state().restart_requested);
}

#[test]
fn theme_clair() {
    let mut h = harness();
    click(&mut h, "Configuration");
    click(&mut h, "Apparence");
    click(&mut h, "Clair");
    click(&mut h, "Mon bureau");
}

/// Fenêtre à sa taille minimale : rien ne doit se chevaucher.
#[test]
fn fenetre_etroite() {
    let mut h = harness_sized(MINI);
    click(&mut h, "Bibliothèque");
    click(&mut h, "Grille");
    click(&mut h, "Liste");
    click(&mut h, "Mon bureau");
    click(&mut h, "Passer en revue");
    click(&mut h, "Configuration");
}

/// Maintient le clic sur `label` et tire de `dx` points vers la droite (négatif : vers la gauche),
/// en plusieurs images, comme une vraie souris.
/// Un vrai glisser-déposer à la souris, d'un point à un autre, par petits pas.
fn drag_between(h: &mut Harness<'static, AtelierApp>, from: egui::Pos2, to: egui::Pos2) {
    let button = |pos, pressed| egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE };
    h.event(egui::Event::PointerMoved(from));
    h.event(button(from, true));
    h.step();
    for k in 1..=16 {
        h.event(egui::Event::PointerMoved(from + (to - from) * (k as f32 / 16.0)));
        h.step();
    }
    h.event(button(to, false));
    h.run_steps(3);
}

/// L'entrée « attention » de la colonne de gauche (le même nom existe ailleurs).
fn side_attention(h: &Harness<'static, AtelierApp>, label: &str) -> egui::Pos2 {
    h.query_all_by_label(label).map(|n| n.rect()).find(|r| r.right() < 260.0).unwrap_or_else(|| panic!("pas d'entrée « {label} » à gauche")).center()
}

#[test]
fn glisser_un_projet_vers_une_attention() {
    let mut h = harness();
    click(&mut h, "Bibliothèque");
    let attention_of = |h: &Harness<'static, AtelierApp>, name: &str| h.state().data.projects.iter().find(|p| p.name == name).unwrap().attention;
    // L'attention est en tête de la colonne, au-dessus des domaines.
    let focus = side_attention(&h, "Focus");
    let domains = side_attention(&h, "Tous les domaines");
    assert!(focus.y < domains.y, "le bloc Attention est tout en haut");

    // Grille : une carte déposée sur « Focus ».
    click(&mut h, "Grille");
    assert_eq!(attention_of(&h, "BunFlow"), Attention::Flow);
    let card = h.get_by_label("BunFlow").rect().center();
    let target = side_attention(&h, "Focus");
    drag_between(&mut h, card, target);
    assert_eq!(attention_of(&h, "BunFlow"), Attention::Focus, "déposé sur Focus depuis la grille");

    // Liste : une ligne déposée sur « En réserve ».
    click(&mut h, "Liste");
    let row = h.get_by_label("BunFlow").rect().center();
    let target = side_attention(&h, "En réserve");
    drag_between(&mut h, row, target);
    assert_eq!(attention_of(&h, "BunFlow"), Attention::Reserve, "déposé sur En réserve depuis la liste");

    // Carte : un nœud emporté hors du graphe, déposé sur « Au fil de l'eau ».
    click(&mut h, "Carte");
    let node = h.get_by_label("BunFlow").rect().center();
    let target = side_attention(&h, "Au fil de l'eau");
    drag_between(&mut h, node, target);
    assert_eq!(attention_of(&h, "BunFlow"), Attention::Flow, "déposé depuis la carte");
}

fn drag(h: &mut Harness<'static, AtelierApp>, label: &str, dx: f32) {
    let start = h.get_by_label(label).rect().center();
    let button = |pos, pressed| egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE };
    h.event(egui::Event::PointerMoved(start));
    h.event(button(start, true));
    h.step();
    let mut pos = start;
    for _ in 0..12 {
        pos.x += dx / 12.0;
        h.event(egui::Event::PointerMoved(pos));
        h.step();
    }
    h.event(button(pos, false));
    h.run();
}

#[test]
fn glisser_pour_faire_defiler() {
    let mut h = harness();
    // Dix projets en focus : la rangée de cartes compactes déborde.
    for i in 4..10 {
        h.state_mut().data.projects[i].attention = Attention::Focus;
    }
    h.run();
    // Une carte en focus qui n'est pas ouverte dans le panneau (son nom n'y est qu'une fois).
    let selected = h.state().st.selected;
    let card = h.state().data.focus().into_iter().find(|&i| Some(i) != selected).unwrap();
    let first = h.state().data.projects[card].name.clone();
    let before = h.query_all_by_label(&first).map(|n| n.rect().left()).fold(f32::MAX, f32::min);
    drag(&mut h, &first, -240.0);
    let after = h.query_all_by_label(&first).map(|n| n.rect().left()).fold(f32::MAX, f32::min);
    assert!(after < before - 100.0, "la rangée focus doit défiler : {before} → {after}");
    // Glisser n'est pas cliquer : la carte n'a pas été sélectionnée.
    assert_eq!(h.state().st.selected, Some(2));

    // Même chose sur la rangée de lanceurs : elle défile, et rien n'est lancé.
    let before = h.get_by_label("VS Code").rect().left();
    drag(&mut h, "VS Code", -150.0);
    let after = h.get_by_label("VS Code").rect().left();
    assert!(after < before - 60.0, "la rangée de lanceurs doit défiler : {before} → {after}");
    assert!(h.state().launched.is_empty(), "un glisser ne doit rien lancer");
}

#[test]
fn etapes_ajouter_cocher_retirer() {
    let mut h = harness();
    let p = h.state().st.selected.unwrap();
    let before = h.state().data.projects[p].steps.len();
    // Ajouter une étape (champ + bouton « Ajouter l'étape »).
    h.state_mut().st.new_step = "Tourner la vidéo de présentation".into();
    h.run();
    click(&mut h, "Ajouter l'étape");
    assert_eq!(h.state().data.projects[p].steps.len(), before + 1);
    assert_eq!(h.state().data.projects[p].steps.last().unwrap().text, "Tourner la vidéo de présentation");
    // La cocher, puis la retirer.
    click(&mut h, "Tourner la vidéo de présentation");
    assert!(h.state().data.projects[p].steps.last().unwrap().done);
    click(&mut h, "Retirer « Tourner la vidéo de présentation »");
    assert_eq!(h.state().data.projects[p].steps.len(), before);
    assert!(h.state().data.projects[p].activity.iter().any(|a| a.text.contains("Étape retirée")), "le journal garde la trace");
}

#[test]
fn supprimer_un_projet() {
    let mut h = harness();
    let before = h.state().data.projects.len();
    click(&mut h, "Modifier");
    click(&mut h, "Supprimer le projet");
    assert!(matches!(h.state().st.modal, Some(Modal::ConfirmDelete(_))));
    click(&mut h, "Supprimer");
    assert_eq!(h.state().data.projects.len(), before - 1);
    assert!(!h.state().data.projects.iter().any(|p| p.name == "ImmoFormation"));
    assert_eq!(h.state().st.selected, None);
}

#[test]
fn recherche_floue_sur_le_bureau() {
    let mut h = harness();
    h.state_mut().st.search = "imo".into();
    h.run();
    let hits = h.state().st.search_hits.clone().expect("des résultats");
    let first = &h.state().data.projects[hits[0].index];
    assert_eq!(first.name, "ImmoFormation");
}

/// Premier lancement sur une vraie arborescence : scan réel en arrière-plan, fiches créées par
/// les règles locales, fichier enregistré.
/// Aucun dossier source n'est choisi à la place de l'utilisateur : l'accueil le lui demande,
/// et aucun scan ne part tout seul.
#[test]
fn premier_lancement_sans_dossier_source() {
    let base = std::env::temp_dir().join(format!("atelier-sans-source-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let store_path = base.join("atelier.json");
    let mut h = Harness::builder()
        .with_size(egui::vec2(NORMAL.0, NORMAL.1))
        .with_max_steps(40)
        .build_eframe(move |cc| AtelierApp::new_with_store(cc, crate::store::Store::open(&store_path)));
    h.state_mut().dry_run = true;
    h.run_steps(4);
    assert!(h.state().data.config.roots.is_empty(), "pas de dossier source par défaut");
    assert!(h.state().st.scan_started.is_none(), "pas de scan sans dossier source");
    h.get_by_label("Choisir un dossier source");
    let markers = &h.state().data.config.markers;
    for m in ["CLAUDE.md", "AGENTS.md", "AGENT.md", "README.md"] {
        assert!(markers.iter().any(|(n, on)| n == m && *on), "{m} coché par défaut");
    }
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn premier_lancement_scan_reel_et_enregistrement() {
    let base = std::env::temp_dir().join(format!("atelier-e2e-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let root = base.join("projets");
    for (dir, file, content) in [
        ("site-vitrine", "package.json", r#"{"name":"site","description":"Le site de la boutique","dependencies":{"next":"15"}}"#),
        ("outil-rust", "Cargo.toml", "[package]\nname = \"outil\"\ndescription = \"Un petit outil en ligne de commande\"\n"),
        ("clients/client-a", ".git", ""),
    ] {
        std::fs::create_dir_all(root.join(dir)).unwrap();
        if dir.ends_with("client-a") {
            std::fs::create_dir_all(root.join(dir).join(file)).unwrap();
        } else {
            std::fs::write(root.join(dir).join(file), content).unwrap();
        }
    }
    std::fs::create_dir_all(root.join("photos-vacances")).unwrap();
    let store_path = base.join("data").join("atelier.json");

    let root_str = root.display().to_string();
    let path_for_app = store_path.clone();
    let mut h = Harness::builder()
        .with_size(egui::vec2(NORMAL.0, NORMAL.1))
        .with_max_steps(40)
        .build_eframe(move |cc| {
            let mut data = crate::model::Data::fresh();
            // Sans IA : les règles locales créent les fiches dès la fin du scan.
            (data.config.ollama_on, data.config.mcp_on) = (false, false);
            data.config.roots = vec![crate::model::ScanRoot { path: root_str.clone(), ..Default::default() }];
            AtelierApp::with_data(cc, data, Some(crate::store::Store::open(&path_for_app)))
        });
    h.state_mut().dry_run = true;
    // L'app relit son fichier chaque seconde : elle ne « se stabilise » jamais, on avance pas à pas.
    h.run_steps(4);
    assert!(store_path.exists(), "le fichier est créé dès le premier lancement");
    h.get_by_label("Scanner mes dossiers").click_accesskit();
    h.run_steps(2);
    // Le scan puis le classement tournent dans des fils : on attend leurs messages.
    for _ in 0..400 {
        let done = h.state().st.scan.is_some() && h.state().st.analysis.as_ref().is_some_and(|a| a.finished);
        if done {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
        h.step();
    }
    let names: Vec<String> = h.state().data.projects.iter().map(|p| p.name.clone()).collect();
    for n in ["site-vitrine", "outil-rust", "client-a"] {
        assert!(names.contains(&n.to_owned()), "{n} manque : {names:?}");
    }
    assert!(!names.contains(&"photos-vacances".to_owned()) && !names.contains(&"clients".to_owned()), "{names:?}");
    let outil = h.state().data.projects.iter().find(|p| p.name == "outil-rust").unwrap();
    assert_eq!(outil.description, "Un petit outil en ligne de commande");
    // L'enregistrement se fait dans la seconde (temps simulé).
    h.run_steps(8);
    let saved = crate::store::Store::open(&store_path).load().unwrap().unwrap();
    assert_eq!(saved.projects.len(), 3);
    let _ = std::fs::remove_dir_all(&base);
}

/// Mode Ollama sans modèle : le scan ne crée rien tout seul, l'accueil propose de choisir.
#[test]
fn sans_modele_ollama_le_scan_attend_un_choix() {
    let base = std::env::temp_dir().join(format!("atelier-choix-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(base.join("projets/outil")).unwrap();
    std::fs::write(base.join("projets/outil/Cargo.toml"), "[package]\nname=\"outil\"\n").unwrap();
    let root = base.join("projets").display().to_string();
    let store = base.join("atelier.json");
    let mut h = Harness::builder().with_size(egui::vec2(NORMAL.0, NORMAL.1)).with_max_steps(40).build_eframe(move |cc| {
        let mut data = crate::model::Data::fresh();
        data.config.roots = vec![crate::model::ScanRoot { path: root.clone(), ..Default::default() }];
        AtelierApp::with_data(cc, data, Some(crate::store::Store::open(&store)))
    });
    h.state_mut().dry_run = true;
    h.run_steps(2);
    h.get_by_label("Scanner mes dossiers").click_accesskit();
    for _ in 0..300 {
        if h.state().st.scan.is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
        h.step();
    }
    h.run_steps(3);
    assert!(h.state().data.projects.is_empty(), "rien de créé sans modèle choisi");
    h.get_by_label("Créer les 1 fiches (règles)").click_accesskit();
    h.run_steps(2);
    assert_eq!(h.state().data.projects.len(), 1);
    let _ = std::fs::remove_dir_all(&base);
}

/// La bordure gauche du panneau projet est au même endroit sur les quatre onglets : aucun onglet
/// ne déborde (un débordement élargit le panneau à gauche et rogne sa bordure).
/// Il dessine l'image : il lui faut un GPU, donc il tourne en local (`cargo test -- --ignored`).
#[test]
#[ignore = "demande un GPU"]
fn bordure_du_panneau_identique_sur_tous_les_onglets() {
    // Le rendu demande un GPU : sans (CI Linux), le test ne vérifie rien.
    let edge = |tab: PanelTab| {
        let mut h = harness();
        h.state_mut().st.tab = tab;
        h.run_steps(3);
        let image = h.render().ok()?;
        let start = (NORMAL.0 - crate::theme::PANEL_W - 30.0) as u32;
        // Première colonne claire (le trait de la bordure) sur deux hauteurs.
        Some([300u32, 600].map(|y| (start..start + 60).find(|&x| {
            let p = image.get_pixel(x, y);
            (u32::from(p[0]) + u32::from(p[1]) + u32::from(p[2])) / 3 > 35
        })))
    };
    let Some(reference) = edge(PanelTab::Resources) else { return };
    assert!(reference.iter().all(Option::is_some), "bordure trouvée");
    for tab in [PanelTab::Summary, PanelTab::Activity, PanelTab::Edit] {
        assert_eq!(edge(tab), Some(reference), "onglet {tab:?} : bordure décalée ou rognée");
    }
}

#[test]
fn archives_dans_le_bloc_attention() {
    let mut h = harness();
    click(&mut h, "Bibliothèque");
    click(&mut h, "Liste");
    let archived = h.state().data.archived();
    assert!(archived > 0);
    // « Archivés » montre les archivés seuls, même si un filtre d'attention était actif.
    let focus = side_attention(&h, "Focus");
    h.query_all_by_label("Focus").find(|n| n.rect().center() == focus).unwrap().click_accesskit();
    h.run();
    let archives = side_attention(&h, "Archivés");
    h.query_all_by_label("Archivés").find(|n| n.rect().center() == archives).unwrap().click_accesskit();
    h.run();
    assert!(h.state().st.lib_archived && h.state().st.lib_attention.is_none());
    let archived_names: Vec<String> = h.state().data.projects.iter().filter(|p| p.status == crate::model::Status::Archived).map(|p| p.name.clone()).collect();
    for name in &archived_names {
        assert!(h.query_by_label(name).is_some(), "« {name} » est listé");
    }
    // Déposé sur une attention, un archivé reprend du service.
    let row = h.get_by_label(&archived_names[0]).rect().center();
    let target = side_attention(&h, "Au fil de l'eau");
    drag_between(&mut h, row, target);
    let p = h.state().data.projects.iter().find(|p| p.name == archived_names[0]).unwrap();
    assert_eq!((p.status, p.attention), (crate::model::Status::Active, Attention::Flow));
    // Et un projet déposé sur « Archivés » est archivé.
    h.query_all_by_label("Archivés").find(|n| n.rect().center() == archives).unwrap().click_accesskit();
    h.run();
    let row = h.get_by_label("BunFlow").rect().center();
    let target = side_attention(&h, "Archivés");
    drag_between(&mut h, row, target);
    let p = h.state().data.projects.iter().find(|p| p.name == "BunFlow").unwrap();
    assert_eq!(p.status, crate::model::Status::Archived);
}

/// Ce qui dépasse à droite de la fenêtre (rôle, nom, bord droit), sur l'écran affiché.
fn overflowing(h: &Harness<'static, AtelierApp>, width: f32) -> Vec<String> {
    use egui_kittest::kittest::NodeT as _;
    let mut out = Vec::new();
    let root = h.root();
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if let Some(b) = node.accesskit_node().bounding_box()
            && b.x1 > f64::from(width) + 0.5
            && b.x1 - b.x0 < f64::from(width)
        {
            out.push(format!("{:?} « {} » : {:.0} → {:.0}", node.accesskit_node().role(), node.accesskit_node().label().unwrap_or_default(), b.x0, b.x1));
        }
        stack.extend(node.children());
    }
    out
}

/// Dans la plus petite fenêtre permise, rien ne dépasse à droite des pages : sinon les
/// boutons alignés à droite (Copier, Tester, Télécharger…) sortent de la fenêtre. Avec des valeurs
/// longues (chemins, modèles) et chaque mode d'IA.
#[test]
fn aucun_ecran_ne_deborde_dans_une_petite_fenetre() {
    use crate::app::{CfgSection, LibView};
    let long_models = || {
        Some(Ok(["qwen3-embedding:4b", "winnow:latest", "un-modele-au-nom-vraiment-tres-long-pour-voir:latest-instruct-q4_K_M"]
            .map(|n| crate::ollama::ModelInfo { name: n.into(), size: 12_700_000_000, family: "qwen3".into(), parameters: "4B".into() })
            .to_vec()))
    };
    let mut problems = Vec::new();
    let mut check = |name: String, setup: &dyn Fn(&mut AtelierApp)| {
        let mut h = harness_sized(MINI);
        {
            let app = h.state_mut();
            app.st.store_path = Some(r"C:\Users\Quelquun\AppData\Roaming\Atelier\un-dossier-de-donnees-au-nom-tres-long\atelier.json".into());
            app.st.models = long_models();
            app.st.ollaya_models = long_models();
            app.data.config.ollama_embed_model = "qwen3-embedding:4b".into();
            setup(app);
        }
        h.run_steps(3);
        let found = overflowing(&h, MINI.0);
        if !found.is_empty() {
            problems.push(format!("{name} :\n    {}", found.iter().take(6).cloned().collect::<Vec<_>>().join("\n    ")));
        }
    };
    // Les pages qui défilent à la verticale (les rangées qui défilent à l'horizontale, lanceurs
    // du panneau et projets du moment, débordent exprès : elles sont laissées de côté).
    for view in [LibView::Map, LibView::Grid, LibView::List] {
        check(format!("Bibliothèque {view:?}"), &|app| {
            app.st.screen = Screen::Library;
            app.st.lib_view = view;
            app.st.selected = None;
        });
    }
    check("À revisiter".into(), &|app| app.st.screen = Screen::Revisit);
    for section in CfgSection::ALL {
        for (ollama, mcp) in [(true, true), (true, false), (false, true), (false, false)] {
            check(format!("Configuration {section:?} (Ollama {ollama}, MCP {mcp})"), &|app| {
                app.st.screen = Screen::Config;
                app.st.cfg = section;
                (app.data.config.ollama_on, app.data.config.mcp_on) = (ollama, mcp);
            });
        }
    }
    assert!(problems.is_empty(), "Débordements :\n{}", problems.join("\n"));
}

/// Un fichier d'avant : le lanceur « Claude Code » d'origine (`claude` dans un terminal) passe à
/// Claude Desktop ; un lanceur retouché à la main reste tel quel.
#[test]
fn ancien_lanceur_claude_code_vers_claude_desktop() {
    let mut data = crate::model::Data::fresh();
    let mut old = data.launchers[0].clone();
    old.kind = LauncherKind::Command;
    old.command = "claude".into();
    old.in_terminal = true;
    let mut custom = old.clone();
    custom.command = "claude --model opus".into();
    data.launchers = vec![old, custom];
    data.normalize();
    assert_eq!(data.launchers[0].kind, LauncherKind::Url);
    assert_eq!(data.launchers[0].command, "claude://code/new?folder={dossier_url}");
    assert!(!data.launchers[0].in_terminal);
    assert_eq!(data.launchers[1].command, "claude --model opus");
    assert_eq!(crate::model::url_encode(r"C:\Mes projets\é"), "C%3A%5CMes%20projets%5C%C3%A9");
}

/// La ligne de la liste (ou la carte de la grille) d'un projet : son nom existe aussi dans le
/// panneau quand il est ouvert ; la ligne est la plus large.
fn project_row(h: &Harness<'static, AtelierApp>, name: &str) -> egui::Rect {
    h.query_all_by_label(name).map(|n| n.rect()).max_by(|a, b| a.width().total_cmp(&b.width())).unwrap_or_else(|| panic!("pas de ligne « {name} »"))
}

fn click_with(h: &mut Harness<'static, AtelierApp>, at: egui::Rect, modifiers: egui::Modifiers) {
    h.event(egui::Event::PointerMoved(at.center()));
    h.event(egui::Event::ModifiersChanged(modifiers));
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton { pos: at.center(), button: egui::PointerButton::Primary, pressed, modifiers });
    }
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
    h.run();
}

/// Ctrl + clic et Maj + clic sélectionnent plusieurs projets ; glisser l'un d'eux emporte tout le
/// groupe vers une attention ou un domaine ; Échap vide la sélection.
#[test]
fn selection_multiple_et_glisser_groupe() {
    let mut h = harness();
    click(&mut h, "Bibliothèque");
    click(&mut h, "Liste");
    let names: Vec<String> = h.state().data.projects.iter().filter(|p| p.status != crate::model::Status::Archived && p.attention != Attention::Reserve).take(4).map(|p| p.name.clone()).collect();
    let r0 = project_row(&h, &names[0]);
    click_with(&mut h, r0, egui::Modifiers::NONE);
    let r1 = project_row(&h, &names[1]);
    click_with(&mut h, r1, egui::Modifiers::COMMAND);
    let r2 = project_row(&h, &names[2]);
    click_with(&mut h, r2, egui::Modifiers::COMMAND);
    assert_eq!(h.state().st.lib_multi.len(), 3, "un clic, puis deux Ctrl + clic");
    assert!(h.query_by_label("3 projets sélectionnés").is_some(), "la barre de la sélection");
    let attention_of = |h: &Harness<'static, AtelierApp>, name: &str| h.state().data.projects.iter().find(|p| p.name == *name).unwrap().attention;
    let from = project_row(&h, &names[1]).center();
    let to = side_attention(&h, "En réserve");
    drag_between(&mut h, from, to);
    for n in &names[..3] {
        assert_eq!(attention_of(&h, n), Attention::Reserve, "{n} a suivi le groupe");
    }
    assert_ne!(attention_of(&h, &names[3]), Attention::Reserve, "hors de la sélection");
    // Le même groupe, déposé sur un domaine.
    let domain = h.state().data.domains.len() - 1;
    let domain_name = h.state().data.domains[domain].name.clone();
    let from = project_row(&h, &names[0]).center();
    let to = side_attention(&h, &domain_name);
    drag_between(&mut h, from, to);
    // Déposé sur un domaine : le groupe le reçoit en plus, son domaine principal ne change pas.
    for n in &names[..3] {
        let i = h.state().data.projects.iter().position(|p| p.name == *n).unwrap();
        assert!(h.state().data.has_domain(i, domain), "{n} a reçu le domaine");
        assert_ne!(h.state().data.projects[i].domain, domain, "{n} garde son domaine principal");
    }
    h.key_press(egui::Key::Escape);
    h.run();
    assert!(h.state().st.lib_multi.is_empty(), "Échap vide la sélection");
}

/// Premier lancement : l'assistant s'ouvre seul, sans rien cocher. Un dossier coché est scanné,
/// les projets trouvés sont cochés à l'étape « Vérifier » ; celui qu'on décoche est écarté des
/// prochains scans, l'autre devient une fiche.
#[test]
fn assistant_dossiers_sources() {
    use crate::screens::setup::{Folder, Step};
    let base = std::env::temp_dir().join(format!("atelier-assistant-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let root = base.join("projets");
    std::fs::create_dir_all(root.join("outil")).unwrap();
    std::fs::write(root.join("outil").join("Cargo.toml"), "[package]\nname = \"outil\"\ndescription = \"Un petit outil\"\n").unwrap();
    std::fs::create_dir_all(root.join("site")).unwrap();
    std::fs::write(root.join("site").join("package.json"), r#"{"name":"site"}"#).unwrap();
    std::fs::create_dir_all(root.join("photos")).unwrap();
    let store_path = base.join("data").join("atelier.json");
    let mut h = Harness::builder()
        .with_size(egui::vec2(NORMAL.0, NORMAL.1))
        .with_max_steps(40)
        .build_eframe(move |cc| AtelierApp::new_with_store(cc, crate::store::Store::open(&store_path)));
    h.state_mut().dry_run = true;
    h.run_steps(4);
    let root_str = root.display().to_string();
    {
        let Some(Modal::Setup(s)) = &mut h.state_mut().st.modal else { panic!("l'assistant s'ouvre au premier lancement") };
        assert!(s.folders.iter().all(|f| !f.checked), "rien n'est coché d'office");
        // « Ajouter un autre dossier… » ouvre le sélecteur du système : on pose le dossier.
        s.folders.push(Folder::new("projets".into(), root_str.clone(), true, 3));
    }
    h.run_steps(2);
    h.get_by_label("Scanner ce dossier").click_accesskit();
    let step = |h: &Harness<'static, AtelierApp>| match &h.state().st.modal {
        Some(Modal::Setup(s)) => Some(s.step),
        _ => None,
    };
    for _ in 0..400 {
        if step(&h) == Some(Step::Review) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
        h.step();
    }
    assert_eq!(step(&h), Some(Step::Review), "le scan mène à « Vérifier »");
    assert_eq!(h.state().data.config.roots.len(), 1);
    assert!(h.state().data.projects.is_empty(), "rien n'est créé avant la validation");
    h.run_steps(2);
    h.get_by_label("Garder site").click_accesskit();
    h.run_steps(2);
    h.get_by_label("Suivant : créer 1 fiche").click_accesskit();
    h.run_steps(2);
    h.get_by_label("Créer la fiche").click_accesskit();
    h.run_steps(3);
    let names: Vec<String> = h.state().data.projects.iter().map(|p| p.name.clone()).collect();
    assert_eq!(names, vec!["outil".to_owned()]);
    let site = crate::model::path_key(&root.join("site").display().to_string());
    assert!(h.state().data.config.ignored.iter().any(|p| crate::model::path_key(p) == site), "le dossier décoché est écarté");
    assert!(h.query_by_label("1 fiche créée").is_some());
    let _ = std::fs::remove_dir_all(&base);
}

/// Changer le chemin d'un dossier source à la main relance le scan quand on valide.
#[test]
fn chemin_modifie_relance_le_scan() {
    let base = std::env::temp_dir().join(format!("atelier-chemin-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(base.join("a")).unwrap();
    std::fs::create_dir_all(base.join("b").join("outil")).unwrap();
    std::fs::write(base.join("b").join("outil").join("Cargo.toml"), "[package]\nname = \"outil\"\n").unwrap();
    let mut h = harness();
    h.state_mut().data.config.roots = vec![crate::model::ScanRoot { path: base.join("a").display().to_string(), last_scan: 1, ..Default::default() }];
    h.state_mut().st.screen = Screen::Config;
    h.state_mut().st.cfg = crate::app::CfgSection::Sources;
    h.run();
    let field = h.query_all_by_role(egui::accesskit::Role::TextInput).next().unwrap().rect();
    click_with(&mut h, field, egui::Modifiers::NONE);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.event(egui::Event::Text(base.join("b").display().to_string()));
    h.run_steps(2);
    assert_eq!(h.state().data.config.roots[0].last_scan, 0, "les chiffres de l'ancien chemin sont effacés");
    h.key_press(egui::Key::Enter);
    h.run_steps(2);
    for _ in 0..300 {
        if h.state().st.scan.is_some() && h.state().st.scan_started.is_none() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
        h.step();
    }
    let scan = h.state().st.scan.as_ref().expect("le scan est reparti");
    assert!(scan.dirs.iter().any(|d| d.name == "outil"), "le nouveau dossier est relevé");
    let _ = std::fs::remove_dir_all(&base);
}

/// Plusieurs domaines par projet : le principal, ceux des tags ; un tag retiré ne revient pas au
/// classement suivant ; l'étoile garde l'ancien domaine principal en plus.
#[test]
fn domaines_multiples_et_tags() {
    let mut data = crate::model::demo();
    data.ensure_ontology();
    let (p, main) = (0, data.projects[0].domain);
    let other = (0..data.domains.len()).find(|&d| d != main).unwrap();
    assert_eq!(data.domains_of(p), vec![main]);
    data.add_domain(p, other);
    assert_eq!(data.domains_of(p), vec![main, other], "le principal d'abord");
    assert!(data.has_domain(p, other) && data.projects[p].domain == main, "le principal ne change pas");
    // L'étoile : l'autre devient principal, l'ancien reste.
    data.set_main_domain(p, other, true);
    assert_eq!(data.domains_of(p), vec![other, main]);
    // Retirer le domaine principal : le suivant prend sa place.
    let c = data.domain_concept(other).unwrap();
    data.untag(p, c);
    assert_eq!(data.domains_of(p), vec![main]);
    // Le seul domaine ne se retire pas.
    let c_main = data.domain_concept(main).unwrap();
    data.untag(p, c_main);
    assert_eq!(data.domains_of(p), vec![main]);
    // Un tag retiré reste noté : un classement qui le propose ne le remet pas.
    data.add_domain(p, other);
    data.untag(p, c);
    let fp = data.ontology.fingerprint();
    data.apply_scores(p, &[(c, 0.9)], fp);
    assert!(!data.has_domain(p, other), "retiré pour de bon");
    // Créer un tag au premier niveau de « Domaine » crée un domaine.
    let facet = data.ontology.facet_named(crate::ontology::DOMAIN_FACET).unwrap();
    let before = data.domains.len();
    let id = data.create_concept(facet, "Jeux de société", None);
    assert_eq!(data.domains.len(), before + 1);
    data.tag(p, id);
    assert!(data.domains_of(p).contains(&before));
    // L'autocomplétion : par le nom, sans ce qui est déjà rattaché.
    assert!(data.suggest_concepts(p, "jeux", 5).iter().all(|c| c.id != id), "déjà rattaché : pas proposé");
    assert_eq!(data.suggest_concepts(1, "jeux de", 5).first().map(|c| c.id), Some(id));
}

/// Les mots-clés font trouver un projet que sa description ne décrit pas (« emploi »).
#[test]
fn mots_cles_dans_la_recherche() {
    let mut data = crate::model::demo();
    let p = data.projects.iter().position(|p| !crate::search::fold(&p.search_text("")).contains("emploi")).unwrap();
    let id = data.projects[p].id;
    let found = |data: &crate::model::Data| crate::search::rank(data, "emploi", None, &Default::default()).iter().any(|h| data.projects[h.index].id == id);
    assert!(!found(&data));
    data.projects[p].keywords = crate::classify::clean_keywords(vec!["emploi".into(), " Entretien d'embauche ".into(), "EMPLOI".into(), String::new()]);
    assert_eq!(data.projects[p].keywords, vec!["emploi".to_owned(), "Entretien d'embauche".to_owned()], "nettoyés, sans doublon");
    assert!(found(&data), "trouvé par ses mots-clés");
    assert!(crate::search::embed_text(&data, &data.projects[p]).contains("Entretien d'embauche"), "ils entrent dans l'embedding");
}

/// Un logo SVG : collé ou déposé, il remplace l'icône ; il est gardé dans le fichier ; un texte
/// qui n'est pas du SVG est refusé.
#[test]
fn logo_svg_des_lanceurs() {
    const LOGO: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><circle cx="12" cy="12" r="10" fill="#e07a5f"/></svg>"##;
    assert!(crate::launchers::check_svg("pas du svg").is_err());
    assert_eq!(crate::launchers::check_svg(&format!("  {LOGO}\n")).unwrap(), LOGO);
    let mut h = harness();
    let id = h.state().data.default_launchers[0];
    h.state_mut().data.launchers[id].svg = LOGO.into();
    let json = serde_json::to_string(&h.state().data).unwrap();
    let back: crate::model::Data = serde_json::from_str(&json).unwrap();
    assert_eq!(back.launchers[id].svg, LOGO, "gardé dans le fichier");
    // Le panneau dessine le logo à la place de l'icône (sans erreur de chargement).
    h.run_steps(3);
    let name = h.state().data.launchers[id].name.clone();
    assert!(h.query_by_label(&name).is_some());
}

/// L'onglet « Modifier » : taper un mot propose les tags de l'ontologie ; un clic en ajoute un.
#[test]
fn ajouter_un_tag_depuis_la_fiche() {
    let mut h = harness();
    h.state_mut().data.ensure_ontology();
    let p = h.state().st.selected.unwrap();
    let main = h.state().data.projects[p].domain;
    let other = (0..h.state().data.domains.len()).find(|&d| d != main).unwrap();
    let name = h.state().data.domains[other].name.clone();
    h.state_mut().st.tab = PanelTab::Edit;
    h.state_mut().st.tag_query = name.chars().take(5).collect();
    h.run();
    // La proposition est plus bas dans le panneau (hors de l'écran) : clic par l'accessibilité.
    h.query_all_by_label_contains(&name).find(|n| n.rect().left() > 900.0).expect("le domaine est proposé").click_accesskit();
    h.run();
    assert!(h.state().data.has_domain(p, other), "ajouté comme domaine en plus");
    assert_eq!(h.state().data.projects[p].domain, main, "le principal ne change pas");
}


/// Les paquets : un bloc par gestionnaire dans la colonne de la Bibliothèque ; un clic filtre ;
/// la recherche trouve les projets par le nom d'un paquet (« expo »).
#[test]
fn filtrer_et_chercher_par_paquet() {
    use crate::packages::{Manager, Package};
    let mut h = harness();
    let expo = Package { manager: Manager::Npm, name: "expo".into() };
    for i in [0, 2] {
        h.state_mut().data.projects[i].packages = vec![expo.clone(), Package { manager: Manager::Npm, name: "react".into() }];
    }
    h.state_mut().data.projects[1].packages = vec![Package { manager: Manager::Cargo, name: "egui".into() }];
    let names: Vec<String> = [0, 2].iter().map(|&i| h.state().data.projects[i].name.clone()).collect();
    click(&mut h, "Bibliothèque");
    click(&mut h, "Liste");
    // Les gestionnaires sont repliés : on ouvre npm.
    assert!(h.query_all_by_label("expo").all(|n| n.rect().right() >= 260.0), "npm replié par défaut");
    let npm = side_attention(&h, "npm");
    click_with(&mut h, egui::Rect::from_center_size(npm, egui::vec2(10.0, 10.0)), egui::Modifiers::NONE);
    let item = side_attention(&h, "expo");
    click_with(&mut h, egui::Rect::from_center_size(item, egui::vec2(10.0, 10.0)), egui::Modifiers::NONE);
    assert_eq!(h.state().st.lib_package.as_ref(), Some(&expo));
    let shown: Vec<String> = h.state().data.projects.iter().filter(|p| h.query_all_by_label(&p.name).any(|n| n.rect().width() > 300.0)).map(|p| p.name.clone()).collect();
    for n in &names {
        assert!(shown.contains(n), "{n} utilise expo");
    }
    assert!(!shown.contains(&h.state().data.projects[1].name), "pas d'expo dans celui-ci");
    // La recherche par le nom du paquet.
    let hits = crate::search::rank(&h.state().data, "expo", None, &Default::default());
    let found: Vec<String> = hits.iter().map(|x| h.state().data.projects[x.index].name.clone()).collect();
    for n in &names {
        assert!(found.contains(n), "{n} trouvé par « expo » : {found:?}");
    }
}



/// Ouvrir la fenêtre de mise à jour lance la vérification tout de suite.
#[test]
fn ouvrir_la_mise_a_jour_verifie_tout_de_suite() {
    let mut h = harness();
    // Aucun réseau : un serveur qui ne répond pas, la vérification échoue vite.
    h.state_mut().st.updater.api = "http://127.0.0.1:9".into();
    h.state_mut().dry_run = false;
    assert_eq!(h.state().st.updater.phase, UpdatePhase::Idle);
    click_now(&mut h, "Mise à jour");
    assert!(matches!(h.state().st.modal, Some(Modal::Update)));
    assert_ne!(h.state().st.updater.phase, UpdatePhase::Idle, "la vérification est partie");
    h.state_mut().dry_run = true;
}


/// F12 ouvre les outils de développement ; l'onglet « État » décrit l'application.
#[test]
fn outils_de_developpement() {
    let mut h = harness();
    assert!(!h.state().st.devtools.open);
    // L'onglet « État » (les écrans d'egui utilisent des pictogrammes absents des polices de test).
    h.state_mut().st.devtools.tab = crate::devtools::Tab::State;
    h.key_press(egui::Key::F12);
    h.run();
    assert!(h.state().st.devtools.open, "F12 ouvre les outils");
    assert!(h.query_by_label("Inspecteur").is_some(), "les onglets");
    assert!(h.query_by_label("Copier l'état").is_some());
    h.key_press(egui::Key::F12);
    h.run();
    assert!(!h.state().st.devtools.open, "F12 les referme");
}


/// Le logo d'un projet : un SVG gardé tel quel (ses couleurs), une image réduite à 128 px et
/// gardée en PNG ; dessiné à la place de l'icône ; « Retirer le logo » revient à l'icône.
#[test]
fn logo_du_projet() {
    use crate::model::Logo;
    use base64::Engine as _;
    const SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><circle cx="12" cy="12" r="10" fill="#e07a5f"/><path d="M4 12h16" stroke="currentColor"/></svg>"##;
    assert_eq!(crate::logo::from_bytes(SVG.as_bytes(), "svg").unwrap(), Logo::Svg(SVG.into()), "couleurs gardées");
    assert_eq!(crate::logo::from_bytes(SVG.as_bytes(), "").unwrap(), Logo::Svg(SVG.into()), "repéré sans extension");
    let mut png = std::io::Cursor::new(Vec::new());
    image::RgbaImage::from_pixel(300, 150, image::Rgba([200, 80, 40, 255])).write_to(&mut png, image::ImageFormat::Png).unwrap();
    let Logo::Png(b64) = crate::logo::from_bytes(png.get_ref(), "png").unwrap() else { panic!("une image") };
    let back = image::load_from_memory(&base64::engine::general_purpose::STANDARD.decode(b64).unwrap()).unwrap();
    assert_eq!((back.width(), back.height()), (128, 64), "réduite, proportions gardées");
    assert!(crate::logo::from_bytes(b"pas une image", "png").is_err());
    assert!(crate::logo::from_bytes(b"x", "pdf").is_err());
    assert!(!serde_json::to_string(&crate::model::Project::default()).unwrap().contains("\"logo\""), "rien d'écrit sans logo");

    let mut h = harness();
    let p = h.state().st.selected.unwrap();
    h.state_mut().data.projects[p].logo = Logo::Svg(SVG.into());
    h.state_mut().data.projects[(p + 1) % 3].logo = crate::logo::from_bytes(png.get_ref(), "png").unwrap();
    let json = serde_json::to_string(&h.state().data).unwrap();
    let back: crate::model::Data = serde_json::from_str(&json).unwrap();
    assert_eq!(back.projects[p].logo, Logo::Svg(SVG.into()), "gardé dans le fichier");
    // Dessiné au bureau et dans le panneau, puis retiré depuis l'onglet « Modifier ».
    h.state_mut().st.tab = PanelTab::Edit;
    h.run();
    h.get_by_label("Retirer le logo").click_accesskit();
    h.run();
    assert!(h.state().data.projects[p].logo.is_none(), "retour à l'icône");
}
