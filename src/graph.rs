//! Le graphe de la Bibliothèque : une simulation de forces (dans l'esprit de d3-force), sans
//! dépendance.
//!
//! - Chaque domaine est un nœud « moyeu » ; chaque projet est un nœud relié à son domaine.
//! - Des liens plus légers relient les projets rangés ensemble (même dossier parent, ou une fiche
//!   rangée dans le dossier d'une autre) et ceux de domaines différents qui partagent leurs technos.
//! - Les nœuds se repoussent, les liens tirent comme des ressorts, une légère gravité garde
//!   l'ensemble au centre, et les nœuds ne se chevauchent pas.
//! - La « température » `alpha` baisse à chaque pas : le graphe s'anime puis se pose. Attraper un
//!   nœud ou changer les filtres le réchauffe.
//!
//! Ce module ne dessine rien (voir `screens/map.rs`) : il est testé seul.

use std::collections::{BTreeMap, HashMap};
use std::hash::{DefaultHasher, Hash, Hasher};

use egui::{Vec2, vec2};

use crate::model::{Data, path_key};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    Domain(usize),
    Project(u64),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdgeKind {
    /// Projet → son domaine.
    Domain,
    /// Rangés ensemble : même dossier parent, ou fiche rangée dans le dossier d'une autre.
    Family,
    /// Domaines différents, technos en commun.
    Stack,
}

#[derive(Clone, Debug)]
pub struct Node {
    pub key: Key,
    /// Index dans `data.projects` (projet) ou `data.domains` (domaine).
    pub index: usize,
    pub domain: usize,
    pub pos: Vec2,
    pub vel: Vec2,
    /// Rayon en unités du monde (≈ points à 100 %).
    pub radius: f32,
    /// Négative : le nœud repousse les autres.
    charge: f32,
    /// Tenu par la souris : la simulation ne le déplace plus.
    pub fixed: bool,
}

impl Node {
    pub fn is_hub(&self) -> bool {
        matches!(self.key, Key::Domain(_))
    }
}

#[derive(Clone, Debug)]
pub struct Edge {
    pub a: usize,
    pub b: usize,
    pub kind: EdgeKind,
    rest: f32,
    strength: f32,
    /// Part du mouvement prise par `b` (celui qui a le moins de liens bouge le plus).
    bias: f32,
}

/// En dessous, le graphe est au repos.
pub const ALPHA_MIN: f32 = 0.001;
/// ≈ 300 pas pour passer de 1 à `ALPHA_MIN`.
const ALPHA_DECAY: f32 = 0.0228;
const VELOCITY_DECAY: f32 = 0.4;
/// Au-delà, deux nœuds s'ignorent (sinon les gros domaines repoussent tout jusqu'aux bords).
const DISTANCE_MAX2: f32 = 560.0 * 560.0;
const COLLIDE_PADDING: f32 = 3.0;
const COLLIDE_STRENGTH: f32 = 0.7;
/// Un dossier parent qui regroupe plus de fiches que ça est un dossier de rangement général.
const FAMILY_MAX: usize = 12;

#[derive(Default)]
pub struct Sim {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    /// Pour chaque nœud, ses voisins (indices de nœuds).
    pub neighbors: Vec<Vec<usize>>,
    pub alpha: f32,
    pub alpha_target: f32,
    signature: Option<u64>,
}

/// Un petit décalage déterministe, pour séparer deux nœuds exactement superposés.
fn jiggle(i: usize, j: usize) -> Vec2 {
    let h = (i.wrapping_mul(7919) ^ j.wrapping_mul(104_729)) % 1000;
    let a = h as f32 * 0.006_283;
    vec2(a.cos(), a.sin()) * 0.5
}

fn polar(angle: f32, r: f32) -> Vec2 {
    vec2(angle.cos(), angle.sin()) * r
}

fn hub_radius(count: usize) -> f32 {
    16.0 + 2.2 * (count as f32).sqrt()
}

fn project_radius(idle_days: u32) -> f32 {
    match idle_days {
        0..=7 => 8.0,
        8..=30 => 6.5,
        _ => 5.0,
    }
}

impl Sim {
    pub fn warm(&self) -> bool {
        self.alpha > ALPHA_MIN || self.alpha_target > 0.0
    }

    /// (Re)construit le graphe si les projets visibles ont changé. Les nœuds qui restent gardent
    /// leur place ; les nouveaux éclosent depuis leur domaine. Renvoie `true` si le graphe a changé.
    pub fn sync(&mut self, data: &Data, visible: &[usize]) -> bool {
        let mut visible: Vec<usize> = visible.iter().copied().filter(|&i| i < data.projects.len()).collect();
        visible.sort_unstable();
        let signature = {
            let mut h = DefaultHasher::new();
            data.domains.len().hash(&mut h);
            for &i in &visible {
                let p = &data.projects[i];
                (i, p.id, p.domain, &p.stack, p.folders.first(), project_radius(p.idle_days).to_bits()).hash(&mut h);
            }
            h.finish()
        };
        if self.signature == Some(signature) {
            return false;
        }
        let first = self.signature.is_none() || self.nodes.is_empty();
        self.signature = Some(signature);
        let old: HashMap<Key, (Vec2, Vec2)> = self.nodes.iter().map(|n| (n.key, (n.pos, n.vel))).collect();

        // Les domaines qui ont au moins un projet visible.
        let mut per_domain: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        for &i in &visible {
            let d = data.projects[i].domain.min(data.domains.len().saturating_sub(1));
            per_domain.entry(d).or_default().push(i);
        }
        let hubs = per_domain.len().max(1) as f32;
        let ring = 150.0 + 70.0 * hubs.sqrt();
        let mut nodes = Vec::with_capacity(visible.len() + per_domain.len());
        let mut hub_of: HashMap<usize, usize> = HashMap::new();
        for (k, (&d, members)) in per_domain.iter().enumerate() {
            let key = Key::Domain(d);
            let (pos, vel) = old.get(&key).copied().unwrap_or((polar(k as f32 * std::f32::consts::TAU / hubs - 1.2, if hubs > 1.0 { ring } else { 0.0 }), Vec2::ZERO));
            hub_of.insert(d, nodes.len());
            let count = members.len() as f32;
            nodes.push(Node { key, index: d, domain: d, pos, vel, radius: hub_radius(members.len()), charge: -(220.0 + 18.0 * count), fixed: false });
        }
        let mut node_of: HashMap<usize, usize> = HashMap::new();
        for (&d, members) in &per_domain {
            let hub = nodes[hub_of[&d]].pos;
            let hub_r = nodes[hub_of[&d]].radius;
            for (j, &i) in members.iter().enumerate() {
                let p = &data.projects[i];
                let key = Key::Project(p.id);
                // Une nouvelle fiche éclot près de son domaine (en spirale, angle d'or).
                let spawn = || {
                    let r = if first { hub_r + 4.0 + 2.0 * (j as f32).sqrt() } else { hub_r + 18.0 + 9.0 * (j as f32).sqrt() };
                    hub + polar(j as f32 * 2.399_963 + d as f32, r)
                };
                let (pos, vel) = old.get(&key).copied().unwrap_or_else(|| (spawn(), Vec2::ZERO));
                node_of.insert(i, nodes.len());
                nodes.push(Node { key, index: i, domain: d, pos, vel, radius: project_radius(p.idle_days), charge: -45.0, fixed: false });
            }
        }

        // Les liens.
        let mut edges: Vec<(usize, usize, EdgeKind, f32, f32)> = Vec::new();
        for (&d, members) in &per_domain {
            let hub = hub_of[&d];
            let rest = nodes[hub].radius + 34.0 + 3.0 * (members.len() as f32).sqrt();
            for i in members {
                edges.push((hub, node_of[i], EdgeKind::Domain, rest, 0.7));
            }
        }
        // Rangés ensemble.
        let roots: Vec<String> = data.config.roots.iter().map(|r| path_key(&r.path)).collect();
        let folder_node: HashMap<String, usize> = visible.iter().filter_map(|&i| data.projects[i].folders.first().map(|f| (path_key(f), node_of[&i]))).collect();
        let mut by_parent: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for &i in &visible {
            let Some(parent) = data.projects[i].folders.first().and_then(|f| crate::model::parent_path(f)).map(path_key) else { continue };
            if parent.is_empty() {
                continue;
            }
            match folder_node.get(&parent) {
                // Une fiche rangée dans le dossier d'une autre (une brique, une copie…).
                Some(&owner) if owner != node_of[&i] => edges.push((owner, node_of[&i], EdgeKind::Family, 40.0, 0.5)),
                Some(_) => {}
                None if !roots.contains(&parent) => by_parent.entry(parent).or_default().push(node_of[&i]),
                None => {}
            }
        }
        for group in by_parent.values().filter(|g| (2..=FAMILY_MAX).contains(&g.len())) {
            for (k, &a) in group.iter().enumerate() {
                let b = group[(k + 1) % group.len()];
                if group.len() > 2 || k == 0 {
                    edges.push((a, b, EdgeKind::Family, 46.0, 0.25));
                }
            }
        }
        // Entre deux domaines, un lien de famille reste visible mais tire à peine : chaque
        // domaine garde sa grappe.
        for e in edges.iter_mut().filter(|e| e.2 == EdgeKind::Family && nodes[e.0].domain != nodes[e.1].domain) {
            e.3 = 110.0;
            e.4 = 0.02;
        }
        // Technos en commun, entre domaines différents : les deux plus proches voisins de chacun.
        let stacks: Vec<Vec<String>> = nodes.iter().map(|n| if n.is_hub() { Vec::new() } else { data.projects[n.index].stack.iter().map(|s| s.to_lowercase()).collect() }).collect();
        let mut best: Vec<Vec<(f32, usize)>> = vec![Vec::new(); nodes.len()];
        for a in 0..nodes.len() {
            for b in (a + 1)..nodes.len() {
                if nodes[a].is_hub() || nodes[b].is_hub() || nodes[a].domain == nodes[b].domain {
                    continue;
                }
                let shared = stacks[a].iter().filter(|s| stacks[b].contains(s)).count();
                let union = stacks[a].len() + stacks[b].len() - shared;
                let score = shared as f32 / union.max(1) as f32;
                if shared >= 2 && score >= 0.5 {
                    best[a].push((score, b));
                    best[b].push((score, a));
                }
            }
        }
        for (a, list) in best.iter_mut().enumerate() {
            list.sort_by(|x, y| y.0.total_cmp(&x.0).then(x.1.cmp(&y.1)));
            for &(_, b) in list.iter().take(2) {
                if a < b {
                    edges.push((a, b, EdgeKind::Stack, 170.0, 0.012));
                }
            }
        }
        edges.sort_by_key(|e| (e.0, e.1, e.2 as u8));
        edges.dedup_by_key(|e| (e.0.min(e.1), e.0.max(e.1)));

        let mut degree = vec![0usize; nodes.len()];
        let mut neighbors = vec![Vec::new(); nodes.len()];
        for &(a, b, ..) in &edges {
            degree[a] += 1;
            degree[b] += 1;
            neighbors[a].push(b);
            neighbors[b].push(a);
        }
        self.edges = edges
            .into_iter()
            .map(|(a, b, kind, rest, strength)| Edge { a, b, kind, rest, strength, bias: degree[a] as f32 / (degree[a] + degree[b]).max(1) as f32 })
            .collect();
        self.nodes = nodes;
        self.neighbors = neighbors;
        self.alpha = if first { 1.0 } else { self.alpha.max(0.6) };
        true
    }

    /// Un pas de simulation.
    pub fn tick(&mut self) {
        self.alpha += (self.alpha_target - self.alpha) * ALPHA_DECAY;
        let alpha = self.alpha;
        let n = self.nodes.len();
        // Ressorts.
        for e in &self.edges {
            let (a, b) = (&self.nodes[e.a], &self.nodes[e.b]);
            let mut d = (b.pos + b.vel) - (a.pos + a.vel);
            if d.length_sq() < 1e-6 {
                d = jiggle(e.a, e.b);
            }
            let l = d.length();
            let pull = d * ((l - e.rest) / l * alpha * e.strength);
            self.nodes[e.b].vel -= pull * e.bias;
            self.nodes[e.a].vel += pull * (1.0 - e.bias);
        }
        // Répulsion et collisions (toutes les paires : quelques centaines de nœuds au plus).
        for i in 0..n {
            for j in (i + 1)..n {
                let mut d = self.nodes[j].pos - self.nodes[i].pos;
                let mut l2 = d.length_sq();
                if l2 > DISTANCE_MAX2 {
                    continue;
                }
                if l2 < 1e-6 {
                    d = jiggle(i, j);
                    l2 = d.length_sq();
                }
                let (ci, cj) = (self.nodes[i].charge, self.nodes[j].charge);
                let soft = l2.max(36.0);
                self.nodes[i].vel += d * (cj * alpha / soft);
                self.nodes[j].vel -= d * (ci * alpha / soft);
                // Pas de chevauchement, même au repos (non pondéré par alpha).
                let (ri, rj) = (self.nodes[i].radius, self.nodes[j].radius);
                let r = ri + rj + COLLIDE_PADDING;
                if l2 < r * r {
                    let l = l2.sqrt();
                    let push = d * ((r - l) / l * COLLIDE_STRENGTH);
                    let (wi, wj) = (rj * rj / (ri * ri + rj * rj), ri * ri / (ri * ri + rj * rj));
                    self.nodes[i].vel -= push * wi;
                    self.nodes[j].vel += push * wj;
                }
            }
        }
        // Gravité vers le centre (plus faible à l'horizontale : le graphe s'étale en paysage, comme
        // l'écran), puis intégration.
        for node in &mut self.nodes {
            let g = if node.is_hub() { 0.05 } else { 0.02 };
            node.vel -= node.pos * vec2(0.5, 1.0) * (g * alpha);
            if node.fixed {
                node.vel = Vec2::ZERO;
                continue;
            }
            node.vel *= 1.0 - VELOCITY_DECAY;
            node.pos += node.vel;
        }
    }

    /// Fait tourner la simulation jusqu'au repos (tests, captures).
    pub fn settle(&mut self) {
        self.alpha_target = 0.0;
        for _ in 0..800 {
            if self.alpha <= ALPHA_MIN {
                break;
            }
            self.tick();
        }
        for node in &mut self.nodes {
            node.vel = Vec2::ZERO;
        }
    }

    /// Réchauffe le graphe (un nœud attrapé, un nouveau tirage…).
    pub fn reheat(&mut self, alpha: f32) {
        self.alpha = self.alpha.max(alpha);
    }

    /// Secoue le graphe : une impulsion à chaque nœud, puis il se repose ailleurs.
    pub fn shake(&mut self) {
        for (k, node) in self.nodes.iter_mut().enumerate() {
            node.vel += jiggle(k, k * 31 + 7) * if node.is_hub() { 40.0 } else { 70.0 };
        }
        self.reheat(0.8);
    }

    /// Le cadrage qui montre tous les nœuds (ou ceux d'un domaine) : (centre, zoom).
    pub fn fit(&self, view: Vec2, domain: Option<usize>) -> Option<(Vec2, f32)> {
        let mut min = vec2(f32::MAX, f32::MAX);
        let mut max = vec2(f32::MIN, f32::MIN);
        for n in self.nodes.iter().filter(|n| domain.is_none_or(|d| n.domain == d)) {
            // De la place pour les étiquettes : sous les domaines, à droite des projets.
            let (left, right, below) = if n.is_hub() { (60.0, 60.0, 34.0) } else { (n.radius, n.radius + 90.0, n.radius) };
            min = min.min(n.pos - vec2(left, n.radius));
            max = max.max(n.pos + vec2(right, n.radius + below));
        }
        if min.x > max.x {
            return None;
        }
        let size = (max - min).max(vec2(240.0, 240.0));
        let zoom = ((view.x - 60.0).max(100.0) / size.x).min((view.y - 100.0).max(100.0) / size.y).clamp(0.25, if domain.is_some() { 2.2 } else { 1.6 });
        Some(((min + max) / 2.0, zoom))
    }
}

/// L'état de la vue graphe : simulation, caméra, interaction en cours.
pub struct GraphView {
    pub sim: Sim,
    /// Le point du monde au centre de la vue.
    pub center: Vec2,
    pub zoom: f32,
    /// La caméra suit le graphe (cadrage automatique) tant que l'utilisateur ne l'a pas déplacée.
    pub auto_fit: bool,
    /// Prochaine image : caméra placée d'un coup (sans glissement).
    pub snap: bool,
    /// Tests et captures : le graphe se pose immédiatement, sans animation d'ambiance.
    pub instant: bool,
    pub dragging: Option<usize>,
    /// Le dernier nœud survolé (sa mise en lumière s'efface en douceur).
    pub last_hover: Option<usize>,
    /// Le dernier domaine filtré (même raison).
    pub last_domain: Option<usize>,
    /// Le filtre de domaine vu à l'image précédente : s'il change, la caméra recadre.
    pub filter_seen: Option<Option<usize>>,
}

impl Default for GraphView {
    fn default() -> Self {
        Self { sim: Sim::default(), center: Vec2::ZERO, zoom: 1.0, auto_fit: true, snap: true, instant: false, dragging: None, last_hover: None, last_domain: None, filter_seen: None }
    }
}

impl GraphView {
    /// Zoom autour d'un point de l'écran (le point du monde sous la souris ne bouge pas).
    /// `offset` : position de ce point par rapport au centre de la vue.
    pub fn zoom_at(&mut self, offset: Vec2, factor: f32) {
        let world = self.center + offset / self.zoom;
        self.zoom = (self.zoom * factor).clamp(0.2, 4.0);
        self.center = world - offset / self.zoom;
    }

    /// Rapproche la caméra de sa cible. Renvoie `true` tant qu'elle bouge.
    pub fn ease_to(&mut self, target: (Vec2, f32), dt: f32) -> bool {
        let k = 1.0 - (-dt * 5.0).exp();
        let (c, z) = target;
        self.center += (c - self.center) * k;
        self.zoom = (self.zoom.ln() + (z.ln() - self.zoom.ln()) * k).exp();
        (c - self.center).length() * self.zoom > 0.5 || (z / self.zoom - 1.0).abs() > 0.002
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settled(data: &Data) -> Sim {
        let mut sim = Sim::default();
        sim.sync(data, &(0..data.projects.len()).collect::<Vec<_>>());
        sim.settle();
        sim
    }

    #[test]
    fn le_graphe_se_pose_sans_chevauchement_et_par_domaine() {
        let data = crate::model::demo();
        let sim = settled(&data);
        assert!(sim.alpha <= ALPHA_MIN, "au repos");
        let hubs = sim.nodes.iter().filter(|n| n.is_hub()).count();
        assert_eq!(sim.nodes.len(), data.projects.len() + hubs);
        // Pas de chevauchement notable.
        for (i, a) in sim.nodes.iter().enumerate() {
            for b in &sim.nodes[i + 1..] {
                let d = (a.pos - b.pos).length();
                assert!(d >= (a.radius + b.radius) * 0.85, "{:?} et {:?} se chevauchent ({d:.1})", a.key, b.key);
            }
        }
        // Chaque projet est plus près de son domaine que des autres (à quelques exceptions près).
        let projects: Vec<&Node> = sim.nodes.iter().filter(|n| !n.is_hub()).collect();
        let near_own = projects
            .iter()
            .filter(|p| {
                let nearest = sim.nodes.iter().filter(|n| n.is_hub()).min_by(|a, b| (a.pos - p.pos).length().total_cmp(&(b.pos - p.pos).length())).unwrap();
                nearest.domain == p.domain
            })
            .count();
        assert!(near_own * 10 >= projects.len() * 9, "{near_own}/{} près de leur domaine", projects.len());
    }

    #[test]
    fn un_filtre_garde_les_places_et_rechauffe() {
        let data = crate::model::demo();
        let mut sim = settled(&data);
        let before: HashMap<Key, Vec2> = sim.nodes.iter().map(|n| (n.key, n.pos)).collect();
        assert!(!sim.sync(&data, &(0..data.projects.len()).collect::<Vec<_>>()), "rien n'a changé");
        let fewer: Vec<usize> = (1..data.projects.len()).collect();
        assert!(sim.sync(&data, &fewer));
        assert!(sim.alpha >= 0.6, "réchauffé");
        assert_eq!(sim.nodes.iter().filter(|n| !n.is_hub()).count(), data.projects.len() - 1);
        for n in &sim.nodes {
            assert_eq!(Some(&n.pos), before.get(&n.key), "{:?} garde sa place", n.key);
        }
    }

    #[test]
    fn liens_de_famille_et_de_technos() {
        let mut data = crate::model::demo();
        data.config.roots = vec![crate::model::ScanRoot { path: r"C:\P".into(), ..Default::default() }];
        let folders = [r"C:\P\Shop", r"C:\P\Shop\app", r"C:\P\Rangement\A", r"C:\P\Rangement\B", r"C:\P\Seul"];
        for (p, f) in data.projects.iter_mut().zip(folders) {
            p.folders = vec![f.into()];
        }
        for p in data.projects.iter_mut().skip(folders.len()) {
            p.folders = vec![format!(r"C:\P\{}", p.name)];
        }
        let sim = settled(&data);
        let node = |i: usize| sim.nodes.iter().position(|n| n.key == Key::Project(data.projects[i].id)).unwrap();
        let family = |a: usize, b: usize| sim.edges.iter().any(|e| e.kind == EdgeKind::Family && ((e.a, e.b) == (node(a), node(b)) || (e.a, e.b) == (node(b), node(a))));
        assert!(family(0, 1), "la brique est reliée à sa fiche");
        assert!(family(2, 3), "deux fiches du même dossier");
        assert!(!family(0, 4), "le dossier source ne relie pas ses enfants");
        // Les liens de technos relient des domaines différents.
        for e in sim.edges.iter().filter(|e| e.kind == EdgeKind::Stack) {
            assert_ne!(sim.nodes[e.a].domain, sim.nodes[e.b].domain);
        }
        assert!(sim.edges.iter().all(|e| e.a != e.b));
    }

    #[test]
    fn camera_cadrage_et_zoom_sous_la_souris() {
        let data = crate::model::demo();
        let sim = settled(&data);
        let (center, zoom) = sim.fit(vec2(1000.0, 700.0), None).unwrap();
        assert!((0.25..=1.6).contains(&zoom));
        let mut view = GraphView { center, zoom, ..Default::default() };
        let offset = vec2(120.0, -80.0);
        let under = view.center + offset / view.zoom;
        view.zoom_at(offset, 1.7);
        assert!((view.center + offset / view.zoom - under).length() < 1e-3, "le point sous la souris ne bouge pas");
        // Cadrer un domaine zoome davantage que tout cadrer.
        let (_, z_domain) = sim.fit(vec2(1000.0, 700.0), Some(0)).unwrap();
        assert!(z_domain >= zoom);
        // La caméra finit par atteindre sa cible.
        let mut moving = true;
        for _ in 0..240 {
            moving = view.ease_to((center, zoom), 1.0 / 60.0);
        }
        assert!(!moving);
    }
}
