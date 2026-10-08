//! Mise en forme des nombres et des dates, en français et en anglais
//! (document DA : dates en Ga et Ma, puissances de dix en exposant, chiffres
//! tabulaires).

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Lang {
    #[default]
    Fr,
    En,
}

impl Lang {
    pub fn from_code(code: &str) -> Self {
        if code.starts_with("en") {
            Lang::En
        } else {
            Lang::Fr
        }
    }
}

const SUPERSCRIPT: [char; 10] = ['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹'];

fn superscript(n: i32) -> String {
    let mut s = String::new();
    if n < 0 {
        s.push('⁻');
    }
    for ch in n.unsigned_abs().to_string().chars() {
        s.push(SUPERSCRIPT[ch.to_digit(10).unwrap_or(0) as usize]);
    }
    s
}

/// Nombre avec `decimals` décimales, virgule décimale et espace fine
/// insécable entre milliers (usage français).
pub fn number(x: f64, decimals: usize) -> String {
    number_in(Lang::Fr, x, decimals)
}

pub fn number_in(lang: Lang, x: f64, decimals: usize) -> String {
    if !x.is_finite() {
        return "—".into();
    }
    let s = format!("{:.*}", decimals, x.abs());
    let (int, frac) = s.split_once('.').map_or((s.as_str(), None), |(a, b)| (a, Some(b)));
    let sep = if lang == Lang::Fr { '\u{202F}' } else { ',' };
    let mut grouped = String::new();
    for (i, ch) in int.chars().enumerate() {
        if i > 0 && (int.len() - i) % 3 == 0 {
            grouped.push(sep);
        }
        grouped.push(ch);
    }
    let mut out = String::new();
    // « −0 » n'a pas de sens à l'affichage.
    if x < 0.0 && s.chars().any(|c| c.is_ascii_digit() && c != '0') {
        out.push('−');
    }
    out.push_str(&grouped);
    if let Some(f) = frac {
        out.push(if lang == Lang::Fr { ',' } else { '.' });
        out.push_str(f);
    }
    out
}

/// Écriture scientifique avec l'exposant en exposant : « 1,2 × 10⁻⁴ ».
pub fn power_of_ten(x: f64) -> String {
    power_of_ten_in(Lang::Fr, x)
}

pub fn power_of_ten_in(lang: Lang, x: f64) -> String {
    if x == 0.0 {
        return "0".into();
    }
    if !x.is_finite() {
        return "—".into();
    }
    let e = x.abs().log10().floor() as i32;
    if (-2..=3).contains(&e) {
        let decimals = if e >= 1 { 0 } else { (1 - e) as usize };
        return number_in(lang, x, decimals);
    }
    let m = x / 10f64.powi(e);
    // Arrondi qui déborde (9,96 → 10,0).
    let (m, e) = if (m.abs() * 10.0).round() >= 100.0 { (m / 10.0, e + 1) } else { (m, e) };
    let mant = number_in(lang, m, 1);
    let mant = mant.strip_suffix(",0").or_else(|| mant.strip_suffix(".0")).unwrap_or(&mant).to_string();
    if mant == "1" {
        format!("10{}", superscript(e))
    } else if mant == "−1" {
        format!("−10{}", superscript(e))
    } else {
        format!("{mant} × 10{}", superscript(e))
    }
}

/// Durée en années, avec l'unité adaptée : « 4,12 Ga », « 350 Ma », « 12,5 ka ».
pub fn duration(years: f64, lang: Lang) -> String {
    let a = years.abs();
    let (v, unit) = if a >= 1e9 {
        (years / 1e9, "Ga")
    } else if a >= 1e6 {
        (years / 1e6, "Ma")
    } else if a >= 1e3 {
        (years / 1e3, "ka")
    } else {
        return match lang {
            Lang::Fr => format!("{} an{}", number_in(lang, years, 0), if a >= 2.0 { "s" } else { "" }),
            Lang::En => format!("{} yr", number_in(lang, years, 0)),
        };
    };
    let decimals = if v.abs() >= 100.0 {
        0
    } else if v.abs() >= 10.0 {
        1
    } else {
        2
    };
    format!("{} {unit}", number_in(lang, v, decimals))
}

/// Date de jeu : temps écoulé depuis la formation de la partie.
pub fn game_date(years: f64, lang: Lang) -> String {
    match lang {
        Lang::Fr => format!("an {}", duration(years, lang)),
        Lang::En => format!("year {}", duration(years, lang)),
    }
}

/// Vitesse du temps : « 100 ka/s ».
pub fn speed(years_per_second: f64, lang: Lang) -> String {
    let d = duration(years_per_second, lang);
    // « 1,00 Ma » → « 1 Ma » : les crans nommés sont des nombres ronds.
    let d = d.replace(",00 ", " ").replace(".00 ", " ").replace(",0 ", " ").replace(".0 ", " ");
    format!("{d}/s")
}

/// Température, en °C ou en K selon le réglage.
pub fn temperature(k: f64, celsius: bool, lang: Lang) -> String {
    if celsius {
        format!("{} °C", number_in(lang, k - 273.15, 1))
    } else {
        format!("{} K", number_in(lang, k, 1))
    }
}

/// Fraction en pour cent.
pub fn percent(f: f64, lang: Lang) -> String {
    let p = f * 100.0;
    let decimals = if p.abs() >= 10.0 {
        0
    } else if p.abs() >= 1.0 {
        1
    } else {
        2
    };
    format!("{} %", number_in(lang, p, decimals))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn french_numbers() {
        assert_eq!(number(1234567.891, 2), "1\u{202F}234\u{202F}567,89");
        assert_eq!(number(-0.0001, 1), "0,0");
        assert_eq!(number(-12.0, 0), "−12");
        assert_eq!(number_in(Lang::En, 1234.5, 1), "1,234.5");
    }

    #[test]
    fn powers_of_ten() {
        assert_eq!(power_of_ten(1.2e-4), "1,2 × 10⁻⁴");
        assert_eq!(power_of_ten(1e12), "10¹²");
        assert_eq!(power_of_ten(9.96e6), "10⁷");
        assert_eq!(power_of_ten(250.0), "250");
        assert_eq!(power_of_ten(0.5), "0,50");
    }

    #[test]
    fn dates() {
        assert_eq!(duration(4.12e9, Lang::Fr), "4,12 Ga");
        assert_eq!(duration(350e6, Lang::Fr), "350 Ma");
        assert_eq!(duration(12_500.0, Lang::Fr), "12,5 ka");
        assert_eq!(duration(800.0, Lang::Fr), "800 ans");
        assert_eq!(speed(1e6, Lang::Fr), "1 Ma/s");
        assert_eq!(speed(2.5e5, Lang::Fr), "250 ka/s");
        assert_eq!(game_date(2.5e7, Lang::En), "year 25.0 Ma");
        assert_eq!(temperature(288.15, true, Lang::Fr), "15,0 °C");
        assert_eq!(percent(0.0021, Lang::Fr), "0,21 %");
    }
}
