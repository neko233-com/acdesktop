use serde::Serialize;

/// A small numeric summary of one sample series.
#[derive(Debug, Clone, Serialize)]
pub struct Distribution {
    pub min: f64,
    pub median: f64,
    pub max: f64,
    pub stdev: f64,
}

impl Distribution {
    pub(super) fn from_u64(values: &[u64]) -> Option<Self> {
        if values.is_empty() {
            return None;
        }
        let floats: Vec<f64> = values.iter().map(|&value| value as f64).collect();
        Some(Self::from_f64(&floats))
    }

    fn from_f64(values: &[f64]) -> Self {
        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let len = sorted.len();
        let median = if len % 2 == 1 {
            sorted[len / 2]
        } else {
            (sorted[len / 2 - 1] + sorted[len / 2]) / 2.0
        };
        let mean = sorted.iter().sum::<f64>() / len as f64;
        let variance = sorted
            .iter()
            .map(|value| (value - mean).powi(2))
            .sum::<f64>()
            / len as f64;
        Self {
            min: sorted[0],
            median,
            max: sorted[len - 1],
            stdev: variance.sqrt(),
        }
    }
}
