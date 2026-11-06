//! Le contexte donné à l'agent MCP, pour qu'il connaisse les projets sans fouiller le disque :
//! un résumé dès la connexion, une vue d'ensemble (par attention, domaines, technos), une
//! recherche hybride (mots, sens par les embeddings, ontologie), les projets proches d'un projet
//! et les technologies utilisées. Ce module calcule ; `mcp.rs` expose les outils.

use std::collections::HashMap;

use serde_json::{Value, json};

use crate::model::{Attention, Data, Project, Status};
use crate::packages::{Manager, Package};
use crate::store::SemanticIndex;

/// Une fiche en quelques champs : de quoi juger sans tout ouvrir.
fn brief(data: &Data, i: usize) -> Value {
    let p = &data.projects[i];
    json!({
        "id": p.id,
        "nom": p.name,
        "description": p.description,
        "domaines": data.domains_of(i).into_iter().filter_map(|d| data.domains.get(d).map(|x| x.name.clone())).collect::<Vec<_>>(),
        "technos": p.stack,
        "etat": p.status.label(),
        "attention": p.attention.label(),
        "inactif_depuis_jours": p.idle_days,
    })
}

fn attention_of(p: &Project) -> &'static str {
    if p.status == Status::Archived {
        return "archives";
    }
    match p.attention {
        Attention::Focus => "focus",
        Attention::Flow => "au_fil_de_l_eau",
        Attention::Reserve => "en_reserve",
    }
}

/// Le résumé glissé dans les consignes à la connexion : l'agent sait d'emblée où en sont les
/// projets et comment chercher.
pub fn snapshot(data: &Data, index: &SemanticIndex) -> String {
    let count = |a: &str| data.projects.iter().filter(|p| attention_of(p) == a).count();
    let focus: Vec<&str> = data.projects.iter().filter(|p| attention_of(p) == "focus").map(|p| p.name.as_str()).collect();
    let mut domains: Vec<(String, usize)> = data.domains.iter().enumerate().map(|(d, x)| (x.name.clone(), (0..data.projects.len()).filter(|&i| data.has_domain(i, d)).count())).filter(|(_, n)| *n > 0).collect();
    domains.sort_by_key(|d| std::cmp::Reverse(d.1));
    let model = &data.config.ollama_embed_model;
    let indexed = data.projects.iter().filter(|p| index.vectors.contains_key(&p.id)).count();
    let semantic = if data.config.ollama_on && !model.is_empty() && index.model == *model && indexed > 0 {
        format!("prête ({model}, {indexed}/{} fiches indexées)", data.projects.len())
    } else {
        "indisponible (recherche par les mots et l'ontologie seulement)".to_owned()
    };
    format!(
        "\n\nCONTEXTE ACTUEL (à la connexion) — {} projets. Focus ({}) : {}. Au fil de l'eau : {}. En réserve : {}. Archivés : {}. \
Domaines : {}. Recherche par le sens : {}. Pour comprendre ou conseiller : `vue_d_ensemble`, `chercher_projets` (mots, sens, \
ontologie), `projets_proches`, `technologies`, `voir_projet` ; méthode complète : prompt `conseiller`. Ne fouille pas le \
disque : les fiches donnent les dossiers, lis seulement le README ou le CLAUDE.md d'un projet si la fiche ne suffit pas.",
        data.projects.len(),
        focus.len(),
        if focus.is_empty() { "aucun".to_owned() } else { focus.join(", ") },
        count("au_fil_de_l_eau"),
        count("en_reserve"),
        count("archives"),
        domains.iter().take(10).map(|(n, c)| format!("{n} {c}")).collect::<Vec<_>>().join(", "),
        semantic
    )
}

/// La vue d'ensemble : les projets par attention, les domaines, les technos les plus utilisées.
pub fn overview(data: &Data) -> Value {
    let mut groups: HashMap<&str, Vec<Value>> = HashMap::new();
    let mut order: Vec<usize> = (0..data.projects.len()).collect();
    order.sort_by_key(|&i| data.projects[i].idle_days);
    for i in order {
        groups.entry(attention_of(&data.projects[i])).or_default().push(brief(data, i));
    }
    let mut domains: Vec<Value> = data
        .domains
        .iter()
        .enumerate()
        .map(|(d, x)| (x.name.clone(), (0..data.projects.len()).filter(|&i| data.has_domain(i, d)).count()))
        .filter(|(_, n)| *n > 0)
        .map(|(n, c)| json!({ "nom": n, "projets": c }))
        .collect();
    domains.sort_by_key(|v| std::cmp::Reverse(v["projets"].as_u64().unwrap_or(0)));
    let mut technos: HashMap<&str, usize> = HashMap::new();
    for p in &data.projects {
        for t in &p.stack {
            *technos.entry(t.as_str()).or_default() += 1;
        }
    }
    let mut technos: Vec<(&str, usize)> = technos.into_iter().collect();
    technos.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    let mut take = |k: &str| groups.remove(k).unwrap_or_default();
    json!({
        "focus": take("focus"),
        "au_fil_de_l_eau": take("au_fil_de_l_eau"),
        "en_reserve": take("en_reserve"),
        "archives": take("archives"),
        "domaines": domains,
        "technos": technos.iter().take(25).map(|(t, c)| json!({ "techno": t, "projets": c })).collect::<Vec<_>>(),
        "note": "Projets triés du plus récemment actif au plus ancien. Détail : voir_projet ; voisins : projets_proches."
    })
}

/// La recherche hybride : par les mots (flou, mots-clés, paquets), par le sens (si l'embedding
/// de la question est là) et par l'ontologie. Chaque résultat dit par quoi il a été trouvé.
pub fn search(data: &Data, index: &SemanticIndex, query: &str, query_vec: Option<&[f32]>, limit: usize) -> Value {
    let hits = crate::search::rank(data, query, query_vec, index);
    json!(hits
        .iter()
        .take(limit)
        .map(|h| {
            let by = match (h.via.and_then(|c| data.ontology.concept(c)), h.semantic_only) {
                (Some(c), _) => format!("tag « {} »", data.ontology.path(c.id)),
                (None, true) => "sens proche".to_owned(),
                (None, false) => "mots".to_owned(),
            };
            let mut v = brief(data, h.index);
            v["trouve_par"] = json!(by);
            v["score"] = json!((h.score * 100.0).round() / 100.0);
            v
        })
        .collect::<Vec<_>>())
}

/// Les paquets de chaque projet, relus dans les manifestes (les fiches peuvent dater).
pub fn fresh_packages(data: &Data) -> Vec<Vec<Package>> {
    data.projects.iter().map(|p| if p.packages.is_empty() { crate::packages::of_folders(&p.folders) } else { p.packages.clone() }).collect()
}

/// Les projets proches d'un projet : par le sens (embeddings), les tags (les rares comptent
/// plus), les paquets et les technos, les domaines, le dossier parent. Avec les raisons.
pub fn neighbors(data: &Data, index: &SemanticIndex, i: usize, packages: &[Vec<Package>], limit: usize) -> Value {
    let p = &data.projects[i];
    // Les tags, sans les domaines de premier niveau (comptés à part, plus bas).
    let domain_facet = data.ontology.domain_facet();
    let tags = |k: usize| -> Vec<u64> {
        data.projects[k]
            .tags
            .iter()
            .filter(|t| t.weight >= crate::ontology::LINK_MIN)
            .filter(|t| data.ontology.concept(t.concept).is_some_and(|c| !(Some(c.facet) == domain_facet && c.parent.is_none())))
            .map(|t| t.concept)
            .collect()
    };
    let mut df: HashMap<u64, usize> = HashMap::new();
    for k in 0..data.projects.len() {
        for c in tags(k) {
            *df.entry(c).or_default() += 1;
        }
    }
    let n = data.projects.len().max(1) as f32;
    let idf = |c: &u64| (1.0 + n / df.get(c).copied().unwrap_or(1) as f32).ln();
    let (my_tags, my_domains) = (tags(i), data.domains_of(i));
    let my_vec = index.vectors.get(&p.id).map(|(_, v)| v.as_slice());
    let parent = p.folders.first().and_then(|f| crate::model::parent_path(f)).map(crate::model::path_key);
    let mut scored: Vec<(f32, Value)> = Vec::new();
    for k in (0..data.projects.len()).filter(|&k| k != i) {
        let q = &data.projects[k];
        let mut reasons: Vec<String> = Vec::new();
        let mut score = 0.0;
        if let (Some(a), Some((_, b))) = (my_vec, index.vectors.get(&q.id)) {
            let sim = crate::search::cosine(a, b);
            score += 0.45 * sim.max(0.0);
            if sim >= 0.6 {
                reasons.push(format!("sens proche ({:.0} %)", sim * 100.0));
            }
        }
        let theirs = tags(k);
        let shared: Vec<&u64> = my_tags.iter().filter(|c| theirs.contains(c)).collect();
        if !shared.is_empty() {
            let union: f32 = my_tags.iter().chain(theirs.iter().filter(|c| !my_tags.contains(c))).map(idf).sum();
            score += 0.3 * shared.iter().map(|c| idf(c)).sum::<f32>() / union.max(0.001);
            reasons.push(format!("tags : {}", shared.iter().filter_map(|c| data.ontology.concept(**c).map(|x| x.name.clone())).collect::<Vec<_>>().join(", ")));
        }
        let common: Vec<&Package> = packages[i].iter().filter(|x| x.manager != Manager::Docker && packages[k].contains(x)).collect();
        if !common.is_empty() {
            let total = packages[i].len() + packages[k].len() - common.len();
            score += 0.15 * common.len() as f32 / total.max(1) as f32;
            reasons.push(format!("paquets : {}", common.iter().take(6).map(|x| x.name.as_str()).collect::<Vec<_>>().join(", ")));
        }
        let stack: Vec<&String> = p.stack.iter().filter(|s| q.stack.contains(s)).collect();
        if !stack.is_empty() && common.is_empty() {
            score += 0.05 * stack.len() as f32 / p.stack.len().max(1) as f32;
            reasons.push(format!("technos : {}", stack.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")));
        }
        let domains: Vec<String> = data.domains_of(k).into_iter().filter(|d| my_domains.contains(d)).filter_map(|d| data.domains.get(d).map(|x| x.name.clone())).collect();
        if !domains.is_empty() {
            score += 0.05;
            reasons.push(format!("domaine : {}", domains.join(", ")));
        }
        if parent.is_some() && q.folders.first().and_then(|f| crate::model::parent_path(f)).map(crate::model::path_key) == parent {
            score += 0.05;
            reasons.push("même dossier parent".to_owned());
        }
        if score > 0.08 && !reasons.is_empty() {
            let mut v = brief(data, k);
            v["pourquoi"] = json!(reasons);
            v["proximite"] = json!((score * 100.0).round() / 100.0);
            scored.push((score, v));
        }
    }
    scored.sort_by(|a, b| b.0.total_cmp(&a.0));
    json!({ "projet": brief(data, i), "proches": scored.into_iter().take(limit).map(|(_, v)| v).collect::<Vec<_>>() })
}

/// Les technologies : les technos des fiches et les paquets (par gestionnaire), avec les projets
/// qui les utilisent. Avec `techno` : seulement les projets qui utilisent celle-là.
pub fn technologies(data: &Data, packages: &[Vec<Package>], techno: Option<&str>) -> Value {
    let names = |list: &[usize]| list.iter().map(|&i| data.projects[i].name.clone()).collect::<Vec<_>>();
    if let Some(t) = techno.map(str::trim).filter(|t| !t.is_empty()) {
        let words: Vec<String> = t.to_lowercase().split_whitespace().map(str::to_owned).collect();
        let using: Vec<usize> = (0..data.projects.len())
            .filter(|&i| data.projects[i].stack.iter().any(|s| s.eq_ignore_ascii_case(t)) || crate::packages::match_score(&words, &packages[i]) >= 0.9)
            .collect();
        return json!({ "techno": t, "projets": using.iter().map(|&i| {
            let mut v = brief(data, i);
            v["paquets_lies"] = json!(packages[i].iter().filter(|p| crate::packages::match_score(&words, std::slice::from_ref(p)) >= 0.9).map(|p| format!("{} {}", p.manager.label(), p.name)).collect::<Vec<_>>());
            v
        }).collect::<Vec<_>>() });
    }
    let mut stack: HashMap<&str, Vec<usize>> = HashMap::new();
    for (i, p) in data.projects.iter().enumerate() {
        for s in &p.stack {
            stack.entry(s.as_str()).or_default().push(i);
        }
    }
    let mut stack: Vec<(&str, Vec<usize>)> = stack.into_iter().collect();
    stack.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(b.0)));
    let mut by_manager: Vec<Value> = Vec::new();
    for m in Manager::ALL.iter().copied().chain([Manager::Docker]) {
        let mut users: HashMap<&str, Vec<usize>> = HashMap::new();
        for (i, list) in packages.iter().enumerate() {
            for p in list.iter().filter(|p| p.manager == m) {
                users.entry(p.name.as_str()).or_default().push(i);
            }
        }
        if users.is_empty() {
            continue;
        }
        let total = users.len();
        let mut users: Vec<(&str, Vec<usize>)> = users.into_iter().collect();
        users.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(b.0)));
        by_manager.push(json!({
            "gestionnaire": m.label(),
            "paquets_differents": total,
            "les_plus_utilises": users.iter().take(15).map(|(n, l)| json!({ "paquet": n, "projets": names(l) })).collect::<Vec<_>>()
        }));
    }
    json!({
        "technos": stack.iter().take(30).map(|(t, l)| json!({ "techno": t, "projets": names(l) })).collect::<Vec<_>>(),
        "paquets": by_manager,
        "note": "Pour une techno ou un paquet précis : technologies avec « techno »."
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contexte_pour_l_agent() {
        let mut data = crate::model::demo();
        data.ensure_ontology();
        let index = SemanticIndex::default();
        let snap = snapshot(&data, &index);
        assert!(snap.contains("Focus") && snap.contains("chercher_projets") && snap.contains("indisponible"), "{snap}");
        let o = overview(&data);
        let total: usize = ["focus", "au_fil_de_l_eau", "en_reserve", "archives"].iter().map(|k| o[k].as_array().unwrap().len()).sum();
        assert_eq!(total, data.projects.len(), "chaque projet dans un groupe");
        // Deux projets qui partagent des paquets et un dossier parent sont proches.
        let pk = |n: &str| Package { manager: Manager::Npm, name: n.into() };
        let mut packages = vec![Vec::new(); data.projects.len()];
        packages[0] = vec![pk("expo"), pk("react")];
        packages[1] = vec![pk("expo"), pk("react"), pk("zod")];
        let near = neighbors(&data, &index, 0, &packages, 5);
        let first = &near["proches"][0];
        assert_eq!(first["id"], data.projects[1].id, "{near}");
        assert!(first["pourquoi"].to_string().contains("expo"));
        let tech = technologies(&data, &packages, Some("expo"));
        assert_eq!(tech["projets"].as_array().unwrap().len(), 2);
        let all = technologies(&data, &packages, None);
        assert_eq!(all["paquets"][0]["gestionnaire"], "npm");
    }
}
