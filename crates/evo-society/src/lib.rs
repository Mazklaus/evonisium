//! Sociétés et histoire (étape 7).
//!
//! Une espèce passée en mode historique (étape 6) y va des bandes et des
//! tribus jusqu'à un équivalent du XXIe siècle : agriculture et
//! domestication, institutions, économie, écriture, sciences, guerre et
//! effets des sociétés sur la planète. Aucun âge n'est écrit d'avance : une
//! société peut rester nomade, revenir en arrière ou suivre une autre voie.
//!
//! Comme `evo-culture`, la crate ne dépend pas du monde : le moteur lui
//! donnera le milieu de chaque cellule (Planète), les espèces sauvages
//! (populations) et les cultures (`evo-culture`) ; elle lui rendra les flux
//! vers la planète, les pressions de sélection de la domestication et
//! l'état des sociétés publié. Sur banc d'essai, `bench` écrit le milieu à
//! la main.

pub mod bench;
pub mod catalog;
pub mod chronicle;
pub mod disease;
pub mod economy;
pub mod goods;
pub mod land;
pub mod science;
pub mod society;
pub mod state;
pub mod world;
