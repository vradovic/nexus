use std::{collections::HashSet, fs, path::Path};

use nexus_shared::AppError;

#[derive(Clone, Debug)]
pub struct ProfanityFilter {
    words: HashSet<String>,
}

impl ProfanityFilter {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, AppError> {
        let contents = fs::read_to_string(path)
            .map_err(|_| AppError::internal("failed to read bad words file"))?;

        Ok(Self::from_words(
            contents
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty() && !line.starts_with('#')),
        ))
    }

    pub fn from_words<'a>(words: impl IntoIterator<Item = &'a str>) -> Self {
        Self {
            words: words
                .into_iter()
                .map(|word| word.to_lowercase())
                .collect::<HashSet<_>>(),
        }
    }

    pub fn mask(&self, body: &str) -> String {
        let mut masked = String::with_capacity(body.len());
        let mut current_word = String::new();

        for character in body.chars() {
            if character.is_alphanumeric() || character == '_' {
                current_word.push(character);
                continue;
            }

            self.push_masked_word(&mut masked, &current_word);
            current_word.clear();
            masked.push(character);
        }

        self.push_masked_word(&mut masked, &current_word);
        masked
    }

    fn push_masked_word(&self, masked: &mut String, word: &str) {
        if word.is_empty() {
            return;
        }

        if self.words.contains(&word.to_lowercase()) {
            for _ in word.chars() {
                masked.push('*');
            }
            return;
        }

        masked.push_str(word);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_configured_words_case_insensitively() {
        let filter = ProfanityFilter::from_words(["badword", "curse"]);

        assert_eq!(filter.mask("a BadWord and curse!"), "a ******* and *****!");
    }

    #[test]
    fn does_not_mask_substrings_inside_other_words() {
        let filter = ProfanityFilter::from_words(["bad"]);

        assert_eq!(filter.mask("bad badge"), "*** badge");
    }
}
