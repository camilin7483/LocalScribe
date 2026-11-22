fn words(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split(|character: char| !character.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
}

pub fn rate(reference: &str, hypothesis: &str) -> Option<f64> {
    let reference: Vec<_> = words(reference).collect();
    if reference.is_empty() {
        return None;
    }
    let mut row: Vec<_> = (0..=reference.len()).collect();
    for (index, word) in words(hypothesis).enumerate() {
        let mut diagonal = row[0];
        row[0] = index + 1;
        for (col, expected) in reference.iter().enumerate() {
            let above = row[col + 1];
            row[col + 1] = (diagonal + usize::from(*expected != word))
                .min(above + 1)
                .min(row[col] + 1);
            diagonal = above;
        }
    }
    Some(row[reference.len()] as f64 / reference.len() as f64)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn counts_substitutions_insertions_and_deletions() {
        assert_eq!(rate("Buenos días.", "buenos DÍAS"), Some(0.0));
        assert_eq!(rate("a b c", "a x"), Some(2.0 / 3.0));
        assert_eq!(rate("a", "a b c"), Some(2.0));
        assert_eq!(rate("", "test"), None);
    }
}
