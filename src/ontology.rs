//! L'ontologie des projets : des facettes (axes indépendants : domaine, nature, finalité…), des
//! concepts définis dans chaque facette (définition, synonymes, parent), et les rattachements
//! pondérés des projets à ces concepts.
//!
//! - Une IA qui voit tous les projets d'un coup la **propose** (Ollama, ou l'agent MCP) ;
//!   l'utilisateur la **retouche à la main** (ce qu'il a touché est gardé aux propositions suivantes).
//! - Un modèle de décision **Ollaya** classe chaque projet : une question oui/non par concept,
//!   avec sa définition comme critère. La probabilité devient le poids du rattachement.
//! - La recherche s'en sert : « logement » trouve le concept « Immobilier » (synonyme, ou sens
//!   proche par embeddings), puis les projets qui y sont rattachés.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::model::{Data, Project};

/// La facette reprise des anciens domaines : son meilleur concept donne le domaine du projet.
pub const DOMAIN_FACET: &str = "Domaine";
/// À partir de ce poids, un projet est rattaché au concept (affichage, recherche).
pub const LINK_MIN: f32 = 0.5;
/// En dessous, un rattachement proposé par Ollaya n'est pas gardé.
const KEEP_MIN: f32 = 0.2;
/// Ollaya accepte 64 questions par requête ; on reste en dessous.
pub const QUESTIONS_PER_CALL: usize = 48;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Ontology {
    pub facets: Vec<Facet>,
    pub concepts: Vec<Concept>,
    pub next_id: u64,
    /// Dernière proposition de l'IA appliquée : quand (horodatage Unix, 0 = jamais) et par quoi.
    pub generated_at: u64,
    pub generated_by: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Facet {
    pub id: u64,
    pub name: String,
    pub description: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Concept {
    pub id: u64,
    pub facet: u64,
    pub name: String,
    /// Une phrase : elle sert de critère au modèle de décision.
    pub definition: String,
    /// D'autres mots pour le dire (« logement » pour « Immobilier ») : la recherche s'en sert.
    pub synonyms: Vec<String>,
    /// Concept plus général, dans la même facette.
    pub parent: Option<u64>,
    /// Créé ou retouché à la main : une nouvelle proposition de l'IA le garde tel quel.
    pub manual: bool,
}

/// Le rattachement d'un projet à un concept.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tag {
    pub concept: u64,
    /// 0–1 : la probabilité donnée par Ollaya, ou 1 pour un rattachement fait à la main.
    pub weight: f32,
    /// Fait à la main : un nouveau classement ne le touche pas.
    pub manual: bool,
}

/// Comment appliquer une proposition de l'IA.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Merge {
    /// Les concepts proposés remplacent ceux de l'IA ; les concepts manuels restent.
    Replace,
    /// Seuls les concepts qui n'existent pas encore sont ajoutés.
    AddNew,
}

fn same_name(a: &str, b: &str) -> bool {
    crate::search::fold(a.trim()) == crate::search::fold(b.trim())
}

impl Ontology {
    fn new_id(&mut self) -> u64 {
        self.next_id = self.next_id.max(self.facets.iter().map(|f| f.id).chain(self.concepts.iter().map(|c| c.id)).max().unwrap_or(0) + 1);
        self.next_id += 1;
        self.next_id - 1
    }

    pub fn facet(&self, id: u64) -> Option<&Facet> {
        self.facets.iter().find(|f| f.id == id)
    }

    pub fn concept(&self, id: u64) -> Option<&Concept> {
        self.concepts.iter().find(|c| c.id == id)
    }

    pub fn facet_named(&self, name: &str) -> Option<u64> {
        self.facets.iter().find(|f| same_name(&f.name, name)).map(|f| f.id)
    }

    /// La facette des domaines : « Domaine » (ou « Domain », dans des données en anglais).
    pub fn domain_facet(&self) -> Option<u64> {
        self.facet_named(DOMAIN_FACET).or_else(|| self.facet_named("Domain"))
    }

    pub fn concept_named(&self, facet: u64, name: &str) -> Option<u64> {
        self.concepts.iter().find(|c| c.facet == facet && same_name(&c.name, name)).map(|c| c.id)
    }

    pub fn add_facet(&mut self, name: &str, description: &str) -> u64 {
        let id = self.new_id();
        self.facets.push(Facet { id, name: name.trim().to_owned(), description: description.trim().to_owned() });
        id
    }

    pub fn add_concept(&mut self, facet: u64, name: &str, definition: &str) -> u64 {
        let id = self.new_id();
        self.concepts.push(Concept { id, facet, name: name.trim().to_owned(), definition: definition.trim().to_owned(), ..Default::default() });
        id
    }

    /// Les concepts d'une facette, chaque parent suivi de ses enfants, avec leur profondeur.
    pub fn tree(&self, facet: u64) -> Vec<(usize, &Concept)> {
        fn walk<'a>(o: &'a Ontology, facet: u64, parent: Option<u64>, depth: usize, out: &mut Vec<(usize, &'a Concept)>) {
            for c in o.concepts.iter().filter(|c| c.facet == facet && c.parent == parent) {
                out.push((depth, c));
                if depth < 6 {
                    walk(o, facet, Some(c.id), depth + 1, out);
                }
            }
        }
        let mut out = Vec::new();
        walk(self, facet, None, 0, &mut out);
        // Un parent introuvable (supprimé, autre facette) : le concept remonte au premier niveau.
        for c in self.concepts.iter().filter(|c| c.facet == facet) {
            if !out.iter().any(|(_, x)| x.id == c.id) {
                out.push((0, c));
            }
        }
        out
    }

    /// « Formation › Immobilier » : le concept et ses parents.
    pub fn path(&self, id: u64) -> String {
        let mut names = Vec::new();
        let mut cur = self.concept(id);
        while let Some(c) = cur {
            if names.len() > 6 {
                break;
            }
            names.push(c.name.as_str());
            cur = c.parent.and_then(|p| self.concept(p));
        }
        names.reverse();
        names.join(" › ")
    }

    /// Une empreinte de ce qui compte pour le classement (noms, définitions, facettes) : un projet
    /// classé avec une autre empreinte est à reclasser.
    pub fn fingerprint(&self) -> u64 {
        let mut text = String::new();
        for c in &self.concepts {
            let facet = self.facet(c.facet).map(|f| f.name.as_str()).unwrap_or("");
            text.push_str(&format!("{}|{}|{}|{}\n", c.id, facet, c.name, c.definition));
        }
        crate::store::text_hash(&text)
    }

    /// Le texte d'un concept pour les embeddings.
    pub fn concept_text(&self, c: &Concept) -> String {
        let facet = self.facet(c.facet).map(|f| f.name.as_str()).unwrap_or("");
        format!("{} ({facet}). {} {}", c.name, c.definition, c.synonyms.join(", "))
    }
}

// ---- dans les données --------------------------------------------------------------------------

impl Data {
    /// Première ouverture avec l'ontologie : les domaines deviennent la facette « Domaine », et
    /// chaque projet y est rattaché par son domaine actuel.
    pub fn ensure_ontology(&mut self) {
        if !self.ontology.facets.is_empty() {
            return;
        }
        let facet = self.ontology.add_facet(DOMAIN_FACET, "Le sujet principal du projet : de quoi il parle, à qui il sert.");
        let ids: Vec<u64> = self.domains.iter().map(|d| self.ontology.add_concept(facet, &d.name, &format!("Projets du domaine « {} ».", d.name))).collect();
        for p in &mut self.projects {
            if let Some(&id) = ids.get(p.domain)
                && p.tags.is_empty()
            {
                p.tags.push(Tag { concept: id, weight: 1.0, manual: false });
            }
        }
    }

    /// Chaque concept de premier niveau de la facette « Domaine » a son domaine (carte, filtres).
    pub fn sync_domains(&mut self) {
        let Some(facet) = self.ontology.domain_facet() else { return };
        let names: Vec<String> = self.ontology.concepts.iter().filter(|c| c.facet == facet && c.parent.is_none() && !c.name.trim().is_empty()).map(|c| c.name.clone()).collect();
        for n in names {
            self.domain_named(&n);
        }
    }

    /// Supprime un concept : ses enfants remontent d'un niveau, les rattachements disparaissent.
    pub fn remove_concept(&mut self, id: u64) {
        let parent = self.ontology.concept(id).and_then(|c| c.parent);
        for c in &mut self.ontology.concepts {
            if c.parent == Some(id) {
                c.parent = parent;
            }
        }
        self.ontology.concepts.retain(|c| c.id != id);
        for p in &mut self.projects {
            p.tags.retain(|t| t.concept != id);
        }
    }

    pub fn remove_facet(&mut self, id: u64) {
        let ids: Vec<u64> = self.ontology.concepts.iter().filter(|c| c.facet == id).map(|c| c.id).collect();
        for c in ids {
            self.remove_concept(c);
        }
        self.ontology.facets.retain(|f| f.id != id);
    }

    /// Combien de projets sont rattachés à ce concept.
    pub fn concept_count(&self, id: u64) -> usize {
        self.projects.iter().filter(|p| p.tags.iter().any(|t| t.concept == id && t.weight >= LINK_MIN)).count()
    }

    /// Les projets classés avec une autre version de l'ontologie (ou jamais classés).
    pub fn outdated_tags(&self) -> Vec<usize> {
        let fp = self.ontology.fingerprint();
        (0..self.projects.len()).filter(|&i| self.projects[i].tags_hash != fp).collect()
    }

    /// Applique les réponses d'Ollaya à un projet : les rattachements manuels restent, les autres
    /// sont remplacés ; le meilleur concept de la facette « Domaine » donne son domaine.
    pub fn apply_scores(&mut self, i: usize, scores: &[(u64, f32)], fingerprint: u64) {
        let domain_facet = self.ontology.domain_facet();
        // Un tag retiré à la main (poids 0) ne revient pas, pas même comme domaine principal.
        let rejected: Vec<u64> = self.projects[i].tags.iter().filter(|t| t.manual && t.weight < LINK_MIN).map(|t| t.concept).collect();
        let best_domain = scores
            .iter()
            .filter(|(id, _)| !rejected.contains(id))
            .filter(|(id, w)| *w >= LINK_MIN && self.ontology.concept(*id).is_some_and(|c| Some(c.facet) == domain_facet && c.parent.is_none()))
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .and_then(|(id, _)| self.ontology.concept(*id))
            .map(|c| c.name.clone());
        let p = &mut self.projects[i];
        p.tags.retain(|t| t.manual);
        for &(concept, weight) in scores {
            if weight >= KEEP_MIN && !p.tags.iter().any(|t| t.concept == concept) {
                p.tags.push(Tag { concept, weight, manual: false });
            }
        }
        p.tags.sort_by(|a, b| b.weight.total_cmp(&a.weight));
        p.tags_hash = fingerprint;
        if let Some(name) = best_domain {
            let d = self.domain_named(&name);
            self.projects[i].domain = d;
        }
    }

    /// Les concepts d'un projet (au-dessus du seuil), du plus sûr au moins sûr.
    pub fn concepts_of(&self, i: usize) -> Vec<(&Concept, f32)> {
        self.projects[i].tags.iter().filter(|t| t.weight >= LINK_MIN).filter_map(|t| self.ontology.concept(t.concept).map(|c| (c, t.weight))).collect()
    }

    // ---- tags et domaines multiples ------------------------------------------------------------
    //
    // Un projet a un domaine PRINCIPAL (`Project::domain` : sa couleur et son île sur la carte) et
    // autant de tags qu'il faut, dans chaque facette. Les tags de la facette « Domaine » lui
    // donnent des domaines en plus. Retirer un tag le garde noté (poids 0, à la main) : un
    // classement suivant ne le remet pas.

    /// L'index du domaine qui porte ce nom (sans le créer).
    fn domain_index(&self, name: &str) -> Option<usize> {
        self.domains.iter().position(|d| same_name(&d.name, name))
    }

    /// Le domaine d'un concept de la facette « Domaine » : celui de son ancêtre de premier niveau.
    pub fn domain_of_concept(&self, id: u64) -> Option<usize> {
        let facet = self.ontology.domain_facet()?;
        let mut c = self.ontology.concept(id).filter(|c| c.facet == facet)?;
        for _ in 0..8 {
            match c.parent.and_then(|p| self.ontology.concept(p)) {
                Some(parent) => c = parent,
                None => break,
            }
        }
        self.domain_index(&c.name)
    }

    /// Le concept de premier niveau de la facette « Domaine » qui porte le nom de ce domaine.
    pub fn domain_concept(&self, d: usize) -> Option<u64> {
        let facet = self.ontology.domain_facet()?;
        let name = &self.domains.get(d)?.name;
        self.ontology.concepts.iter().find(|c| c.facet == facet && c.parent.is_none() && same_name(&c.name, name)).map(|c| c.id)
    }

    /// Les domaines d'un projet : le principal d'abord, puis ceux de ses tags « Domaine ».
    pub fn domains_of(&self, i: usize) -> Vec<usize> {
        let p = &self.projects[i];
        let mut out = vec![p.domain];
        for t in p.tags.iter().filter(|t| t.weight >= LINK_MIN) {
            if let Some(d) = self.domain_of_concept(t.concept)
                && !out.contains(&d)
            {
                out.push(d);
            }
        }
        out
    }

    pub fn has_domain(&self, i: usize, d: usize) -> bool {
        self.domains_of(i).contains(&d)
    }

    /// Rattache un projet à un concept, à la main : un classement suivant ne le touche pas.
    pub fn tag(&mut self, i: usize, concept: u64) {
        if self.ontology.concept(concept).is_none() || i >= self.projects.len() {
            return;
        }
        let p = &mut self.projects[i];
        p.tags.retain(|t| t.concept != concept);
        p.tags.insert(0, Tag { concept, weight: 1.0, manual: true });
    }

    /// Retire un tag. Le domaine principal passe au domaine suivant s'il y en a un ; le seul
    /// domaine d'un projet ne se retire pas (il faut en choisir un autre d'abord).
    pub fn untag(&mut self, i: usize, concept: u64) {
        if i >= self.projects.len() {
            return;
        }
        if let Some(d) = self.domain_of_concept(concept)
            && d == self.projects[i].domain
            && self.domain_concept(d) == Some(concept)
        {
            match self.domains_of(i).into_iter().find(|&x| x != d) {
                Some(next) => self.projects[i].domain = next,
                None => return,
            }
        }
        let p = &mut self.projects[i];
        p.tags.retain(|t| t.concept != concept);
        p.tags.push(Tag { concept, weight: 0.0, manual: true });
    }

    /// Ajoute un domaine à un projet, sans changer son domaine principal.
    pub fn add_domain(&mut self, i: usize, d: usize) {
        self.ensure_ontology();
        let concept = match self.domain_concept(d) {
            Some(c) => c,
            None => {
                let Some(name) = self.domains.get(d).map(|x| x.name.clone()) else { return };
                let facet = self.ontology.domain_facet().unwrap_or_else(|| self.ontology.add_facet(DOMAIN_FACET, ""));
                self.create_concept(facet, &name, None)
            }
        };
        self.tag(i, concept);
    }

    /// Change le domaine principal (couleur et île sur la carte). `keep_old` : l'ancien reste un
    /// domaine du projet (l'étoile déplacée dans la fiche) ; sinon il est remplacé (correction).
    pub fn set_main_domain(&mut self, i: usize, d: usize, keep_old: bool) {
        if i >= self.projects.len() || d >= self.domains.len() || self.projects[i].domain == d {
            return;
        }
        if keep_old {
            let old = self.projects[i].domain;
            self.add_domain(i, old);
        }
        self.projects[i].domain = d;
    }

    /// Crée un concept à la main (ou renvoie celui qui porte déjà ce nom dans la facette). Au
    /// premier niveau de la facette « Domaine », il crée aussi le domaine.
    pub fn create_concept(&mut self, facet: u64, name: &str, parent: Option<u64>) -> u64 {
        if let Some(id) = self.ontology.concept_named(facet, name) {
            return id;
        }
        let parent = parent.filter(|p| self.ontology.concept(*p).is_some_and(|x| x.facet == facet));
        let id = self.ontology.add_concept(facet, name, "");
        if let Some(c) = self.ontology.concepts.iter_mut().find(|c| c.id == id) {
            c.manual = true;
            c.parent = parent;
        }
        self.sync_domains();
        id
    }

    /// Les concepts dont le nom (ou un synonyme) contient `query`, les plus proches d'abord :
    /// ceux qui commencent par le texte, puis les autres. Sans les concepts déjà rattachés au
    /// projet `i`.
    pub fn suggest_concepts(&self, i: usize, query: &str, limit: usize) -> Vec<&Concept> {
        let q = crate::search::fold(query.trim());
        if q.is_empty() {
            return Vec::new();
        }
        let taken: Vec<u64> = self.projects.get(i).map(|p| p.tags.iter().filter(|t| t.weight >= LINK_MIN).map(|t| t.concept).collect()).unwrap_or_default();
        let mut hits: Vec<(u8, &Concept)> = self
            .ontology
            .concepts
            .iter()
            .filter(|c| !taken.contains(&c.id))
            .filter_map(|c| {
                let name = crate::search::fold(&c.name);
                let rank = if name.starts_with(&q) {
                    0
                } else if name.contains(&q) {
                    1
                } else if c.synonyms.iter().any(|s| crate::search::fold(s).contains(&q)) {
                    2
                } else {
                    return None;
                };
                Some((rank, c))
            })
            .collect();
        hits.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.name.len().cmp(&b.1.name.len())));
        hits.into_iter().take(limit).map(|(_, c)| c).collect()
    }

    /// Applique une proposition de l'IA (voir `Merge`). Renvoie (ajoutés, retirés).
    pub fn merge_ontology(&mut self, proposal: &Ontology, mode: Merge, by: &str) -> (usize, usize) {
        let mut added = 0;
        let before: Vec<u64> = self.ontology.concepts.iter().map(|c| c.id).collect();
        let mut kept: Vec<u64> = self.ontology.concepts.iter().filter(|c| c.manual || mode == Merge::AddNew).map(|c| c.id).collect();
        // Facettes et concepts : on garde l'identifiant d'un même nom (les rattachements suivent).
        for pf in &proposal.facets {
            let facet = match self.ontology.facet_named(&pf.name) {
                Some(id) => {
                    if let Some(f) = self.ontology.facets.iter_mut().find(|f| f.id == id)
                        && f.description.trim().is_empty()
                    {
                        f.description = pf.description.clone();
                    }
                    id
                }
                None => self.ontology.add_facet(&pf.name, &pf.description),
            };
            let mut ids = std::collections::HashMap::new();
            for pc in proposal.concepts.iter().filter(|c| c.facet == pf.id) {
                let id = match self.ontology.concept_named(facet, &pc.name) {
                    Some(id) => {
                        if let Some(c) = self.ontology.concepts.iter_mut().find(|c| c.id == id)
                            && !c.manual
                            && mode == Merge::Replace
                        {
                            c.definition = pc.definition.clone();
                            c.synonyms = pc.synonyms.clone();
                        }
                        id
                    }
                    None => {
                        added += 1;
                        let id = self.ontology.add_concept(facet, &pc.name, &pc.definition);
                        if let Some(c) = self.ontology.concepts.iter_mut().find(|c| c.id == id) {
                            c.synonyms = pc.synonyms.clone();
                        }
                        id
                    }
                };
                ids.insert(pc.id, id);
                kept.push(id);
            }
            // Les parents, une fois tous les identifiants connus.
            for pc in proposal.concepts.iter().filter(|c| c.facet == pf.id) {
                let (Some(&id), Some(parent)) = (ids.get(&pc.id), pc.parent.and_then(|p| ids.get(&p)).copied()) else { continue };
                if let Some(c) = self.ontology.concepts.iter_mut().find(|c| c.id == id)
                    && !c.manual
                {
                    c.parent = Some(parent);
                }
            }
        }
        let removed: Vec<u64> = before.into_iter().filter(|id| !kept.contains(id)).collect();
        for id in &removed {
            self.remove_concept(*id);
        }
        self.ontology.facets.retain(|f| !self.ontology.concepts.iter().all(|c| c.facet != f.id) || f.name == DOMAIN_FACET || proposal.facets.iter().any(|p| same_name(&p.name, &f.name)));
        self.ontology.generated_at = crate::model::now();
        self.ontology.generated_by = by.to_owned();
        self.sync_domains();
        (added, removed.len())
    }
}

// ---- classement par Ollaya ---------------------------------------------------------------------

/// Ce qu'Ollaya lit d'un projet. Les noms de dossiers (« ImmoFormation › Qualiopi ») disent
/// souvent ce que la fiche ne dit pas.
pub fn project_state(data: &Data, p: &Project) -> Value {
    json!({
        "nom": p.name,
        "dossier": crate::search::folder_context(data, p),
        "description": p.description,
        "type": p.kind,
        "technos": p.stack,
        "resultat_vise": p.outcome,
        "objectif": p.objective,
    })
}

/// Les questions d'un lot de concepts : une question oui/non par concept, sa définition en critère.
pub fn questions(onto: &Ontology, concepts: &[&Concept]) -> Value {
    let mut q = serde_json::Map::new();
    for c in concepts {
        let facet = onto.facet(c.facet).map(|f| f.name.as_str()).unwrap_or("Catégorie");
        let path = onto.path(c.id);
        let yes = if c.definition.trim().is_empty() { format!("Le projet relève de « {path} ».") } else { c.definition.trim().to_owned() };
        q.insert(
            format!("c{}", c.id),
            json!({
                "type": "noul",
                "instructions": format!("{facet} : ce projet relève-t-il de « {path} » ?"),
                "criteria": { "true": yes, "false": format!("Le projet ne relève pas de « {path} ».") }
            }),
        );
    }
    Value::Object(q)
}

/// Les réponses d'Ollaya : (concept, probabilité « oui »).
pub fn parse_answers(answers: &Value) -> Vec<(u64, f32)> {
    answers
        .as_object()
        .map(|o| {
            o.iter()
                .filter_map(|(k, v)| {
                    let id = k.strip_prefix('c')?.parse().ok()?;
                    let p = v["noul"].as_f64()?;
                    Some((id, p as f32))
                })
                .collect()
        })
        .unwrap_or_default()
}

// ---- proposition par un modèle de langage ------------------------------------------------------

pub const GENERATION_SYSTEM: &str = "Tu es un bibliothécaire qui conçoit des ontologies pour ranger des projets personnels. \
Tu réponds uniquement en JSON, en français.";

/// Le prompt : tous les projets d'un coup (vue d'ensemble), et ce que l'utilisateur a fait à la main.
pub fn generation_prompt(data: &Data) -> String {
    let mut out = String::from(
        "Voici tous les projets de l'utilisateur. Propose une ontologie pour les classer :\n\
- 2 à 4 FACETTES, des axes indépendants (par exemple : domaine ou sujet ; nature du projet : application, outil, \
formation, recherche… ; finalité : apprentissage, client, produit, usage perso…). La facette « Domaine » existe déjà : garde-la.\n\
- 4 à 12 CONCEPTS par facette, adaptés à CES projets (pas de concept vide), sans doublon entre eux.\n\
- Pour chaque concept : un nom court, une DÉFINITION d'une phrase qui permette de décider si un projet en relève \
(elle servira de critère à un modèle de décision), 2 à 5 SYNONYMES en français (mots qu'on taperait pour le chercher), \
et éventuellement le nom d'un concept PARENT plus général dans la même facette.\n\n",
    );
    let manual: Vec<String> = data
        .ontology
        .concepts
        .iter()
        .filter(|c| c.manual)
        .map(|c| format!("- {} › {} : {}", data.ontology.facet(c.facet).map(|f| f.name.as_str()).unwrap_or("?"), c.name, c.definition))
        .collect();
    if !manual.is_empty() {
        out.push_str("Concepts créés par l'utilisateur, à reprendre tels quels :\n");
        out.push_str(&manual.join("\n"));
        out.push_str("\n\n");
    }
    out.push_str(&format!("Projets ({}) :\n", data.projects.len()));
    for p in data.projects.iter().take(400) {
        let description: String = p.description.chars().take(160).collect();
        let stack = if p.stack.is_empty() { String::new() } else { format!(" ({})", p.stack.join(", ")) };
        out.push_str(&format!("- {} : {description} [{}]{stack}\n", p.name, p.kind));
    }
    out
}

pub fn generation_schema() -> Value {
    let concept = json!({
        "type": "object",
        "properties": {
            "nom": { "type": "string" },
            "definition": { "type": "string" },
            "synonymes": { "type": "array", "items": { "type": "string" } },
            "parent": { "type": "string" }
        },
        "required": ["nom", "definition", "synonymes"]
    });
    json!({
        "type": "object",
        "properties": {
            "facettes": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "nom": { "type": "string" },
                        "description": { "type": "string" },
                        "concepts": { "type": "array", "items": concept }
                    },
                    "required": ["nom", "description", "concepts"]
                }
            }
        },
        "required": ["facettes"]
    })
}

/// Une ontologie à partir du JSON du modèle (ou d'un agent MCP) : `{ facettes: [{ nom, description,
/// concepts: [{ nom, definition, synonymes, parent }] }] }`.
pub fn from_json(v: &Value) -> Result<Ontology, String> {
    let facets = v["facettes"].as_array().ok_or("Il manque la liste « facettes ».")?;
    let mut o = Ontology::default();
    for f in facets {
        let name = f["nom"].as_str().map(str::trim).unwrap_or("");
        if name.is_empty() {
            continue;
        }
        let facet = o.facet_named(name).unwrap_or_else(|| o.add_facet(name, f["description"].as_str().unwrap_or("")));
        let concepts = f["concepts"].as_array().cloned().unwrap_or_default();
        for c in &concepts {
            let cname = c["nom"].as_str().map(str::trim).unwrap_or("");
            if cname.is_empty() || o.concept_named(facet, cname).is_some() {
                continue;
            }
            let id = o.add_concept(facet, cname, c["definition"].as_str().unwrap_or(""));
            let synonyms: Vec<String> = c["synonymes"].as_array().map(|a| a.iter().filter_map(|s| s.as_str()).map(|s| s.trim().to_owned()).filter(|s| !s.is_empty()).collect()).unwrap_or_default();
            if let Some(x) = o.concepts.iter_mut().find(|x| x.id == id) {
                x.synonyms = synonyms;
            }
        }
        for c in &concepts {
            let (Some(cname), Some(parent)) = (c["nom"].as_str(), c["parent"].as_str().map(str::trim).filter(|p| !p.is_empty())) else { continue };
            let (Some(id), Some(pid)) = (o.concept_named(facet, cname), o.concept_named(facet, parent)) else { continue };
            if id != pid
                && let Some(x) = o.concepts.iter_mut().find(|x| x.id == id)
            {
                x.parent = Some(pid);
            }
        }
    }
    if o.concepts.is_empty() {
        return Err("La proposition ne contient aucun concept.".into());
    }
    Ok(o)
}

/// L'ontologie en JSON (pour l'agent MCP, et pour relire une proposition).
pub fn to_json(data: &Data) -> Value {
    json!({
        "facettes": data.ontology.facets.iter().map(|f| json!({
            "id": f.id,
            "nom": f.name,
            "description": f.description,
            "concepts": data.ontology.tree(f.id).into_iter().map(|(_, c)| json!({
                "id": c.id,
                "nom": c.name,
                "definition": c.definition,
                "synonymes": c.synonyms,
                "parent": c.parent.and_then(|p| data.ontology.concept(p)).map(|p| p.name.clone()),
                "manuel": c.manual,
                "projets": data.concept_count(c.id),
            })).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proposal() -> Ontology {
        from_json(&json!({ "facettes": [
            { "nom": "Domaine", "description": "Le sujet", "concepts": [
                { "nom": "Immobilier", "definition": "Logement, agences, biens immobiliers.", "synonymes": ["logement", "maison"] },
                { "nom": "Formation", "definition": "Apprendre, enseigner.", "synonymes": ["cours"] },
                { "nom": "Location", "definition": "Louer un bien.", "synonymes": [], "parent": "Immobilier" }
            ]},
            { "nom": "Nature", "description": "Ce que c'est", "concepts": [
                { "nom": "Application web", "definition": "Une application qui tourne dans le navigateur.", "synonymes": ["site"] }
            ]}
        ]}))
        .unwrap()
    }

    #[test]
    fn reprise_des_domaines() {
        let mut data = crate::model::demo();
        data.ontology = Ontology::default();
        for p in &mut data.projects {
            p.tags.clear();
        }
        data.ensure_ontology();
        let facet = data.ontology.domain_facet().expect("la facette Domaine");
        assert_eq!(data.ontology.concepts.iter().filter(|c| c.facet == facet).count(), data.domains.len());
        let immo = data.projects.iter().position(|p| p.name == "ImmoFormation").unwrap();
        let tag = &data.projects[immo].tags[0];
        assert_eq!(data.ontology.concept(tag.concept).unwrap().name, data.domains[data.projects[immo].domain].name);
    }

    #[test]
    fn proposition_lue_et_fusionnee() {
        let p = proposal();
        let nature = p.facet_named("Nature").unwrap();
        assert_eq!(p.concepts.iter().filter(|c| c.facet == nature).count(), 1);
        let location = p.concepts.iter().find(|c| c.name == "Location").unwrap();
        assert_eq!(p.path(location.id), "Immobilier › Location");

        let mut data = crate::model::demo();
        // Un concept fait à la main reste, même absent de la proposition.
        let domain = data.ontology.domain_facet().unwrap();
        let mine = data.ontology.add_concept(domain, "Mes essais", "Bricolages perso.");
        data.ontology.concepts.iter_mut().find(|c| c.id == mine).unwrap().manual = true;
        let formation_before = data.ontology.concept_named(domain, "Formation");
        let (added, _removed) = data.merge_ontology(&p, Merge::Replace, "test");
        assert!(added >= 3);
        assert!(data.ontology.concept(mine).is_some(), "le concept manuel est gardé");
        assert_eq!(data.ontology.concept_named(domain, "Formation"), formation_before, "même nom : même identifiant, les rattachements suivent");
        assert!(data.ontology.facet_named("Nature").is_some());
        assert!(data.domains.iter().any(|d| d.name == "Immobilier"), "un nouveau domaine pour la carte");
    }

    #[test]
    fn questions_et_reponses_d_ollaya() {
        let o = proposal();
        let concepts: Vec<&Concept> = o.concepts.iter().collect();
        let q = questions(&o, &concepts);
        let immo = o.concepts.iter().find(|c| c.name == "Immobilier").unwrap();
        let key = format!("c{}", immo.id);
        assert_eq!(q[&key]["type"], "noul");
        assert!(q[&key]["instructions"].as_str().unwrap().contains("Immobilier"));
        assert_eq!(q[&key]["criteria"]["true"], "Logement, agences, biens immobiliers.");
        let answers = json!({ key.clone(): { "type": "noul", "noul": 0.91 }, "autre": { "noul": 0.3 } });
        assert_eq!(parse_answers(&answers), vec![(immo.id, 0.91)]);
    }

    #[test]
    fn scores_appliques_au_projet() {
        let mut data = crate::model::demo();
        data.merge_ontology(&proposal(), Merge::Replace, "test");
        let domain = data.ontology.domain_facet().unwrap();
        let immo_c = data.ontology.concept_named(domain, "Immobilier").unwrap();
        let form_c = data.ontology.concept_named(domain, "Formation").unwrap();
        let i = data.projects.iter().position(|p| p.name == "ImmoFormation").unwrap();
        data.projects[i].tags.push(Tag { concept: 999, weight: 1.0, manual: true });
        let fp = data.ontology.fingerprint();
        data.apply_scores(i, &[(immo_c, 0.92), (form_c, 0.7), (12345, 0.05)], fp);
        let p = &data.projects[i];
        assert_eq!(p.tags[0].concept, 999, "le rattachement manuel reste");
        assert!(p.tags.iter().any(|t| t.concept == immo_c && (t.weight - 0.92).abs() < 1e-6));
        assert!(!p.tags.iter().any(|t| t.concept == 12345), "trop faible : pas gardé");
        assert_eq!(data.domains[p.domain].name, "Immobilier", "le meilleur domaine devient le domaine du projet");
        assert!(!data.outdated_tags().contains(&i));
    }
}
