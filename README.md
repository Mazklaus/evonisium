# Evonisium

Simulateur de l'évolution du vivant sur une planète elle-même évolutive : des premières cellules jusqu'aux sociétés et à leurs sciences. La vie y émerge de règles physiques, chimiques et génétiques, sans scénario écrit.

Les documents de conception (vision et architecture, planète, génétique, organismes, intelligence et sociétés) décrivent le projet complet et sa feuille de route en huit étapes. Ce dépôt en contient le code.

## État : étape 2, planète vivante

La planète évolue : plaques qui dérivent, climat à l'équilibre avec glaciations, cycles du carbone, de l'oxygène, du phosphore, du fer et du soufre en réservoirs globaux, couplés à la vie microbienne par le registre de flux. Les cellules minimales déposées près des sources hydrothermales inventent la photosynthèse par évolution (pigments évalués sous le spectre de l'étoile, duplication, transfert horizontal, tunnel stochastique, accélérateur journalisé en dernier recours), et l'oxygène s'accumule sans script. Toute action extérieure passe par une file d'ordres datée ; une partie se rejoue à l'identique depuis sa graine et ses ordres. Le détail est dans [docs/etape-2.md](docs/etape-2.md), le rapport de la porte dans [docs/etape-2-porte.md](docs/etape-2-porte.md). L'étape 1 est décrite dans [docs/etape-1.md](docs/etape-1.md).

Le client de jeu Godot arrive à l'étape 3 ; le moteur de simulation reste indépendant de tout affichage.

## Organisation

| Crate | Chantier | Contenu |
|---|---|---|
| `evo-core` | socle commun | hasard à graine (un flux par système), horloges, événements enrichis (cause, origine, intérêt), registre de flux |
| `evo-planet` | Planète et environnement | paramètres de planète (six mondes de la vague 1), grille géodésique, tectonique, climat, cycles géochimiques en boîtes, pools chimiques |
| `evo-genetics` | Génétique et évolution | génome à domaines, mutations, Kimura, Wright-Fisher, Hardy-Weinberg, régime « apparition puis fixation », tunnel stochastique, transfert horizontal, journal des modifications, registre des lignées |
| `evo-life` | Organismes et écosystèmes | catalogue métabolique, phénotype, taux de croissance r et coefficient de sélection s, communautés microbiennes, pigments et spectre de l'étoile, chemin vers la photosynthèse |
| `evo-sim` | Vision et architecture | monde, file d'ordres et rejeu, évolution, historique, état publié, porte de l'étape 2, outil en ligne de commande, mesures |

## Utilisation

Il faut une chaîne Rust récente (`rustup`, édition 2021).

```sh
cargo test --workspace                                   # tests, dont la validation scientifique
cargo run --release -p evo-sim -- run --level 6 --steps 300
cargo run --release -p evo-sim -- porte --data docs/etape-2-porte --out docs/etape-2-porte.md
cargo run --release -p evo-sim -- bench --levels 6,7 --out docs/etape-1-mesures.md
```

Options de `porte` : `--worlds` (liste de mondes : terre, ocean, desert, super-terre, petite, sans-lune), `--level`, `--step-years`, `--max-years`, `--threshold` (fraction d'O₂), `--hold-years`, `--data` (historiques et événements en TSV).

Options de `run` : `--seed` (graine de la partie), `--level` (grille : 4 donne 2 562 cellules, 6 donne 40 962, 7 donne 163 842), `--steps` (pas planétaires), `--step-years` (durée d'un pas), `--every` (fréquence des bilans).

La même graine redonne exactement la même histoire, quel que soit le nombre de coeurs.
