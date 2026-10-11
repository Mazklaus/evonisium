# Son, livraison 1 : l'ère microbienne

Conception : doc « Evonisium : design sonore » (choix de l'utilisateur du 10 octobre 2026 : musique C + B + A entièrement générée) et paragraphe « Son » du doc d'architecture (décision de Vision du 10 octobre 2026). Banc d'écoute des esquisses : https://claude.ai/artifact/NuBtUYchmXps1Jxt44akrK

## Ce qui sonne

| Vue | Ce qu'on entend | Ce qui le pilote |
| --- | --- | --- |
| Globe | Musique des ères microbiennes ; rafales de vent rares, orages au loin | Lignées vivantes, température, O₂, glace |
| Loupe de l'inspecteur | Monde liquide : bulles, cliquetis des flagelles, battement des divisions ; musique en retrait | Lignées vivantes |
| Au sol | Musique et vent, en attendant les voix des espèces (livraison 2) | — |
| Menus | Musique seule, plus douce | — |
| Interface | Plume à l'ouverture d'une fiche, page tournée au changement d'écran, tampon quand un point de sauvegarde est écrit | — |

Musique (C + B + A) : l'ère microbienne n'a pas encore de mélodie, seulement une nappe grave, des cloches de verre, des bulles et de rares phrases de kalimba.

- plus de lignées vivantes, plus de voix ;
- plus chaud, mode plus lumineux (du phrygien au lydien) et registre plus haut ;
- plus d'oxygène, timbre plus clair et moins de bulles ;
- des repos de 25 à 50 s, après des phrases de 50 à 90 s, où ne reste que la nappe : le son ne doit jamais être bruyant en continu sur une partie de 20 à 60 heures ;
- les moments forts de la chronique (score d'intérêt ≥ 0,5) ont un motif : jalon ou innovation (accord de cloches), catastrophe (cloche froide), extinction (coup grave). Un motif au plus toutes les 8 s.

Le vent ne fait jamais de fond continu : des rafales de 5 à 12 s, séparées de 30 à 150 s de calme, plus rares quand le climat est froid ou pris par la glace (retour d'écoute de l'utilisateur du 10 octobre 2026 : vent et ressac trop présents). Les orages sont rares et tombent plutôt pendant les rafales.

En pause, la musique se retire de moitié. **Ctrl+M** coupe ou rend le son. Les volumes (général, musique, ambiances, interface, narrateur) se règlent dans l'écran des réglages, s'entendent pendant le glissement et sont gardés dans `user://son.cfg`.

## Narrateur vocal

Choix de l'utilisateur du 10 et du 11 octobre 2026 : une voix masculine de synthèse, « gilles » (Piper, domaine public), qui raconte comme un documentaire et ne parle qu'aux moments clés ; textes écrits par gabarits (`crates/evo-son/src/recit.rs`).

- **Moments clés** (activés par défaut) : vie déposée, étapes de la photosynthèse et de la cellule complexe, rhodopsine, seuils d'oxygène, glaciations globales. Seulement les événements de score d'intérêt ≥ 0,75, au plus un toutes les 150 s, jamais par-dessus une autre narration. La musique et les ambiances s'effacent pendant qu'il parle.
- **Scène** (option) : quand la loupe s'ouvre sur une cellule, le lieu, la température et les espèces présentes, de la plus abondante à la plus discrète.
- **Fiche** (option) : quand une fiche d'espèce s'ouvre, son histoire (âge, origine, espèce mère), ses traits, son aire et sa tendance, puis une anecdote inventée là où elle est la plus abondante, tirée du milieu réel (source chaude, banquise, tapis de surface, grand fond) et de ses voisines.
- Sous-titres en bas de l'écran (option). Le narrateur ne parle qu'en français pour l'instant.

La voix n'est pas dans le dépôt : un bouton des réglages télécharge depuis GitHub le moteur sherpa-onnx v1.12.14 (Apache 2.0, 27 Mo) et la voix (67 Mo), puis les déplie avec `tar` dans `user://voix/`. Le jeu lance le moteur phrase par phrase sur un fil à part, en avance sur la lecture ; sans voix installée, rien n'est dit. Le guide du client peut faire parler le narrateur par l'autoload `Son` : `dire(texte, priorité)`, `taire()`, `parle()`, signaux `sous_titre` et `phrase_finie`.

## Architecture

- `crates/evo-son` : toute la synthèse, sans Godot et testable (tests : chaque vue sonne sans saturer, le vent ne souffle qu'en rafales rares, les bruits d'interface finissent, la vie et l'O₂ changent la musique, la musique se repose, la coupure fait taire). Aucun fichier audio ; hasard propre, jamais celui du moteur.
- `crates/evo-godot/src/son.rs` : la classe `EvoSon` (nœud). La synthèse tourne sur son propre fil et garde 80 ms d'avance ; le fil principal recopie seulement ce tampon dans un `AudioStreamGenerator` de 150 ms.
- `crates/evo-godot/src/session/son.rs` : lecture du monde publié et de la chronique, avec un curseur propre au son (la chronique du client garde le sien). L'histoire déjà écrite ne sonne pas au chargement.
- `client/scripts/son/son.gd` (autoload `Son`) : dit à `EvoSon` où est le joueur et ce qu'il ouvre, en observant l'interface sans la modifier. Rien ne sonne sans fenêtre (portes de la CI), sauf avec `-- --avec-son`.

## Mesures

`cargo run --release -p evo-son --example ecoute -- dossier 60` rend six scènes en WAV et mesure le coût :

| Scène | Part d'un cœur |
| --- | --- |
| Globe, archéen | 1,3 % |
| Globe, monde oxygéné et riche | 3,1 % |
| Globe, Terre boule de neige | 2,1 % |
| Loupe | 2,7 % |
| Menu | 1,6 % |

Moyenne 2,1 %, sous le budget de 5 à 10 % d'un cœur fixé par Vision (pris sur les 2 cœurs de l'affichage). Niveau sur le globe : environ −25 dB efficaces au volume par défaut, crête sous −8 dB. Vérifié aussi dans Godot 4.6.1 (partie enregistrée avec `--write-movie`) : globe puis loupe.

## Suite

- Livraison 2, avec l'étape 5 : voix des espèces tirées de l'organe sonore (paramètres demandés au fil Animaux et écosystèmes), scène au sol, cycle jour et nuit, budget de densité sonore par scène, ères animales de la musique, narrateur (voix masculine à choisir, textes par gabarits).
- Livraison 3, avec les étapes 6 et 7 : sociétés.
