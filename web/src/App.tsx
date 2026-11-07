import { SearchLandscape } from "./SearchLandscape";
import { useState } from "react";
import {
  ArrowRight,
  ArrowDown,
  Github,
  Network,
  HardDrive,
  Terminal,
  Search,
  Cpu,
  Copy,
  Check,
  Sparkles,
} from "lucide-react";
import { I18n, useI18n } from "./i18n";
import { Demo } from "./Demo";
import { FeatureArtwork } from "./FeatureArtwork";
import { FeatureCard } from "./FeatureCard";
import { MacComputer } from "./MacComputer";
import { Button } from "./components/ui/button";
import { repository } from "./data";
import type { Locale } from "./seo";

const iconUrl = `${import.meta.env.BASE_URL}icon.svg`;

const mechanisms = [
  {
    name: "Ollama",
    subtitle: "Résumés et descriptions",
    icon: Sparkles,
    code: ["qwen2.5:3b", "Atlas", "Desktop app", "Rust · egui"],
  },
  {
    name: "Ollaya",
    subtitle: "Décisions par concept",
    icon: Network,
    code: ["decider:2b", "IA & agents", "94 %", "Web & SaaS", "72 %"],
  },
  {
    name: "Embeddings",
    subtitle: "Recherche par le sens",
    icon: Search,
    code: ["qwen3-embedding:4b", "Scribe Agent", "0.94", "Promptbook", "0.81"],
  },
];
function Landing() {
  const { tr, lang, setLang } = useI18n();
  const [model, setModel] = useState(0);
  const [copied, setCopied] = useState(false);
  const selected = mechanisms[model];
  const copy = () =>
    navigator.clipboard
      .writeText("claude mcp add rust-project -- rust-project.exe --mcp")
      .then(() => setCopied(true));
  return (
    <>
      <header className="site-header">
        <div className="site-nav">
          <a className="site-brand" href="#">
            <img src={iconUrl} alt="" />
            Rust Project
          </a>
          <nav>
            <a href="#demo">{tr("Le logiciel")}</a>
            <a href="#fonctionnement">{tr("Comment ça marche")}</a>
            <a href={repository + "#readme"} target="_blank" rel="noreferrer">
              {tr("Documentation")}
            </a>
          </nav>
          <div className="site-nav-actions">
            <a
              className="github-icon"
              href={repository}
              aria-label="GitHub"
              target="_blank"
              rel="noreferrer"
            >
              <Github size={18} />
            </a>
            <select
              aria-label={tr("Langue")}
              value={lang}
              onChange={(e) => setLang(e.target.value as "fr" | "en")}
            >
              <option value="fr">FR</option>
              <option value="en">EN</option>
            </select>
            <Button asChild size="sm">
              <a href={repository + "/releases"}>{tr("Télécharger")}</a>
            </Button>
          </div>
        </div>
      </header>
      <main>
        <section className="landing-hero">
          <span className="product-eyebrow">RUST PROJECT</span>
          <h1>
            {lang === "fr" ? (
              <>Vos projets, réunis.</>
            ) : (
              <>Your projects, together.</>
            )}
          </h1>
          <p>
            {tr(
              "Rust Project retrouve vos projets dans vos dossiers, les classe et garde vos prochaines étapes. Le tout sur votre machine.",
            )}
          </p>
          <div className="hero-actions">
            <Button asChild>
              <a href={repository + "/releases"}>
                {tr("Télécharger Rust Project")}
                <ArrowDown size={15} />
              </a>
            </Button>
            <Button asChild variant="outline">
              <a href="#demo">
                {tr("Essayer la démo")}
                <ArrowRight size={15} />
              </a>
            </Button>
          </div>
          <span className="hero-note">
            <HardDrive size={13} />
            {tr("Local et open source")}
            <span>Rust · Ollama · Ollaya · MCP</span>
          </span>
        </section>
        <MacComputer>
          <Demo />
        </MacComputer>
        <section className="feature-strip" id="fonctionnement">
          <FeatureCard>
            <FeatureArtwork kind={0} />

            <h3>
              {lang === "fr" ? "Découverte des projets" : "Project discovery"}
            </h3>
            <p>
              {tr(
                "Le scan reconnaît les projets à leurs manifestes et rattache les dossiers frontend, api ou db à leur parent.",
              )}
            </p>
          </FeatureCard>
          <FeatureCard>
            <FeatureArtwork kind={1} />

            <h3>{lang === "fr" ? "Ontologie" : "Ontology"}</h3>
            <p>
              {tr(
                "La recherche combine correspondances textuelles, embeddings et concepts. Le graphe montre comment vos projets sont rangés.",
              )}
            </p>
          </FeatureCard>
          <FeatureCard>
            <FeatureArtwork kind={2} />

            <h3>{lang === "fr" ? "Lanceurs" : "Launchers"}</h3>
            <p>
              {tr(
                "VS Code, Claude Code, un terminal ou un lanceur personnalisé, dans le dossier du projet.",
              )}
            </p>
          </FeatureCard>
          <FeatureCard>
            <FeatureArtwork kind={3} />

            <h3>{lang === "fr" ? "Données locales" : "Local data"}</h3>
            <p>
              {tr(
                "Une application de bureau écrite en Rust. Le code est disponible sous licences MIT et Apache 2.0.",
              )}
            </p>
          </FeatureCard>
          <FeatureCard>
            <FeatureArtwork kind={4} />
            <h3>{lang === "fr" ? "Serveur MCP" : "MCP server"}</h3>
            <p>
              {lang === "fr"
                ? "Votre agent consulte les fiches, ajoute des étapes et rattache les projets aux concepts."
                : "Your agent reads project records, adds steps and links projects to concepts."}
            </p>
          </FeatureCard>
          <FeatureCard>
            <FeatureArtwork kind={5} />
            <h3>{lang === "fr" ? "Recherche hybride" : "Hybrid search"}</h3>
            <p>
              {lang === "fr"
                ? "Retrouvez un nom, une stack ou une idée. La recherche associe texte, embeddings et ontologie."
                : "Find a name, a stack or an idea. Search combines text, embeddings and ontology."}
            </p>
          </FeatureCard>
        </section>
        <SearchLandscape />
        <section className="model-section">
          <div className="section-copy">
            <span className="product-eyebrow">OLLAMA / OLLAYA</span>
            <h2>{lang === "fr" ? "Modèles locaux" : "Local models"}</h2>
            <p>
              {tr(
                "Les modèles tournent en local. Choisissez ceux qui décrivent vos projets, calculent les embeddings et évaluent les concepts.",
              )}
            </p>
            <a href="#demo" className="plain-link">
              {tr("Essayer la démo")}
              <ArrowRight size={15} />
            </a>
          </div>
          <div className="model-example">
            <div className="example-heading">
              <Cpu size={16} />
              <code>{selected.code[0]}</code>
              <span>{tr("Données fictives")}</span>
            </div>
            <div className="model-example-body">
              {model === 0 ? (
                <pre>
                  <span>{"{"}</span>
                  <br />
                  {"  "}
                  <b>"name"</b>
                  {': "Atlas",'}
                  <br />
                  {"  "}
                  <b>"description"</b>
                  {":"}
                  <br />
                  {'    "Desktop dashboard that maps'}
                  <br />
                  {'     every side project on disk.",'}
                  <br />
                  {"  "}
                  <b>"stack"</b>
                  {': ["Rust", "egui"]'}
                  <br />
                  <span>{"}"}</span>
                </pre>
              ) : (
                <div className="score-chart">
                  {[0, 1].map((_, i) => (
                    <div key={i}>
                      <div>
                        <span>{selected.code[1 + i * 2]}</span>
                        <code>{selected.code[2 + i * 2]}</code>
                      </div>
                      <div className="score-track">
                        <i
                          key={`${model}-${i}`}
                          style={{
                            width: i ? (model === 2 ? "81%" : "72%") : "94%",
                          }}
                        />
                      </div>
                    </div>
                  ))}
                  <div className="chart-axis">
                    <span>0</span>
                    <span>0.5</span>
                    <span>1</span>
                  </div>
                </div>
              )}
            </div>
            <div className="model-tabs" role="tablist">
              {mechanisms.map((item, i) => (
                <button
                  key={item.name}
                  role="tab"
                  aria-selected={model === i}
                  onClick={() => setModel(i)}
                >
                  <item.icon size={18} />
                  <strong>{item.name}</strong>
                  <small>{tr(item.subtitle)}</small>
                </button>
              ))}
            </div>
          </div>
        </section>
        <section className="mcp-section">
          <div className="mcp-diagram">
            <div className="diagram-agent">
              <Terminal size={24} />
              <span>Agent MCP</span>
            </div>
            <div className="connector-line">
              <i />
            </div>
            <div className="diagram-core">
              <img src={iconUrl} alt="" />
              <span>Rust Project</span>
            </div>
            <div className="connector-line">
              <i />
            </div>
            <div className="diagram-tools">
              <code>lister_projets</code>
              <code>ajouter_etape</code>
              <code>rattacher_concept</code>
            </div>
          </div>
          <div className="section-copy">
            <span className="product-eyebrow">MODEL CONTEXT PROTOCOL</span>
            <h2>{lang === "fr" ? "Connexion MCP" : "MCP connection"}</h2>
            <p>
              {tr(
                "Le serveur MCP donne accès aux fiches, aux étapes et à l'ontologie. Les dossiers sources restent ceux que vous avez choisis.",
              )}
            </p>
            <div className="command-line">
              <code>claude mcp add rust-project -- rust-project.exe --mcp</code>
              <button
                aria-label={tr(copied ? "Copié" : "Copier")}
                onClick={copy}
              >
                {copied ? <Check size={16} /> : <Copy size={16} />}
              </button>
            </div>
          </div>
        </section>
        <section className="landing-close">
          <h2>{tr("Choisissez vos dossiers. Retrouvez vos projets.")}</h2>
          <div>
            <Button asChild>
              <a href={repository + "/releases"}>
                {tr("Télécharger Rust Project")}
                <ArrowDown size={16} />
              </a>
            </Button>
            <Button asChild variant="outline">
              <a href={repository}>
                <Github size={16} />
                {tr("Voir le code source")}
              </a>
            </Button>
          </div>
        </section>
      </main>
      <footer className="site-footer">
        <div>
          <a className="site-brand" href="#">
            <img src={iconUrl} alt="" />
            Rust Project
          </a>
          <span>{tr("Local et open source")}</span>
        </div>
        <div>
          <a href={repository + "#readme"}>{tr("Documentation")}</a>
          <a href={repository + "/releases"}>GitHub Releases</a>
          <a href={repository + "/blob/main/LICENSE-MIT"}>MIT</a>
          <a href={repository + "/blob/main/LICENSE-APACHE"}>Apache 2.0</a>
        </div>
      </footer>
    </>
  );
}
export function App({ initialLocale }: { initialLocale?: Locale } = {}) {
  return (
    <I18n initialLocale={initialLocale}>
      <Landing />
    </I18n>
  );
}
