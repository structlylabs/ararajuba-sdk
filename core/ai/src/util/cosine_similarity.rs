//! Cosine similarity between embedding vectors.

/// Calculate the cosine similarity between two embedding vectors.
///
/// Returns a value between -1.0 and 1.0, where:
/// - 1.0 means the vectors are identical in direction
/// - 0.0 means the vectors are orthogonal
/// - -1.0 means the vectors are opposite
///
/// Returns `None` if either vector is empty or they have different lengths.
///
/// # Example
/// ```
/// use ararajuba_core::util::cosine_similarity::cosine_similarity;
///
/// let a = vec![1.0, 0.0, 0.0];
/// let b = vec![1.0, 0.0, 0.0];
/// assert!((cosine_similarity(&a, &b).unwrap() - 1.0).abs() < 1e-6);
/// ```
pub fn cosine_similarity(a: &[f64], b: &[f64]) -> Option<f64> {
    if a.is_empty() || b.is_empty() || a.len() != b.len() {
        return None;
    }

    let mut dot = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;

    for (ai, bi) in a.iter().zip(b.iter()) {
        dot += ai * bi;
        norm_a += ai * ai;
        norm_b += bi * bi;
    }

    let denom = norm_a.sqrt() * norm_b.sqrt();
    if denom == 0.0 {
        return None;
    }

    Some(dot / denom)
}

/// Cosine similarity for f32 vectors.
pub fn cosine_similarity_f32(a: &[f32], b: &[f32]) -> Option<f32> {
    if a.is_empty() || b.is_empty() || a.len() != b.len() {
        return None;
    }

    let mut dot: f32 = 0.0;
    let mut norm_a: f32 = 0.0;
    let mut norm_b: f32 = 0.0;

    for (ai, bi) in a.iter().zip(b.iter()) {
        dot += ai * bi;
        norm_a += ai * ai;
        norm_b += bi * bi;
    }

    let denom = norm_a.sqrt() * norm_b.sqrt();
    if denom == 0.0 {
        return None;
    }

    Some(dot / denom)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identical_vectors() {
        let a = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&a, &a).unwrap();
        assert!((sim - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_orthogonal_vectors() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let sim = cosine_similarity(&a, &b).unwrap();
        assert!(sim.abs() < 1e-10);
    }

    #[test]
    fn test_opposite_vectors() {
        let a = vec![1.0, 0.0];
        let b = vec![-1.0, 0.0];
        let sim = cosine_similarity(&a, &b).unwrap();
        assert!((sim - (-1.0)).abs() < 1e-10);
    }

    #[test]
    fn test_different_lengths() {
        let a = vec![1.0, 2.0];
        let b = vec![1.0, 2.0, 3.0];
        assert!(cosine_similarity(&a, &b).is_none());
    }

    #[test]
    fn test_empty_vectors() {
        let a: Vec<f64> = vec![];
        let b: Vec<f64> = vec![];
        assert!(cosine_similarity(&a, &b).is_none());
    }

    #[test]
    fn test_zero_vector() {
        let a = vec![0.0, 0.0];
        let b = vec![1.0, 2.0];
        assert!(cosine_similarity(&a, &b).is_none());
    }

    #[test]
    fn test_f32_identical() {
        let a: Vec<f32> = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity_f32(&a, &a).unwrap();
        assert!((sim - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_known_similarity() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![4.0, 5.0, 6.0];
        let sim = cosine_similarity(&a, &b).unwrap();
        // Expected: (4+10+18) / (sqrt(14) * sqrt(77)) ≈ 0.9746
        assert!((sim - 0.9746).abs() < 0.001);
    }
}
