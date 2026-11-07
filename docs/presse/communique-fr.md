# Communiqué de presse

[Ville], le [jour] novembre 2026

## Rust Project, l'application gratuite qui retrouve vos projets perso oubliés et les remet sur votre bureau

**Une application de bureau open source pour Windows, macOS et Linux. Elle repère les projets éparpillés sur un ordinateur et les fait décrire et classer par une IA, en local avec Ollama ou par un agent comme Claude Code. Elle les range ensuite avec leurs prochaines étapes. Ni compte, ni cloud.**

Un dossier « test-v2 », un autre « nouveau-projet-final », un dépôt GitHub plus ouvert depuis l'été : à force de coder pour le plaisir, on sème ses projets un peu partout et on ne sait plus où on en est. Rust Project sert à les retrouver et à savoir sur lequel reprendre. C'est un projet personnel développé en France par [Prénom Nom].

**Retrouver.** On indique à l'application les dossiers à explorer, et elle les parcourt jusqu'à trois niveaux de profondeur. Elle reconnaît les projets à leurs marqueurs :

- un dépôt Git ;
- un fichier `Cargo.toml` ou `package.json` ;
- un README ;
- des consignes pour agents IA.

Les morceaux d'un même projet (`frontend`, `api`, `db`…) restent attachés à leur parent au lieu de devenir des fiches à part. Avec l'outil en ligne de commande de GitHub, les dépôts sans copie locale sont repris aussi. Sur la machine de développement, environ 400 dossiers sont analysés en deux secondes.

**Ranger.** Chaque projet devient une fiche : une description d'une phrase, un domaine, des technologies, un objectif, des étapes et des notes. « Mon bureau » met en avant les projets du moment et la prochaine action de chacun. La bibliothèque montre tous les autres en grille, en liste ou sur une carte vivante, un graphe où les projets gravitent autour de leurs domaines. La recherche tolère les fautes de frappe et comprend le sens des mots : « logement » retrouve un projet de formation immobilière.

**Rouvrir.** Un clic ou un raccourci clavier ouvre le dossier du projet dans VS Code, Cursor, un IDE JetBrains, un terminal ou Claude Code. Avec Claude Code, la nouvelle conversation démarre déjà remplie avec les étapes qui restent à faire.

**L'IA, au choix.** Pour trier et décrire les projets, Rust Project s'appuie sur l'une de ces trois options :

- **Un modèle qui tourne sur l'ordinateur grâce à Ollama.** Les projets ne partent alors vers aucun service en ligne.
- **Un agent IA comme Claude Code**, grâce à un serveur MCP intégré qui compte 26 outils et cinq commandes prêtes à l'emploi.
- **De simples règles, sans IA.**

Les dossiers à explorer restent le choix de l'utilisateur : l'agent ne peut pas en ajouter. Supprimer une fiche ne touche jamais aux fichiers.

> « [Citation à écrire avec vos mots. Piste : J'avais des dizaines de projets commencés, répartis entre plusieurs disques et GitHub, et je ne savais plus où j'en étais. Je voulais un endroit qui me dise sur quoi reprendre, sans envoyer mes dossiers dans le cloud.] »
> — [Prénom Nom], à l'origine de Rust Project

**Disponibilité.** Rust Project est disponible gratuitement dès aujourd'hui pour Windows (x64), macOS (Intel et Apple Silicon) et Linux (x64), sur https://github.com/djdevpro/rustproject/releases. Il se met ensuite à jour tout seul. Son code, écrit en Rust, est publié sous licence MIT ou Apache 2.0, au choix. L'interface existe en dix langues, dont le français.

**Pour l'essayer sans montrer vos propres dossiers :** ouvrez Configuration › Données, puis « Afficher les données de démonstration ». Des projets fictifs s'affichent, et rien n'est enregistré.

### À propos de Rust Project

Rust Project est un logiciel libre développé par [Prénom Nom], [une phrase : métier, ville, parcours]. Sa première version publique est sortie le 6 novembre 2026. C'est un projet indépendant, sans lien avec la Rust Foundation ni avec le projet du langage Rust.

### Contact presse

[Prénom Nom] · [e-mail] · [téléphone, facultatif]

Code source et téléchargements : https://github.com/djdevpro/rustproject

Visuels en haute définition, dossier de presse et démo en visio sur simple demande.
