//! Fuzzy search in the suggestions and the palette.

use nucleo_matcher::{
    Config, Matcher, Utf32Str,
    pattern::{CaseMatching, Normalization, Pattern},
};

/// The index of each name that fuzzy matches `query`, the best match first. Names that
/// match as well keep their order. A match has the letters of the query in order, with
/// gaps: `stc` matches `struct_class`.
pub fn ranked(query: &str, names: &[&str]) -> Vec<usize> {
    let pattern = Pattern::parse(query, CaseMatching::Smart, Normalization::Smart);
    let mut matcher = Matcher::new(Config::DEFAULT);
    let mut buf = Vec::new();
    let mut hits: Vec<(usize, u32)> = names
        .iter()
        .enumerate()
        .filter_map(|(i, n)| Some((i, pattern.score(Utf32Str::new(n, &mut buf), &mut matcher)?)))
        .collect();
    hits.sort_by_key(|&(i, score)| (std::cmp::Reverse(score), i));
    hits.into_iter().map(|(i, _)| i).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn best_first() {
        let names = ["report returns", "run", "report scaling"];
        assert_eq!(ranked("run", &names)[0], 1);
        assert_eq!(ranked("", &names), [0, 1, 2]);
    }
}
