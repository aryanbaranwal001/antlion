//! Search in the output of a run, and fuzzy search in the lists.

use nucleo_matcher::{
    Config, Matcher, Utf32Str,
    pattern::{CaseMatching, Normalization, Pattern},
};

use super::runner::Line;

/// The index of each name that fuzzy matches `query`, in the order of `names`. A match
/// has the letters of the query in order, with gaps: `stc` matches `struct_class`.
pub fn fuzzy(query: &str, names: &[&str]) -> Vec<usize> {
    let pattern = Pattern::parse(query, CaseMatching::Smart, Normalization::Smart);
    let mut matcher = Matcher::new(Config::DEFAULT);
    let mut buf = Vec::new();
    names
        .iter()
        .enumerate()
        .filter(|(_, n)| {
            pattern
                .score(Utf32Str::new(n, &mut buf), &mut matcher)
                .is_some()
        })
        .map(|(i, _)| i)
        .collect()
}

/// Whether `text` holds `query`. Case is ignored unless the query has a capital letter,
/// as with `smartcase` in vim.
pub fn holds(text: &str, query: &str) -> bool {
    if query.chars().any(char::is_uppercase) {
        text.contains(query)
    } else {
        text.to_lowercase().contains(&query.to_lowercase())
    }
}

/// One search in the output: the lines that match, and the match the user is on.
pub struct Search {
    pub query: String,
    /// The index of each line that matches, in order.
    pub hits: Vec<usize>,
    /// The match the user is on, in `hits`.
    pub at: usize,
    /// How many lines were searched, so new output is searched once.
    scanned: usize,
}

impl Search {
    pub fn new(query: String) -> Self {
        Search {
            query,
            hits: Vec::new(),
            at: 0,
            scanned: 0,
        }
    }

    /// Search the lines that came since the last call.
    pub fn update(&mut self, lines: &[Line]) {
        for (i, l) in lines.iter().enumerate().skip(self.scanned) {
            if holds(&l.text, &self.query) {
                self.hits.push(i);
            }
        }
        self.scanned = lines.len();
    }

    /// Move to the next match, or the one before when `step` is negative, going round.
    /// The line of the new match, if there is any.
    pub fn step(&mut self, step: isize) -> Option<usize> {
        if self.hits.is_empty() {
            return None;
        }
        let n = self.hits.len() as isize;
        self.at = (self.at as isize + step).rem_euclid(n) as usize;
        Some(self.hits[self.at])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(texts: &[&str]) -> Vec<Line> {
        texts
            .iter()
            .map(|t| Line {
                text: t.to_string(),
                err: false,
            })
            .collect()
    }

    #[test]
    fn fuzzy_names() {
        let names = [
            "scalar_store",
            "storage",
            "struct_class",
            "struct_store",
            "ttl",
        ];
        assert_eq!(fuzzy("stc", &names), [2, 3]);
        assert_eq!(fuzzy("", &names), [0, 1, 2, 3, 4]);
        assert_eq!(fuzzy("zzz", &names), Vec::<usize>::new());
    }

    #[test]
    fn smart_case() {
        assert!(holds("DIFFER", "differ"));
        assert!(holds("differ", "differ"));
        assert!(holds("DIFFER", "DIFFER"));
        assert!(!holds("differ", "Differ"));
    }

    #[test]
    fn hits_and_steps() {
        let mut all = lines(&["a cost", "b", "c cost"]);
        let mut s = Search::new("cost".to_string());
        s.update(&all);
        assert_eq!(s.hits, [0, 2]);
        all.extend(lines(&["d", "cost again"]));
        s.update(&all);
        assert_eq!(s.hits, [0, 2, 4]);
        assert_eq!(s.step(1), Some(2));
        assert_eq!(s.step(1), Some(4));
        assert_eq!(s.step(1), Some(0));
        assert_eq!(s.step(-1), Some(4));
        assert_eq!(Search::new("x".into()).step(1), None);
    }
}
