# Client Godot d'Evonisium

Le client de jeu : le globe de l'Atlas naturaliste et les écrans du premier jouable (étape 3), les outils de naturaliste de l'étape 4 et la descente au sol de l'étape 5. Il est écrit en GDScript (dossier `client/`) au-dessus d'une extension native en Rust, la crate `evo-godot`, qui fait tourner le moteur dans son propre fil et ne lui parle que par la file d'ordres et le canal d'observation.

## Lancer le jeu

Il faut Rust (stable) et Godot 4.6.1 (version standard, pas .NET).

```sh
# 1. L'extension native, toujours en mode optimisé (le moteur non optimisé est trop lent)
cargo build --release -p evo-godot

# 2. Le jeu
godot --path client
```

Sous Windows, même chose dans un terminal : `cargo build --release -p evo-godot`, puis ouvrir `client/project.godot` dans Godot ou lancer `Godot_v4.6.1-stable_win64.exe --path client`. Le fichier `client/evonisium.gdextension` va chercher la bibliothèque dans `target/release/` (`libevo_godot.so`, `evo_godot.dll` ou `libevo_godot.dylib`).

La première ouverture dans l'éditeur importe les polices et l'icône ; aucune autre étape d'import n'est nécessaire.

## Ce que contient le client

| Fichier | Rôle |
|---|---|
| `scripts/app.gd` | État global (autoload `App`) : la session vers le moteur, les réglages, les textes en français et en anglais, le passage d'un écran à l'autre, la liste des sauvegardes. |
| `scripts/atlas.gd` | Direction artistique (autoload `Atlas`) : couleurs, polices, thème des cartouches et des boutons. |
| `scripts/main.gd` | Scène principale : le globe en 3D, et l'écran courant par-dessus. |
| `scripts/globe.gd` | Le globe (incréments G1 à G3) : maillage, textures de données, interpolation entre deux pas, glissement des plaques, caméra en orbite (redressée vers l'horizon près du sol), survol et choix d'une cellule, zone d'intérêt envoyée au moteur ; aux bandes Z3 et Z4, région subdivisée autour du point regardé ; barrières et poussées climatiques posées par le joueur ; écart « avec et sans ». |
| `shaders/globe_atlas.gdshader` | Le rendu Atlas du globe : lavis de la mer par passes de profondeur, isobathes, terres à l'aquarelle, trait de côte à l'encre, graticule, hachures de nuit, calques de données en lavis. |
| `shaders/contour_encre.gdshader` | Contour à l'encre en post-traitement (sauts de profondeur et de normale), qui monte avec le zoom. |
| `shaders/corps_atlas.gdshader`, `corps_contour.gdshader`, `organe.gdshader` | Corps du vivant (palier 2) : lavis du pigment, hachures dans l'ombre, irisation, contour à la plume, voile de la vue anatomie, organes colorés par appareil. |
| `shaders/atmosphere.gdshader` | Limbe de l'atmosphère (couleur tirée de sa composition) et contour du globe en double filet. |
| `scripts/screens/` | Accueil, création de la planète, ensemencement, partie, chargement. |
| `scripts/ui/` | Barre du temps, frise, calques et légende, inspecteur et loupe, fiche d'espèce, arbre du vivant, chronique et règles d'arrêt, alertes, interventions, sauvegarde, réglages, narrateur ; à l'étape 4 : « avec et sans » (`avec_sans.gd`), réseau trophique (`reseau.gd`), colonne stratigraphique (`strates.gd`), portrait 3D (`portrait.gd`), anatomie (`anatomie.gd`), comparateur (`comparateur.gd`). |
| `tests/porte.gd`, `tests/porte4.gd` | Scénarios des portes des étapes 3 et 4 (voir plus bas). |

## Commandes

| Touche | Effet |
|---|---|
| Espace | Pause / lecture |
| 1 à 5 | Crans de vitesse : 1 ka/s, 10 ka/s, 100 ka/s, 1 Ma/s, 10 Ma/s |
| N | Aller au prochain événement notable |
| Alt+1 à Alt+8, Alt+0 | Calques ; vue naturelle |
| C, T, I, A | Chronique, arbre du vivant, interventions, « avec et sans » |
| F5 | Sauvegarder |
| Glisser, molette, flèches, + et − | Tourner et zoomer le globe ; double clic : s'approcher d'un lieu |
| Échap | Fermer la fiche ouverte |

## La porte de l'étape 3, côté client

```sh
# Partie jouée, avec rendu : créer la planète, ensemencer, intervenir, voir l'oxygène monter
godot --path client --resolution 1600x900 -- --porte --niveau=4 --sortie=/tmp/porte

# Rejeu, sans affichage : deux chemins de caméra et un point de sauvegarde rechargé
godot --headless --path client -- --porte --rejeu --niveau=4 --pas=40 --sortie=/tmp/rejeu
```

Le scénario écrit `rapport.json` (images par seconde, durée d'une image, temps passé dans le pont, dates, empreintes d'état) et, pour la partie jouée, des captures de chaque écran. Le code de sortie vaut 0 si la porte est franchie. L'intégration continue passe les deux scénarios sous Linux (rendu Vulkan logiciel lavapipe sous Xvfb) et le rejeu sous Windows, puis compare les empreintes des deux systèmes.

## La porte de l'étape 4, côté client

```sh
# Impact, barrière, poussée climatique, « et sans l'impact ? », réseau
# trophique, strates, régions et paysages, anatomie, comparateur
godot --path client --resolution 1600x900 -- --porte4 --niveau=4 --sortie=/tmp/porte4

# Essai court du globe G3 seul : un massif vu en région puis en paysage
godot --path client --resolution 1600x900 -- --porte4 --seul=g3 --sortie=/tmp/g3
```

Le scénario écrit `rapport.json` et une capture par écran ; code de sortie 0 si la porte est franchie. Rapport complet : `docs/etape-4-client.md`.
