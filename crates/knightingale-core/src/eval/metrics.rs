//! WER / CER / SER metrics via Levenshtein DP.

/// Levenshtein edit distance between two sequences.
pub fn levenshtein<T: PartialEq>(a: &[T], b: &[T]) -> usize {
    let (n, m) = (a.len(), b.len());
    if n == 0 {
        return m;
    }
    if m == 0 {
        return n;
    }
    let mut prev = (0..=m).collect::<Vec<_>>();
    let mut curr = vec![0usize; m + 1];
    for i in 1..=n {
        curr[0] = i;
        for j in 1..=m {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            curr[j] = (prev[j] + 1).min(curr[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[m]
}

/// Word Error Rate as a fraction (0.0 = perfect).
pub fn wer(reference: &str, hypothesis: &str) -> f64 {
    let r: Vec<&str> = reference.split_whitespace().collect();
    let h: Vec<&str> = hypothesis.split_whitespace().collect();
    if r.is_empty() {
        return if h.is_empty() { 0.0 } else { 1.0 };
    }
    let dist = levenshtein(&r, &h);
    dist as f64 / r.len() as f64
}

/// Character Error Rate.
pub fn cer(reference: &str, hypothesis: &str) -> f64 {
    let r: Vec<char> = reference.chars().collect();
    let h: Vec<char> = hypothesis.chars().collect();
    if r.is_empty() {
        return if h.is_empty() { 0.0 } else { 1.0 };
    }
    let dist = levenshtein(&r, &h);
    dist as f64 / r.len() as f64
}

/// Sentence Error Rate over a slice of (ref, hyp) pairs.
pub fn ser(pairs: &[(&str, &str)]) -> f64 {
    if pairs.is_empty() {
        return 0.0;
    }
    let wrong = pairs.iter().filter(|(r, h)| r.trim() != h.trim()).count();
    wrong as f64 / pairs.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wer_perfect_match() {
        assert_eq!(wer("hello world", "hello world"), 0.0);
    }

    #[test]
    fn wer_one_substitution() {
        // "hello world" vs "hello earth": 1 substitution / 2 ref words.
        assert_eq!(wer("hello world", "hello earth"), 0.5);
    }

    #[test]
    fn cer_substitution() {
        assert_eq!(cer("cat", "bat"), 1.0 / 3.0);
    }
}
