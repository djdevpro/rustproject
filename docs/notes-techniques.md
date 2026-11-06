# Atelier — gestionnaire de projets personnels

Application de bureau en Rust (egui 0.37). Elle découvre tes projets dans tes dossiers, les fait
classer par une IA (Ollama, ou un agent via MCP), et les range sur un bureau avec étapes, notes et
lanceurs (VS Code, Claude Code, terminal…).

```bash
cargo run                      # l'application
cargo run -- --mcp             # le serveur MCP (sans interface), pour un agent IA
cargo test                     # tous les tests : moteur + interface (sans GPU)
```

## Où sont les données

- `%APPDATA%\Atelier\atelier.json` : projets, étapes, notes, réglages. Écrit à chaque changement,
  de façon atomique (fichier temporaire puis renommage). Variable `ATELIER_DATA` pour en changer.
- `%APPDATA%\Atelier\embeddings.json` : l'index de la recherche sémantique.
- L'interface relit le fichier chaque seconde : ce que l'agent MCP écrit apparaît tout seul.
- Configuration › Données : bascule vers des données de démonstration (rien n'est enregistré).

## Comment les projets sont trouvés

1. **Dossiers sources** (Configuration) : autant que tu veux, chacun avec sa profondeur (1 à 3).
   Aucun par défaut, et l'agent IA ne peut pas en ajouter : c'est toi qui les choisis.
2. **Le scan** (`scan.rs`) parcourt seulement des dossiers, niveau par niveau, en sautant les
   dossiers cachés et les exclusions (`node_modules`, `target`…). Pour chaque dossier, il relève :
   marqueurs (`.git`, `Cargo.toml`, `package.json`…), sous-dossiers, dernière activité, début du
   README, et ce que disent les manifestes (nom, description, technos). Il ne descend pas dans un
   dossier déjà reconnu comme projet (le README seul, indice faible, n'arrête pas la descente).
   Marqueurs par défaut : `.git`, manifestes, `CLAUDE.md`, `AGENTS.md`, `AGENT.md`, `README.md`.
   Sur `Projet perso` : 399 dossiers en ~2 s.
   **Briques** : `frontend`, `api`, `db`, `site`, `packages/x`… sont rattachées à leur projet
   parent (qui devient le projet, avec leurs technos) au lieu de devenir des fiches à part.
   **Dossiers écartés** (copies, sauvegardes) : bouton « Écarter » ou outil MCP ; le scan les ignore.
3. **Le tri**, au choix (Configuration › IA & taxonomie) :
   - **Ollama** : chaque dossier est résumé et envoyé au modèle choisi, qui répond en JSON
     contraint par un schéma (projet ou pas, nom, description en **une phrase**, domaine, type,
     technos). Une réponse illisible retombe sur les règles.
   - **Agent MCP** : l'agent (Claude Code…) appelle `scanner_dossiers`, décide, puis
     `ajouter_projet`, en suivant la méthode que le serveur lui donne (voir plus bas). Rien
     d'automatique côté Atelier.
   - **Sans IA** : un dossier avec un marqueur devient un projet, décrit par son manifeste ou la
     première phrase de son README.
   En mode Ollama sans modèle choisi, rien n'est créé automatiquement : l'accueil propose le choix.
4. **GitHub** : tous tes dépôts (les tiens, organisations, collaborations) via la CLI `gh` ; un
   dépôt sans dossier local devient une fiche « GitHub ».
5. **À la main** : « Ajouter un projet » → sélecteur de dossier → analyse → fiche modifiable.

## Ollama

Configuration › IA : adresse du serveur, « Tester et lister » (l'équivalent de `ollama list`),
choix du **modèle de langage** (classement) et du **modèle d'embeddings** (recherche),
téléchargement de modèles avec progression. Conseillés : `qwen2.5:3b` (classement, ≈ 1,9 Go) et
`qwen3-embedding:4b` (embeddings, le meilleur en français pour sa taille). Les embeddings sont
demandés avec un contexte de 2 048 jetons : `qwen3-embedding:4b` prend ≈ 4 Go de mémoire graphique
au lieu de ≈ 10 Go avec le contexte par défaut d'Ollama.

## Ontologie et Ollaya

- **L'ontologie** (Configuration › Ontologie) : des facettes (axes de rangement : domaine, nature,
  finalité…) et leurs concepts, chacun avec une définition d'une phrase, des synonymes et un parent.
  Les anciens domaines deviennent la facette « Domaine ». Tout est éditable à la main ; un concept
  retouché devient « manuel » et les propositions suivantes de l'IA le gardent.
- **La proposition par l'IA** : « Proposer avec l'IA » envoie toutes les fiches d'un coup au modèle
  de langage d'Ollama, qui propose une ontologie. Rien n'est appliqué sans accord : « Remplacer
  (garde tes concepts manuels) », « Ajouter seulement les nouveaux » ou « Ignorer ». Un agent MCP
  peut faire de même (`lire_ontologie`, `definir_ontologie`, `modifier_concept`).
- **Le classement par Ollaya** (serveur de modèles de décision, `http://127.0.0.1:11435`, réglé dans
  IA & modèles) : pour chaque projet, une question oui/non par concept, la définition en critère.
  À partir de 50 %, le projet est rattaché ; le meilleur concept de la facette « Domaine » devient
  son domaine. Le classement ne part que sur un clic (pause, reprise, arrêt possibles) ; après un changement d'ontologie,
  « Classer N projets ». Conseillé : `decider:2b` (≈ 0,5 s par projet avec une douzaine de concepts).
  Côté MCP : `lister_modeles_decision`, `classer_projets`, `rattacher_concept`.
- Essai réel sur une copie des données :
  `ATELIER_ESSAI=C:\chemin\copie\atelier.json cargo test essai_reel_ollaya -- --ignored --nocapture`.

## Recherche

Hybride : par les mots (flou), par le sens (embeddings), et par l'ontologie : « logement » trouve le
concept « Immobilier » (synonyme, ou sens proche), puis les projets qui y sont rattachés (« via Immobilier »).

- **Floue**, toujours active : « imo » trouve ImmoFormation (lettres dans l'ordre), les lettres
  inversées et les accents sont tolérés.
- **Sémantique**, avec un modèle d'embeddings : chaque fiche devient un vecteur ; la requête
  aussi, dès qu'on arrête de taper. « logement » trouve ImmoFormation (marqué « ≈ sens proche »).
- Une recherche sur le bureau liste tous les projets, réserve comprise, du plus pertinent au moins.

## Serveur MCP

```bash
claude mcp add atelier -- "C:\chemin\vers\atelier.exe" --mcp
```

Le chemin exact et la configuration JSON pour d'autres clients sont dans Configuration › IA (mode
« Agent MCP »), avec un bouton pour copier.

**Ce que l'agent sait sans qu'on le lui dise.** À la connexion, le serveur envoie ses
*instructions* (champ `instructions` de la poignée de main MCP) ; Claude Code les ajoute au prompt
système de l'agent. Elles posent cinq règles : une fiche par projet, **jamais pour une brique**
(frontend, api, db, site, packages/x… vont dans les dossiers du parent), ni pour un dossier de
rangement, ni pour une copie ; une description d'une phrase ; les dossiers sources restent à toi.
Le rappel est aussi glissé dans la réponse de `scanner_dossiers`, au moment où l'agent décide.

**Deux commandes prêtes** (prompts MCP ; dans Claude Code, tape `/` puis le nom) :

- `/mcp__atelier__ranger_mes_projets` : découvrir les projets et créer les fiches, méthode complète.
- `/mcp__atelier__nettoyer_mes_fiches` : repérer les fiches qui sont des briques ou des copies
  (`reperer_briques`) et les fusionner dans leur projet (`fusionner_projets`).

Outils : `lister_dossiers_sources`, `scanner_dossiers`, `lister_depots_github`, `lister_domaines`,
`lister_projets`, `voir_projet`, `ajouter_projet` (avec `dossiers_supplementaires` pour les
briques), `modifier_projet`, `rattacher_dossier`, `fusionner_projets`, `reperer_briques`,
`ignorer_dossier`, `supprimer_projet`, `ajouter_etape`, `cocher_etape`, `supprimer_etape`.

## La carte (Bibliothèque)

Un graphe vivant, calculé par une simulation de forces écrite à la main (`graph.rs`, dans l'esprit
de d3-force) et dessinée par `screens/map.rs` :

- les **domaines** sont des moyeux lumineux, les **projets** gravitent autour, reliés à leur domaine ;
  pointillés gris : projets rangés ensemble (même dossier, ou fiche dans le dossier d'une autre) ;
  pointillés ambre, au survol : technos communes entre domaines différents ;
- les nœuds se repoussent, les liens tirent comme des ressorts, rien ne se chevauche ; le graphe
  éclot à l'ouverture puis se pose, et flotte doucement ;
- **survol** : le nœud et ses voisins s'allument, des particules courent sur ses liens ;
  **clic** sur un domaine : filtre et la caméra glisse jusqu'à lui ; sur un projet : il s'ouvre ;
- **glisser** le fond pour se déplacer, **molette** pour zoomer sous la souris, **double-clic** pour
  recentrer ; un nœud se prend à la main (ses voisins suivent), et emporté hors de la carte il se
  dépose sur une attention ; « Secouer » relance la simulation ;
- les noms s'affichent par priorité (survolé, sélectionné, focus, récents) sans jamais se
  chevaucher : zoomer en révèle davantage.

## Sur le bureau

- **Étapes** : cocher, ajouter (champ + Entrée), retirer (croix) ; tout est enregistré et journalisé.
- **Notes** (onglet Activité), **fiche** modifiable (onglet Modifier, dont la description en une
  phrase), **suppression** avec confirmation (la fiche seulement, jamais le dossier).
- **Lanceurs** : tuiles qui ouvrent réellement VS Code, Claude Code, un terminal… dans le dossier.

## Le code

| Fichier | Rôle |
|---|---|
| `model.rs` | Projets, taxonomie, lanceurs, configuration (sérialisables), données de démo |
| `store.rs` | Le fichier JSON, sa relecture, l'index sémantique |
| `scan.rs` | Les dossiers des dossiers sources, sur 1 à 3 niveaux |
| `classify.rs` | Projet ou pas, et la fiche : règles locales, prompt et lecture de la réponse d'Ollama |
| `ollama.rs` | Modèles, réponses JSON, embeddings, téléchargement |
| `github.rs` | Dépôts via `gh` |
| `search.rs` | Recherche floue et sémantique |
| `graph.rs` | La simulation de forces de la carte (nœuds, liens, refroidissement, cadrage) |
| `jobs.rs` | Tâches de fond (scan, classement, modèles, embeddings…) : l'interface ne se fige jamais |
| `mcp.rs` | Le serveur MCP (JSON-RPC sur l'entrée et la sortie standard) |
| `launch.rs` | Lancement réel des lanceurs, redémarrage |
| `app.rs` | Le shell : navigation, actions, messages des tâches, enregistrement |
| `panel.rs`, `launchers.rs`, `update.rs`, `screens/` | Panneau projet, lanceurs, mise à jour, écrans |
| `theme.rs`, `icons.rs`, `widgets.rs` | Apparence et widgets |
| `tests.rs` | Tests d'interface (egui_kittest) et parcours complets |

Les tests ne touchent jamais tes vraies données : ils tournent sur la démo ou sur des fichiers et
dossiers temporaires, et ne lancent aucun programme (`dry_run`). Ollama est testé contre un faux
serveur local.

## Pas encore fait

- La mise à jour (recherche et téléchargement de versions) reste simulée ; le redémarrage est réel.
- En mode Ollama, les dossiers que le modèle juge « pas un projet » ne sont pas mémorisés : un
  nouveau classement les ré-examine (l'agent MCP, lui, peut les écarter avec `ignorer_dossier`).
