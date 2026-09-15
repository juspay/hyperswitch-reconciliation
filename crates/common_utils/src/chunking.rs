pub fn chunked<I>(iter: I, chunk_size: usize) -> impl Iterator<Item = Vec<I::Item>>
where
    I: IntoIterator,
{
    let mut iter = iter.into_iter().peekable();
    std::iter::from_fn(move || {
        iter.peek()
            .is_some()
            .then(|| iter.by_ref().take(chunk_size).collect())
    })
}

#[cfg(test)]
mod tests {
    use super::chunked;

    #[test]
    fn empty_yields_no_chunks() {
        assert!(chunked(Vec::<u32>::new(), 100).next().is_none());
    }

    #[test]
    fn under_size_returns_single_chunk() {
        let chunks: Vec<Vec<u32>> = chunked(0..50, 100).collect();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks.first().map(Vec::len), Some(50));
    }

    #[test]
    fn over_size_preserves_all_entries() {
        let chunks: Vec<Vec<u32>> = chunked(0..8500, 1000).collect();
        assert_eq!(chunks.len(), 9);
        assert!(chunks.iter().all(|c| c.len() <= 1000));
        let merged: Vec<u32> = chunks.into_iter().flatten().collect();
        assert_eq!(merged, (0..8500).collect::<Vec<_>>());
    }

    #[test]
    fn works_with_hashmap() {
        use rustc_hash::FxHashMap;
        let input: FxHashMap<String, usize> = (0..2500).map(|i| (format!("k{i}"), i)).collect();
        let chunks: Vec<FxHashMap<String, usize>> = chunked(input.clone(), 1000)
            .map(|chunk| chunk.into_iter().collect())
            .collect();
        assert_eq!(chunks.len(), 3);
        let merged: FxHashMap<String, usize> = chunks.into_iter().flatten().collect();
        assert_eq!(merged, input);
    }
}
