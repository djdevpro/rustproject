import { useId } from "react";
import "./feature-artwork.css";
// Both curved branches share a point on the continuous horizontal bus.
const mcpConnections =
  "M96 90H210M156 90Q165 90 165 81V63Q165 54 174 54H210M156 90Q165 90 165 99V117Q165 126 174 126H210";
import { FolderOpen } from "lucide-react";

export function FeatureArtwork({ kind }: { kind: number }) {
  const id = useId().replace(/:/g, "");
  const rim = `url(#${id}-line)`;
  const surface = `url(#${id}-surface)`;
  const glow = `url(#${id}-glow)`;
  const light = [
    [170, 78, 96, 56],
    [150, 87, 72, 72],
    [172, 108, 95, 60],
    [151, 64, 88, 60],
    [168, 90, 92, 62],
    [154, 79, 108, 49],
  ][kind];
  return (
    <div className={`feature-art art-${kind}`} aria-hidden="true">
      <svg viewBox="0 0 300 180" fill="none">
        <defs>
          <radialGradient id={`${id}-halo`}>
            <stop stopColor="#c2c2c2" stopOpacity=".075" />
            <stop offset="1" stopColor="#c2c2c2" stopOpacity="0" />
          </radialGradient>
          <linearGradient id={`${id}-line`} x1="0" y1="0" x2="1" y2="1">
            <stop stopColor="#c7c7c7" stopOpacity=".64" />
            <stop offset=".45" stopColor="#9a9a9a" stopOpacity=".28" />
            <stop offset="1" stopColor="#a0a0a0" stopOpacity=".08" />
          </linearGradient>
          <linearGradient id={`${id}-surface`} x1="0" y1="0" x2="0" y2="1">
            <stop stopColor="#222222" />
            <stop offset="1" stopColor="#161616" />
          </linearGradient>
          {kind === 3 &&
            [0, 1, 2].map((tier) => (
              <linearGradient
                key={tier}
                id={`${id}-storage-${tier}`}
                className={`storage-gradient storage-gradient-${tier}`}
                x1="0"
                y1="0"
                x2="0"
                y2="1"
              >
                <stop className="storage-top-stop" stopColor="#222222" />
                <stop
                  className="storage-bottom-stop"
                  offset="1"
                  stopColor="#161616"
                />
              </linearGradient>
            ))}
          <linearGradient id={`${id}-beam`}>
            <stop stopColor="#c9c9c9" stopOpacity="0" />
            <stop offset=".5" stopColor="#c9c9c9" stopOpacity=".7" />
            <stop offset="1" stopColor="#c9c9c9" stopOpacity="0" />
          </linearGradient>
          <filter
            id={`${id}-glow`}
            x="-100%"
            y="-100%"
            width="300%"
            height="300%"
          >
            <feGaussianBlur stdDeviation="2.5" />
          </filter>
        </defs>
        <ellipse
          cx={light[0]}
          cy={light[1]}
          rx={light[2]}
          ry={light[3]}
          fill={`url(#${id}-halo)`}
        />
        <g
          stroke={rim}
          strokeWidth=".8"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          {kind === 0 && (
            <>
              <path d="M55 49V134Q55 140 61 140H84M55 79H84M55 109H84" />
              <g transform="translate(43 29)">
                <FolderOpen
                  className="discovery-folder"
                  width={25}
                  height={25}
                  stroke="#b2b2b2"
                  strokeWidth={1}
                />
              </g>
              <text
                x="79"
                y="43"
                fill="#8c8c8c"
                stroke="none"
                fontSize="9"
                fontFamily="monospace"
              >
                ~/Projects/Atlas
              </text>
              {[79, 109, 139].map((y, i) => (
                <g
                  key={y}
                  className="discovery-file"
                  style={{ animationDelay: `${i * 0.15}s` }}
                >
                  <rect
                    x="84"
                    y={y - 12}
                    width="161"
                    height="25"
                    rx="3"
                    fill={i === 0 ? surface : "#161616"}
                    opacity={i === 0 ? 1 : 0.7}
                  />
                  <path
                    className="discovery-glyph"
                    d={`M94 ${y - 7}h8l4 4v11H94ZM102 ${y - 7}v4h4`}
                  />
                  <text
                    className="discovery-filename"
                    x="118"
                    y={y + 4}
                    fill={i === 0 ? "#b9b9b9" : "#919191"}
                    stroke="none"
                    fontSize="10"
                    fontFamily="monospace"
                  >
                    {["Cargo.toml", "frontend/", "api/"][i]}
                  </text>
                  {i === 0 && (
                    <>
                      <circle
                        cx="233"
                        cy={y}
                        r="3"
                        fill="#c6c6c6"
                        stroke="none"
                        filter={glow}
                      />
                      <path d={`M229 ${y}l2 2 4-4`} stroke="#bebebe" />
                    </>
                  )}
                </g>
              ))}
              <g className="scan-beam">
                <path d="M35 59H263" stroke={`url(#${id}-beam)`} />
                <path
                  d="M35 59H263"
                  stroke={`url(#${id}-beam)`}
                  filter={glow}
                />
              </g>
            </>
          )}
          {kind === 1 && (
            <>
              <g stroke="#a6a6a6" opacity=".06">
                <path d="M55 40H245M55 64H245M55 88H245M55 112H245M55 136H245M78 25V155M102 25V155M126 25V155M150 25V155M174 25V155M198 25V155M222 25V155" />
              </g>
              <path
                d="M150 87L76 45M150 87L228 51M150 87L231 137M150 87L83 137"
                stroke="#a1a1a1"
                strokeOpacity=".28"
              />
              <path
                d="M76 45Q45 85 83 137M228 51Q253 88 231 137"
                strokeDasharray="1 5"
                opacity=".7"
              />
              <circle
                className="ontology-orbit"
                cx="150"
                cy="87"
                r="31"
                strokeDasharray="1 6"
              />
              <circle
                cx="150"
                cy="87"
                r="47"
                strokeDasharray="1 8"
                opacity=".3"
              />
              {[
                [150, 87, 15],
                [76, 45, 8],
                [228, 51, 10],
                [231, 137, 7],
                [83, 137, 11],
              ].map(([x, y, r], i) => (
                <g key={i} className={`ontology-node ontology-node-${i}`}>
                  <circle
                    className="ontology-node-rim"
                    cx={x}
                    cy={y}
                    r={r}
                    fill={surface}
                  />
                  <circle
                    className="ontology-node-light"
                    cx={x}
                    cy={y}
                    r={i === 0 ? 5 : 2}
                    fill="#bdbdbd"
                    stroke="none"
                    filter={glow}
                    opacity=".65"
                  />
                  <circle
                    className="ontology-node-light"
                    cx={x}
                    cy={y}
                    r={i === 0 ? 3 : 1.8}
                    fill="#cbcbcb"
                    stroke="none"
                  />
                </g>
              ))}
              <g
                stroke="none"
                fill="#929292"
                fontSize="7"
                fontFamily="monospace"
              >
                <text x="58" y="28">
                  Rust
                </text>
                <text x="214" y="31">
                  egui
                </text>
                <text x="62" y="160">
                  desktop
                </text>
                <text x="217" y="158">
                  tools
                </text>
              </g>
            </>
          )}
          {kind === 2 && (
            <>
              <g className="launcher-back">
                <rect
                  x="43"
                  y="29"
                  width="160"
                  height="102"
                  rx="5"
                  fill={surface}
                />
                <path d="M43 47H203M64 64H107M64 75H134M64 86H117" />
                <path d="M55 58V111" opacity=".4" />
                <g fill="#8a8a8a" stroke="none">
                  <circle cx="54" cy="38" r="1.5" />
                  <circle cx="61" cy="38" r="1.5" />
                  <circle cx="68" cy="38" r="1.5" />
                </g>
                <text
                  x="80"
                  y="41"
                  fontSize="7"
                  fill="#939393"
                  stroke="none"
                  fontFamily="monospace"
                >
                  atlas/src
                </text>
              </g>
              <g className="launcher-front">
                <rect
                  x="102"
                  y="68"
                  width="153"
                  height="88"
                  rx="5"
                  fill="#141414"
                  stroke="#000"
                  strokeOpacity=".7"
                  strokeWidth="4"
                />
                <rect
                  x="99"
                  y="64"
                  width="153"
                  height="88"
                  rx="5"
                  fill={surface}
                />
                <path d="M99 83H252" />
                <text
                  x="111"
                  y="76"
                  fontSize="7"
                  fill="#989898"
                  stroke="none"
                  fontFamily="monospace"
                >
                  ~/Projects/Atlas
                </text>
                <path
                  className="terminal-prompt"
                  d="M113 99L118 103L113 107"
                  stroke="#bcbcbc"
                />
                <text
                  className="terminal-command"
                  x="127"
                  y="107"
                  fill="#b4b4b4"
                  stroke="none"
                  fontSize="8"
                  fontFamily="monospace"
                >
                  cargo run
                </text>
                <rect
                  className="terminal-cursor"
                  x="180"
                  y="99"
                  width="4"
                  height="9"
                  fill="#c8c8c8"
                  stroke="none"
                />
                <path d="M113 122H193M113 134H172" opacity=".45" />
                <circle cx="238" cy="74" r="2" fill="#afafaf" stroke="none" />
              </g>
            </>
          )}
          {kind === 3 && (
            <>
              <ellipse
                cx="150"
                cy="150"
                rx="88"
                ry="12"
                strokeDasharray="1 5"
                opacity=".55"
              />
              <ellipse
                cx="150"
                cy="148"
                rx="65"
                ry="8"
                fill="#000"
                opacity=".2"
                stroke="none"
              />
              {[104, 77, 50].map((y, i) => (
                <g
                  key={y}
                  className="storage-layer"
                  style={{ animationDelay: `${i * 0.15}s` }}
                >
                  <path
                    className="storage-shell"
                    d={`M87 ${y}v23c0 18 126 18 126 0v-23`}
                    fill={`url(#${id}-storage-${i})`}
                  />
                  <ellipse
                    className="storage-shell"
                    cx="150"
                    cy={y}
                    rx="63"
                    ry="14"
                    fill={`url(#${id}-storage-${i})`}
                  />
                  <ellipse cx="150" cy={y} rx="54" ry="9" opacity=".3" />
                  <path d={`M101 ${y + 20}h25M130 ${y + 20}h7`} opacity=".65" />
                  <circle
                    className="storage-led"
                    cx="198"
                    cy={y + 19}
                    r="3"
                    fill="#c6c6c6"
                    stroke="none"
                    filter={glow}
                    opacity=".6"
                  />
                  <circle
                    className="storage-led"
                    cx="198"
                    cy={y + 19}
                    r="1.4"
                    fill="#d0d0d0"
                    stroke="none"
                  />
                </g>
              ))}
              <text
                x="123"
                y="53"
                fill="#a0a0a0"
                stroke="none"
                fontSize="8"
                fontFamily="monospace"
              >
                localhost
              </text>
            </>
          )}
          {kind === 4 && (
            <>
              <path
                d={mcpConnections}
                stroke="#a2a2a2"
                strokeOpacity=".3"
                strokeLinecap="round"
                strokeLinejoin="round"
              />
              <path
                className="art-signal mcp-signal"
                d={mcpConnections}
                stroke="#d5d5d5"
                strokeDasharray="6 220"
                strokeLinecap="round"
                strokeLinejoin="round"
              />
              <rect
                x="26"
                y="61"
                width="70"
                height="58"
                rx="5"
                fill={surface}
              />
              <path d="M26 74H96M42 85L48 90L42 95M55 95H65" />
              <circle cx="35" cy="68" r="1.3" fill="#a4a4a4" stroke="none" />
              <text
                x="44"
                y="109"
                fill="#939393"
                stroke="none"
                fontSize="7"
                fontFamily="monospace"
              >
                stdio
              </text>
              {[42, 78, 114].map((y, i) => (
                <g key={y}>
                  <rect
                    x="210"
                    y={y}
                    width="65"
                    height="24"
                    rx="3"
                    fill={surface}
                  />
                  <circle
                    className="mcp-port"
                    cx="210"
                    cy={y + 12}
                    r="3"
                    fill="#c5c5c5"
                    stroke="none"
                    filter={glow}
                    opacity=".55"
                  />
                  <circle
                    className="mcp-port"
                    cx="210"
                    cy={y + 12}
                    r="1.3"
                    fill="#c8c8c8"
                    stroke="none"
                  />
                  <text
                    x="220"
                    y={y + 15}
                    fill="#a7a7a7"
                    stroke="none"
                    fontSize="8"
                    fontFamily="monospace"
                  >
                    {["projects", "steps", "concepts"][i]}
                  </text>
                </g>
              ))}
              <text
                x="120"
                y="76"
                fill="#8d8d8d"
                stroke="none"
                fontSize="7"
                fontFamily="monospace"
              >
                MCP
              </text>
            </>
          )}
          {kind === 5 && (
            <>
              <rect
                x="46"
                y="29"
                width="209"
                height="29"
                rx="5"
                fill={surface}
              />
              <circle cx="62" cy="42" r="4" />
              <path d="M65 45L69 49" />
              <text
                x="80"
                y="47"
                fill="#a0a0a0"
                stroke="none"
                fontSize="9"
                fontFamily="monospace"
              >
                agent
              </text>
              <rect x="230" y="37" width="15" height="13" rx="2" opacity=".5" />
              <path d="M235 41v5h5" opacity=".6" />
              {[76, 106, 136].map((y, i) => (
                <g
                  key={y}
                  className="search-result"
                  style={{ animationDelay: `${i * 0.18}s` }}
                >
                  <rect
                    x="55"
                    y={y}
                    width="190"
                    height="24"
                    rx="3"
                    fill={surface}
                    opacity={1 - i * 0.2}
                  />
                  <path d={`M66 ${y + 8}h7l3 3-3 5h-7Z`} opacity=".8" />
                  <text
                    x="85"
                    y={y + 15}
                    fill={i === 0 ? "#bdbdbd" : "#929292"}
                    stroke="none"
                    fontSize="8"
                    fontFamily="monospace"
                  >
                    {["Scribe Agent", "Promptbook", "Inbox Bot"][i]}
                  </text>
                  <path
                    className="relevance-bar search-relevance"
                    d={`M212 ${y + 19}h${23 - i * 5}`}
                    stroke="#a7a7a7"
                    opacity=".4"
                  />
                  <text
                    className="search-score"
                    x="215"
                    y={y + 13}
                    fill="#aaaaaa"
                    stroke="none"
                    fontSize="8"
                    fontFamily="monospace"
                  >
                    {[".94", ".81", ".72"][i]}
                  </text>
                </g>
              ))}
            </>
          )}
        </g>
      </svg>
    </div>
  );
}
