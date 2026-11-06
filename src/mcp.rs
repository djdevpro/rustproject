//! Le serveur MCP : `atelier --mcp` parle le Model Context Protocol (JSON-RPC 2.0, une ligne par
//! message, sur l'entrée et la sortie standard). Un agent IA (Claude Code…) peut ainsi lister les
//! dossiers sources, scanner, puis créer, modifier ou supprimer des projets.
//!
//! Chaque outil relit `atelier.json`, applique sa modification et l'enregistre : l'interface, si
//! elle est ouverte, voit le changement et se recharge.
//!
//! À déclarer dans Claude Code : `claude mcp add atelier -- "<chemin>\atelier.exe" --mcp`

use std::io::{BufRead, Write};

use serde_json::{Value, json};

use crate::classify::{Proposal, apply, by_rules};
use crate::ollaya::Ollaya;
use crate::ontology::{self, Merge};
use crate::model::{ActivityKind, AddedBy, Attention, Data, Status, Step, path_key};
use crate::scan::{ScanOptions, scan};
use crate::store::Store;

const PROTOCOLS: [&str; 3] = ["2025-06-18", "2025-03-26", "2024-11-05"];

/// Les consignes du serveur : le client MCP (Claude Code, Claude Desktop…) les ajoute au prompt
/// système de l'agent dès la connexion. C'est ici qu'on lui apprend à se servir d'Atelier.
const INSTRUCTIONS: &str = "\
Atelier est le gestionnaire de projets personnels de l'utilisateur (une application de bureau). Ces outils lisent et \
écrivent sa base ; l'interface se met à jour seule. Ton rôle : une liste de projets JUSTE, un projet = une fiche.

RÈGLE 1 — UN PROJET, PAS SES BRIQUES. Les sous-dossiers frontend, backend, api, server, client, web, app, mobile, admin, \
db, docs, infra, site, packages/x, apps/x, services/x… sont des BRIQUES du projet parent, même s'ils ont leur propre \
package.json, Cargo.toml ou .git. Jamais de fiche pour une brique : elle va dans les dossiers du parent \
(`dossiers_supplementaires` de `ajouter_projet`, ou `rattacher_dossier`). Un parent sans marqueur qui contient app, db, \
site… est le projet.
RÈGLE 2 — UN DOSSIER DE RANGEMENT N'EST PAS UN PROJET : un dossier qui regroupe des projets indépendants (enfants aux \
noms de produits différents) n'a pas de fiche ; ses enfants, oui.
RÈGLE 3 — UNE COPIE N'EST PAS UN PROJET (« - Copie », « -save », « -sauv », « backup », suffixe de commit comme \
site-deploy-64a6597) : rattache-la au projet d'origine ou écarte-la (`ignorer_dossier`).
RÈGLE 4 — DESCRIPTION : UNE phrase, 15 mots au plus, qui dit à quoi sert le projet (pas sa stack). Réutilise les \
domaines existants plutôt que d'en créer.
RÈGLE 5 — Les dossiers sources sont choisis par l'utilisateur dans Atelier, jamais par toi. Ne supprime aucune fiche \
sans son accord ; dans le doute (projet ou brique ?), lis le README et les sous-dossiers, puis demande.

MÉTHODE : `lister_domaines` et `lister_projets` (l'existant) → `scanner_dossiers` (candidats non suivis ; briques \
repérées et masquées) → `ajouter_projet` pour chaque vrai projet. Fiches existantes à remettre d'aplomb : \
`reperer_briques` puis `fusionner_projets`. Termine toujours par un bilan court : créées, rattachées, écartées, questions.
Méthodes détaillées : prompts `ranger_mes_projets` et `nettoyer_mes_fiches`.

ONTOLOGIE — les projets se classent aussi dans une ontologie : des facettes (axes indépendants : domaine, nature, \
finalité…) et des concepts définis. Pour la proposer : `lire_ontologie`, `lister_projets` (TOUS les projets, pour la vue \
d'ensemble), puis `definir_ontologie` (une définition d'une phrase par concept : c'est le critère du classement ; des \
synonymes en français : ils servent à la recherche). Les concepts marqués « manuel » viennent de l'utilisateur : ne les \
contredis pas. Méthode complète : prompt `concevoir_ontologie`. `classer_projets` (un modèle de décision Ollaya range \
chaque projet) seulement si l'utilisateur le demande.";

/// Rappel glissé dans la réponse du scan : l'agent le lit au moment où il décide.
const SCAN_REMINDER: &str = "Une fiche par PROJET. Les briques (frontend, api, db, site, packages/x…) vont dans \
`dossiers_supplementaires` du projet parent ; les dossiers de rangement et les copies n'ont pas de fiche. Description : \
une phrase de 15 mots au plus.";

const PROMPT_RANGER: &str = "\
Range mes projets dans Atelier{portee}.

1. Appelle `lister_domaines` et `lister_projets` pour connaître l'existant (ne crée rien en double).
2. Appelle `scanner_dossiers`. Chaque dossier arrive avec ses indices : marqueurs (.git, package.json, CLAUDE.md, \
AGENTS.md, README…), sous-dossiers, technos, début du README, `parent`, `briques` (ses sous-dossiers déjà reconnus comme \
parties de lui), `copie_probable`. Les briques déjà repérées sont masquées (`inclure_briques: true` pour les voir).
3. Pour chaque dossier, décide :
   - PROJET : une chose que je fais ou veux faire avancer (appli, site, outil, formation, étude…). → `ajouter_projet` \
avec un nom lisible, une description d'UNE phrase (15 mots au plus, à quoi ça sert), un domaine existant si possible, \
un type, les technos, et `dossiers_supplementaires` = ses briques (le champ `briques` du scan, plus celles que tu repères).
   - BRIQUE : une partie d'un projet (frontend, backend, api, db, docs, le site d'un produit, packages/x…), même avec \
son propre manifeste ou .git. → pas de fiche ; elle rejoint les dossiers du projet parent.
   - RANGEMENT : un dossier qui regroupe des projets indépendants. → pas de fiche ; ses enfants sont les projets.
   - COPIE, SAUVEGARDE, DÉPLOIEMENT FIGÉ. → pas de fiche ; `rattacher_dossier` au projet d'origine, ou `ignorer_dossier`.
   - INCERTAIN → mets-le de côté et pose-moi la question à la fin.
   Repère utile : des enfants aux noms techniques (app, api, db, site, web…) = un projet et ses briques ; des enfants aux \
noms de produits différents = un dossier de rangement.
4. Projet inactif depuis plus de 120 jours : etat « En pause », attention « En réserve ».
5. Appelle `lister_depots_github` : relie chaque dépôt à sa fiche (`modifier_projet` avec `depot`). Un dépôt sans \
dossier local qui est un vrai projet (pas un fork) peut avoir sa propre fiche.
6. Termine par un bilan : fiches créées (nom → domaine), briques rattachées, dossiers écartés et pourquoi, questions.";

const PROMPT_ONTOLOGIE: &str = "\
Conçois une ontologie COMPLÈTE pour ranger tous mes projets dans Atelier.

1. `lire_ontologie` (l'existant : les concepts « manuel » viennent de moi, garde-les) puis `lister_projets` (TOUS les \
projets : nom, description, type, technos, dossiers ; un nom de dossier dit souvent ce que la fiche ne dit pas, ex. \
« ImmoFormation › site internet »).
2. Choisis les FACETTES : des axes indépendants qui répondent chacun à une question (« De quoi ça parle ? » = Domaine, \
à garder ; « Qu'est-ce que c'est ? » = nature : application, outil, bibliothèque, formation, recherche… ; « Pour quoi \
faire ? » = finalité : apprendre, client, produit, usage perso… ; d'autres si mes projets l'appellent).
3. Dans chaque facette, autant de CONCEPTS qu'il faut pour que CHAQUE projet y trouve au moins une place, sans concept \
vide ni doublon. Hiérarchise quand un concept est trop large (« Immobilier » › « Location »). Pour chaque concept : un \
nom court, une DÉFINITION d'une phrase qui permette de décider oui ou non (c'est le critère du classement automatique), \
3 à 6 SYNONYMES en français, les mots que je taperais pour le chercher (« logement », « appartement » pour Immobilier).
4. Vérifie la couverture : chaque projet doit avoir un concept dans chaque facette ; sinon, ajoute ou élargis un concept.
5. `definir_ontologie` avec TOUTES les facettes (mode « remplacer » : mes concepts manuels restent).
6. Termine par un résumé court (facettes, nombre de concepts, concepts ajoutés) et DEMANDE-moi si je veux lancer le \
classement (`classer_projets`) : ne le lance pas sans mon accord.";

const PROMPT_NETTOYER: &str = "\
Fais le ménage dans mes fiches Atelier : beaucoup ont été créées automatiquement, et certaines ne sont pas de vrais \
projets mais des briques (app, db, site… d'un même projet) ou des copies.

1. Appelle `reperer_briques` : il regroupe les fiches suspectes par dossier parent, avec des indices (nom de brique, \
sous-dossier d'une autre fiche, copie probable) et le début du README du parent.
2. Pour chaque groupe, décide :
   - Le parent est un projet et les fiches sont ses briques → s'il n'a pas de fiche, crée-la (`ajouter_projet` avec le \
dossier parent et une description d'une phrase), puis `fusionner_projets` (id = le parent, ids = les briques). Dossiers, \
étapes, notes et technos sont conservés.
   - Le parent est un dossier de rangement et les fiches sont de vrais projets → ne touche à rien.
   - Copies et sauvegardes → `fusionner_projets` dans la fiche d'origine.
3. Au passage, corrige les descriptions (une phrase, 15 mots au plus) et les domaines fourre-tout.
4. Les fusions, tu peux les faire. Les suppressions, demande-moi d'abord.
5. Termine par un bilan : fusions faites, fiches laissées telles quelles, questions.";

/// Les prompts MCP : des commandes prêtes (dans Claude Code : `/mcp__atelier__ranger_mes_projets`).
fn prompts() -> Value {
    json!([
        {
            "name": "ranger_mes_projets",
            "title": "Ranger mes projets",
            "description": "Découvre les projets des dossiers sources et crée une fiche par vrai projet (briques rattachées, copies écartées).",
            "arguments": [{ "name": "dossier", "description": "Se limiter à ce dossier (facultatif)", "required": false }]
        },
        {
            "name": "concevoir_ontologie",
            "title": "Concevoir une ontologie complète",
            "description": "Lit tous les projets et propose une ontologie complète (facettes, concepts définis, synonymes, hiérarchie) ; ne lance pas le classement sans accord.",
            "arguments": []
        },
        {
            "name": "nettoyer_mes_fiches",
            "title": "Nettoyer mes fiches",
            "description": "Repère les fiches qui sont des briques ou des copies d'un projet et les fusionne dans leur projet.",
            "arguments": []
        }
    ])
}

fn get_prompt(params: &Value) -> Result<Value, (i64, String)> {
    let name = params["name"].as_str().unwrap_or("");
    let text = match name {
        "ranger_mes_projets" => {
            let scope = params["arguments"]["dossier"].as_str().map(str::trim).filter(|d| !d.is_empty());
            PROMPT_RANGER.replace("{portee}", &scope.map(|d| format!(" (seulement ce qui se trouve sous « {d} »)")).unwrap_or_default())
        }
        "nettoyer_mes_fiches" => PROMPT_NETTOYER.to_owned(),
        "concevoir_ontologie" => PROMPT_ONTOLOGIE.to_owned(),
        _ => return Err((-32602, format!("Prompt inconnu : {name}"))),
    };
    Ok(json!({ "description": name, "messages": [{ "role": "user", "content": { "type": "text", "text": text } }] }))
}

/// Boucle principale : lit l'entrée standard jusqu'à sa fermeture.
pub fn serve() -> i32 {
    let mut store = Store::open(Store::default_path());
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    eprintln!("atelier mcp · données : {}", store.path().display());
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        if let Some(reply) = handle_line(&mut store, &line) {
            let _ = writeln!(stdout, "{reply}");
            let _ = stdout.flush();
        }
    }
    0
}

/// Traite un message JSON-RPC ; renvoie la réponse (rien pour une notification).
pub fn handle_line(store: &mut Store, line: &str) -> Option<String> {
    // Certains clients (PowerShell…) préfixent le flux d'une marque d'encodage UTF-8 : on l'ignore.
    let line = line.trim_start_matches('\u{feff}');
    let msg: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(e) => return Some(json!({ "jsonrpc": "2.0", "id": null, "error": { "code": -32700, "message": format!("JSON invalide : {e}") } }).to_string()),
    };
    let id = msg.get("id").cloned();
    let method = msg["method"].as_str().unwrap_or("");
    let params = msg.get("params").cloned().unwrap_or(Value::Null);
    // Une notification (sans id) n'appelle pas de réponse.
    let id = id?;
    let result = match method {
        "initialize" => Ok(initialize(&params)),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": tools() })),
        "tools/call" => Ok(call_tool(store, &params)),
        "prompts/list" => Ok(json!({ "prompts": prompts() })),
        "prompts/get" => get_prompt(&params),
        _ => Err((-32601, format!("Méthode inconnue : {method}"))),
    };
    Some(match result {
        Ok(r) => json!({ "jsonrpc": "2.0", "id": id, "result": r }),
        Err((code, message)) => json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } }),
    }
    .to_string())
}

fn initialize(params: &Value) -> Value {
    let asked = params["protocolVersion"].as_str().unwrap_or(PROTOCOLS[0]);
    let version = if PROTOCOLS.contains(&asked) { asked } else { PROTOCOLS[0] };
    json!({
        "protocolVersion": version,
        "capabilities": { "tools": { "listChanged": false }, "prompts": { "listChanged": false } },
        "serverInfo": { "name": "atelier", "version": env!("CARGO_PKG_VERSION") },
        "instructions": INSTRUCTIONS
    })
}

fn tool(name: &str, description: &str, properties: Value, required: &[&str]) -> Value {
    json!({ "name": name, "description": description, "inputSchema": { "type": "object", "properties": properties, "required": required } })
}

/// Les noms des outils proposés à l'agent (pour les montrer dans la configuration).
pub fn tool_names() -> Vec<String> {
    tools().iter().filter_map(|t| t["name"].as_str().map(str::to_owned)).collect()
}

fn tools() -> Vec<Value> {
    let s = |d: &str| json!({ "type": "string", "description": d });
    let n = |d: &str| json!({ "type": "integer", "description": d });
    let fields = json!({
        "nom": s("Nom lisible du projet"),
        "description": s("UNE phrase courte (15 mots au plus) qui dit ce qu'est le projet"),
        "dossier": s("Chemin du dossier du projet"),
        "depot": s("URL du dépôt distant (GitHub…)"),
        "domaine": s("Domaine (voir lister_domaines ; un nouveau nom est créé s'il n'existe pas)"),
        "type": s("Type de projet, ex. « Application web », « Outil en ligne de commande »"),
        "technos": { "type": "array", "items": { "type": "string" }, "description": "Technologies" },
        "etat": s("Idée, En cours, En pause, Terminé ou Archivé"),
        "attention": s("Focus, Au fil de l'eau ou En réserve"),
        "prochaine_action": s("La toute prochaine chose à faire"),
        "resultat_vise": s("Ce que l'utilisateur veut avoir entre les mains")
    });
    let list = |d: &str| json!({ "type": "array", "items": { "type": "string" }, "description": d });
    let mut add_fields = fields.clone();
    add_fields["dossiers_supplementaires"] = list("Les briques du projet (frontend, api, db, site, packages/x…) : rattachées à cette fiche, jamais en fiches séparées");
    let mut update_fields = fields;
    update_fields["id"] = n("Identifiant du projet");
    update_fields["dossier"] = s("Nouveau dossier principal (les dossiers rattachés sont gardés)");
    update_fields["dossiers"] = list("Remplace TOUS les dossiers du projet (le premier est le dossier principal)");
    vec![
        tool("lister_dossiers_sources", "Les dossiers où se trouvent les projets (choisis par l'utilisateur dans Atelier), leur profondeur de scan (1 à 3) et les dossiers écartés.", json!({}), &[]),
        tool(
            "scanner_dossiers",
            "Liste les dossiers des dossiers sources sur 1 à 3 niveaux, avec leurs indices : marqueurs, sous-dossiers, début du README, technos, parent, briques repérées, copie probable. Les dossiers déjà suivis et les briques sont masqués par défaut. Ne crée rien.",
            json!({
                "inclure_suivis": { "type": "boolean", "description": "Inclure les dossiers déjà suivis (non par défaut)" },
                "inclure_briques": { "type": "boolean", "description": "Inclure les briques déjà rattachées à un projet parent (non par défaut)" },
                "dossier": s("Se limiter aux dossiers situés sous ce chemin (facultatif)")
            }),
            &[],
        ),
        tool("lister_depots_github", "Les dépôts GitHub de l'utilisateur (via la CLI gh), avec ceux déjà suivis.", json!({}), &[]),
        tool("lister_domaines", "Les domaines de la taxonomie, avec leur nombre de projets.", json!({}), &[]),
        tool("lister_projets", "Les projets suivis (résumé). Filtre facultatif sur le nom ou la description.", json!({ "filtre": s("Texte à chercher") }), &[]),
        tool("voir_projet", "La fiche complète d'un projet (étapes, journal…).", json!({ "id": n("Identifiant") }), &["id"]),
        tool(
            "ajouter_projet",
            "Crée la fiche d'UN projet (jamais d'une brique, d'un dossier de rangement ou d'une copie). Ses briques vont dans `dossiers_supplementaires`. Refuse un dossier déjà suivi (renvoie son id).",
            add_fields,
            &["nom", "description"],
        ),
        tool("modifier_projet", "Modifie les champs donnés d'un projet.", update_fields, &["id"]),
        tool(
            "rattacher_dossier",
            "Ajoute un dossier (une brique, une copie…) aux dossiers d'un projet. Si ce dossier a déjà sa propre fiche, utilise plutôt `fusionner_projets`.",
            json!({ "id": n("Identifiant du projet qui reçoit le dossier"), "dossier": s("Chemin du dossier à rattacher") }),
            &["id", "dossier"],
        ),
        tool(
            "fusionner_projets",
            "Fusionne des fiches dans une autre : leurs dossiers, étapes, notes, liens et technos rejoignent la fiche `id`, puis elles disparaissent. Pour les briques ou copies qui avaient leur propre fiche.",
            json!({ "id": n("La fiche qui reste (le vrai projet)"), "ids": { "type": "array", "items": { "type": "integer" }, "description": "Les fiches absorbées" } }),
            &["id", "ids"],
        ),
        tool(
            "reperer_briques",
            "Regroupe par dossier parent les fiches existantes qui ressemblent à des briques ou à des copies (nom de brique, sous-dossier d'une autre fiche, copie probable), avec le README et les sous-dossiers du parent. Ne modifie rien.",
            json!({}),
            &[],
        ),
        tool(
            "ignorer_dossier",
            "Écarte un dossier des prochains scans (copie, sauvegarde, dossier sans intérêt), ou le rétablit avec `annuler`. Refuse un dossier suivi par une fiche.",
            json!({ "dossier": s("Chemin du dossier"), "annuler": { "type": "boolean", "description": "Rétablir un dossier écarté" } }),
            &["dossier"],
        ),
        tool("supprimer_projet", "Supprime la fiche d'un projet (pas son dossier).", json!({ "id": n("Identifiant") }), &["id"]),
        tool("ajouter_etape", "Ajoute une étape (case à cocher) à un projet.", json!({ "id": n("Identifiant du projet"), "texte": s("L'étape") }), &["id", "texte"]),
        tool("cocher_etape", "Coche ou décoche une étape.", json!({ "id": n("Identifiant du projet"), "index": n("Index de l'étape (0 = première)"), "fait": { "type": "boolean" } }), &["id", "index", "fait"]),
        tool("supprimer_etape", "Supprime une étape.", json!({ "id": n("Identifiant du projet"), "index": n("Index de l'étape") }), &["id", "index"]),
        // ---- ontologie et classement (Ollaya)
        tool("lire_ontologie", "L'ontologie : facettes (axes de rangement) et concepts (définition, synonymes, parent, manuel, nombre de projets rattachés).", json!({}), &[]),
        tool(
            "definir_ontologie",
            "Enregistre une ontologie COMPLÈTE, conçue après avoir lu TOUS les projets (méthode : prompt `concevoir_ontologie`) : des facettes indépendantes (Domaine, nature, finalité…), dans chacune autant de concepts qu'il faut pour que chaque projet y trouve sa place, chacun avec une définition d'une phrase (critère du classement) et des synonymes en français (pour la recherche). Les concepts retouchés à la main par l'utilisateur sont toujours gardés. Même nom = même concept (les rattachements suivent). Ne classe rien.",
            json!({
                "facettes": {
                    "type": "array",
                    "description": "Toutes les facettes, avec tous leurs concepts",
                    "items": {
                        "type": "object",
                        "properties": {
                            "nom": s("Nom de la facette (« Domaine » existe déjà)"),
                            "description": s("La question à laquelle elle répond"),
                            "concepts": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "nom": s("Nom court"),
                                        "definition": s("Une phrase qui permet de décider si un projet en relève"),
                                        "synonymes": list("Mots qu'on taperait pour le chercher, en français"),
                                        "parent": s("Nom d'un concept plus général de la même facette (facultatif)")
                                    },
                                    "required": ["nom", "definition", "synonymes"]
                                }
                            }
                        },
                        "required": ["nom", "description", "concepts"]
                    }
                },
                "mode": s("« remplacer » (les concepts proposés remplacent ceux de l'IA ; par défaut) ou « ajouter » (seulement les nouveaux)")
            }),
            &["facettes"],
        ),
        tool(
            "modifier_concept",
            "Crée (sans id), modifie ou supprime (`supprimer: true`) un concept.",
            json!({
                "id": n("Identifiant du concept (absent : création)"),
                "facette": s("Nom de la facette (création ou déplacement ; créée si besoin)"),
                "nom": s("Nom court"),
                "definition": s("Une phrase qui permet de décider si un projet en relève"),
                "synonymes": list("Autres mots pour le chercher"),
                "parent": s("Nom du concept parent, dans la même facette (vide : premier niveau)"),
                "supprimer": { "type": "boolean" }
            }),
            &[],
        ),
        tool(
            "rattacher_concept",
            "Rattache un projet à un concept (ou le détache avec `retirer`). Un rattachement fait ainsi est gardé par les classements suivants.",
            json!({ "id": n("Identifiant du projet"), "concept": n("Identifiant du concept"), "retirer": { "type": "boolean" } }),
            &["id", "concept"],
        ),
        tool("lister_modeles_decision", "Les modèles de décision installés dans Ollaya, et celui qui classe les projets.", json!({}), &[]),
        tool(
            "classer_projets",
            "Range des projets dans l'ontologie avec un modèle de décision Ollaya (une question oui/non par concept ; à partir de 0,5 le projet est rattaché). Sans `ids` : les projets jamais classés ou classés avec une ancienne ontologie, 40 au plus par appel (rappelle jusqu'à ce qu'il n'en reste plus).",
            json!({ "ids": { "type": "array", "items": { "type": "integer" }, "description": "Les projets à classer (facultatif)" }, "modele": s("Modèle Ollaya (par défaut : celui choisi dans Atelier)") }),
            &[],
        ),
    ]
}

/// Résultat d'outil : du texte (JSON lisible) et `isError`.
fn text_result(value: Result<Value, String>) -> Value {
    match value {
        Ok(v) => json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&v).unwrap_or_default() }], "isError": false }),
        Err(e) => json!({ "content": [{ "type": "text", "text": e }], "isError": true }),
    }
}

fn call_tool(store: &mut Store, params: &Value) -> Value {
    let name = params["name"].as_str().unwrap_or("");
    let args = params.get("arguments").cloned().unwrap_or(json!({}));
    text_result(run_tool(store, name, &args))
}

fn load(store: &mut Store) -> Result<Data, String> {
    Ok(store.load()?.unwrap_or_else(Data::fresh))
}

fn project_summary(data: &Data, i: usize) -> Value {
    let p = &data.projects[i];
    json!({
        "id": p.id, "nom": p.name, "description": p.description, "etat": p.status.label(), "attention": p.attention.label(),
        "domaine": data.domains.get(p.domain).map(|d| d.name.clone()), "type": p.kind, "technos": p.stack,
        "dossiers": p.folders, "depot": p.remote, "etapes": p.steps.len(),
        "concepts": data.concepts_of(i).into_iter().map(|(c, _)| data.ontology.path(c.id)).collect::<Vec<_>>()
    })
}

fn find(data: &Data, args: &Value) -> Result<usize, String> {
    let id = args["id"].as_u64().ok_or("Paramètre « id » manquant")?;
    data.index_of(id).ok_or_else(|| format!("Aucun projet avec l'id {id}"))
}

fn strings(v: &Value) -> Option<Vec<String>> {
    v.as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_owned)).collect())
}

/// Erreur si `folder` appartient à une autre fiche que la `i`-ième : il faut alors fusionner.
fn owned_elsewhere(data: &Data, i: usize, folder: &str) -> Result<(), String> {
    match data.find_by_folder(folder) {
        Some(j) if j != i => {
            let (me, other) = (data.projects.get(i).map(|p| p.id), &data.projects[j]);
            let hint = me.map(|id| format!(" : utilise fusionner_projets {{ \"id\": {id}, \"ids\": [{}] }}", other.id)).unwrap_or_default();
            Err(format!("« {folder} » est déjà suivi par la fiche {} ({}){hint}", other.id, other.name))
        }
        _ => Ok(()),
    }
}

/// Les fiches qui ressemblent à des briques ou à des copies, regroupées par dossier parent.
fn suspects(data: &Data) -> Vec<Value> {
    use std::collections::BTreeMap;
    use std::path::Path;
    let roots: Vec<String> = data.config.roots.iter().map(|r| path_key(&r.path)).collect();
    let owner = |key: &str| data.projects.iter().find(|p| p.folders.iter().any(|f| path_key(f) == key)).map(|p| p.id);
    let mut groups: BTreeMap<String, (String, Vec<Value>)> = BTreeMap::new();
    for p in &data.projects {
        let Some(folder) = p.folders.first() else { continue };
        let path = Path::new(folder);
        let (Some(parent), Some(name)) = (path.parent(), path.file_name()) else { continue };
        let name = name.to_string_lossy();
        let parent_name = parent.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let key = path_key(&parent.to_string_lossy());
        let mut hints: Vec<String> = Vec::new();
        if let Some(o) = parent.ancestors().find_map(|a| owner(&path_key(&a.to_string_lossy())).filter(|&o| o != p.id)) {
            hints.push(format!("sous-dossier de la fiche {o}"));
        }
        if crate::scan::brick_like(&name, &parent_name) {
            hints.push("nom de brique".into());
        }
        if crate::scan::copy_like(&name) {
            let lower = name.to_lowercase();
            let original = data.projects.iter().filter(|q| q.id != p.id && q.name.len() >= 3 && lower.starts_with(&q.name.to_lowercase()))
                // De préférence dans le même dossier parent, puis le nom le plus long.
                .max_by_key(|q| (q.folders.first().and_then(|f| Path::new(f).parent()).is_some_and(|pp| path_key(&pp.to_string_lossy()) == key), q.name.len()));
            hints.push(match original {
                Some(q) => format!("copie probable de la fiche {} ({})", q.id, q.name),
                None => "copie ou sauvegarde probable".into(),
            });
        }
        // Au premier niveau d'un dossier source, seules les fiches avec un indice sont suspectes.
        if roots.contains(&key) && hints.is_empty() {
            continue;
        }
        let entry = groups.entry(key).or_insert_with(|| (parent.to_string_lossy().into_owned(), Vec::new()));
        entry.1.push(json!({ "id": p.id, "nom": p.name, "dossier": folder, "description": p.description, "indices": hints }));
    }
    groups
        .into_iter()
        .filter(|(_, (_, fiches))| fiches.len() >= 2 || fiches.iter().any(|f| f["indices"].as_array().is_some_and(|a| !a.is_empty())))
        .map(|(key, (path, fiches))| {
            let dir = Path::new(&path);
            let children: Vec<String> = std::fs::read_dir(dir)
                .map(|rd| rd.flatten().filter(|e| e.file_type().is_ok_and(|t| t.is_dir())).map(|e| e.file_name().to_string_lossy().into_owned()).filter(|n| !n.starts_with('.')).take(40).collect())
                .unwrap_or_default();
            json!({
                "dossier_parent": path, "fiche_du_parent": owner(&key), "est_un_dossier_source": roots.contains(&key),
                "readme_du_parent": readme_head(dir, 400), "sous_dossiers_du_parent": children, "fiches": fiches
            })
        })
        .collect()
}

/// Le début du README d'un dossier, s'il en a un.
fn readme_head(dir: &std::path::Path, chars: usize) -> Option<String> {
    let entry = std::fs::read_dir(dir).ok()?.flatten().find(|e| {
        let n = e.file_name().to_string_lossy().to_lowercase();
        n.starts_with("readme") && e.file_type().is_ok_and(|t| t.is_file())
    })?;
    let bytes = std::fs::read(entry.path()).ok()?;
    Some(String::from_utf8_lossy(&bytes[..bytes.len().min(chars * 4)]).chars().take(chars).collect())
}

/// Applique les champs fournis à un projet existant.
fn set_fields(data: &mut Data, i: usize, args: &Value) -> Result<(), String> {
    let mut wanted: Vec<String> = strings(&args["dossiers"]).unwrap_or_default();
    wanted.extend(args["dossier"].as_str().map(str::to_owned));
    for f in &wanted {
        owned_elsewhere(data, i, f)?;
    }
    let domain = args["domaine"].as_str().map(|d| data.domain_named(d));
    let p = &mut data.projects[i];
    let txt = |k: &str| args[k].as_str().map(|s| s.trim().to_owned());
    if let Some(v) = txt("nom").filter(|v| !v.is_empty()) {
        p.name = v;
    }
    if let Some(v) = txt("description") {
        p.description = v;
    }
    if let Some(v) = txt("dossier").filter(|v| !v.is_empty()) {
        // Le nouveau dossier principal remplace l'ancien ; les dossiers rattachés restent.
        if p.folders.first().is_none_or(|main| path_key(main) != path_key(&v)) {
            p.folders.retain(|f| path_key(f) != path_key(&v));
            if !p.folders.is_empty() {
                p.folders.remove(0);
            }
            p.folders.insert(0, v);
        }
    }
    if let Some(v) = strings(&args["dossiers"]) {
        p.folders = v.into_iter().map(|f| f.trim().to_owned()).filter(|f| !f.is_empty()).collect();
    }
    if let Some(v) = txt("depot") {
        p.remote = (!v.is_empty()).then_some(v);
    }
    if let Some(v) = txt("type") {
        p.kind = v;
    }
    if let Some(v) = strings(&args["technos"]) {
        p.stack = v;
    }
    if let Some(v) = txt("prochaine_action") {
        p.next_action = v;
    }
    if let Some(v) = txt("resultat_vise") {
        p.outcome = v;
    }
    if let Some(d) = domain {
        p.domain = d;
    }
    if let Some(s) = args["etat"].as_str() {
        p.status = Status::parse(s).ok_or_else(|| format!("État inconnu : {s}"))?;
    }
    if let Some(a) = args["attention"].as_str() {
        p.attention = Attention::parse(a).ok_or_else(|| format!("Attention inconnue : {a}"))?;
    }
    Ok(())
}

pub fn run_tool(store: &mut Store, name: &str, args: &Value) -> Result<Value, String> {
    let mut data = load(store)?;
    match name {
        "lister_dossiers_sources" => Ok(json!({
            "sources": data.config.roots.iter().map(|r| json!({ "chemin": r.path, "profondeur": r.depth, "actif": r.enabled })).collect::<Vec<_>>(),
            "ecartes": data.config.ignored,
            "note": "Seul l'utilisateur ajoute ou retire des dossiers sources, dans Atelier."
        })),
        "scanner_dossiers" => {
            if data.config.roots.is_empty() {
                return Err("Aucun dossier source : l'utilisateur doit en choisir un dans Atelier (Configuration → Sources).".into());
            }
            let include = args["inclure_suivis"].as_bool().unwrap_or(false);
            let include_bricks = args["inclure_briques"].as_bool().unwrap_or(false);
            let under = args["dossier"].as_str().map(path_key).filter(|k| !k.is_empty());
            let result = scan(&ScanOptions::from_config(&data.config), |_| {});
            let mut hidden_bricks = 0;
            let dirs: Vec<Value> = result
                .dirs
                .iter()
                .filter(|f| under.as_ref().is_none_or(|u| path_key(&f.path).starts_with(&format!("{u}\\"))))
                .filter_map(|f| {
                    let tracked = data.find_by_folder(&f.path).map(|i| data.projects[i].id);
                    if tracked.is_some() && !include {
                        return None;
                    }
                    if f.brick_of.is_some() && !include_bricks {
                        hidden_bricks += 1;
                        return None;
                    }
                    Some(json!({
                        "chemin": f.path, "nom": f.name, "niveau": f.depth, "parent": f.parent, "marqueurs": f.markers,
                        "projet_probable": f.likely_project, "briques": f.bricks, "brique_de": f.brick_of, "copie_probable": crate::scan::copy_like(&f.name),
                        "sous_dossiers": f.children, "technos": f.manifest.stack, "description_manifeste": f.manifest.description,
                        "readme": f.readme.as_deref().map(|r| r.chars().take(400).collect::<String>()), "inactif_depuis_jours": f.idle_days,
                        "deja_suivi": tracked
                    }))
                })
                .collect();
            Ok(json!({ "rappel": SCAN_REMINDER, "dossiers": dirs, "briques_masquees": hidden_bricks, "erreurs": result.errors, "tronque": result.truncated }))
        }
        "lister_depots_github" => {
            let repos = crate::github::list_repos()?;
            Ok(json!(repos.iter().map(|r| json!({
                "nom": r.name, "complet": r.full_name, "description": r.description, "url": r.html_url, "langage": r.language,
                "archive": r.archived, "prive": r.private, "fork": r.fork, "deja_suivi": crate::github::local_match(&data, r).map(|i| data.projects[i].id)
            })).collect::<Vec<_>>()))
        }
        "lister_domaines" => Ok(json!(data.domains.iter().enumerate().map(|(i, d)| json!({ "nom": d.name, "projets": data.projects.iter().filter(|p| p.domain == i).count() })).collect::<Vec<_>>())),
        "lister_projets" => {
            let filter = args["filtre"].as_str().unwrap_or("").to_owned();
            let ids: Vec<usize> = if filter.trim().is_empty() {
                (0..data.projects.len()).collect()
            } else {
                crate::search::rank(&data, &filter, None, &Default::default()).into_iter().map(|h| h.index).collect()
            };
            Ok(json!(ids.into_iter().map(|i| project_summary(&data, i)).collect::<Vec<_>>()))
        }
        "voir_projet" => {
            let i = find(&data, args)?;
            serde_json::to_value(&data.projects[i]).map_err(|e| e.to_string())
        }
        "ajouter_projet" => {
            let name = args["nom"].as_str().ok_or("Paramètre « nom » manquant")?.trim().to_owned();
            let folder = args["dossier"].as_str().map(|s| s.trim().to_owned()).filter(|s| !s.is_empty());
            if let Some(f) = &folder
                && let Some(i) = data.find_by_folder(f)
            {
                return Ok(json!({ "deja_suivi": data.projects[i].id }));
            }
            // Partir de ce que le scan sait du dossier (technos, marqueurs, activité), puis appliquer
            // ce que l'agent a fourni.
            let base = folder.as_ref().and_then(|f| {
                let opts = ScanOptions { roots: Vec::new(), ..ScanOptions::from_config(&data.config) };
                let parent = std::path::Path::new(f).parent()?.to_string_lossy().into_owned();
                let r = scan(&ScanOptions { roots: vec![(0, parent, 1)], ..opts }, |_| {});
                r.dirs.into_iter().find(|d| path_key(&d.path) == path_key(f)).map(|d| by_rules(&d))
            });
            let mut p = base.unwrap_or(Proposal { folder: folder.clone(), ..Default::default() });
            // Les briques déclarées par l'agent (en plus de celles que le scan a reconnues).
            for extra in strings(&args["dossiers_supplementaires"]).unwrap_or_default() {
                let extra = extra.trim().to_owned();
                if extra.is_empty() || p.extra_folders.iter().chain(folder.iter()).any(|f| path_key(f) == path_key(&extra)) {
                    continue;
                }
                owned_elsewhere(&data, usize::MAX, &extra).map_err(|e| format!("{e}. Crée la fiche sans ce dossier, puis fusionne l'autre fiche dedans (fusionner_projets)."))?;
                p.extra_folders.push(extra);
            }
            p.extra_folders.retain(|f| data.find_by_folder(f).is_none());
            p.name = name;
            p.description = args["description"].as_str().unwrap_or("").trim().to_owned();
            p.remote = args["depot"].as_str().map(str::to_owned).or(p.remote);
            if let Some(k) = args["type"].as_str() {
                p.kind = k.to_owned();
            }
            if let Some(t) = strings(&args["technos"]) {
                p.stack = t;
            }
            if let Some(d) = args["domaine"].as_str() {
                p.domain = d.to_owned();
            }
            p.confidence = 0.9;
            let i = apply(&mut data, &p, AddedBy::Mcp).ok_or("Projet déjà suivi")?;
            set_fields(&mut data, i, args)?;
            data.projects[i].log(ActivityKind::Ai, "Fiche créée par l'agent IA (MCP)");
            store.save(&data)?;
            Ok(json!({ "id": data.projects[i].id, "projet": project_summary(&data, i) }))
        }
        "modifier_projet" => {
            let i = find(&data, args)?;
            set_fields(&mut data, i, args)?;
            data.projects[i].log(ActivityKind::Ai, "Fiche modifiée par l'agent IA (MCP)");
            store.save(&data)?;
            Ok(project_summary(&data, i))
        }
        "supprimer_projet" => {
            let i = find(&data, args)?;
            let p = data.projects.remove(i);
            store.save(&data)?;
            Ok(json!({ "supprime": p.id, "nom": p.name }))
        }
        "ajouter_etape" => {
            let i = find(&data, args)?;
            let text = args["texte"].as_str().ok_or("Paramètre « texte » manquant")?.trim().to_owned();
            data.projects[i].steps.push(Step { text, done: false });
            store.save(&data)?;
            Ok(json!({ "etapes": data.projects[i].steps }))
        }
        "cocher_etape" | "supprimer_etape" => {
            let i = find(&data, args)?;
            let k = args["index"].as_u64().ok_or("Paramètre « index » manquant")? as usize;
            let steps = &mut data.projects[i].steps;
            if k >= steps.len() {
                return Err(format!("Pas d'étape {k} (le projet en a {})", steps.len()));
            }
            if name == "cocher_etape" {
                steps[k].done = args["fait"].as_bool().unwrap_or(true);
            } else {
                steps.remove(k);
            }
            store.save(&data)?;
            Ok(json!({ "etapes": data.projects[i].steps }))
        }
        "rattacher_dossier" => {
            let i = find(&data, args)?;
            let folder = args["dossier"].as_str().ok_or("Paramètre « dossier » manquant")?.trim().to_owned();
            if !std::path::Path::new(&folder).is_dir() {
                return Err(format!("Dossier introuvable : {folder}"));
            }
            owned_elsewhere(&data, i, &folder)?;
            let p = &mut data.projects[i];
            if !p.folders.iter().any(|f| path_key(f) == path_key(&folder)) {
                p.folders.push(folder.clone());
                p.log(ActivityKind::Ai, format!("Dossier rattaché par l'agent IA : {folder}"));
                store.save(&data)?;
            }
            Ok(project_summary(&data, i))
        }
        "fusionner_projets" => {
            let target = data.projects[find(&data, args)?].id;
            let ids: Vec<u64> = args["ids"].as_array().ok_or("Paramètre « ids » manquant")?.iter().filter_map(Value::as_u64).filter(|&id| id != target).collect();
            if ids.is_empty() {
                return Err("Aucune fiche à fusionner (« ids » vide ou égal à « id »)".into());
            }
            if let Some(missing) = ids.iter().find(|&&id| data.index_of(id).is_none()) {
                return Err(format!("Aucun projet avec l'id {missing}"));
            }
            let mut merged = Vec::new();
            for id in ids {
                let Some(i) = data.index_of(id) else { continue };
                let p = data.projects.remove(i);
                let t = data.index_of(target).ok_or("Fiche cible disparue")?;
                let into = &mut data.projects[t];
                for f in p.folders {
                    if !into.folders.iter().any(|g| path_key(g) == path_key(&f)) {
                        into.folders.push(f);
                    }
                }
                for s in p.stack {
                    if !into.stack.contains(&s) {
                        into.stack.push(s);
                    }
                }
                into.steps.extend(p.steps);
                into.notes.extend(p.notes);
                for l in p.links {
                    if !into.links.contains(&l) {
                        into.links.push(l);
                    }
                }
                if into.remote.is_none() {
                    into.remote = p.remote;
                }
                if into.description.trim().is_empty() {
                    into.description = p.description;
                }
                into.idle_days = into.idle_days.min(p.idle_days);
                into.log(ActivityKind::Ai, format!("Fiche « {} » fusionnée ici par l'agent IA", p.name));
                merged.push(json!({ "id": id, "nom": p.name }));
            }
            store.save(&data)?;
            let t = data.index_of(target).ok_or("Fiche cible disparue")?;
            Ok(json!({ "fusionnees": merged, "projet": project_summary(&data, t) }))
        }
        "reperer_briques" => Ok(json!({ "rappel": SCAN_REMINDER, "groupes": suspects(&data) })),
        "ignorer_dossier" => {
            let folder = args["dossier"].as_str().ok_or("Paramètre « dossier » manquant")?.trim().to_owned();
            let key = path_key(&folder);
            if args["annuler"].as_bool().unwrap_or(false) {
                data.config.ignored.retain(|f| path_key(f) != key);
            } else {
                if let Some(j) = data.find_by_folder(&folder) {
                    let p = &data.projects[j];
                    return Err(format!("« {folder} » est suivi par la fiche {} ({}) : fusionne-la dans son projet, ou supprime-la avec l'accord de l'utilisateur, avant de l'écarter.", p.id, p.name));
                }
                if !data.config.ignored.iter().any(|f| path_key(f) == key) {
                    data.config.ignored.push(folder);
                }
            }
            store.save(&data)?;
            Ok(json!({ "ecartes": data.config.ignored }))
        }
        "lire_ontologie" => Ok(ontology::to_json(&data)),
        "definir_ontologie" => {
            let proposal = ontology::from_json(args)?;
            let mode = if args["mode"].as_str().is_some_and(|m| m.trim().to_lowercase().starts_with("ajout")) { Merge::AddNew } else { Merge::Replace };
            let (added, removed) = data.merge_ontology(&proposal, mode, "l'agent MCP");
            store.save(&data)?;
            Ok(json!({
                "ajoutes": added, "retires": removed, "concepts": data.ontology.concepts.len(),
                "suite": "Résume l'ontologie à l'utilisateur et demande-lui s'il veut lancer le classement (`classer_projets`) : ne le lance pas sans son accord."
            }))
        }
        "modifier_concept" => {
            let id = args["id"].as_u64();
            if args["supprimer"].as_bool().unwrap_or(false) {
                let id = id.ok_or("Paramètre « id » manquant")?;
                data.ontology.concept(id).ok_or_else(|| format!("Aucun concept avec l'id {id}"))?;
                data.remove_concept(id);
                store.save(&data)?;
                return Ok(json!({ "supprime": id }));
            }
            let facet = args["facette"].as_str().map(str::trim).filter(|f| !f.is_empty()).map(|f| data.ontology.facet_named(f).unwrap_or_else(|| data.ontology.add_facet(f, "")));
            let id = match id {
                Some(id) => {
                    data.ontology.concept(id).ok_or_else(|| format!("Aucun concept avec l'id {id}"))?;
                    id
                }
                None => {
                    let facet = facet.ok_or("Pour créer un concept : « facette » et « nom »")?;
                    let name = args["nom"].as_str().map(str::trim).filter(|n| !n.is_empty()).ok_or("Pour créer un concept : « facette » et « nom »")?;
                    if let Some(existing) = data.ontology.concept_named(facet, name) {
                        return Err(format!("« {name} » existe déjà dans cette facette (id {existing})."));
                    }
                    data.ontology.add_concept(facet, name, "")
                }
            };
            let parent = args["parent"].as_str().map(str::trim);
            let c_facet = facet.or_else(|| data.ontology.concept(id).map(|c| c.facet)).unwrap_or(0);
            let parent_id = match parent {
                Some("") => Some(None),
                Some(p) => Some(Some(data.ontology.concept_named(c_facet, p).ok_or_else(|| format!("Pas de concept « {p} » dans cette facette"))?)),
                None => None,
            };
            let c = data.ontology.concepts.iter_mut().find(|c| c.id == id).ok_or("Concept introuvable")?;
            if let Some(f) = facet {
                c.facet = f;
            }
            if let Some(v) = args["nom"].as_str().map(str::trim).filter(|v| !v.is_empty()) {
                c.name = v.to_owned();
            }
            if let Some(v) = args["definition"].as_str() {
                c.definition = v.trim().to_owned();
            }
            if let Some(v) = strings(&args["synonymes"]) {
                c.synonyms = v;
            }
            if let Some(p) = parent_id {
                c.parent = p.filter(|p| *p != id);
            }
            let out = json!({ "id": c.id, "nom": c.name, "chemin": data.ontology.path(id) });
            data.sync_domains();
            store.save(&data)?;
            Ok(out)
        }
        "rattacher_concept" => {
            let i = find(&data, args)?;
            let concept = args["concept"].as_u64().ok_or("Paramètre « concept » manquant")?;
            let name = data.ontology.concept(concept).map(|c| c.name.clone()).ok_or_else(|| format!("Aucun concept avec l'id {concept}"))?;
            let p = &mut data.projects[i];
            p.tags.retain(|t| t.concept != concept);
            if !args["retirer"].as_bool().unwrap_or(false) {
                p.tags.insert(0, ontology::Tag { concept, weight: 1.0, manual: true });
            }
            let out = json!({ "projet": p.name, "concept": name, "rattache": !args["retirer"].as_bool().unwrap_or(false) });
            store.save(&data)?;
            Ok(out)
        }
        "lister_modeles_decision" => {
            let models = Ollaya::new(&data.config.ollaya_url).models()?;
            Ok(json!({
                "serveur": data.config.ollaya_url,
                "modele_choisi": data.config.ollaya_model,
                "modeles": models.iter().map(|m| json!({ "nom": m.name, "taille": m.size_label(), "parametres": m.parameters })).collect::<Vec<_>>(),
            }))
        }
        "classer_projets" => {
            let model = args["modele"].as_str().map(str::trim).filter(|m| !m.is_empty()).map(str::to_owned).unwrap_or_else(|| data.config.ollaya_model.clone());
            if model.is_empty() {
                return Err("Aucun modèle de décision : l'utilisateur en choisit un dans Atelier (Configuration › IA & modèles › Ollaya), ou passe « modele ».".into());
            }
            if data.ontology.concepts.is_empty() {
                return Err("L'ontologie est vide : propose-la d'abord (`definir_ontologie`).".into());
            }
            let todo: Vec<usize> = match args["ids"].as_array() {
                Some(ids) => ids.iter().filter_map(|v| v.as_u64()).filter_map(|id| data.index_of(id)).collect(),
                None => data.outdated_tags().into_iter().take(40).collect(),
            };
            let ollaya = Ollaya::new(&data.config.ollaya_url);
            let onto = data.ontology.clone();
            let fingerprint = onto.fingerprint();
            let mut done = Vec::new();
            for i in todo {
                let scores = ollaya.classify(&model, &onto, &ontology::project_state(&data, &data.projects[i]))?;
                data.apply_scores(i, &scores, fingerprint);
                let concepts: Vec<Value> = data.concepts_of(i).into_iter().map(|(c, w)| json!({ "concept": data.ontology.path(c.id), "poids": (w * 100.0).round() / 100.0 })).collect();
                done.push(json!({ "id": data.projects[i].id, "nom": data.projects[i].name, "concepts": concepts }));
                // Enregistré au fil de l'eau : l'interface montre l'avancement.
                store.save(&data)?;
            }
            Ok(json!({ "classes": done, "restants": data.outdated_tags().len() }))
        }
        _ => Err(format!("Outil inconnu : {name}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store(tag: &str) -> Store {
        let dir = std::env::temp_dir().join(format!("atelier-mcp-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        Store::open(dir.join("atelier.json"))
    }

    fn call(store: &mut Store, id: u64, method: &str, params: Value) -> Value {
        let line = json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }).to_string();
        serde_json::from_str(&handle_line(store, &line).expect("une réponse")).unwrap()
    }

    fn tool_json(store: &mut Store, name: &str, args: Value) -> Value {
        let r = call(store, 9, "tools/call", json!({ "name": name, "arguments": args }));
        assert_eq!(r["result"]["isError"], false, "{r}");
        serde_json::from_str(r["result"]["content"][0]["text"].as_str().unwrap()).unwrap()
    }

    #[test]
    fn poignee_de_main_et_liste_des_outils() {
        let mut store = temp_store("init");
        let r = call(&mut store, 1, "initialize", json!({ "protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": { "name": "test" } }));
        assert_eq!(r["result"]["protocolVersion"], "2025-06-18");
        assert_eq!(r["result"]["serverInfo"]["name"], "atelier");
        assert!(handle_line(&mut store, r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#).is_none(), "pas de réponse à une notification");
        let tools = call(&mut store, 2, "tools/list", json!({}));
        let names: Vec<&str> = tools["result"]["tools"].as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap()).collect();
        for n in ["scanner_dossiers", "ajouter_projet", "modifier_projet", "supprimer_projet", "ajouter_etape", "cocher_etape"] {
            assert!(names.contains(&n), "{n}");
        }
        assert_eq!(call(&mut store, 3, "inconnue", json!({}))["error"]["code"], -32601);
        // Une marque d'encodage en tête de flux (PowerShell) est tolérée.
        let bom = format!("\u{feff}{}", json!({ "jsonrpc": "2.0", "id": 4, "method": "ping" }));
        assert!(handle_line(&mut store, &bom).unwrap().contains("\"result\""));
    }

    #[test]
    fn consignes_et_prompts() {
        let mut store = temp_store("prompts");
        let r = call(&mut store, 1, "initialize", json!({ "protocolVersion": "2025-06-18" }));
        assert!(r["result"]["capabilities"]["prompts"].is_object());
        let instructions = r["result"]["instructions"].as_str().unwrap();
        assert!(instructions.contains("BRIQUES") && instructions.contains("ranger_mes_projets"));
        let list = call(&mut store, 2, "prompts/list", json!({}));
        let names: Vec<&str> = list["result"]["prompts"].as_array().unwrap().iter().map(|p| p["name"].as_str().unwrap()).collect();
        assert_eq!(names, ["ranger_mes_projets", "concevoir_ontologie", "nettoyer_mes_fiches"]);
        let onto = call(&mut store, 6, "prompts/get", json!({ "name": "concevoir_ontologie" }));
        let text = onto["result"]["messages"][0]["content"]["text"].as_str().unwrap();
        assert!(text.contains("definir_ontologie") && text.contains("sans mon accord"), "{text}");
        let got = call(&mut store, 3, "prompts/get", json!({ "name": "ranger_mes_projets", "arguments": { "dossier": r"C:\Projets" } }));
        let text = got["result"]["messages"][0]["content"]["text"].as_str().unwrap();
        assert!(text.contains(r"sous « C:\Projets »") && text.contains("BRIQUE"), "{text}");
        assert_eq!(call(&mut store, 4, "prompts/get", json!({ "name": "inconnu" }))["error"]["code"], -32602);
        // Les dossiers sources restent l'affaire de l'utilisateur.
        let tools = call(&mut store, 5, "tools/list", json!({}));
        assert!(!tools.to_string().contains("ajouter_dossier_source"));
    }

    /// Un dossier source : « Shop » (un projet sans marqueur, avec ses briques app, db et site),
    /// sa copie, et un dossier de rangement qui contient deux vrais projets.
    fn sample_tree(tag: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!("atelier-mcp-tree-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for (dir, file) in [
            ("Shop/app", "package.json"),
            ("Shop/db", "schema.sql"),
            ("Shop/site", "package.json"),
            ("Shop - Copie/app", "package.json"),
            ("Rangement/Facturo", "Cargo.toml"),
            ("Rangement/Quizz", "package.json"),
        ] {
            std::fs::create_dir_all(root.join(dir)).unwrap();
            std::fs::write(root.join(dir).join(file), "{}").unwrap();
        }
        std::fs::write(root.join("Shop/README.md"), "# Shop\nBoutique en ligne de produits locaux.").unwrap();
        root
    }

    fn with_root(store: &mut Store, root: &std::path::Path) {
        let mut data = Data::fresh();
        data.config.roots.push(crate::model::ScanRoot { path: root.to_string_lossy().into_owned(), depth: 3, ..Default::default() });
        store.save(&data).unwrap();
    }

    #[test]
    fn le_scan_masque_les_briques_et_les_rattache() {
        let root = sample_tree("scan");
        let mut store = temp_store("scan");
        assert_eq!(call(&mut store, 1, "tools/call", json!({ "name": "scanner_dossiers", "arguments": {} }))["result"]["isError"], true, "pas de dossier source");
        with_root(&mut store, &root);
        let scan = tool_json(&mut store, "scanner_dossiers", json!({}));
        assert!(scan["rappel"].as_str().unwrap().contains("dossiers_supplementaires"));
        let dirs = scan["dossiers"].as_array().unwrap();
        let by_name = |n: &str| dirs.iter().find(|d| d["nom"] == n);
        let shop = by_name("Shop").expect("le parent est proposé");
        assert_eq!((shop["projet_probable"].as_bool(), shop["briques"].as_array().unwrap().len()), (Some(true), 3));
        let shown = |suffix: &str| dirs.iter().any(|d| d["chemin"].as_str().unwrap().ends_with(suffix));
        assert!(!shown(r"Shop\app") && !shown(r"Shop\db") && !shown(r"Shop\site"), "les briques sont masquées");
        assert!(scan["briques_masquees"].as_u64().unwrap() >= 3);
        assert_eq!(by_name("Shop - Copie").unwrap()["copie_probable"], true);
        assert!(by_name("Facturo").is_some() && by_name("Quizz").is_some(), "un dossier de rangement n'absorbe pas ses projets");

        // L'agent crée le projet avec ses briques : elles ne reviennent plus au scan.
        let bricks: Vec<Value> = shop["briques"].as_array().unwrap().clone();
        let created = tool_json(&mut store, "ajouter_projet", json!({ "nom": "Shop", "description": "Boutique en ligne de produits locaux.", "dossier": shop["chemin"], "dossiers_supplementaires": bricks }));
        assert_eq!(created["projet"]["dossiers"].as_array().unwrap().len(), 4);
        let again = tool_json(&mut store, "scanner_dossiers", json!({ "inclure_briques": true }));
        assert!(again["dossiers"].as_array().unwrap().iter().all(|d| d["nom"] != "app" || d["chemin"].as_str().unwrap().contains("Copie")));

        // La copie est écartée des prochains scans.
        let copy = root.join("Shop - Copie").to_string_lossy().into_owned();
        tool_json(&mut store, "ignorer_dossier", json!({ "dossier": copy }));
        let last = tool_json(&mut store, "scanner_dossiers", json!({}));
        assert!(last["dossiers"].as_array().unwrap().iter().all(|d| !d["chemin"].as_str().unwrap().contains("Copie")));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn reperer_et_fusionner_des_fiches_briques() {
        let root = sample_tree("fusion");
        let mut store = temp_store("fusion");
        with_root(&mut store, &root);
        let path = |p: &str| root.join(p).to_string_lossy().into_owned();
        // Des fiches créées « à l'aveugle » pour les briques (comme le mode règles d'avant).
        let mut ids = Vec::new();
        for b in ["app", "db", "site"] {
            let c = tool_json(&mut store, "ajouter_projet", json!({ "nom": b, "description": "Brique.", "dossier": path(&format!("Shop/{b}")) }));
            ids.push(c["id"].as_u64().unwrap());
        }
        tool_json(&mut store, "ajouter_etape", json!({ "id": ids[0], "texte": "Brancher le paiement" }));
        tool_json(&mut store, "ajouter_projet", json!({ "nom": "Facturo", "description": "Facturation.", "dossier": path("Rangement/Facturo") }));

        let found = tool_json(&mut store, "reperer_briques", json!({}));
        let groups = found["groupes"].as_array().unwrap();
        let shop = groups.iter().find(|g| g["dossier_parent"].as_str().unwrap().ends_with("Shop")).expect("groupe Shop");
        assert_eq!(shop["fiches"].as_array().unwrap().len(), 3);
        assert!(shop["readme_du_parent"].as_str().unwrap().contains("Boutique"));
        assert!(groups.iter().all(|g| !g["dossier_parent"].as_str().unwrap().ends_with("Rangement")), "un projet seul dans un rangement n'est pas suspect");

        // Une brique déjà suivie ne peut pas être rattachée ailleurs sans fusion.
        let parent = tool_json(&mut store, "ajouter_projet", json!({ "nom": "Shop", "description": "Boutique en ligne.", "dossier": path("Shop") }));
        let pid = parent["id"].as_u64().unwrap();
        let r = call(&mut store, 7, "tools/call", json!({ "name": "rattacher_dossier", "arguments": { "id": pid, "dossier": path("Shop/app") } }));
        assert!(r["result"]["content"][0]["text"].as_str().unwrap().contains("fusionner_projets"));

        let merged = tool_json(&mut store, "fusionner_projets", json!({ "id": pid, "ids": ids }));
        assert_eq!(merged["fusionnees"].as_array().unwrap().len(), 3);
        assert_eq!(merged["projet"]["dossiers"].as_array().unwrap().len(), 4);
        assert_eq!(merged["projet"]["etapes"], 1, "les étapes suivent");
        assert_eq!(tool_json(&mut store, "lister_projets", json!({})).as_array().unwrap().len(), 2);
        // Et la copie se rattache au projet d'origine.
        tool_json(&mut store, "rattacher_dossier", json!({ "id": pid, "dossier": path("Shop - Copie") }));
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Essai réel, sur une COPIE des données et le vrai serveur Ollaya (ignoré par défaut) :
    /// `ATELIER_ESSAI=chemin\copie\atelier.json cargo test essai_reel_ollaya -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn essai_reel_ollaya() {
        let path = std::env::var("ATELIER_ESSAI").expect("ATELIER_ESSAI : le chemin d'une copie de atelier.json");
        let mut store = Store::open(path);
        let model = std::env::var("ATELIER_ESSAI_MODELE").unwrap_or_else(|_| "decider:2b".into());
        println!("Modèles : {}", run_tool(&mut store, "lister_modeles_decision", &json!({})).unwrap()["modeles"]);
        let nature = json!({ "mode": "ajouter", "facettes": [{ "nom": "Nature", "description": "Ce qu'est le projet", "concepts": [
            { "nom": "Application desktop", "definition": "Un logiciel installé sur l'ordinateur, avec une interface graphique.", "synonymes": ["bureau", "logiciel"] },
            { "nom": "Application web", "definition": "Un site ou une application qui tourne dans le navigateur, avec un serveur.", "synonymes": ["site", "saas"] },
            { "nom": "Outil en ligne de commande", "definition": "Un programme utilisé dans le terminal, sans interface graphique.", "synonymes": ["cli", "terminal"] },
            { "nom": "Agent IA", "definition": "Un système qui utilise des modèles de langage pour agir de façon autonome.", "synonymes": ["llm", "agent"] },
            { "nom": "Apprentissage", "definition": "Un tutoriel, un cours ou un exercice pour apprendre une technologie.", "synonymes": ["tuto", "cours"] }
        ]}]});
        println!("definir_ontologie : {}", run_tool(&mut store, "definir_ontologie", &nature).unwrap());
        let projects = run_tool(&mut store, "lister_projets", &json!({})).unwrap();
        let wanted = ["atelier", "MetaGPT", "tuto1", "clone", "TechniApp"];
        let ids: Vec<u64> = projects.as_array().unwrap().iter().filter(|p| wanted.contains(&p["nom"].as_str().unwrap_or(""))).filter_map(|p| p["id"].as_u64()).collect();
        let start = std::time::Instant::now();
        let r = run_tool(&mut store, "classer_projets", &json!({ "ids": ids, "modele": model })).unwrap();
        println!("classer_projets en {:.1} s :", start.elapsed().as_secs_f32());
        for c in r["classes"].as_array().unwrap() {
            println!("  {} → {}", c["nom"], c["concepts"]);
        }
    }

    #[test]
    fn ontologie_et_classement_par_l_agent() {
        let mut store = temp_store("onto");
        let created = tool_json(&mut store, "ajouter_projet", json!({ "nom": "ImmoFormation", "description": "Formation aux métiers de l'immobilier.", "domaine": "Formation" }));
        let id = created["id"].as_u64().unwrap();
        let r = tool_json(&mut store, "definir_ontologie", json!({ "facettes": [
            { "nom": "Domaine", "description": "Le sujet", "concepts": [
                { "nom": "Immobilier", "definition": "Logement, agences, biens.", "synonymes": ["logement"] },
                { "nom": "Formation", "definition": "Apprendre, enseigner.", "synonymes": ["cours"] }
            ]},
            { "nom": "Nature", "description": "Ce que c'est", "concepts": [
                { "nom": "Plateforme web", "definition": "Une application dans le navigateur.", "synonymes": [] }
            ]}
        ]}));
        assert!(r["ajoutes"].as_u64().unwrap() >= 2, "{r}");
        let onto = tool_json(&mut store, "lire_ontologie", json!({}));
        let facets = onto["facettes"].as_array().unwrap();
        let concept_id = |name: &str| facets.iter().flat_map(|f| f["concepts"].as_array().unwrap()).find(|c| c["nom"] == name).and_then(|c| c["id"].as_u64()).unwrap();
        let (immo, web) = (concept_id("Immobilier"), concept_id("Plateforme web"));

        // Un concept retouché par l'agent : sous « Immobilier ».
        let moved = tool_json(&mut store, "modifier_concept", json!({ "facette": "Domaine", "nom": "Location", "definition": "Louer un bien.", "parent": "Immobilier" }));
        assert_eq!(moved["chemin"], "Immobilier › Location");

        // Classement par un faux Ollaya : Immobilier oui, le reste non.
        let mut data = Store::open(store.path()).load().unwrap().unwrap();
        let answers: serde_json::Map<String, Value> = data.ontology.concepts.iter().map(|c| (format!("c{}", c.id), json!({ "type": "noul", "noul": if c.id == immo { 0.94 } else { 0.08 } }))).collect();
        data.config.ollaya_url = crate::ollama::tests::fake_server(vec![json!({ "answers": answers }).to_string()]);
        data.config.ollaya_model = "decider:2b".into();
        store.save(&data).unwrap();
        let r = tool_json(&mut store, "classer_projets", json!({ "ids": [id] }));
        assert_eq!(r["classes"][0]["concepts"][0]["concept"], "Immobilier", "{r}");

        // Un rattachement à la main, gardé par les classements suivants.
        tool_json(&mut store, "rattacher_concept", json!({ "id": id, "concept": web }));
        let listed = tool_json(&mut store, "lister_projets", json!({}));
        let concepts: Vec<&str> = listed[0]["concepts"].as_array().unwrap().iter().filter_map(|c| c.as_str()).collect();
        assert!(concepts.contains(&"Immobilier") && concepts.contains(&"Plateforme web"), "{concepts:?}");
        let data = Store::open(store.path()).load().unwrap().unwrap();
        let p = &data.projects[data.index_of(id).unwrap()];
        assert!(p.tags.iter().any(|t| t.concept == web && t.manual));
        assert_eq!(data.domains[p.domain].name, "Immobilier", "le meilleur domaine devient le domaine du projet");
    }

    #[test]
    fn creer_modifier_cocher_supprimer() {
        let mut store = temp_store("crud");
        let created = tool_json(&mut store, "ajouter_projet", json!({ "nom": "ImmoFormation", "description": "Formation aux métiers de l'immobilier.", "domaine": "Formation", "attention": "Focus" }));
        let id = created["id"].as_u64().unwrap();
        tool_json(&mut store, "modifier_projet", json!({ "id": id, "prochaine_action": "Finaliser le plan", "etat": "En pause" }));
        tool_json(&mut store, "ajouter_etape", json!({ "id": id, "texte": "Rédiger la leçon 1" }));
        let steps = tool_json(&mut store, "cocher_etape", json!({ "id": id, "index": 0, "fait": true }));
        assert_eq!(steps["etapes"][0]["done"], true);

        // Le fichier contient bien tout ça (c'est ce que l'interface relira).
        let data = Store::open(store.path()).load().unwrap().unwrap();
        let p = &data.projects[data.index_of(id).unwrap()];
        assert_eq!((p.attention, p.status, p.next_action.as_str()), (Attention::Focus, Status::Paused, "Finaliser le plan"));
        assert_eq!(p.added_by, AddedBy::Mcp);

        let listed = tool_json(&mut store, "lister_projets", json!({ "filtre": "imo" }));
        assert_eq!(listed[0]["id"], id, "recherche floue côté MCP aussi");
        tool_json(&mut store, "supprimer_projet", json!({ "id": id }));
        assert!(tool_json(&mut store, "lister_projets", json!({})).as_array().unwrap().is_empty());
        let r = call(&mut store, 5, "tools/call", json!({ "name": "voir_projet", "arguments": { "id": id } }));
        assert_eq!(r["result"]["isError"], true);
    }
}
