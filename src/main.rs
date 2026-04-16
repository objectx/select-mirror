fn find_best(results: &[(String, Option<f64>)]) -> Option<&str> {
    results
        .iter()
        .filter_map(|(mirror, elapsed)| elapsed.map(|t| (mirror.as_str(), t)))
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(mirror, _)| mirror)
}

fn main() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_best_returns_fastest() {
        let results = vec![
            ("http://a.example.com".to_string(), Some(1.5)),
            ("http://b.example.com".to_string(), Some(0.3)),
            ("http://c.example.com".to_string(), Some(2.1)),
        ];
        assert_eq!(find_best(&results), Some("http://b.example.com"));
    }

    #[test]
    fn find_best_skips_failed_mirrors() {
        let results = vec![
            ("http://a.example.com".to_string(), None),
            ("http://b.example.com".to_string(), Some(0.5)),
        ];
        assert_eq!(find_best(&results), Some("http://b.example.com"));
    }

    #[test]
    fn find_best_returns_none_when_all_failed() {
        let results = vec![
            ("http://a.example.com".to_string(), None),
            ("http://b.example.com".to_string(), None),
        ];
        assert_eq!(find_best(&results), None);
    }

    #[test]
    fn find_best_returns_none_for_empty_input() {
        let results: Vec<(String, Option<f64>)> = vec![];
        assert_eq!(find_best(&results), None);
    }
}
