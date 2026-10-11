# Étape 6, volet cognition et cultures

Feuille de route (document Vision, « Périmètre consolidé de l'étape 6 ») : une crate `evo-culture` (traits culturels, modes de transmission et biais, critère de bascule, culture à l'équilibre, langues par jeu de signalisation simplifié, catalogue des techniques et voies de remplacement, générateur de mythes et de noms), l'apprentissage des agents dans `evo-agents` pour caler le modèle agrégé, le format de l'état culturel publié et la demande des quatre lectures cognitives au fil « Animaux et écosystèmes ». Le tout sur bancs d'essai, sans toucher `evo-life`, `evo-sim`, `evo-planet` ni `evo-genetics`.

**Verdict : les volets de la porte qui se jouent sur banc d'essai sont tenus.** Il reste ceux qui demandent l'intégration au moteur (Terre et monde de la vague 1, date de la bascule sur la Terre, vitesse moteur compris), à faire avec le fil « Animaux et écosystèmes » après la porte de l'étape 5.

| Critère de la porte | Résultat | Preuve |
|---|---|---|
| Une technique transmise au moins 10 générations par apprentissage social, sans être codée, sur deux mondes ; des traditions dans plusieurs lignées | oui sur banc : chimpanzés et corbeaux sur une terre, dauphins dans une mer ; les techniques sont inventées par le modèle (jamais posées) et durent des milliers de générations ; elles ne s'apprennent pas seul (maîtrise individuelle sous le seuil) | `evo-culture/tests/cas.rs`, `traditions_in_several_lineages` |
| Bascule : gain culturel positif deux pas de suite, puis mode historique | oui : une espèce à langage bascule au deuxième pas positif, les chimpanzés jamais | `switch_to_historical_mode` |
| Cas tasmanien | oui : une île de 4 000 personnes coupée du continent perd outils d'os, vêtements cousus, pêche et propulseurs, garde le feu, la taille, l'épieu ; le continent garde tout ; l'île devient un peuple et une langue filles | `tasmanian_loss` |
| Cas de Koshima | oui : moitié de la troupe en 7 ans (médiane de 200 parties), palier 0,88 (observé : environ les trois quarts de la troupe vers 1962) | `koshima_diffusion` |
| Le modèle agrégé suit les agents détaillés | oui : diffusion (troupe d'agents qui observent contre modèle, 300 répliques, écart sous 4 erreurs types), savoir-faire de Henrich (−0,48 contre −0,53 par génération à 4 000 ; 1,43 contre 1,42 à 28 000) | `evo-agents/tests/apprentissage.rs` |
| Une intervention vue par un peuple fait naître un mythe | oui ; une espèce sans pensée symbolique la note seulement inexpliquée | `intervention_gives_a_myth` |
| Rejeu identique, notables compris | oui : même état publié (et même empreinte) que l'état soit lu ou non, sur un cœur ou plusieurs | `replay_is_identical` |
| Vitesse en mode historique, une espèce historique sur 2 000 cellules | 88 années par seconde sur 4 fils ; partie parallèle ≈ 32 ms par an sur un cœur, partie séquentielle ≈ 3 ms, d'où environ 120 a/s attendus sur 6 cœurs (à mesurer sur la machine cible avec l'intégration) | `cargo run --release -p evo-culture --example vitesse` |

## Comment c'est fait

**Lectures cognitives** (`cognition.rs`). `Cognition` reçoit les quatre lectures du plan de construction : huit capacités entre 0 et 1 et le budget cérébral qui les borne, l'organe de manipulation (main, bouche, bec, trompe, tentacule) et sa qualité, le débit de cinq canaux de communication, la période juvénile ; plus le temps de génération et la socialité, déjà connus du moteur. `Learning::of` en tire le mode d'apprentissage social (observation, imitation, enseignement, langage), la fidélité de copie, les occasions d'apprendre par an, ce qu'un individu trouve seul, l'inventivité, la pensée symbolique, le conformisme et l'expressivité de la langue. Ces grandeurs servent aux agents comme au modèle agrégé : c'est ce partage qui permet de caler l'un sur l'autre.

**Gain culturel et bascule.** Le savoir-faire suit Henrich (2004) : chacun copie le plus habile avec une perte α = difficulté × (1 − fidélité) et un bruit de Gumbel β, d'où un gain moyen β(γ + ln n) − α par génération pour n individus connectés (le groupe et ses voisins, au poids de leurs liens). Le gain culturel d'une espèce est ce gain sur une difficulté de référence (150) dans son meilleur groupe : positif, la culture est cumulative. Il faut alors un langage et une bande d'une cinquantaine d'individus connectés ; l'enseignement sans langage demanderait des dizaines de milliers d'individus. Deux pas positifs de suite font passer l'espèce en mode historique (règle de Vision) ; c'est l'état du monde qui le décide.

**Groupe culturel** (`world.rs`). Un groupe par population de cellule d'une espèce à apprentissage social, avec ses traits (64 au plus, en creux) : fréquence, maîtrise pour une technique, date d'arrivée. Deux régimes :

- à l'équilibre, quand le pas dépasse 100 ans : un trait se maintient si λ (apprenants × fidélité × attrait) dépasse la perte μ (une génération), à la fréquence 1 − μ/λ, et une technique si son gain de savoir-faire est positif ou qu'on la trouve seul ; les techniques gagnent toute la composante reliée, les traits neutres restent à leur groupe et dérivent (remplacés en 2·N générations en moyenne) ;
- pas à pas, au pas d'un an : diffusion logistique résolue exactement, conformisme, tirage des petits effectifs, maîtrise de Henrich, passage entre voisins, inventions. Les groupes sont calculés en parallèle sur l'état d'avant le pas.

Les traits générés (croyances, rites, arts, tabous, mythes) se font concurrence dans des niches (genre et objet) : un nouveau rite des morts doit déloger l'ancien. Leurs effets sont bornés à ±0,1 sur la cohésion, la coopération et la légitimité.

**Catalogue des techniques** (`catalog.rs`). Seize familles, du bâton à sonder au filet et à l'hameçon, avec prérequis, difficulté, exigences du corps (organe, causalité physique, planification, pensée symbolique) et voies de remplacement : aquatique (éponge protectrice, coquille aiguisée, cuisson aux sources chaudes, harpon), souterraine (galerie aménagée, foyer de grotte), sans mains (brindille tenue au bec, noix lâchées sur la pierre) et pauvre en oxygène (fermentation en fosse à la place du feu sous 16 % d'oxygène). Une voie absente veut dire « des mains ou rien » (taille de la pierre, propulseur, couture).

**Langues** (`language.rs`). Un lexique de 64 sens par groupe, une forme (un nombre) par sens, qui joue le rôle de convention ; chaque sens se renouvelle au rythme de Swadesh (86 % gardés en mille ans) et s'aligne sur ses voisins (une fois sur deux sur leur majorité, sinon sur l'un d'eux). La phonologie (inventaire de sons, syllabes) appartient à la langue ; un changement de son touche tous les mots à la fois. Le premier sens est le nom que le peuple se donne, le deuxième celui de sa langue. Côté agents, `Signalling` joue le jeu de Lewis par renforcement : la population converge vers une convention partagée (réussite 1,0), et une espèce moins expressive ne dit qu'une part des situations.

**Peuples et langues** (`labels.rs`). Étiquettes émergentes : un groupe reste dans son étiquette tant qu'il est relié au cœur (le groupe le plus peuplé) par des groupes assez proches de ce cœur (moitié du vocabulaire de base pour une langue ; moitié culture, moitié langue, sous 0,6, pour un peuple) ; sinon il rejoint l'étiquette voisine la plus proche, ou forme avec ses voisins détachés une étiquette fille. L'arbre des langues et l'arbre des cultures en sortent.

**Mythes** (`myths.rs`). Un phénomène vu dans une cellule (intervention du joueur ou catastrophe) est noté inexpliqué ; il fait naître un mythe avec une probabilité qui croît avec la pensée symbolique (rien sous 0,25). Le mythe a une figure nommée dans la langue du peuple, un thème (colère, don, châtiment, origine, gardien), un court récit par gabarit, parfois un tabou ; il devient un trait du groupe et se transmet comme les autres.

**Notables** (`world.rs`). Pour une espèce en mode historique, le moteur tire un inventeur à chaque invention et un passeur quand une technique entre chez un autre peuple, avec le hasard de l'état simulé, quelle que soit la caméra. L'inventeur empêche son invention de se perdre de son vivant ; le passeur double sa transmission dans le groupe qui la reçoit. Au plus 2 000 vivants.

**Agents** (`evo-agents/src/learning.rs`). `Band` (une troupe qui apprend par observation), `SkillBand` (Henrich individu par individu), `Signalling` (jeu de Lewis), et `CultureHint`, qui donne aux agents de la scène les quatre actions culturelles du lexique (fabriquer un outil, enseigner, échanger, célébrer un rite) selon l'état culturel de leur groupe, pour l'affichage. Les quatre actions sont ajoutées au lexique d'`evo-morph`, avec des postures provisoires que le palier 3 du client reprendra.

## État culturel publié

`evo_culture::CulturalState` (`state.rs`, version 1), tiré de l'état à chaque pas, lu seul, en identifiants stables :

- `species` : mode d'apprentissage, fidélité, gain culturel, entrée en mode historique, nombre de groupes ;
- `groups` : par cellule et espèce, peuple, langue, effectif, cohésion, et les traits avec fréquence, maîtrise et générations de transmission ;
- `traits` : registre des traits portés (genre, nom, effets, origine pour les traits générés ; les techniques portent le nom de leur voie la plus portée) ;
- `peoples` et `languages` : tous, vivants et éteints, avec leur parent (la généalogie), leur nom, et quelques mots pour les langues ;
- `notables` : vivants et 256 derniers morts ;
- `myths` : titre, figure, thème, récit, phénomène, intervention ou non ;
- `unexplained` : phénomènes vus et restés inexpliqués.

`CulturalState::fingerprint` sert aux tests de rejeu.

## Simplifications et limites

- **Lectures cognitives de banc d'essai.** Les espèces des tests (macaque, chimpanzé, humain, corbeau, dauphin, espèce à tentacules) ont des lectures écrites à la main (`cognition::presets`). Le fil « Animaux et écosystèmes » calculera les vraies dans le constructeur de phénotypes.
- **Constantes d'ordre de grandeur.** Fidélités par mode, taux d'invention (10⁻⁷ par individu connecté et par an), difficultés des techniques : calées sur Koshima, la Tasmanie et le contraste chimpanzé-humain, pas mesurées.
- **Langue réduite à un lexique.** Pas de grammaire ; les mots d'une espèce qui ne parle pas par le son sont une transcription. Deux étiquettes (peuples, langues) ne fusionnent jamais.
- **Traits neutres locaux à l'équilibre.** Avant la bascule, croyances, rites et arts restent au groupe qui les invente et y dérivent ; seules les techniques se partagent dans une région reliée.
- **Vitesse mesurée sur 4 fils**, pas sur la machine cible ; l'écologie et la démographie au pas de dix ans s'y ajouteront avec l'intégration.
