//! The Porter stemming algorithm (M.F. Porter, "An algorithm for
//! suffix stripping", 1980) — the reference stemmer behind two
//! decades of IR systems. It reduces inflected words to a common
//! root by stripping English suffixes in five ordered steps, each
//! guarded by the *measure* `m` of the remaining stem (`[C](VC)^m[V]`
//! — the count of vowel–consonant alternations) and the `*v*` / `*d`
//! / `*o` stem conditions from the paper.
//!
//! This is the morphological complement to the phonetic keys
//! (`soundex`) and edit distances (`jaro`, `editdist`): stemming
//! makes "running" and "run" collide at index time.
//!
//! ```
//! use izanagi_kit::porter::stem;
//!
//! assert_eq!(stem(b"running"), b"run".to_vec());
//! assert_eq!(stem(b"relational"), b"relat".to_vec());
//! assert_eq!(stem(b"ponies"), b"poni".to_vec());
//! ```

/// Is `w[i]` a consonant? `y` counts as a consonant when it follows
/// a vowel (Porter's rule), and at word start.
fn is_cons(w: &[u8], i: usize) -> bool {
    match w[i] {
        b'a' | b'e' | b'i' | b'o' | b'u' => false,
        b'y' => i == 0 || !is_cons(w, i - 1),
        _ => true,
    }
}

/// `m(stem)` — the number of `VC` transitions in the consonant/
/// vowel class string.
fn measure(w: &[u8]) -> usize {
    if w.is_empty() {
        return 0;
    }
    let mut m = 0usize;
    // Find first vowel, then count consonant runs that end it.
    let mut in_cons = is_cons(w, 0);
    for i in 1..w.len() {
        let c = is_cons(w, i);
        if !in_cons && c {
            m += 1;
        }
        in_cons = c;
    }
    m
}

/// `*v*` — the stem contains a vowel.
fn has_vowel(w: &[u8]) -> bool {
    (0..w.len()).any(|i| !is_cons(w, i))
}

/// `*d` — the stem ends in a double consonant.
fn ends_double_cons(w: &[u8]) -> bool {
    w.len() >= 2 && w[w.len() - 1] == w[w.len() - 2] && is_cons(w, w.len() - 1)
}

/// `*o` — the stem ends `cvc` where the final consonant is not
/// `w`, `x`, or `y`.
fn ends_cvc(w: &[u8]) -> bool {
    let n = w.len();
    if n < 3 {
        return false;
    }
    if !is_cons(w, n - 3) || is_cons(w, n - 2) || !is_cons(w, n - 1) {
        return false;
    }
    !matches!(w[n - 1], b'w' | b'x' | b'y')
}

fn ends(w: &[u8], suf: &[u8]) -> bool {
    w.len() >= suf.len() && &w[w.len() - suf.len()..] == suf
}

/// Replace `suf` with `rep` if the remaining stem has `m > m_min`.
fn replace_if_m(w: &mut Vec<u8>, suf: &[u8], rep: &[u8], m_min: usize) -> bool {
    if !ends(w, suf) {
        return false;
    }
    let stem = &w[..w.len() - suf.len()];
    if measure(stem) > m_min {
        w.truncate(w.len() - suf.len());
        w.extend_from_slice(rep);
    }
    true // rule matched (even if condition failed → stop step)
}

/// Porter stem of an ASCII word; non-letters return as-is.
/// Input is lower-cased internally.
pub fn stem(word: &[u8]) -> Vec<u8> {
    if !word.iter().all(|c| c.is_ascii_alphabetic()) {
        return word.to_vec();
    }
    let mut w: Vec<u8> = word.iter().map(|c| c.to_ascii_lowercase()).collect();
    if w.len() <= 2 {
        return w;
    }

    // ---- Step 1a: plurals -------------------------------------
    if ends(&w, b"sses") || ends(&w, b"ies") {
        w.truncate(w.len() - 2);
    } else if ends(&w, b"ss") {
        // no-op
    } else if ends(&w, b"s") {
        w.pop();
    }

    // ---- Step 1b: -ed / -ing ----------------------------------
    let mut flag = false;
    if ends(&w, b"eed") {
        if measure(&w[..w.len() - 3]) > 0 {
            w.pop(); // eed → ee
        }
    } else {
        for suf in [&b"ed"[..], &b"ing"[..]] {
            if ends(&w, suf) {
                let stem = &w[..w.len() - suf.len()];
                if has_vowel(stem) {
                    w.truncate(w.len() - suf.len());
                    flag = true;
                }
                break; // longest matching suffix wins (ed/ing same len → first match stops)
            }
        }
        if flag {
            if ends(&w, b"at") || ends(&w, b"bl") || ends(&w, b"iz") {
                w.push(b'e');
            } else if ends_double_cons(&w) && !matches!(w[w.len() - 1], b'l' | b's' | b'z') {
                w.pop();
            } else if measure(&w) == 1 && ends_cvc(&w) {
                w.push(b'e');
            }
        }
    }

    // ---- Step 1c: (*v*) y → i ---------------------------------
    if ends(&w, b"y") && has_vowel(&w[..w.len() - 1]) {
        let n = w.len();
        w[n - 1] = b'i';
    }

    // ---- Step 2: derivational suffixes, m > 0 -----------------
    // Longest-suffix-first rule ordering (Porter's switch list).
    for &(suf, rep) in &[
        (&b"ational"[..], &b"ate"[..]),
        (b"tional", b"tion"),
        (b"enci", b"ence"),
        (b"anci", b"ance"),
        (b"abli", b"able"),
        (b"izer", b"ize"),
        (b"alli", b"al"),
        (b"entli", b"ent"),
        (b"eli", b"e"),
        (b"ousli", b"ous"),
        (b"ization", b"ize"),
        (b"ation", b"ate"),
        (b"ator", b"ate"),
        (b"alism", b"al"),
        (b"iveness", b"ive"),
        (b"fulness", b"ful"),
        (b"ousness", b"ous"),
        (b"aliti", b"al"),
        (b"iviti", b"ive"),
        (b"biliti", b"ble"),
    ] {
        if replace_if_m(&mut w, suf, rep, 0) {
            break;
        }
    }

    // ---- Step 3: more suffixes, m > 0 -------------------------
    for &(suf, rep) in &[
        (&b"icate"[..], &b"ic"[..]),
        (b"ative", b""),
        (b"alize", b"al"),
        (b"iciti", b"ic"),
        (b"ical", b"ic"),
        (b"ful", b""),
        (b"ness", b""),
    ] {
        if replace_if_m(&mut w, suf, rep, 0) {
            break;
        }
    }

    // ---- Step 4: residual suffixes, m > 1 ---------------------
    for &suf in &[
        &b"ement"[..],
        b"ance",
        b"ence",
        b"able",
        b"ible",
        b"ment",
        b"ent",
        b"ant",
        b"ism",
        b"ate",
        b"iti",
        b"ous",
        b"ive",
        b"ize",
        b"ion",
        b"al",
        b"er",
        b"ic",
        b"ou",
    ] {
        if !ends(&w, suf) {
            continue;
        }
        let stem_end = w.len() - suf.len();
        let stem = &w[..stem_end];
        if suf == b"ion" && !(stem.ends_with(b"s") || stem.ends_with(b"t")) {
            break; // -ion only deletes after s/t
        }
        if measure(stem) > 1 {
            w.truncate(stem_end);
        }
        break;
    }

    // ---- Step 5a: final -e ------------------------------------
    if ends(&w, b"e") {
        let stem = &w[..w.len() - 1];
        let m = measure(stem);
        if m > 1 || (m == 1 && !ends_cvc(stem)) {
            w.pop();
        }
    }
    // ---- Step 5b: -ll → -l ------------------------------------
    if measure(&w) > 1 && ends_double_cons(&w) && w[w.len() - 1] == b'l' {
        w.pop();
    }
    w
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paper_vectors_1a_1c() {
        for &(w, want) in &[
            (b"caresses" as &[u8], &b"caress"[..]),
            (b"ponies", b"poni"),
            (b"ties", b"ti"),
            (b"caress", b"caress"),
            (b"cats", b"cat"),
            (b"feed", b"feed"),
            (b"agreed", b"agre"),
            (b"plastered", b"plaster"),
            (b"bled", b"bled"),
            (b"motoring", b"motor"),
            (b"sing", b"sing"),
            (b"conflated", b"conflat"),
            (b"troubled", b"troubl"),
            (b"sized", b"size"),
            (b"hopping", b"hop"),
            (b"tanned", b"tan"),
            (b"falling", b"fall"),
            (b"hissing", b"hiss"),
            (b"fizzed", b"fizz"),
            (b"failing", b"fail"),
            (b"filing", b"file"),
            (b"happy", b"happi"),
            (b"sky", b"sky"),
        ] {
            assert_eq!(stem(w), want, "{:?}", String::from_utf8_lossy(w));
        }
    }

    #[test]
    fn paper_vectors_step2() {
        for &(w, want) in &[
            (b"relational" as &[u8], &b"relat"[..]),
            (b"conditional", b"condit"),
            (b"rational", b"ration"),
            (b"valenci", b"valenc"),
            (b"hesitanci", b"hesit"),
            (b"digitizer", b"digit"),
            (b"conformabli", b"conform"),
            (b"radicalli", b"radic"),
            (b"differentli", b"differ"),
            (b"vileli", b"vile"),
            (b"analogousli", b"analog"),
            (b"vietnamization", b"vietnam"),
            (b"predication", b"predic"),
            (b"operator", b"oper"),
            (b"feudalism", b"feudal"),
            (b"decisiveness", b"decis"),
            (b"hopefulness", b"hope"),
            (b"callousness", b"callous"),
            (b"formaliti", b"formal"),
            (b"sensitiviti", b"sensit"),
            (b"sensibiliti", b"sensibl"),
        ] {
            assert_eq!(stem(w), want, "{:?}", String::from_utf8_lossy(w));
        }
    }

    #[test]
    fn paper_vectors_step3_5() {
        for &(w, want) in &[
            (b"triplicate" as &[u8], &b"triplic"[..]),
            (b"formative", b"form"),
            (b"formalize", b"formal"),
            (b"electriciti", b"electr"),
            (b"electrical", b"electr"),
            (b"hopeful", b"hope"),
            (b"goodness", b"good"),
            (b"revival", b"reviv"),
            (b"allowance", b"allow"),
            (b"inference", b"infer"),
            (b"airliner", b"airlin"),
            (b"gyroscopic", b"gyroscop"),
            (b"adjustable", b"adjust"),
            (b"defensible", b"defens"),
            (b"irritant", b"irrit"),
            (b"replacement", b"replac"),
            (b"adjustment", b"adjust"),
            (b"dependent", b"depend"),
            (b"adoption", b"adopt"),
            (b"communism", b"commun"),
            (b"activate", b"activ"),
            (b"angulariti", b"angular"),
            (b"homologous", b"homolog"),
            (b"effective", b"effect"),
            (b"bowdlerize", b"bowdler"),
            (b"probate", b"probat"),
            (b"rate", b"rate"),
            (b"cease", b"ceas"),
            (b"controll", b"control"),
            (b"roll", b"roll"),
        ] {
            assert_eq!(stem(w), want, "{:?}", String::from_utf8_lossy(w));
        }
    }

    #[test]
    fn edges() {
        assert_eq!(stem(b""), b"".to_vec());
        assert_eq!(stem(b"a"), b"a".to_vec());
        assert_eq!(stem(b"be"), b"be".to_vec());
        // Non-letters pass through untouched.
        assert_eq!(stem(b"o'clock"), b"o'clock".to_vec());
        assert_eq!(stem(b"123"), b"123".to_vec());
        // Case folds.
        assert_eq!(stem(b"RUNNING"), b"run".to_vec());
    }

    #[test]
    fn deterministic_twice() {
        assert_eq!(stem(b"organization"), stem(b"organization"));
    }
}
