import { useId } from "react";
import { projects, domains } from "./data";
import { useI18n } from "./i18n";

// The annotation sits on the cubic at t=.75, not at an arbitrary CSS offset.
const anchor = { x: 722.8125, y: 174.5 };
const curve = "M0 350 C310 350 610 260 1000 24";

export function SearchLandscape() {
  const id = useId().replace(/:/g, "");
  const { tr, lang } = useI18n();
  return (
    <section className="search-landscape">
      <div className="landscape-art" aria-hidden="true">
        <svg viewBox="0 0 1000 380" preserveAspectRatio="none" fill="none">
          <defs>
            <linearGradient id={`${id}-fill`} x1="0" y1="0" x2=".12" y2="1">
              <stop stopColor="#cecece" stopOpacity=".075" />
              <stop offset=".55" stopColor="#acacac" stopOpacity=".025" />
              <stop offset="1" stopColor="#acacac" stopOpacity="0" />
            </linearGradient>
            <linearGradient id={`${id}-edge`} x1="0" y1="1" x2="1" y2="0">
              <stop stopColor="#cdcdcd" stopOpacity="0" />
              <stop offset=".35" stopColor="#cdcdcd" stopOpacity=".12" />
              <stop offset=".68" stopColor="#d7d7d7" stopOpacity=".48" />
              <stop offset="1" stopColor="#cdcdcd" stopOpacity="0" />
            </linearGradient>
            <linearGradient
              id={`${id}-guide`}
              gradientUnits="userSpaceOnUse"
              x1={anchor.x}
              y1={anchor.y}
              x2={anchor.x}
              y2="380"
            >
              <stop stopColor="#cdcdcd" stopOpacity=".22" />
              <stop offset="1" stopColor="#cdcdcd" stopOpacity="0" />
            </linearGradient>
            <radialGradient id={`${id}-light`}>
              <stop stopColor="#b3b3b3" stopOpacity=".07" />
              <stop offset="1" stopColor="#b3b3b3" stopOpacity="0" />
            </radialGradient>
          </defs>
          <ellipse
            cx="700"
            cy="220"
            rx="310"
            ry="155"
            fill={`url(#${id}-light)`}
          />
          <path d={`${curve} L1000 380 H0Z`} fill={`url(#${id}-fill)`} />
          <path
            d={curve}
            stroke={`url(#${id}-edge)`}
            strokeWidth=".8"
            vectorEffect="non-scaling-stroke"
          />
          <path
            d={`M${anchor.x} ${anchor.y + 7}V380`}
            stroke={`url(#${id}-guide)`}
            strokeWidth=".8"
            vectorEffect="non-scaling-stroke"
          />
          <circle
            cx={anchor.x}
            cy={anchor.y}
            r="3"
            fill="#191919"
            stroke="#aeaeae"
            strokeOpacity=".5"
            strokeWidth=".7"
          />
          <circle
            className="landscape-pulse"
            cx={anchor.x}
            cy={anchor.y}
            r="9"
            stroke="#c6c6c6"
            strokeOpacity=".15"
            strokeWidth=".7"
          />
        </svg>
        <div
          className="landscape-label"
          style={{ left: `${anchor.x / 10}%`, top: `${anchor.y / 3.8}%` }}
        >
          <small>{tr("Données fictives")}</small>
          <div>
            <span>Scribe Agent</span>
            <b>0.94</b>
          </div>
        </div>
      </div>
      <div className="section-copy">
        <h2>
          {lang === "fr" ? (
            <>
              Retrouver un projet,
              <br />
              même sans son nom.
            </>
          ) : (
            <>
              Find a project,
              <br />
              even without its name.
            </>
          )}
        </h2>
        <p>
          {lang === "fr"
            ? "Un nom oublié, une idée à reprendre. Les embeddings rapprochent votre recherche du contenu des fiches, même quand les mots diffèrent."
            : "A forgotten name, an idea to revisit. Embeddings match your search to project records, even when the words differ."}
        </p>
        <dl className="landscape-stats">
          <div>
            <dt>
              {lang === "fr" ? "Projets de démonstration" : "Demo projects"}
            </dt>
            <dd>{projects.length}</dd>
          </div>
          <div>
            <dt>{lang === "fr" ? "Domaines" : "Domains"}</dt>
            <dd>{domains.length}</dd>
          </div>
        </dl>
      </div>
    </section>
  );
}
