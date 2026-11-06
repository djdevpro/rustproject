//! Les paquets d'un projet : les dépendances DIRECTES déclarées dans ses manifestes, dans chacun
//! de ses dossiers, pour tous les gestionnaires courants (voir `Manager` et `read_dir`) : npm
//! (et yarn, pnpm, bun), Cargo, pip (requirements, pyproject, Pipfile, conda), Composer, Go,
//! Maven, Gradle, NuGet, RubyGems, pub (Dart, Flutter), Swift Package Manager, CocoaPods, Hex
//! (Elixir), Deno, vcpkg, Conan, Julia et CRAN (R).
//! Ni les dépendances de dépendances (fichiers de verrouillage, node_modules), ni les paquets
//! locaux du projet (`workspace:`, `file:`, `path = …`) : seulement les paquets publiés qu'il
//! utilise. La détection est automatique ; la Bibliothèque ne montre que les gestionnaires qui
//! ont des paquets, filtre par paquet, et la recherche les trouve (« expo »).

use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Manager {
    Cargo,
    Npm,
    Composer,
    Pip,
    Go,
    Maven,
    Gradle,
    NuGet,
    RubyGems,
    Pub,
    SwiftPm,
    CocoaPods,
    Hex,
    Deno,
    Conda,
    Vcpkg,
    Conan,
    Julia,
    Cran,
    /// Les images Docker (FROM des Dockerfile, `image:` des fichiers compose) : leur propre
    /// rubrique dans la colonne, à part des paquets.
    Docker,
}

impl Manager {
    /// Dans l'ordre de la colonne : les plus répandus d'abord.
    pub const ALL: [Self; 19] = [
        Self::Npm,
        Self::Pip,
        Self::Cargo,
        Self::Composer,
        Self::Go,
        Self::Maven,
        Self::Gradle,
        Self::NuGet,
        Self::RubyGems,
        Self::Pub,
        Self::SwiftPm,
        Self::CocoaPods,
        Self::Hex,
        Self::Deno,
        Self::Conda,
        Self::Vcpkg,
        Self::Conan,
        Self::Julia,
        Self::Cran,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Cargo => "Cargo",
            Self::Npm => "npm",
            Self::Composer => "Composer",
            Self::Pip => "pip",
            Self::Go => "Go",
            Self::Maven => "Maven",
            Self::Gradle => "Gradle",
            Self::NuGet => "NuGet",
            Self::RubyGems => "RubyGems",
            Self::Pub => "pub",
            Self::SwiftPm => "Swift",
            Self::CocoaPods => "CocoaPods",
            Self::Hex => "Hex",
            Self::Deno => "Deno",
            Self::Conda => "conda",
            Self::Vcpkg => "vcpkg",
            Self::Conan => "Conan",
            Self::Julia => "Julia",
            Self::Cran => "CRAN",
            Self::Docker => "Docker",
        }
    }

    /// La couleur de l'écosystème (repère visuel dans la colonne).
    pub fn color(self) -> egui::Color32 {
        crate::theme::rgb(match self {
            Self::Cargo => 0xdea584,
            Self::Npm => 0xe2574c,
            Self::Composer => 0xb8916b,
            Self::Pip => 0x5b9bd5,
            Self::Go => 0x4fc3d9,
            Self::Maven => 0xe07a5f,
            Self::Gradle => 0x5fb3a1,
            Self::NuGet => 0x8f7ae0,
            Self::RubyGems => 0xd9534f,
            Self::Pub => 0x42a5f5,
            Self::SwiftPm => 0xf28c4a,
            Self::CocoaPods => 0xe8a15a,
            Self::Hex => 0xa98bd6,
            Self::Deno => 0x9aa5b1,
            Self::Conda => 0x7cb86f,
            Self::Vcpkg => 0x6b8fcf,
            Self::Conan => 0x6fb1d6,
            Self::Julia => 0xb07cc9,
            Self::Cran => 0x6a9fd8,
            Self::Docker => 0x2496ed,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Package {
    pub manager: Manager,
    pub name: String,
}

fn read(path: &Path) -> Option<String> {
    let meta = std::fs::metadata(path).ok()?;
    // Un manifeste fait quelques kilo-octets ; au-delà, ce n'en est pas un.
    if !meta.is_file() || meta.len() > 2_000_000 {
        return None;
    }
    std::fs::read_to_string(path).ok()
}

/// Les noms des dépendances npm : `dependencies`, `devDependencies`, `peerDependencies`, sans les
/// paquets locaux (`workspace:*`, `file:`, `link:`).
pub fn npm(text: &str) -> Vec<String> {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(text) else { return Vec::new() };
    let mut out = Vec::new();
    for key in ["dependencies", "devDependencies", "peerDependencies", "optionalDependencies"] {
        if let Some(obj) = v[key].as_object() {
            for (name, version) in obj {
                let local = version.as_str().is_some_and(|s| ["workspace:", "file:", "link:", "portal:"].iter().any(|p| s.starts_with(p)));
                // `@types/react` n'est que la description des types de `react` : pas un paquet de plus.
                if !local && !name.starts_with("@types/") {
                    out.push(name.to_lowercase());
                }
            }
        }
    }
    out
}

/// Les noms des dépendances Composer (`require`, `require-dev`), sans PHP ni ses extensions.
pub fn composer(text: &str) -> Vec<String> {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(text) else { return Vec::new() };
    let mut out = Vec::new();
    for key in ["require", "require-dev"] {
        if let Some(obj) = v[key].as_object() {
            for name in obj.keys() {
                let n = name.to_lowercase();
                if n != "php" && !n.starts_with("ext-") && !n.starts_with("lib-") && n.contains('/') {
                    out.push(n);
                }
            }
        }
    }
    out
}

/// Les noms des dépendances Cargo : sections `[dependencies]`, `[dev-dependencies]`,
/// `[build-dependencies]`, `[workspace.dependencies]`, `[target.….dependencies]`, et les tables
/// `[dependencies.serde]`. Sans les crates locales (`path = …`).
pub fn cargo(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut in_deps = false;
    let mut table: Option<(String, bool)> = None;
    let flush = |table: &mut Option<(String, bool)>, out: &mut Vec<String>| {
        if let Some((name, local)) = table.take()
            && !local
        {
            out.push(name);
        }
    };
    for line in text.lines() {
        let l = line.split('#').next().unwrap_or("").trim();
        if l.starts_with('[') {
            flush(&mut table, &mut out);
            let header = l.trim_matches(|c| c == '[' || c == ']').trim();
            let is_deps = |h: &str| h == "dependencies" || h.ends_with(".dependencies") || h.ends_with("dev-dependencies") || h.ends_with("build-dependencies");
            in_deps = is_deps(header);
            // `[dependencies.serde]` : une dépendance décrite en table.
            if !in_deps
                && let Some((section, name)) = header.rsplit_once('.')
                && is_deps(section)
            {
                table = Some((name.trim_matches('"').to_owned(), false));
            }
            continue;
        }
        if let Some((_, local)) = table.as_mut() {
            if l.starts_with("path") && l.contains('=') {
                *local = true;
            }
            continue;
        }
        if in_deps && let Some((k, v)) = l.split_once('=') {
            let name = k.trim().trim_matches('"');
            // `foo = { path = "../foo" }` : une crate du même dépôt.
            if !name.is_empty() && !(v.contains("path") && v.contains('{') && !v.contains("version")) {
                out.push(name.to_owned());
            }
        }
    }
    flush(&mut table, &mut out);
    out
}

/// Le nom d'un paquet Python dans une ligne de dépendance (« fastapi[all]>=0.110 ; python_version… »),
/// normalisé comme le fait pip (minuscules, `_` et `.` deviennent `-`).
fn pip_name(spec: &str) -> Option<String> {
    let spec = spec.trim().trim_matches(|c| c == '"' || c == '\'' || c == ',');
    if spec.is_empty() || spec.starts_with('-') || spec.starts_with('#') || spec.contains("://") {
        return None;
    }
    let name: String = spec.chars().take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')).collect();
    let name = name.to_lowercase().replace(['_', '.'], "-");
    (!name.is_empty() && name != "python").then_some(name)
}

/// Les dépendances d'un requirements.txt.
pub fn requirements(text: &str) -> Vec<String> {
    text.lines().filter_map(|l| pip_name(l.split('#').next().unwrap_or(""))).collect()
}

/// Les dépendances d'un pyproject.toml : `[project] dependencies = [...]` (et les groupes
/// facultatifs), ou les tables Poetry `[tool.poetry.dependencies]`.
pub fn pyproject(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut section = String::new();
    let mut in_array = false;
    for line in text.lines() {
        let l = line.split('#').next().unwrap_or("").trim();
        if in_array {
            for part in l.trim_end_matches(']').split(',') {
                out.extend(pip_name(part));
            }
            if l.contains(']') {
                in_array = false;
            }
            continue;
        }
        if l.starts_with('[') {
            section = l.trim_matches(|c| c == '[' || c == ']').trim().to_owned();
            continue;
        }
        let Some((k, v)) = l.split_once('=') else { continue };
        let (k, v) = (k.trim(), v.trim());
        let poetry = section.starts_with("tool.poetry") && section.ends_with("dependencies");
        if poetry {
            out.extend(pip_name(k));
        } else if ((section == "project" && k == "dependencies") || section == "project.optional-dependencies" || section == "dependency-groups")
            && let Some(rest) = v.strip_prefix('[')
        {
            for part in rest.trim_end_matches(']').split(',') {
                out.extend(pip_name(part));
            }
            in_array = !v.contains(']');
        }
    }
    out
}

/// Les clés d'une section TOML (`[deps]`, `[packages]`…) : `nom = …`.
fn toml_keys(text: &str, sections: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    let mut inside = false;
    for line in text.lines() {
        let l = line.split('#').next().unwrap_or("").trim();
        if l.starts_with('[') {
            inside = sections.contains(&l.trim_matches(|c| c == '[' || c == ']').trim());
        } else if inside && let Some((k, _)) = l.split_once('=') {
            let k = k.trim().trim_matches('"');
            if !k.is_empty() {
                out.push(k.to_owned());
            }
        }
    }
    out
}

/// Ce qui suit `marker` jusqu'au guillemet fermant : `pod 'Alamofire'` → `Alamofire`.
fn quoted_after<'a>(line: &'a str, marker: &str) -> Option<&'a str> {
    let rest = line.trim_start().strip_prefix(marker)?;
    let rest = rest.trim_start_matches(|c: char| c == '(' || c.is_whitespace());
    let quote = rest.chars().next().filter(|c| *c == '"' || *c == '\'')?;
    let body = &rest[1..];
    body.find(quote).map(|end| &body[..end])
}

/// Go : les modules de `require`, sans ceux marqués `// indirect`.
pub fn go_mod(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut block = false;
    for line in text.lines() {
        let l = line.trim();
        if l.starts_with("require (") {
            block = true;
            continue;
        }
        if block && l == ")" {
            block = false;
            continue;
        }
        let spec = if block { Some(l) } else { l.strip_prefix("require ") };
        if let Some(spec) = spec.filter(|s| !s.contains("// indirect") && !s.is_empty() && !s.starts_with("//"))
            && let Some(module) = spec.split_whitespace().next()
        {
            out.push(module.to_owned());
        }
    }
    out
}

/// Ruby : les `gem "rails"` du Gemfile.
pub fn gemfile(text: &str) -> Vec<String> {
    text.lines().filter_map(|l| quoted_after(l, "gem")).map(str::to_owned).collect()
}

/// CocoaPods : les `pod 'Alamofire'` du Podfile.
pub fn podfile(text: &str) -> Vec<String> {
    text.lines().filter_map(|l| quoted_after(l, "pod")).map(|n| n.split('/').next().unwrap_or(n).to_owned()).collect()
}

/// Dart, Flutter : les clés de `dependencies` et `dev_dependencies` (sans le SDK Flutter).
pub fn pubspec(text: &str) -> Vec<String> {
    const SDK: [&str; 5] = ["flutter", "flutter_test", "flutter_localizations", "flutter_driver", "integration_test"];
    let mut out = Vec::new();
    let mut inside = false;
    for line in text.lines() {
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        if indent == 0 {
            inside = matches!(line.trim_end(), "dependencies:" | "dev_dependencies:");
        } else if inside
            && indent == 2
            && let Some((k, _)) = line.trim().split_once(':')
            && !SDK.contains(&k.trim())
        {
            out.push(k.trim().to_owned());
        }
    }
    out
}

/// .NET : les `<PackageReference Include="…">` (et `packages.config`, `<PackageVersion>`).
pub fn nuget(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (tag, attr) in [("<PackageReference", "Include=\""), ("<PackageReference", "Update=\""), ("<PackageVersion", "Include=\""), ("<package ", "id=\"")] {
        for part in text.split(tag).skip(1) {
            let head = part.split('>').next().unwrap_or("");
            if let Some(start) = head.find(attr) {
                let rest = &head[start + attr.len()..];
                if let Some(end) = rest.find('"') {
                    out.push(rest[..end].to_owned());
                }
            }
        }
    }
    out
}

/// Maven : `groupId:artifactId` de chaque `<dependency>` (sans les modules du projet lui-même).
pub fn pom(text: &str) -> Vec<String> {
    let tag = |block: &str, name: &str| {
        let open = format!("<{name}>");
        let start = block.find(&open)? + open.len();
        let end = block[start..].find(&format!("</{name}>"))? + start;
        Some(block[start..end].trim().to_owned())
    };
    text.split("<dependency>")
        .skip(1)
        .filter_map(|part| {
            let block = part.split("</dependency>").next().unwrap_or("");
            let (group, artifact) = (tag(block, "groupId")?, tag(block, "artifactId")?);
            (!group.contains("${project")).then(|| format!("{group}:{artifact}"))
        })
        .collect()
}

/// Gradle (Groovy ou Kotlin) : `implementation("groupe:artefact:version")` et ses variantes.
pub fn gradle(text: &str) -> Vec<String> {
    const CONFIGS: [&str; 10] = ["implementation", "api", "compileOnly", "runtimeOnly", "testImplementation", "androidTestImplementation", "kapt", "ksp", "annotationProcessor", "classpath"];
    let mut out = Vec::new();
    for line in text.lines() {
        let l = line.trim();
        for c in CONFIGS {
            if let Some(spec) = quoted_after(l, c) {
                let mut parts = spec.split(':');
                if let (Some(g), Some(a)) = (parts.next(), parts.next())
                    && !g.is_empty()
                    && !a.is_empty()
                {
                    out.push(format!("{g}:{a}"));
                }
                break;
            }
        }
    }
    out
}

/// Swift Package Manager : le nom du dépôt de chaque `.package(url: "…/swift-nio.git", …)`.
pub fn package_swift(text: &str) -> Vec<String> {
    text.split(".package(")
        .skip(1)
        .filter_map(|part| {
            let head = part.split(')').next().unwrap_or("");
            let url = head.split("url:").nth(1)?.trim_start().strip_prefix('"')?;
            let url = &url[..url.find('"')?];
            Some(url.trim_end_matches('/').trim_end_matches(".git").rsplit('/').next()?.to_owned())
        })
        .collect()
}

/// Elixir : les `{:phoenix, "~> 1.7"}` de `mix.exs`.
pub fn mix_exs(text: &str) -> Vec<String> {
    text.split("{:")
        .skip(1)
        .filter_map(|part| {
            let name: String = part.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
            let rest = &part[name.len()..];
            (!name.is_empty() && rest.trim_start().starts_with(',')).then_some(name)
        })
        .collect()
}

/// Deno : les `imports` de `deno.json` en `jsr:` ou `npm:` (sans la version).
pub fn deno(text: &str) -> Vec<String> {
    // deno.jsonc peut avoir des commentaires : on les retire ligne par ligne.
    let clean: String = text.lines().map(|l| if l.trim_start().starts_with("//") { "" } else { l }).collect::<Vec<_>>().join("\n");
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&clean) else { return Vec::new() };
    let Some(obj) = v["imports"].as_object() else { return Vec::new() };
    obj.values()
        .filter_map(|spec| {
            let spec = spec.as_str()?;
            let bare = spec.strip_prefix("jsr:").or_else(|| spec.strip_prefix("npm:"))?.trim_start_matches('/');
            // `@std/path@^1.0` → `@std/path` ; `chalk@5` → `chalk`.
            let cut = bare.char_indices().skip(1).find(|(_, c)| *c == '@').map(|(i, _)| i).unwrap_or(bare.len());
            Some(bare[..cut].to_owned())
        })
        .collect()
}

/// conda : la liste `dependencies` d'`environment.yml` (les paquets pip qu'elle contient vont à pip).
pub fn conda(text: &str) -> (Vec<String>, Vec<String>) {
    let (mut conda, mut pip) = (Vec::new(), Vec::new());
    let (mut inside, mut in_pip) = (false, false);
    for line in text.lines() {
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        if indent == 0 {
            inside = line.trim_end() == "dependencies:";
            in_pip = false;
            continue;
        }
        let Some(item) = line.trim().strip_prefix("- ") else { continue };
        if !inside {
            continue;
        }
        if item.trim_end() == "pip:" {
            in_pip = true;
            continue;
        }
        if in_pip && indent > 2 {
            pip.extend(pip_name(item));
        } else {
            in_pip = false;
            let name: String = item.chars().take_while(|c| !matches!(c, '=' | '<' | '>' | ' ' | '[')).collect();
            let name = name.rsplit("::").next().unwrap_or("").to_lowercase();
            if !name.is_empty() && name != "python" && name != "pip" {
                conda.push(name);
            }
        }
    }
    (conda, pip)
}

/// vcpkg : la liste `dependencies` de `vcpkg.json` (noms simples ou objets `{"name": …}`).
pub fn vcpkg(text: &str) -> Vec<String> {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(text) else { return Vec::new() };
    v["dependencies"].as_array().map(|a| a.iter().filter_map(|d| d.as_str().or_else(|| d["name"].as_str()).map(str::to_owned)).collect()).unwrap_or_default()
}

/// Conan : la section `[requires]` de `conanfile.txt` (« zlib/1.3 » → « zlib »).
pub fn conanfile(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut inside = false;
    for line in text.lines() {
        let l = line.split('#').next().unwrap_or("").trim();
        if l.starts_with('[') {
            inside = matches!(l, "[requires]" | "[tool_requires]" | "[build_requires]");
        } else if inside && !l.is_empty() {
            out.push(l.split('/').next().unwrap_or(l).to_owned());
        }
    }
    out
}

/// R : les champs `Imports` et `Depends` du fichier DESCRIPTION (sans R lui-même).
pub fn r_description(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut field = String::new();
    for line in text.lines() {
        if !line.starts_with([' ', '\t']) {
            field = line.split(':').next().unwrap_or("").to_owned();
        }
        if !matches!(field.as_str(), "Imports" | "Depends" | "LinkingTo") {
            continue;
        }
        let values = if line.starts_with([' ', '\t']) { line } else { line.split_once(':').map(|(_, v)| v).unwrap_or("") };
        for part in values.split(',') {
            let name: String = part.trim().chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '.').collect();
            if !name.is_empty() && name != "R" {
                out.push(name);
            }
        }
    }
    out
}

/// Le nom d'une image Docker, sans étiquette ni empreinte, et sans le registre par défaut :
/// `docker.io/library/postgres:16-alpine` → `postgres`, `ghcr.io/moi/app:1` → `ghcr.io/moi/app`.
/// Une image faite de variables (`${IMAGE}`) ne dit rien : `None`.
pub fn image_name(spec: &str) -> Option<String> {
    let spec = spec.trim().trim_matches(|c| c == '"' || c == '\'');
    if spec.is_empty() || spec.contains('$') || spec.eq_ignore_ascii_case("scratch") {
        return None;
    }
    let spec = spec.split('@').next().unwrap_or(spec);
    // L'étiquette est après le dernier « : » qui suit le dernier « / » (un port de registre, non).
    let slash = spec.rfind('/').map(|i| i + 1).unwrap_or(0);
    let name = match spec[slash..].find(':') {
        Some(i) => &spec[..slash + i],
        None => spec,
    };
    let name = name.to_lowercase();
    let name = name.trim_start_matches("docker.io/").trim_start_matches("index.docker.io/").trim_start_matches("library/");
    (!name.is_empty()).then(|| name.to_owned())
}

/// Les images des `FROM` d'un Dockerfile, sans les étapes précédentes (`FROM builder`).
pub fn dockerfile(text: &str) -> Vec<String> {
    let mut stages: Vec<String> = Vec::new();
    let mut out = Vec::new();
    for line in text.lines() {
        let mut words = line.split_whitespace();
        if !words.next().is_some_and(|w| w.eq_ignore_ascii_case("FROM")) {
            continue;
        }
        let rest: Vec<&str> = words.filter(|w| !w.starts_with("--")).collect();
        let Some(image) = rest.first() else { continue };
        if !stages.iter().any(|s| s.eq_ignore_ascii_case(image))
            && let Some(name) = image_name(image)
        {
            out.push(name);
        }
        if let (Some(kw), Some(alias)) = (rest.get(1), rest.get(2))
            && kw.eq_ignore_ascii_case("AS")
        {
            stages.push((*alias).to_owned());
        }
    }
    out
}

/// Les images (`image: …`) d'un fichier docker-compose.
pub fn compose(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|l| l.split('#').next().unwrap_or("").trim().strip_prefix("image:"))
        .filter_map(image_name)
        .collect()
}

/// Les fichiers Docker d'un dossier : Dockerfile (et ses variantes), fichiers compose.
fn docker_files(dir: &Path) -> Vec<(std::path::PathBuf, bool)> {
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_lowercase();
            let is_dockerfile = name == "dockerfile" || name.starts_with("dockerfile.") || name.ends_with(".dockerfile");
            let is_compose = (name.starts_with("docker-compose") || name.starts_with("compose")) && (name.ends_with(".yml") || name.ends_with(".yaml"));
            (is_dockerfile || is_compose).then(|| (e.path(), is_compose))
        })
        .collect()
}

/// Les paquets déclarés dans un dossier, pour tous les gestionnaires reconnus.
pub fn read_dir(dir: &Path) -> Vec<Package> {
    let mut out = Vec::new();
    let mut add = |manager: Manager, names: Vec<String>| out.extend(names.into_iter().filter(|n| !n.trim().is_empty()).map(|name| Package { manager, name }));
    let file = |name: &str| read(&dir.join(name));
    if let Some(t) = file("package.json") {
        add(Manager::Npm, npm(&t));
    }
    if let Some(t) = file("Cargo.toml") {
        add(Manager::Cargo, cargo(&t));
    }
    if let Some(t) = file("composer.json") {
        add(Manager::Composer, composer(&t));
    }
    if let Some(t) = file("requirements.txt") {
        add(Manager::Pip, requirements(&t));
    }
    if let Some(t) = file("pyproject.toml") {
        add(Manager::Pip, pyproject(&t));
    }
    if let Some(t) = file("Pipfile") {
        add(Manager::Pip, toml_keys(&t, &["packages", "dev-packages"]).iter().filter_map(|k| pip_name(k)).collect());
    }
    for name in ["environment.yml", "environment.yaml"] {
        if let Some(t) = file(name) {
            let (c, p) = conda(&t);
            add(Manager::Conda, c);
            add(Manager::Pip, p);
        }
    }
    if let Some(t) = file("go.mod") {
        add(Manager::Go, go_mod(&t));
    }
    if let Some(t) = file("pom.xml") {
        add(Manager::Maven, pom(&t));
    }
    for name in ["build.gradle", "build.gradle.kts"] {
        if let Some(t) = file(name) {
            add(Manager::Gradle, gradle(&t));
        }
    }
    if let Some(t) = file("Gemfile") {
        add(Manager::RubyGems, gemfile(&t));
    }
    if let Some(t) = file("pubspec.yaml") {
        add(Manager::Pub, pubspec(&t));
    }
    if let Some(t) = file("Package.swift") {
        add(Manager::SwiftPm, package_swift(&t));
    }
    if let Some(t) = file("Podfile") {
        add(Manager::CocoaPods, podfile(&t));
    }
    if let Some(t) = file("mix.exs") {
        add(Manager::Hex, mix_exs(&t));
    }
    for name in ["deno.json", "deno.jsonc"] {
        if let Some(t) = file(name) {
            add(Manager::Deno, deno(&t));
        }
    }
    if let Some(t) = file("vcpkg.json") {
        add(Manager::Vcpkg, vcpkg(&t));
    }
    if let Some(t) = file("conanfile.txt") {
        add(Manager::Conan, conanfile(&t));
    }
    // Julia : Project.toml (avec une majuscule ; pyproject.toml est celui de Python).
    if std::fs::read_dir(dir).into_iter().flatten().flatten().any(|e| e.file_name() == "Project.toml")
        && let Some(t) = file("Project.toml")
    {
        add(Manager::Julia, toml_keys(&t, &["deps"]));
    }
    if let Some(t) = file("DESCRIPTION").filter(|t| t.starts_with("Package:")) {
        add(Manager::Cran, r_description(&t));
    }
    // Docker : les images des Dockerfile et des fichiers compose.
    for (path, is_compose) in docker_files(dir) {
        if let Some(t) = read(&path) {
            add(Manager::Docker, if is_compose { compose(&t) } else { dockerfile(&t) });
        }
    }
    // .NET : chaque projet (.csproj, .fsproj, .vbproj), packages.config, Directory.Packages.props.
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let name = entry.file_name().to_string_lossy().to_lowercase();
        if (name.ends_with(".csproj") || name.ends_with(".fsproj") || name.ends_with(".vbproj") || name == "packages.config" || name == "directory.packages.props")
            && let Some(t) = read(&entry.path())
        {
            add(Manager::NuGet, nuget(&t));
        }
    }
    out
}

/// Les paquets d'un projet : ceux de tous ses dossiers, sans doublon, triés.
pub fn of_folders(folders: &[String]) -> Vec<Package> {
    let mut out: Vec<Package> = folders.iter().map(|f| f.trim()).filter(|f| !f.is_empty()).flat_map(|f| read_dir(Path::new(f))).collect();
    out.sort();
    out.dedup();
    out
}

/// À quel point une requête désigne un paquet du projet : 1 pour le nom exact (« expo »), 0,9
/// pour un paquet de la même famille (« expo-router », « @expo/vector-icons »), 0 sinon.
/// `words` : les mots de la requête (« expo router » cherche aussi « expo-router »).
pub fn match_score(words: &[String], packages: &[Package]) -> f32 {
    if words.is_empty() || packages.is_empty() {
        return 0.0;
    }
    let dashed = words.join("-");
    let mut best: f32 = 0.0;
    for p in packages {
        let n = p.name.as_str();
        if n == dashed || n == words.concat() {
            return 1.0;
        }
        if words.len() == 1 && (n.starts_with(&format!("{dashed}-")) || n.starts_with(&format!("@{dashed}/")) || n.ends_with(&format!("/{dashed}"))) {
            best = best.max(0.9);
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dependances_directes_seulement() {
        let pkg = r#"{"dependencies":{"expo":"~52","@expo/vector-icons":"^14","@app/ui":"workspace:*"},"devDependencies":{"TypeScript":"5","@types/react":"19"}}"#;
        let mut names = npm(pkg);
        names.sort();
        assert_eq!(names, vec!["@expo/vector-icons", "expo", "typescript"], "sans le paquet local du monorepo");
        let cargo_toml = "[package]\nname = \"outil\"\nversion = \"0.1.0\"\n\n[dependencies]\negui = \"0.37\"\nserde = { version = \"1\", features = [\"derive\"] }\ncommun = { path = \"../commun\" }\n\n[dev-dependencies]\negui_kittest = \"0.37\"\n\n[dependencies.tokio]\nversion = \"1\"\n\n[dependencies.local]\npath = \"../local\"\n";
        assert_eq!(cargo(cargo_toml), vec!["egui", "serde", "egui_kittest", "tokio"], "sans les crates locales");
        let composer_json = r#"{"require":{"php":"^8.3","ext-json":"*","laravel/framework":"^11"},"require-dev":{"phpunit/phpunit":"^11"}}"#;
        assert_eq!(composer(composer_json), vec!["laravel/framework", "phpunit/phpunit"]);
        assert_eq!(requirements("fastapi[all]>=0.110\n# commentaire\nPyYAML==6.0\n-r dev.txt\nopenai_whisper\n"), vec!["fastapi", "pyyaml", "openai-whisper"]);
        let py = "[project]\nname = \"api\"\ndependencies = [\n  \"torch>=2\",\n  \"numpy\",\n]\n\n[tool.poetry.dependencies]\npython = \"^3.12\"\nrequests = \"^2\"\n";
        assert_eq!(pyproject(py), vec!["torch", "numpy", "requests"]);
    }

    #[test]
    fn tous_les_gestionnaires() {
        assert_eq!(go_mod("module x\n\ngo 1.22\n\nrequire (\n\tgithub.com/gin-gonic/gin v1.9.1\n\tgolang.org/x/text v0.14.0 // indirect\n)\nrequire github.com/spf13/cobra v1.8.0\n"), vec!["github.com/gin-gonic/gin", "github.com/spf13/cobra"]);
        assert_eq!(gemfile("source 'https://rubygems.org'\ngem 'rails', '~> 7.1'\n  gem \"pg\"\n"), vec!["rails", "pg"]);
        assert_eq!(podfile("target 'App' do\n  pod 'Alamofire', '~> 5'\n  pod 'Firebase/Analytics'\nend\n"), vec!["Alamofire", "Firebase"]);
        assert_eq!(pubspec("name: app\ndependencies:\n  flutter:\n    sdk: flutter\n  provider: ^6.0.0\n  http: ^1.2.0\ndev_dependencies:\n  flutter_test:\n    sdk: flutter\n  mocktail: ^1.0.0\n"), vec!["provider", "http", "mocktail"]);
        assert_eq!(nuget("<Project><ItemGroup><PackageReference Include=\"Newtonsoft.Json\" Version=\"13\" /><ProjectReference Include=\"..\\Lib\\Lib.csproj\" /></ItemGroup></Project>"), vec!["Newtonsoft.Json"]);
        assert_eq!(pom("<project><dependencies><dependency><groupId>org.springframework.boot</groupId><artifactId>spring-boot-starter-web</artifactId></dependency><dependency><groupId>${project.groupId}</groupId><artifactId>core</artifactId></dependency></dependencies></project>"), vec!["org.springframework.boot:spring-boot-starter-web"]);
        assert_eq!(gradle("dependencies {\n    implementation(\"io.ktor:ktor-server-core:2.3.0\")\n    testImplementation 'junit:junit:4.13.2'\n    implementation project(':lib')\n}\n"), vec!["io.ktor:ktor-server-core", "junit:junit"]);
        assert_eq!(package_swift("dependencies: [\n  .package(url: \"https://github.com/apple/swift-nio.git\", from: \"2.0.0\"),\n  .package(path: \"../Local\"),\n]"), vec!["swift-nio"]);
        assert_eq!(mix_exs("defp deps do\n  [\n    {:phoenix, \"~> 1.7\"},\n    {:ecto_sql, \"~> 3.10\"}\n  ]\nend\n"), vec!["phoenix", "ecto_sql"]);
        let mut d = deno("{\n  // commentaire\n  \"imports\": { \"@std/path\": \"jsr:@std/path@^1.0\", \"chalk\": \"npm:chalk@5\", \"local\": \"./mod.ts\" }\n}");
        d.sort();
        assert_eq!(d, vec!["@std/path", "chalk"]);
        assert_eq!(conda("name: env\ndependencies:\n  - python=3.11\n  - numpy>=1.26\n  - conda-forge::pandas\n  - pip\n  - pip:\n    - openai==1.0\n"), (vec!["numpy".to_owned(), "pandas".to_owned()], vec!["openai".to_owned()]));
        assert_eq!(vcpkg(r#"{"name":"app","dependencies":["fmt",{"name":"boost-asio","features":[]}]}"#), vec!["fmt", "boost-asio"]);
        assert_eq!(conanfile("[requires]\nzlib/1.3\nfmt/10.2.1\n\n[generators]\nCMakeDeps\n"), vec!["zlib", "fmt"]);
        assert_eq!(toml_keys("name = \"App\"\n[deps]\nJSON = \"682c06a0\"\nHTTP = \"cd3eb016\"\n[compat]\njulia = \"1.10\"\n", &["deps"]), vec!["JSON", "HTTP"]);
        assert_eq!(r_description("Package: monpaquet\nImports:\n    dplyr,\n    ggplot2 (>= 3.4)\nDepends: R (>= 4.1), shiny\n"), vec!["dplyr", "ggplot2", "shiny"]);
        // Tout ensemble, dans un dossier : chaque gestionnaire a les siens.
        let dir = std::env::temp_dir().join(format!("atelier-paquets-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("Gemfile"), "gem 'rails'\n").unwrap();
        std::fs::write(dir.join("App.csproj"), "<PackageReference Include=\"Serilog\" />").unwrap();
        std::fs::write(dir.join("go.mod"), "require github.com/gin-gonic/gin v1\n").unwrap();
        let found = of_folders(&[dir.display().to_string()]);
        let managers: Vec<Manager> = found.iter().map(|p| p.manager).collect();
        assert!(managers.contains(&Manager::RubyGems) && managers.contains(&Manager::NuGet) && managers.contains(&Manager::Go), "{found:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn images_docker() {
        let df = "FROM --platform=linux/amd64 node:20-alpine AS build\nRUN npm ci\nFROM build AS test\nFROM docker.io/library/nginx:1.27@sha256:abc\nFROM scratch\n";
        assert_eq!(dockerfile(df), vec!["node", "nginx"], "sans les étapes ni scratch");
        let yml = "services:\n  db:\n    image: postgres:16\n  cache:\n    image: \"bitnami/redis:7.2\"\n  app:\n    image: ghcr.io/moi/app:latest # mon image\n  web:\n    image: ${WEB_IMAGE}\n  proxy:\n    image: localhost:5000/proxy:1\n";
        assert_eq!(compose(yml), vec!["postgres", "bitnami/redis", "ghcr.io/moi/app", "localhost:5000/proxy"]);
        let dir = std::env::temp_dir().join(format!("atelier-docker-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("Dockerfile.dev"), "FROM python:3.12-slim\n").unwrap();
        std::fs::write(dir.join("docker-compose.override.yml"), "services:\n  q:\n    image: rabbitmq:3\n").unwrap();
        let found: Vec<String> = of_folders(&[dir.display().to_string()]).into_iter().filter(|p| p.manager == Manager::Docker).map(|p| p.name).collect();
        assert_eq!(found, vec!["python", "rabbitmq"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn recherche_par_paquet() {
        let pk = |n: &str| Package { manager: Manager::Npm, name: n.into() };
        let words = |q: &str| q.split_whitespace().map(str::to_owned).collect::<Vec<_>>();
        let pkgs = [pk("expo-router"), pk("react")];
        assert_eq!(match_score(&words("react"), &pkgs), 1.0);
        assert_eq!(match_score(&words("expo"), &pkgs), 0.9, "un paquet de la famille");
        assert_eq!(match_score(&words("expo router"), &pkgs), 1.0, "« expo router » = expo-router");
        assert_eq!(match_score(&words("exp"), &pkgs), 0.0, "pas un début de mot");
    }
}
