# Evonisium

Simulateur de l'évolution du vivant sur une planète elle-même évolutive : des premières cellules jusqu'aux sociétés et à leurs sciences. La vie y émerge de règles physiques, chimiques et génétiques, sans scénario écrit.

Les documents de conception (vision et architecture, planète, génétique, organismes, intelligence et sociétés) décrivent le projet complet et sa feuille de route en huit étapes. Ce dépôt en contient le code.

## État : étape 4 terminée, étape 5 en cours

La feuille de route compte huit étapes ; les quatre premières sont faites.

- **Étape 1, socle** : grille géodésique, hasard à graine, génome et génétique des populations ([docs/etape-1.md](docs/etape-1.md)).
- **Étape 2, planète vivante** : tectonique, climat, cycles géochimiques couplés à la vie microbienne ; la photosynthèse s'invente par évolution et l'oxygène s'accumule sans script ([docs/etape-2.md](docs/etape-2.md)).
- **Étape 3, premier jouable** : moteur à la résolution normale (40 962 cellules), vie des terres et des lacs, client Godot avec le globe de l'Atlas naturaliste, sauvegardes, interventions par la réserve d'influence ; une partie se rejoue à l'identique sous Linux, Windows et macOS ([docs/etape-3.md](docs/etape-3.md), [docs/etape-3-client.md](docs/etape-3-client.md)).
- **Étape 4, des microbes aux premiers êtres complexes** : phagotrophie, endosymbiose et cellule eucaryote, sexe, colonies à plusieurs types cellulaires, plan de construction des corps, sortie des eaux ; côté client, rejeu « avec et sans », anatomie, comparateur, réseau trophique, colonne stratigraphique, décor de milieu. Porte franchie sur les six mondes de la vague 1, avec accélération déclarée ([docs/etape-4-moteur.md](docs/etape-4-moteur.md), [docs/etape-4-cellules.md](docs/etape-4-cellules.md), [docs/etape-4-client.md](docs/etape-4-client.md)).
- **Étape 5, animaux et écosystèmes** (en cours) : l'affichage est prêt (descente au sol, animaux animés, foules en imposteurs, [docs/etape-5-client.md](docs/etape-5-client.md)) mais peuplé de figurants d'essai tant que le moteur ne publie pas d'individus.

Toute action extérieure passe par une file d'ordres datée, et le curseur de vitesse ne règle que le rythme d'affichage : la même graine et les mêmes ordres redonnent exactement la même histoire, quel que soit le nombre de cœurs. Les rapports de porte de chaque étape sont dans [docs/](docs/).

## Organisation

| Crate | Chantier | Contenu |
|---|---|---|
| `evo-core` | socle commun | hasard à graine (un flux par système), horloges, événements enrichis (cause, origine, intérêt), registre de flux |
| `evo-planet` | Planète et environnement | paramètres de planète (six mondes de la vague 1), grille géodésique, tectonique, climat, cycles géochimiques en boîtes, pools chimiques |
| `evo-genetics` | Génétique et évolution | génome à domaines, mutations, Kimura, Wright-Fisher, régime « apparition puis fixation », tunnel stochastique, transfert horizontal, journal des modifications, registre des lignées |
| `evo-life` | Organismes et écosystèmes | catalogue métabolique, phénotype construit de façon incrémentale, populations, prédation, organites, colonies, plan de construction des corps |
| `evo-sim` | Vision et architecture | monde, file d'ordres et rejeu, évolution et grandes transitions, historique, influence, sauvegardes, portes des étapes, outil en ligne de commande `evonisium` |
| `evo-engine` | Vision et architecture | le moteur vu du client : fil de simulation, file d'ordres, canal d'observation, état publié, requêtes, points de sauvegarde |
| `evo-view` | Globe 3D et interface | logique d'affichage sans Godot : maillage du globe, textures de données, calques, chronique, arbre du vivant, décor de milieu, scène au sol |
| `evo-morph` | Rendu du vivant | apparence du vivant, pure et déterministe : microbes, corps complets, squelette et locomotion, imposteurs |
| `evo-godot` | client | pont GDExtension entre le moteur et Godot |

Le client de jeu (GDScript, Godot 4.6.1) est dans [client/](client/), décrit dans [client/README.md](client/README.md). Le moteur reste indépendant de tout affichage.

## Utilisation

Il faut une chaîne Rust stable récente (`rustup`) et, pour jouer, Godot 4.6.1 (version standard, pas .NET).

```sh
# Jouer
cargo build --release -p evo-godot     # l'extension native, toujours en mode optimisé
godot --path client

# Moteur seul, sans affichage
cargo test --workspace                 # tests, dont la validation scientifique
cargo run --release -p evo-sim -- run --world terre --level 6 --steps 300
cargo run --release -p evo-sim -- porte --level 6 --out porte.md --reprise reprise/
cargo run --release -p evo-sim -- empreinte --world terre --seed 2026 --level 4 --steps 40
```

Options de `run` : `--world` (terre, ocean, desert, super-terre, petite, sans-lune), `--seed` (graine de la partie), `--level` (grille : 4 donne 2 562 cellules, 6 donne 40 962, 7 donne 163 842), `--steps` (pas planétaires), `--step-years` (durée d'un pas), `--every` (fréquence des bilans), `--out`.

Options de `porte` : `--worlds`, `--seeds`, `--level`, `--max-years`, `--out` (rapport), `--data` (historiques et événements en TSV), `--reprise DOSSIER` (points de reprise tous les 50 pas : une partie interrompue reprend de là). Une porte au niveau 6 dure de 30 minutes à près de 2 heures par monde.

Autres sous-commandes : `empreinte` (empreinte d'état pour comparer les rejeux), `complexite` (suivi des grandes transitions), `bench` (mesures de vitesse), `chrono` (chronométrage par poste depuis un point de sauvegarde), `equivalence` (même partie à deux durées de pas).

## Contribuer

Les signalements de bugs et les idées sont bienvenus dans les issues : voir [CONTRIBUTING.md](CONTRIBUTING.md), le [code de conduite](CODE_OF_CONDUCT.md) et la [politique de sécurité](SECURITY.md).

## Licence

Tous droits réservés : voir [LICENSE](LICENSE). Les polices de `client/fonts/` restent sous la SIL Open Font License 1.1.
