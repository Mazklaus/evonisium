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

Musique (C + B + A) : l'ère microbienne n'a pas encore de mélodie, seulement une nappe grave, des cloches de verre, des bulles et de rares phrases douces (verre doux par défaut ; flûte douce et harpe feutrée au choix, la kalimba du premier jet ayant été refusée).

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

## Peuples (étape 6, sur données d'essai)

Volet « peuples » de la livraison 3 (`evo-son/src/peuples.rs`), lu dans l'état culturel publié par `evo-culture` (format `CulturalState`, version 1), sans rien écrire dans le moteur.

- **Langues parlées.** Les mots de chaque langue (générateur de noms d'`evo-culture`, écrits pour un lecteur français) sont découpés en sons et en syllabes, puis dits par une voix de synthèse à formants : source glottale de Rosenberg, trois résonances du conduit vocal qui glissent d'un son à l'autre, bruit filtré pour les consonnes (occlusives, fricatives, *r* roulé). Une langue qui n'a pas les mêmes sons ne sonne pas pareil, et un changement de son s'entend dans tous ses mots. L'accent tombe sur la première, l'avant-dernière ou la dernière syllabe selon la langue ; la voix descend au fil de la phrase. Hommes, femmes et enfants ont leur hauteur et leur conduit ; la taille du corps de l'espèce les décale.
- **Rumeur.** Au sol, devant un peuple, des gens se parlent dans sa langue : des mots faits de ses syllabes, à quelques pas ou au loin, plus souvent quand le peuple est grand (quatre voix à la fois au plus).
- **Chants rituels.** Le peuple fixe une tradition, la même pour tous ses rites : gamme (le plus souvent sans demi-tons), mesure, façon de chanter à plusieurs (unisson, hétérophonie, bourdon, quartes parallèles), ses instruments (aucun à deux mélodiques parmi flûte d'os, conque, arc musical, pierres sonnantes, guimbarde ; deux à quatre frappés parmi tambour, tambour d'eau, tambour de bois fendu, hochet, mains, bâtons, racleur, bâton de pluie, rhombe) et ses deux manières de célébrer préférées parmi six : appel et réponse, canon à trois groupes, lamentation sur un bourdon fredonné, chant scandé, hoquet (chaque chanteur ne tient que quelques notes d'un motif), transe qui accélère. Les morts appellent souvent la lamentation, la chasse le chant scandé ; une flûte ou des pierres ouvrent le chant seules. L'objet du rite fixe le caractère : lent et grave pour les morts et les ancêtres, vif et frappé pour la chasse, doux et aigu pour la lune, les étoiles et les rêves. Un meneur lance l'appel, le chœur (3 à 9 voix, selon la population) répond sur les syllabes de la langue en commençant par le mot du rite, quatre à six fois ; le tambour entre après le premier appel. La musique du jeu s'efface pendant le chant.
- **Interface pour le client** (`EvoSon`) : `peuple(nom, mots, rites, population, taille, cle)` pour le peuple regardé au sol, `sans_peuple()`, `rite(i)` quand ses agents célèbrent un rite (sinon un chant vient de lui-même toutes les deux à cinq minutes), `prononcer(nom)` pour dire un nom propre dans sa langue.
- **Essai** : `cargo run --release -p evo-son --example peuples -- dossier 3` rend la rumeur, le nom et deux chants de trois peuples du banc d'essai. Coût d'un chant : 4 à 9 % d'un cœur selon la manière (la transe, qui accélère, est la plus chère).

[Simplification] Une voix humaine de synthèse quelle que soit l'espèce ; pas de grammaire ni d'intonation propre à chaque langue au-delà de l'accent de mot. Les espèces qui ne parlent pas par le son auront leurs voix propres avec la livraison 2.

## Fond musical par ère (boucles enregistrées)

Sur demande de l'utilisateur (2026-10-11), le fond de chaque ère n'est plus
seulement généré : des morceaux composés avec Suno, découpés en boucles,
jouent sous les couches générées (cloches, phrases douces, bulles, motifs de la
chronique), qui restent pilotées par l'état de la planète.

- Les boucles sont dans `client/musique/fond/` (Ogg Vorbis, dossier ignoré
  par l'import de Godot), listées dans `boucles.txt` avec leur ère, leur
  tonique, leur mode et leur gain. `EvoSon.fond(dossier)` les décode sur un
  fil à part (`lewton`) au taux du son.
- `evo_son::fond::Fond` joue une boucle de l'ère en cours, reprise pour
  durer deux à quatre minutes, avec fondus de 5 s, puis 20 à 60 s de
  silence où ne restent que les couches générées, puis une autre boucle de
  la même ère. Pendant une boucle, la nappe grave générée se tait et les
  couches générées passent dans la tonalité de la boucle.
- L'ère suit les jalons de la chronique : vie (archéen), montée de
  l'oxygène (grande oxydation), cellule complexe aux étapes 2 à 4, 5 à 7,
  puis 8 et plus (cellules complexes, explosion animale, terres et forêts).
  L'âge des géants et les premiers peuples attendent leurs jalons dans le
  moteur.
- Les boucles restent compressées en mémoire (35 Mo) ; chacune n'est
  décodée, sur un fil à part, qu'au moment où le fond la choisit, pendant le
  silence qui précède (décodées toutes ensemble, elles prendraient plus d'un
  gigaoctet).
- Livrées le 2026-10-11 : 49 boucles de 47 à 80 s tirées des 27 morceaux de
  l'utilisateur (3 à 4 par ère), coupées sur les temps à des endroits qui se
  ressemblent et raccordées par un fondu de 1,5 s, ramenées à environ
  -29 dB efficaces ; tonalité et tempo mesurés (librosa) et notés dans
  `boucles.txt`. L'exemple `cargo run --release -p evo-son --example fond`
  rend le mélange du globe pour chaque ère.
- Sans boucle pour l'ère en cours, la musique reste entièrement générée.
- Droits : ces morceaux viennent du plan gratuit de Suno, pour un usage non
  commercial seulement. Avant toute vente du jeu, il faudra les remplacer
  par des morceaux générés avec un abonnement payant (Pro ou Premier), qui
  ne couvre pas après coup les morceaux du plan gratuit.

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
- Livraison 3, coupée en deux par Vision : le volet peuples avec l'étape 6 (langues et chants, ci-dessus, à brancher sur l'état culturel publié une fois l'étape intégrée au moteur) ; le volet villes avec l'étape 7.
