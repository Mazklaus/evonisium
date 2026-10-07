# Evonisium

Simulateur de l'évolution du vivant sur une planète elle-même évolutive : des premières cellules jusqu'aux sociétés et à leurs sciences. La vie y émerge de règles physiques, chimiques et génétiques, sans scénario écrit.

Les documents de conception (vision et architecture, planète, génétique, organismes, intelligence et sociétés) décrivent le projet complet et sa feuille de route en huit étapes. Ce dépôt en contient le code.

## État : étape 1, prototype scientifique sans affichage

Une planète fixe (grille géodésique de 40 962 cellules, climat moyen, chimie de l'eau de surface) où des cellules minimales sont déposées près des sources hydrothermales. Elles se répandent, s'adaptent à la température locale et se diversifient en guildes métaboliques, par mutation et sélection en régime « apparition puis fixation ». Le détail, les simplifications et les résultats sont dans [docs/etape-1.md](docs/etape-1.md), les mesures de calcul dans [docs/etape-1-mesures.md](docs/etape-1-mesures.md).

Le client de jeu Godot arrive à l'étape 3 ; le moteur de simulation reste indépendant de tout affichage.

## Organisation

| Crate | Chantier | Contenu |
|---|---|---|
| `evo-core` | socle commun | hasard à graine (un flux par système), horloges, événements, registre de flux |
| `evo-planet` | Planète et environnement | paramètres de planète (la Terre est un préréglage), grille géodésique, vecteur d'environnement, liste des pools chimiques |
| `evo-genetics` | Génétique et évolution | génome à domaines, mutations, Kimura, Wright-Fisher, Hardy-Weinberg, régime « apparition puis fixation », registre des lignées |
| `evo-life` | Organismes et écosystèmes | catalogue métabolique, phénotype, taux de croissance r et coefficient de sélection s, communautés microbiennes |
| `evo-sim` | Vision et architecture | monde, ordonnancement des phases, outil en ligne de commande, mesures |

## Utilisation

Il faut une chaîne Rust récente (`rustup`, édition 2021).

```sh
cargo test --workspace                                   # tests, dont la validation scientifique
cargo run --release -p evo-sim -- run --level 6 --steps 300
cargo run --release -p evo-sim -- bench --levels 6,7 --out docs/etape-1-mesures.md
```

Options de `run` : `--seed` (graine de la partie), `--level` (grille : 4 donne 2 562 cellules, 6 donne 40 962, 7 donne 163 842), `--steps` (pas planétaires), `--step-years` (durée d'un pas), `--every` (fréquence des bilans).

La même graine redonne exactement la même histoire, quel que soit le nombre de coeurs.
