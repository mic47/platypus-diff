pub trait AlignmentScoring<T> {
    fn insert_score(&self, inserted: &T, previous_is_same: bool) -> f64;
    fn mutation_score(&self, left: &T, right: &T) -> f64;
}

pub trait Token {
    fn text(&self) -> &str;
    fn start(&self) -> usize;
    fn is_whitespace(&self) -> bool;
}
