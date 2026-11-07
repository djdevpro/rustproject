import {
  createContext,
  useContext,
  useEffect,
  useState,
  type ReactNode,
} from "react";
import nativeEnglish from "./native-en.json";
import { localeFromPath, localePath, seoCopy, type Locale } from "./seo";

const english: Record<string, string> = {
  ...nativeEnglish,
  "Prochaines étapes": "Next steps",
  Modèle: "Model",
  Manuel: "Manual",
  Tester: "Test",
  "Limiter les projets en focus": "Limit focus projects",
  Accent: "Accent",
  Ambre: "Amber",
  Secouer: "Shake",
  Recentrer: "Reset view",
  Explorateur: "Explorer",
  "Tous vos projets. Un même bureau.": "All your projects. One desk.",
  "Rust Project retrouve vos projets dans vos dossiers, les classe et garde vos prochaines étapes. Le tout sur votre machine.":
    "Rust Project finds projects in your folders, organizes them and keeps your next steps. All on your machine.",
  "Essayer la démo": "Try the demo",
  "Télécharger Rust Project": "Download Rust Project",
  "Le logiciel": "The app",
  "Comment ça marche": "How it works",
  Documentation: "Documentation",
  "Un bureau pour vos projets personnels.": "A desk for your side projects.",
  "Choisissez un projet. Retrouvez où vous en étiez.":
    "Pick a project. See where you left off.",
  "Cette démo reprend l'interface du logiciel avec 30 projets fictifs. Les changements restent dans cette session.":
    "This demo reproduces the app interface with 30 sample projects. Changes stay in this session.",
  "Agrandir la démo": "Expand demo",
  "Réduire la démo": "Exit expanded view",
  "Une fiche par projet.": "One record per project.",
  "Le scan reconnaît les projets à leurs manifestes et rattache les dossiers frontend, api ou db à leur parent.":
    "The scanner recognizes projects by their manifests and attaches frontend, api or db folders to their parent.",
  "Un classement que vous pouvez modifier.": "A classification you can edit.",
  "Ollama décrit. Ollaya classe. Vous décidez.":
    "Ollama describes. Ollaya classifies. You decide.",
  "Les modèles tournent en local. Choisissez ceux qui décrivent vos projets, calculent les embeddings et évaluent les concepts.":
    "Models run locally. Choose which ones describe projects, compute embeddings and evaluate concepts.",
  "Des mots, du sens et des liens.": "Words, meaning and connections.",
  "La recherche combine correspondances textuelles, embeddings et concepts. Le graphe montre comment vos projets sont rangés.":
    "Search combines text matches, embeddings and concepts. The graph shows how your projects are organized.",
  "Votre agent peut aussi ranger vos projets.":
    "Your agent can organize your projects too.",
  "Le serveur MCP donne accès aux fiches, aux étapes et à l'ontologie. Les dossiers sources restent ceux que vous avez choisis.":
    "The MCP server exposes projects, steps and the ontology. Source folders remain the ones you chose.",
  "Vos dossiers restent sur votre machine.":
    "Your folders stay on your machine.",
  "Une application de bureau écrite en Rust. Le code est disponible sous licences MIT et Apache 2.0.":
    "A desktop app written in Rust. Source code is available under the MIT and Apache 2.0 licenses.",
  "Ouvrez vos projets depuis le même endroit.":
    "Open your projects from one place.",
  "VS Code, Claude Code, un terminal ou un lanceur personnalisé, dans le dossier du projet.":
    "VS Code, Claude Code, a terminal or a custom launcher, in the project folder.",
  "Choisissez vos dossiers. Retrouvez vos projets.":
    "Choose your folders. Find your projects.",
  "Voir le code source": "View source code",
  "Données fictives": "Sample data",
  "Aucun service local n'est contacté dans cette démo.":
    "No local service is contacted in this demo.",
  "Simulation. Le logiciel de bureau ouvre ce lanceur dans le dossier du projet.":
    "Simulation. The desktop app opens this launcher in the project folder.",
  "Simulation terminée. Les {count} projets fictifs sont disponibles.":
    "Simulation complete. The {count} sample projects are available.",
  "Glisser pour déplacer · molette pour zoomer · cliquer sur un domaine pour filtrer":
    "Drag to move · scroll to zoom · click a domain to filter it",
  "actif cette semaine": "active this week",
  "plus ancien": "older",
  focus: "focus",
  "en réserve": "parked",
  "rangés ensemble": "stored together",
  "technos communes (au survol)": "shared stack (on hover)",
  "WebGL indisponible. Utilisez la grille ou la liste.":
    "WebGL unavailable. Use the grid or list.",
  "Rechercher un nom, un type, une techno (rust, next.js…)":
    "Search name, type, stack (rust, next.js...)",
  "Domaines · suggérés par l'IA": "Domains · suggested by the AI",
  "Tous les domaines": "All domains",
  "Taxonomie IA": "AI taxonomy",
  "7 domaines · générés il y a 2 h": "7 domains · generated 2 h ago",
  "Dossiers scannés il y a 6 h": "Folders scanned 6 h ago",
  "Mon bureau": "My desk",
  "Mes projets du moment": "My current projects",
  "Au fil de l'eau": "Ongoing",
  "En réserve": "Parked",
  "En cours": "Active",
  "En pause": "Paused",
  Idée: "Idea",
  Terminé: "Done",
  Archivé: "Archived",
  Viser: "Aiming for",
  "aujourd'hui": "today",
  hier: "yesterday",
  "il y a {n} j": "{n} d ago",
  "Tout voir dans la bibliothèque": "See everything in the library",
  "Afficher les autres": "Show more",
  "Explorer les {n} projets": "Explore the {n} projects",
  "{n} au total": "{n} in total",
  "{n} non archivés": "{n} not archived",
  "{n} archivés": "{n} archived",
  projets: "projects",
  liens: "links",
  Réorganiser: "Rearrange",
  Récents: "Recent",
  "Tout effacer": "Clear all",
  Nom: "Name",
  "Ajouter une étape…": "Add a step...",
  "Supprimer cette étape": "Remove this step",
  Description: "Description",
  "Résultat recherché": "Desired outcome",
  Enregistrer: "Save",
  "Les modifications restent dans cette session de démonstration.":
    "Changes stay in this demo session.",
  "Ajouter un projet fictif": "Add a sample project",
  "Aucun projet ne correspond.": "No matching projects.",
  Domaine: "Domain",
  Nature: "Kind",
  "Aucune note pour l'instant.": "No notes yet.",
  "Utiliser ce projet en focus": "Bring this project into focus",
  "Garder en réserve": "Keep parked",
  "À redécouvrir": "Rediscover",
  "Démonstration web": "Web demo",
  "Local et open source": "Local and open source",
  "Vue d'ensemble": "Overview",
  "Résumés et descriptions": "Summaries and descriptions",
  "Recherche par le sens": "Semantic search",
  "Décisions par concept": "Decisions by concept",
  "Test simulé réussi": "Simulated test succeeded",
  "Tester et lister": "Test and list",
  "Classer les projets": "Classify projects",
  "Classement simulé terminé": "Simulated classification complete",
  Définition: "Definition",
  Concepts: "Concepts",
  "Ajouter un concept": "Add a concept",
  "Nouveau concept": "New concept",
  "Modifier le concept": "Edit concept",
  Préférences: "Preferences",
  "Les réglages de cette démo sont simulés.":
    "Settings in this demo are simulated.",
  Profondeur: "Depth",
  "Dossiers surveillés": "Watched folders",
  "Ouvrir le dossier": "Open folder",
  Copié: "Copied",
  Copier: "Copy",
  "Rétablir les fixtures": "Reset fixtures",
  Annuler: "Cancel",
  Bibliothèque: "Library",
  Carte: "Map",
  Grille: "Grid",
  Liste: "List",
  "IA & agents": "AI & agents",
  "Outils développeur": "Developer tools",
  Apprentissage: "Learning",
  "Jeux & 3D": "Games & 3D",
  "Audio & voix": "Audio & voice",
  "Écriture & personnel": "Writing & personal",
};

type Translate = (
  key: string,
  values?: Record<string, string | number>,
) => string;
const Context = createContext<{
  lang: Locale;
  setLang: (lang: Locale) => void;
  tr: Translate;
}>({ lang: "fr", setLang: () => {}, tr: (key) => key });
export function I18n({
  children,
  initialLocale,
}: {
  children: ReactNode;
  initialLocale?: Locale;
}) {
  const [lang] = useState<Locale>(
    () =>
      initialLocale ??
      (typeof window === "undefined"
        ? "fr"
        : localeFromPath(window.location.pathname)),
  );
  const setLang = (next: Locale) => {
    if (next === lang) return;
    window.location.assign(
      localePath(next, import.meta.env.BASE_URL) + window.location.hash,
    );
  };
  useEffect(() => {
    document.documentElement.lang = lang;
    document.title = seoCopy[lang].title;
    try {
      localStorage.setItem("rust-project-language", lang);
    } catch {}
  }, [lang]);
  const tr: Translate = (key, values = {}) => {
    let value = (lang === "en" ? english[key] || key : key)
      .replace(/[’]/g, "'")
      .replace(/[—–]/g, ",");
    for (const [name, replacement] of Object.entries(values))
      value = value.replaceAll(`{${name}}`, String(replacement));
    return value;
  };
  return (
    <Context.Provider value={{ lang, setLang, tr }}>
      {children}
    </Context.Provider>
  );
}
export const useI18n = () => useContext(Context);
