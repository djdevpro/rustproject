# Dossier de presse de Rust Project

*Mis à jour le 7 novembre 2026 · version 0.1.9*

> Rust Project est un projet indépendant, sans lien avec la Rust Foundation ni avec le projet du langage Rust.

## En bref

**En une phrase.** Rust Project est une application de bureau gratuite et open source qui retrouve les projets personnels éparpillés sur un ordinateur. Une IA, locale ou agent, les trie. L'application les range ensuite sur un bureau, avec leurs prochaines étapes et des boutons pour les rouvrir dans VS Code, un terminal ou Claude Code.

**En trente secondes.** Quiconque code pour le plaisir finit avec des dizaines de projets commencés, répartis entre des dossiers aux noms flous et GitHub. Rust Project scanne les dossiers qu'on lui indique et reconnaît les projets. Une IA décrit chacun en une phrase et le classe par domaine. L'application les présente ensuite sur un bureau : les projets du moment en haut, la prochaine action de chacun, et un clic pour rouvrir le dossier dans son éditeur. L'IA peut tourner entièrement sur la machine avec Ollama, ou passer par un agent comme Claude Code grâce au protocole MCP. Il n'y a ni compte ni cloud. L'application existe en dix langues, pour Windows, macOS et Linux.

## Fiche d'identité

| | |
|---|---|
| Nom | Rust Project (nom de travail précédent : Atelier) |
| Catégorie | Application de bureau pour gérer ses projets personnels de développement |
| Systèmes | Windows (x64), macOS (binaire universel Intel et Apple Silicon), Linux (x64) |
| Prix | Gratuit |
| Licence | Open source, MIT ou Apache 2.0 au choix |
| Technologies | Rust (édition 2024), interface egui 0.37 |
| Langues | 10 : français, anglais, espagnol, allemand, portugais, italien, russe, chinois, japonais, coréen. L'interface suit la langue du système |
| IA | Facultative : modèle local via Ollama, agent IA via MCP (Claude Code…), ou règles sans IA |
| Données | Un fichier JSON sur l'ordinateur, dans le dossier `.rust-project` du compte utilisateur. Pas de compte, pas de serveur |
| Version | 0.1.9, du 7 novembre 2026. Première version publique (0.1.0) le 6 novembre 2026 |
| Taille | 26 Mo (Windows), 21 Mo (macOS, archive zip), 42 Mo (Linux) |
| Code source | https://github.com/djdevpro/rustproject |
| Téléchargements | https://github.com/djdevpro/rustproject/releases |
| Auteur | [Prénom Nom], [ville] |

## Le problème

Développeurs, étudiants et bidouilleurs accumulent les projets : une idée lancée un week-end, un prototype, un cours suivi à moitié, un outil maison. Ces projets dorment dans des dossiers aux noms approximatifs, sur plusieurs disques ou seulement sur GitHub. On oublie ce qu'ils faisaient, où on s'était arrêté et ce qu'il restait à faire.

Les gestionnaires de tâches n'y changent rien : il faudrait tout y saisir à la main, puis le tenir à jour.

## Ce que fait Rust Project

### Découvrir

- **Dossiers sources.** L'application explore les dossiers que vous choisissez, autant que vous voulez, chacun sur un à trois niveaux de profondeur. Elle saute les dossiers cachés et les exclusions comme `node_modules` ou `target`.
- **Marqueurs.** Elle reconnaît un projet à un dépôt `.git`, à un manifeste (`Cargo.toml`, `package.json`…), à un `README.md` ou à des consignes pour agents IA (`CLAUDE.md`, `AGENTS.md`).
- **Briques.** `frontend`, `api`, `db`, `site` ou `packages/x` sont rattachés à leur projet parent au lieu de devenir des fiches à part.
- **GitHub.** Elle reprend vos dépôts via l'outil `gh` : les vôtres, ceux de vos organisations et ceux de vos collaborations. Un dépôt sans copie locale devient une fiche « GitHub ».
- **Dossiers écartés.** Un clic écarte une copie ou une sauvegarde, que le scan ignore ensuite.
- **Vitesse.** Environ 400 dossiers sont analysés en deux secondes sur la machine de développement.

### Ranger

- **Une fiche par projet** : description d'une phrase, domaine, type, technologies, objectif, dernier point d'étape, prochaine action, étapes à cocher, notes, ressources, tags et logo.
- **Des statuts** : Focus, Au fil de l'eau, En réserve, Archivé.
- **Mon bureau** met en avant les projets du moment, avec leur progression et leur prochaine action.
- **La bibliothèque** présente tous les projets en carte, en grille ou en liste, avec un filtre par domaine.
- **La carte** est un graphe vivant :
  - les domaines sont des moyeux lumineux, et les projets gravitent autour ;
  - au survol, des particules courent sur les liens ;
  - un clic sur un domaine le filtre, et la caméra glisse jusqu'à lui ;
  - on zoome à la molette ;
  - on fait glisser un projet sur un statut pour le classer.
- **L'ontologie** organise le classement en facettes (domaine, nature, finalité…) et en concepts. Chaque concept a une définition, des synonymes et un parent. L'IA peut proposer une ontologie, appliquée seulement avec votre accord. Les concepts retouchés à la main sont conservés.
- **La recherche** combine trois approches :
  - floue : « imo » trouve ImmoFormation ;
  - par le sens : « logement » trouve ImmoFormation, avec la mention « ≈ sens proche » ;
  - par l'ontologie : « logement » mène au concept Immobilier, puis aux projets qui y sont rattachés.

### Rouvrir

- **Des lanceurs.** Des tuiles ouvrent le dossier du projet dans Claude Code, VS Code, Cursor, un IDE JetBrains, un terminal ou l'explorateur de fichiers (le Finder sur Mac). Un clic ou un raccourci clavier suffit, et on peut ajouter n'importe quelle commande.
- **Claude Code.** Le lanceur ouvre une nouvelle conversation dans Claude Desktop, sur le dossier du projet, déjà remplie avec les étapes qui restent à faire.

## L'IA, à la carte

| Mode | Fonctionnement | Où vont les données |
|---|---|---|
| **Ollama (IA locale)** | Chaque dossier est résumé et envoyé au modèle choisi, qui répond en JSON contraint par un schéma. Modèles conseillés : `qwen2.5:3b` (≈ 1,9 Go) pour le classement, `qwen3-embedding:4b` pour la recherche par le sens | Nulle part, si Ollama tourne sur la machine |
| **Agent IA via MCP** | Claude Code, ou tout client MCP, lance `rust-project --mcp`. Le serveur expose 26 outils (scanner, créer et fusionner des fiches, cocher une étape…) et cinq commandes prêtes, dont `ranger_mes_projets` et `nettoyer_mes_fiches` | Vers le service de l'agent choisi, par exemple Anthropic pour Claude Code |
| **Sans IA** | Un dossier qui porte un marqueur devient un projet. Sa description vient de son manifeste ou de la première phrase de son README | Nulle part |

**Garde-fous**

- C'est vous qui choisissez les dossiers à explorer : l'agent IA ne peut pas en ajouter.
- L'ontologie proposée par l'IA attend votre accord.
- Supprimer une fiche ne supprime jamais le dossier.
- Ce que l'agent écrit apparaît en direct dans l'interface.

## Sous le capot (pour les médias techniques)

- **Rust et egui.** Un exécutable natif par système, sans navigateur embarqué.
- **Près de 20 000 lignes de Rust.** La simulation de forces de la carte est écrite à la main, dans l'esprit de d3-force.
- **Tâches de fond.** Le scan, le classement, le téléchargement des modèles et le calcul des embeddings ne figent jamais l'interface.
- **Serveur MCP intégré.** Il passe par JSON-RPC sur l'entrée et la sortie standard. Dès la poignée de main, il transmet ses consignes à l'agent : une fiche par projet, jamais pour une brique, une description d'une phrase.
- **Embeddings.** Ils sont demandés avec un contexte de 2 048 jetons. Ainsi, `qwen3-embedding:4b` prend environ 4 Go de mémoire graphique au lieu d'environ 10 Go.
- **Données.** Un fichier JSON écrit de façon atomique (fichier temporaire puis renommage), que l'interface relit chaque seconde.
- **Mises à jour.** Elles viennent des releases GitHub, sont vérifiées au démarrage et contrôlées par SHA-256.
- **Tests d'interface automatisés.** Ils utilisent egui_kittest et tournent sans GPU. Ils ne touchent jamais les vraies données et ne lancent aucun programme.
- **Intégration continue.** GitHub Actions produit à chaque release les binaires Windows, macOS (universel) et Linux.

## Chiffres clés

| Chiffre | Ce qu'il mesure |
|---|---|
| 0 € | Prix : sans abonnement ni compte |
| 3 | Systèmes : Windows, macOS, Linux |
| 10 | Langues de l'interface |
| ≈ 20 000 | Lignes de Rust |
| 26 | Outils MCP proposés aux agents IA |
| ≈ 2 s | Durée d'analyse d'environ 400 dossiers |
| 1 | Phrase pour décrire chaque projet |
| 10 | Versions publiées les 6 et 7 novembre 2026 (de 0.1.0 à 0.1.9) |

## Visuels

Les visuels peuvent être utilisés librement pour illustrer un article sur Rust Project. Crédit : « Rust Project ».

| Fichier | Contenu | Format |
|---|---|---|
| [`docs/banner.png`](https://raw.githubusercontent.com/djdevpro/rustproject/main/docs/banner.png) | Bannière : logo, slogan « All your side projects in one place. » et capture du bureau | 2000 × 667 |
| [`docs/bureau.png`](https://raw.githubusercontent.com/djdevpro/rustproject/main/docs/bureau.png) | « Mon bureau » : projets du moment, liste des projets en cours, fiche ouverte avec ses lanceurs | 1480 × 920 |
| [`docs/carte.png`](https://raw.githubusercontent.com/djdevpro/rustproject/main/docs/carte.png) | Bibliothèque, vue Carte : domaines proposés par l'IA, projets en orbite | 1480 × 920 |
| [`docs/grille.png`](https://raw.githubusercontent.com/djdevpro/rustproject/main/docs/grille.png) | Bibliothèque, vue Grille, avec une fiche ouverte | 1480 × 920 |
| [`assets/icon.svg`](https://raw.githubusercontent.com/djdevpro/rustproject/main/assets/icon.svg), [`assets/icon.png`](https://raw.githubusercontent.com/djdevpro/rustproject/main/assets/icon.png) | Logo, en vectoriel et en 256 × 256 | SVG, PNG |

Les captures montrent des projets fictifs, avec l'interface en anglais.

## Essayer en deux minutes

1. **Télécharger** le fichier de son système sur https://github.com/djdevpro/rustproject/releases :
   - Windows : `rust-project-0.1.9-windows-x64.exe`
   - macOS : `rust-project-0.1.9-macos-universal.zip`, qui contient l'application
   - Linux : `rust-project-0.1.9-linux-x64`, à rendre exécutable avec `chmod +x`
2. **Passer l'avertissement du système** au premier lancement. Les binaires ne sont pas signés par un certificat d'éditeur.
   - Windows : SmartScreen affiche « Windows a protégé votre ordinateur ». Cliquez sur « Informations complémentaires », puis sur « Exécuter quand même ».
   - macOS : le premier lancement est bloqué. Ouvrez Réglages Système › Confidentialité et sécurité, puis cliquez sur « Ouvrir quand même ».
3. **Essayer sans ses propres dossiers.** Ouvrez Configuration › Données, puis « Afficher les données de démonstration ».
4. **Aller plus loin.** Installez Ollama et choisissez un modèle dans Configuration › IA. Vous pouvez aussi connecter Claude Code :

   ```bash
   claude mcp add --scope user rust-project -- "<chemin vers l'exécutable>" --mcp
   ```

## État du projet

- **Version 0.1.** Le projet est jeune et évolue vite. Les retours passent par les issues GitHub.
- **Binaires non signés.** D'où les avertissements décrits plus haut.
- **Prérequis facultatifs.** La recherche par le sens demande un modèle d'embeddings dans Ollama. L'import des dépôts GitHub demande l'outil `gh`.

## À propos de l'auteur

[Trois ou quatre phrases à écrire : qui vous êtes, ce que vous faites dans la vie, pourquoi et comment vous avez écrit Rust Project. Les journalistes reprennent souvent ce passage tel quel. Prévoir aussi une photo portrait, souvent demandée.]

## Questions fréquentes

**Est-ce lié au langage Rust ou à la Rust Foundation ?**
Non. Rust Project est écrit en Rust, mais c'est un projet indépendant, ni affilié à la Rust Foundation ni soutenu par elle.

**C'est vraiment gratuit ?**
Oui. Il n'y a ni publicité ni compte, et le code est libre (MIT ou Apache 2.0). [Modèle économique éventuel à préciser, ou « aucun : c'est un projet personnel ».]

**Mes dossiers partent-ils sur Internet ?**
Les fiches restent dans un fichier sur l'ordinateur. Avec Ollama installé en local, rien n'est envoyé. Avec un agent IA comme Claude Code, ce que l'agent lit passe par son service. L'application ne se connecte elle-même à Internet que dans trois cas :

- pour vérifier les mises à jour sur GitHub ;
- pour télécharger un modèle Ollama, si vous le demandez ;
- pour lister vos dépôts via `gh`.

**Faut-il une carte graphique ?**
Non. Sans IA, l'application fonctionne avec des règles simples. Avec Ollama, le modèle conseillé pour le classement pèse environ 1,9 Go. Le modèle de recherche par le sens demande environ 4 Go de mémoire graphique.

**En quoi est-ce différent de Notion, Trello ou GitHub Projects ?**
Ces outils demandent de tout saisir et de tout tenir à jour à la main. Rust Project part du disque : il trouve les projets lui-même, les décrit et les rouvre dans les outils du développeur.

**Pourquoi Rust ?**
Pour avoir une application native et rapide, livrée en un seul exécutable par système, sans navigateur embarqué. [À compléter avec votre propre raison, par exemple l'envie d'apprendre le langage.]

**Qu'est-ce que MCP ?**
Le Model Context Protocol est un protocole ouvert qui permet aux agents IA d'utiliser des outils externes. Rust Project en expose 26 : un agent comme Claude Code peut ainsi ranger vos projets à votre place.

**Quelle est la suite ?**
[Feuille de route à préciser : signature des binaires, nouvelles intégrations, etc.]

## Contact

[Prénom Nom] · [e-mail] · [téléphone, facultatif] · [profil LinkedIn ou Bluesky]

https://github.com/djdevpro/rustproject
