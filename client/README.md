# Client Godot d'Evonisium

Le client de l'étape 3 : le globe de l'Atlas naturaliste et les écrans du premier jouable. Il est écrit en GDScript (dossier `client/`) au-dessus d'une extension native en Rust, la crate `evo-godot`, qui fait tourner le moteur dans son propre fil et ne lui parle que par la file d'ordres et le canal d'observation.

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
| `scripts/globe.gd` | Le globe (incréments G1 et G2) : maillage, textures de données, interpolation entre deux pas, glissement des plaques, caméra en orbite, survol et choix d'une cellule, zone d'intérêt envoyée au moteur. |
| `shaders/globe_atlas.gdshader` | Le rendu Atlas du globe : lavis de la mer par passes de profondeur, isobathes, terres à l'aquarelle, trait de côte à l'encre, graticule, hachures de nuit, calques de données en lavis. |
| `shaders/atmosphere.gdshader` | Limbe de l'atmosphère (couleur tirée de sa composition) et contour du globe en double filet. |
| `scripts/screens/` | Accueil, création de la planète, ensemencement, partie, chargement. |
| `scripts/ui/` | Barre du temps, frise, calques et légende, inspecteur et loupe, fiche d'espèce, arbre du vivant, chronique et règles d'arrêt, alertes, interventions, sauvegarde, réglages, narrateur. |
| `tests/porte.gd` | Scénario de la porte de l'étape 3 (voir plus bas). |

## Commandes

| Touche | Effet |
|---|---|
| Espace | Pause / lecture |
| 1 à 5 | Crans de vitesse : 1 ka/s, 10 ka/s, 100 ka/s, 1 Ma/s, 10 Ma/s |
| N | Aller au prochain événement notable |
| Alt+1 à Alt+8, Alt+0 | Calques ; vue naturelle |
| C, T, I | Chronique, arbre du vivant, interventions |
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
